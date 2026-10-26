//! Reinforcements (SPI 20.1x, 4.42): in phase D of the OpStage the
//! schedule names, a listed unit lands with its sheet subtree, the
//! Commonwealth at Cairo and the Axis in the Tripoli box, paying nothing
//! to land or for its first hex. Stacking does not bind an arrival
//! (only returning withdrawn units, out of scope). The Benghazi
//! diversion (55.1) waits for the Logistics Game.

use crate::data::Data;
use crate::event::Event;
use crate::piece::Location;
use crate::setup;
use crate::state::GameState;
use cna_data::scenario::{OffMapBox, Side};
use cna_data::time::GameTime;

impl GameState {
    /// Phase D: every arrival the schedule names for this OpStage. True
    /// when anything landed.
    pub(crate) fn arrive(&mut self, data: &Data, events: &mut Vec<Event>) -> bool {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        let mut any = false;
        for side in [Side::Axis, Side::Cw] {
            let at = match side {
                Side::Cw => Location::Hex(
                    data.map
                        .place("cairo")
                        .expect("places.json has Cairo")
                        .hex
                        .clone(),
                ),
                Side::Axis => Location::Box(OffMapBox::Tripoli),
            };
            for row in data.schedule.arrivals(side, now) {
                for head in &row.units {
                    let pieces =
                        setup::reinforcement(side, head, &data.oa, &data.chars, now, at.clone())
                            .expect("the schedule names units of the OA sheets");
                    let mut ids = Vec::new();
                    for mut p in pieces {
                        if self.pieces.contains_key(&p.id) {
                            continue;
                        }
                        p.landed = true;
                        ids.push(p.id.clone());
                        self.pieces.insert(p.id.clone(), p);
                    }
                    self.attach_where_stacked(Some(&ids));
                    if !ids.is_empty() {
                        any = true;
                        events.push(Event::Arrived {
                            pieces: ids,
                            at: at.clone(),
                        });
                    }
                }
            }
        }
        if self.supply_arrivals(data, events) {
            any = true;
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, mv, pid};
    use crate::order::Order;
    use crate::piece::PieceKind;
    use crate::state::test_support::{pass_stage, set_up};
    use cna_data::cp::Cp;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    #[test]
    fn every_arrival_of_the_scenario_resolves_to_pieces() {
        let data = graziani();
        let mut n = 0;
        for row in data.schedule.rows().filter(|r| r.at <= data.scenario.end) {
            for head in &row.units {
                let pieces = setup::reinforcement(
                    row.side,
                    head,
                    &data.oa,
                    &data.chars,
                    row.at,
                    Location::Box(OffMapBox::Tripoli),
                )
                .unwrap();
                assert!(!pieces.is_empty(), "{head}");
                assert_eq!(pieces[0].id, pid(head.as_str()));
                n += pieces.len();
            }
        }
        assert!(n > 8, "{n} pieces over six Game-Turns");
    }

    #[test]
    fn the_polish_brigade_lands_at_cairo_in_game_turn_one_opstage_three() {
        let data = graziani();
        let mut state = set_up(&data, 3);
        let polish = pid("unit:cw:polish-hq");
        pass_stage(&data, &mut state);
        pass_stage(&data, &mut state);
        assert_eq!((state.clock.game_turn, state.clock.stage), (1, 3));
        assert!(!state.pieces.contains_key(&polish));
        let before = state.pieces.len();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap();
        let arrived = events
            .iter()
            .find_map(|e| match e {
                Event::Arrived { pieces, at } => Some((pieces.clone(), at.clone())),
                _ => None,
            })
            .expect("an arrival in phase D");
        assert_eq!(arrived.1, Location::Hex(hex("E1730")));
        assert_eq!(arrived.0[0], polish);
        assert!(arrived.0.len() > 1);
        assert_eq!(state.pieces.len(), before + arrived.0.len());
        for id in &arrived.0 {
            let p = &state.pieces[id];
            assert_eq!(p.location, Location::Hex(hex("E1730")));
            assert!(p.landed);
            assert_eq!(p.side, Side::Cw);
        }
        // Its first hex costs nothing (20.1x); after that it is an
        // ordinary unit.
        let bn = arrived
            .0
            .iter()
            .find(|id| {
                let p = &state.pieces[*id];
                p.kind == PieceKind::Combat && state.cpa(p).is_some()
            })
            .expect("a Polish combat unit")
            .clone();
        let out = data
            .map
            .neighbours(&hex("E1730"))
            .into_iter()
            .find(|h| data.map.hex(h).unwrap().terrain != cna_data::map::Terrain::Sea)
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        let events = state
            .apply(&data, Side::Cw, mv(&[bn.as_str()], &[out.as_str()]))
            .unwrap();
        assert!(matches!(&events[0], Event::Moved { cost, .. } if *cost == Cp::ZERO));
        assert_eq!(state.pieces[&bn].cp_spent, Cp::ZERO);
        assert!(!state.pieces[&bn].landed);
        // The next stage forgets the landing for the rest.
        pass_stage(&data, &mut state);
        assert!(!state.pieces[&polish].landed);
    }
}
