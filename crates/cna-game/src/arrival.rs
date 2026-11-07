//! Reinforcements (SPI 20.1x, 4.42): in phase D of the OpStage the
//! schedule names, a listed unit lands with its sheet subtree, the
//! Commonwealth at Cairo and the Axis in the Tripoli box, paying nothing
//! to land or for its first hex. Stacking does not bind an arrival
//! (only returning withdrawn units, out of scope). A unit withdrawn
//! under the `withdrawals` module returns as it left. The Benghazi
//! diversion (55.1) waits for the Logistics Game. With the
//! `scenario_arrivals` module on, a scenario's own `arrivals` rows land
//! their units at their OpStage and place, with motorisation points for
//! their trucks, and a unit named there does not arrive by the schedule
//! (R-181). With `returning_transport` on, a return printed "Tpt: See
//! ..." brings back the motorisation points that left on the withdrawals
//! it names (R-157).

use crate::data::Data;
use crate::event::Event;
use crate::piece::Location;
use crate::setup;
use crate::state::GameState;
use cna_data::oa::{Arrival, UnitId};
use cna_data::scenario::{OffMapBox, Side};
use cna_data::schedule::Row;
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
            let own = &data.scenario.side(side).arrivals;
            let own = if self.modules.scenario_arrivals {
                own.as_slice()
            } else {
                &[]
            };
            // A unit a scenario arrival names, or one under it on its OA
            // sheet, comes only by that arrival.
            let named = |unit: &UnitId| {
                let mut at = Some(unit.clone());
                let mut steps = 0;
                while let Some(u) = at {
                    if own.iter().any(|a| a.units.contains(&u)) {
                        return true;
                    }
                    steps += 1;
                    if steps > 16 {
                        break;
                    }
                    at = data.oa.unit(&u).and_then(|r| r.parent.clone());
                }
                false
            };
            // (head, where, motorisation points, OA arrival cut-off, by
            // the scenario's own row)
            let mut landings: Vec<(&UnitId, Location, u32, GameTime, bool)> = Vec::new();
            for row in data.schedule.arrivals(side, now) {
                // A return brings back the transport that left on the
                // withdrawals it names (R-157), with its first unit.
                let mut points = self.transport_back(row);
                for head in &row.units {
                    if !named(head) {
                        landings.push((head, at.clone(), points, now, false));
                        points = 0;
                    }
                }
            }
            for a in own.iter().filter(|a| a.at == now) {
                let place = setup::resolve(&a.placement, &data.map)
                    .expect("cna's arrival placements are on the map");
                assert!(
                    !matches!(place, Location::Pending(_)),
                    "an arrival lands in one hex or box: {:?}",
                    a.placement
                );
                // The motorisation points come once, with the first unit.
                let mut points = if self.modules.abstract_supply {
                    setup::motorising_points(&data.scenario, a.trucks)
                } else {
                    0
                };
                for head in &a.units {
                    // Landing ahead of its OA date, a unit still brings
                    // the units its sheet gives it by then (R-181).
                    let cut = match data.oa.unit(head).and_then(|u| u.arrives) {
                        Some(Arrival::At(t)) => t.max(now),
                        _ => now,
                    };
                    landings.push((head, place.clone(), points, cut, true));
                    points = 0;
                }
            }
            for (head, at, points, cut, by_row) in landings {
                let pieces =
                    setup::reinforcement(side, head, &data.oa, &data.chars, cut, at.clone())
                        .expect("the schedule names units of the OA sheets");
                let mut ids = Vec::new();
                for mut p in pieces {
                    if self.pieces.contains_key(&p.id)
                        || (!by_row && p.id.unit().is_some_and(|u| named(&u)))
                    {
                        continue;
                    }
                    // A withdrawn unit comes back as it left (R-157).
                    if let Some(back) = self.withdrawn.remove(&p.id) {
                        p = back;
                        p.location = at.clone();
                    }
                    p.landed = true;
                    ids.push(p.id.clone());
                    self.pieces.insert(p.id.clone(), p);
                }
                self.attach_where_stacked(Some(&ids));
                if points > 0
                    && !ids.is_empty()
                    && let Some(key) = at.key()
                {
                    *self
                        .motorisation
                        .entry(side)
                        .or_default()
                        .entry(key)
                        .or_default() += points;
                }
                if !ids.is_empty() {
                    any = true;
                    events.push(Event::Arrived {
                        pieces: ids,
                        at: at.clone(),
                    });
                }
            }
        }
        if self.supply_arrivals(data, events) {
            any = true;
        }
        any
    }
}

impl GameState {
    /// The motorisation points that left on the withdrawals `row` names
    /// in `transport_from` and have not come back yet, taken off the
    /// record (R-157); none without `returning_transport` and §32.
    fn transport_back(&mut self, row: &Row) -> u32 {
        if !self.modules.returning_transport || !self.modules.abstract_supply {
            return 0;
        }
        let mut points = 0;
        self.transport_out.retain(|t| {
            let back = row.transport_from.contains(&t.at);
            if back {
                points += t.points;
            }
            !back
        });
        points
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
    fn every_scenario_arrival_lands_in_one_hex_or_box_r_181() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/cna/data/scenarios");
        let mut n = 0;
        for f in std::fs::read_dir(dir).unwrap() {
            let path = f.unwrap().path();
            let slug = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let Ok(data) = Data::load(&slug) else {
                continue;
            };
            for side in [Side::Axis, Side::Cw] {
                for a in &data.scenario.side(side).arrivals {
                    let at = setup::resolve(&a.placement, &data.map).unwrap();
                    assert!(!matches!(at, Location::Pending(_)), "{slug}: {a:?}");
                    for head in &a.units {
                        let pieces = setup::reinforcement(
                            side,
                            head,
                            &data.oa,
                            &data.chars,
                            a.at,
                            at.clone(),
                        )
                        .unwrap();
                        assert!(!pieces.is_empty(), "{slug}: {head}");
                        n += 1;
                    }
                }
            }
        }
        assert!(n >= 1, "Rommel's Arrival lands the 18th Australian");
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
        state.stand_alone(&bn);
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
