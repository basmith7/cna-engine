//! Breakdown (SPI §21) with the `breakdown` module on: every motorised
//! unit collects the breakdown points of each step it takes, in any
//! kind of move, until the OpStage ends (21.2x).

use crate::data::Data;
use crate::event::Event;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::state::Weather;
use cna_data::map::HexId;
use cna_data::oa::Nation;
use cna_data::scenario::Side;
use cna_rules::movement::Step;
use serde::{Deserialize, Serialize};

/// Broken-down TOE points of one weapon system in a hex (21.4x): no CPA,
/// no part in combat, until towed or repaired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wreck {
    pub side: Side,
    pub nation: Option<Nation>,
    pub weapon: String,
    pub points: u32,
}

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

/// Whether at least half of a move's CP went on sandstorm hexes (21.37),
/// from its steps' (hex entered, cost).
pub(crate) fn mostly_in_sandstorm<'a>(
    state: &GameState,
    steps: impl IntoIterator<Item = (&'a HexId, cna_data::cp::Cp)>,
) -> bool {
    let (mut all, mut storm) = (cna_data::cp::Cp::ZERO, cna_data::cp::Cp::ZERO);
    for (to, cost) in steps {
        all += cost;
        if state.weather_at(to) == cna_rules::movement::Weather::Sandstorm {
            storm += cost;
        }
    }
    all > cna_data::cp::Cp::ZERO && storm + storm >= all
}

impl GameState {
    /// A battalion or bigger of `pieces` now at `at` captures the enemy's
    /// broken-down vehicles in `at` and the hexes next to it (21.5x),
    /// unless enemy combat units or HQs stand with them or an enemy ZOC
    /// covers them. They become the captor's where they stand (the
    /// three-hex placement and the passable-terrain test are not
    /// simulated).
    pub(crate) fn capture_wrecks(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        at: &HexId,
        events: &mut Vec<Event>,
    ) {
        use cna_data::characteristics::Equivalent;
        let Some(captor) = pieces
            .iter()
            .map(|id| &self.pieces[id])
            .find(|p| {
                p.kind == PieceKind::Combat
                    && matches!(
                        p.equivalent,
                        Some(Equivalent::Battalion | Equivalent::Brigade)
                    )
            })
            .map(|p| (p.side, p.id.clone()))
        else {
            return;
        };
        let (side, by) = captor;
        let mut hexes = data.map.neighbours(at);
        hexes.push(at.clone());
        for hex in hexes {
            let Some(wrecks) = self.broken_down.get(&hex) else {
                continue;
            };
            if !wrecks.iter().any(|w| w.side != side) {
                continue;
            }
            let guarded = self.pieces.values().any(|p| {
                p.side != side
                    && p.hex() == Some(&hex)
                    && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
            });
            if guarded || self.zoc_at(data, &hex, side) != crate::zoc::Zoc::Clear {
                continue;
            }
            let wrecks = self.broken_down.get_mut(&hex).expect("listed");
            let mut taken = 0;
            for w in wrecks.iter_mut().filter(|w| w.side != side) {
                w.side = side;
                taken += w.points;
            }
            events.push(Event::WrecksCaptured {
                by: by.clone(),
                hex: hex.clone(),
                points: taken,
            });
        }
    }

