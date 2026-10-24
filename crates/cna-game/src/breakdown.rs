//! Breakdown (SPI §21) with the `breakdown` module on: every motorised
//! unit collects the breakdown points of each step it takes, in any
//! kind of move, until the OpStage ends (21.2x).

use crate::data::Data;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_rules::movement::Step;

impl GameState {
    /// The step from `from` to `to` adds its breakdown points to each
    /// motorised piece in `pieces` (21.2x). Supply units' motorisation
    /// points never break down (32.5x).
    pub(crate) fn collect_breakdown(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        from: &HexId,
        to: &HexId,
    ) {
        if !self.modules.breakdown {
            return;
        }
        let (Some(f), Some(t)) = (data.map.hex(from), data.map.hex(to)) else {
            return;
        };
        let hexside = data.map.hexside(from, to);
        let step = Step {
            from: f,
            to: t,
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(from, to)),
            weather: self.weather_at(to),
        };
        let halves = (cna_rules::breakdown::step_points(&data.terrain, &step) * 2.0).round() as u32;
        for id in pieces {
            let p = &self.pieces[id];
            if matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                && self.mobility(data, p).motorised()
            {
                self.pieces.get_mut(id).expect("listed").breakdown_halves += halves;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::order::Order;
    use crate::piece::PieceKind;
    use crate::state::{GameState, Modules};
    use cna_data::scenario::Side;

    /// The Axis's first movement segment of a new module-on game, or
    /// with `modules`.
    fn movement(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        while state.clock.segment != Some(crate::clock::Segment::Movement) {
            let side = state.to_act().unwrap();
            state.apply(data, side, Order::Pass).unwrap();
        }
        state
    }

    #[test]
    fn a_motorised_unit_collects_the_breakdown_points_of_its_steps_21_2x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let base = movement(&data, Modules::ALL);
        // Some motorised Axis unit's first legal step, priced by the rules
        // core.
        let mut found = false;
        for p in base.pieces.values() {
            let Some(at) = p.hex() else { continue };
            if p.side != Side::Axis
                || p.kind != PieceKind::Combat
                || !base.mobility(&data, p).motorised()
            {
                continue;
            }
            for n in data.map.neighbours(at) {
                let mut s = base.clone();
                let order = Order::Move {
                    pieces: vec![p.id.clone()],
                    path: vec![n.clone()],
                    assault: false,
                    fuel_from: None,
                };
                if s.apply(&data, Side::Axis, order.clone()).is_err() {
                    continue;
                }
                let hexside = data.map.hexside(at, &n);
                let step = cna_rules::movement::Step {
                    from: data.map.hex(at).unwrap(),
                    to: data.map.hex(&n).unwrap(),
                    hexside,
                    climbs: hexside.is_some_and(|hs| hs.climbs(at, &n)),
                    weather: base.weather_at(&n),
                };
                let points = cna_rules::breakdown::step_points(&data.terrain, &step);
                assert!(points > 0.0);
                assert_eq!(f64::from(s.pieces[&p.id].breakdown_halves), points * 2.0);
                // Without the module nothing is collected.
                let mut off = base.clone();
                off.modules.breakdown = false;
                off.apply(&data, Side::Axis, order).unwrap();
                assert_eq!(off.pieces[&p.id].breakdown_halves, 0);
                found = true;
                break;
            }
            if found {
                break;
            }
        }
        assert!(found, "no motorised Axis unit could step anywhere");
    }
}