    /// The pieces of a move that has stopped check for breakdown (21.2x):
    /// each over 3 points in a higher column than its last check rolls
    /// once per vehicle weapon system, the column shifted by the model's
    /// BAR and the weather (21.3x): hot, or `storm` (at least half the
    /// move's CP spent under a sandstorm); the points lost are removed. Tanks,
    /// SP guns and self-propelled anti-tank guns only: armoured cars,
    /// recce and trucks are not yet told apart in a unit's TOE.
    pub(crate) fn check_breakdown(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        storm: bool,
        events: &mut Vec<Event>,
    ) {
        use cna_rules::breakdown::{column, losses, must_check, percent, weather_shift};
        if !self.modules.breakdown {
            return;
        }
        let t = &data.breakdown;
        let hot = matches!(self.weather, Weather::Hot);
        for id in pieces {
            let p = &self.pieces[id];
            if p.hex().is_none() {
                continue;
            }
            let points = f64::from(p.breakdown_halves) / 2.0;
            let last = p.breakdown_checked.map(usize::from);
            if !must_check(t, points, last) {
                continue;
            }
            let now = t.band_index(points.ceil() as u32).expect("checked") as u8;
            let groups: Vec<(String, u32, i32)> = p
                .weapons
                .iter()
                .filter(|w| w.points > 0)
                .filter_map(|w| {
                    Some((
                        w.weapon.clone(),
                        w.points,
                        data.weapons.resolve(p.nation?, &w.weapon)?.bar?,
                    ))
                })
                .collect();
            self.pieces.get_mut(id).expect("listed").breakdown_checked = Some(now);
            let mut lost = 0;
            for (weapon, count, bar) in groups {
                let Some(col) = column(t, points, bar + weather_shift(hot, storm)) else {
                    continue;
                };
                let reading = self.rng.d6() * 10 + self.rng.d6();
                let pct = percent(t, col, reading);
                let n = losses(count, pct);
                events.push(Event::BrokeDown {
                    piece: id.clone(),
                    weapon: weapon.clone(),
                    reading,
                    percent: pct,
                    points: n,
                });
                let mut taken = 0;
                for _ in 0..n {
                    if self.take_loss_entry(id, Some(&weapon)) {
                        taken += 1;
                    }
                }
                lost += taken;
                // They stay where the unit stopped (21.41).
                let p = &self.pieces[id];
                let (side, nation) = (p.side, p.nation);
                if taken > 0
                    && let Some(at) = p.hex().cloned()
                {
                    let wrecks = self.broken_down.entry(at).or_default();
                    match wrecks
                        .iter_mut()
                        .find(|w| w.side == side && w.nation == nation && w.weapon == weapon)
                    {
                        Some(w) => w.points += taken,
                        None => wrecks.push(Wreck {
                            side,
                            nation,
                            weapon: weapon.clone(),
                            points: taken,
                        }),
                    }
                }
            }
            self.note_losses(id, lost, events);
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
    fn a_battalion_moving_next_to_enemy_wrecks_takes_them_21_5x() {
        use crate::breakdown::Wreck;
        use crate::event::Event;
        use crate::piece::PieceId;
        let data = Data::load("grazianis-offensive").unwrap();
        let state = movement(&data, Modules::ALL);
        let tanks = PieceId::from(cna_data::oa::UnitId::parse("unit:it:lxl-tank-bn").unwrap());
        let at = state.pieces[&tanks].hex().unwrap().clone();
        for n in data.map.neighbours(&at) {
            let mut s = state.clone();
            // Commonwealth wrecks one hex beyond the step, out of any ZOC.
            let Some(beyond) = data.map.neighbours(&n).into_iter().find(|h| {
                *h != at
                    && s.holder(h).is_none()
                    && s.zoc_at(&data, h, Side::Axis) == crate::zoc::Zoc::Clear
            }) else {
                continue;
            };
            s.broken_down.insert(
                beyond.clone(),
                vec![Wreck {
                    side: Side::Cw,
                    nation: None,
                    weapon: "Mark VI Light".into(),
                    points: 2,
                }],
            );
            let order = Order::Move {
                pieces: vec![tanks.clone()],
                path: vec![n.clone()],
                assault: false,
                fuel_from: None,
            };
            let Ok(events) = s.apply(&data, Side::Axis, order) else {
                continue;
            };
            assert!(events.contains(&Event::WrecksCaptured {
                by: tanks.clone(),
                hex: beyond.clone(),
                points: 2
            }));
            assert_eq!(s.broken_down[&beyond][0].side, Side::Axis);
            return;
        }
        panic!("no step with wrecks beyond it");
    }

    #[test]
    fn a_move_mostly_under_a_sandstorm_shifts_the_check_21_37() {
        use cna_data::cp::Cp;
        use cna_data::map::HexId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        state.weather = crate::state::Weather::Sandstorm { sheets: vec!['C'] };
        let (c, d) = (
            HexId::parse("C3920").unwrap(),
            HexId::parse("D3714").unwrap(),
        );
        let half = [(&c, Cp::whole(2)), (&d, Cp::whole(2))];
        assert!(super::mostly_in_sandstorm(&state, half));
        let less = [(&c, Cp::whole(1)), (&d, Cp::whole(2))];
        assert!(!super::mostly_in_sandstorm(&state, less));
        assert!(!super::mostly_in_sandstorm(&state, []));
    }

    #[test]
    fn a_tank_battalion_that_stops_past_seventy_points_loses_tanks_21_3x() {
        use crate::event::Event;
        use crate::piece::PieceId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        // L3s (BAR 2R) at Bardia, already at 100 points this stage: 71+
        // reads past every escape.
        let tanks = PieceId::from(cna_data::oa::UnitId::parse("unit:it:lxl-tank-bn").unwrap());
        state.pieces.get_mut(&tanks).unwrap().breakdown_halves = 200;
        let at = state.pieces[&tanks].hex().unwrap().clone();
        let toe = state.pieces[&tanks].toe;
        let mut moved = None;
        for n in data.map.neighbours(&at) {
            let mut s = state.clone();
            let order = Order::Move {
                pieces: vec![tanks.clone()],
                path: vec![n],
                assault: false,
                fuel_from: None,
            };
            if let Ok(events) = s.apply(&data, Side::Axis, order) {
                moved = Some((s, events));
                break;
            }
        }
        let (s, events) = moved.expect("the L3s can step somewhere");
        let broke: u32 = events
            .iter()
            .filter_map(|e| match e {
                Event::BrokeDown {
                    piece,
                    percent,
                    points,
                    ..
                } if *piece == tanks => {
                    assert!(*percent > 0);
                    Some(*points)
                }
                _ => None,
            })
            .sum();
        assert!(broke > 0);
        assert_eq!(s.pieces[&tanks].toe, toe - broke);
        // The broken-down tanks stay in the hex the battalion stopped in.
        let stop = s.pieces[&tanks].hex().unwrap();
        assert_eq!(
            s.broken_down[stop].iter().map(|w| w.points).sum::<u32>(),
            broke
        );
        assert_eq!(s.pieces[&tanks].breakdown_checked, Some(8));
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
