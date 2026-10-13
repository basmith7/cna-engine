//! Off-map movement (SPI 8.8x, 8.89): Axis land units move between the
//! Tunis, Gabes, Tripoli and Tripolitania boxes and the map by whole
//! OpStages, one `MoveOffMap` a stage, spending the whole CPA each time;
//! a journey longer than a stage is in transit between its ends, and may
//! turn round. The map end is Nofilia, the road hex nearest the
//! Tripolitania box; more than 5 SP arriving together are strung along
//! the road from it. Fuel and breakdown are not simulated.

use crate::clock::{Phase, Segment};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::{HexId, HexsideFeature, Map};
use cna_data::off_map::OffMapPlace;
use cna_data::scenario::{OffMapBox, Side};

/// SP that fit in one hex of an arriving column (8.8x).
const COLUMN_SP: u32 = 5;

/// How far the arriving column may stretch.
const COLUMN_HEXES: usize = 8;

/// The box a journey ends in.
fn box_of(place: OffMapPlace) -> Option<OffMapBox> {
    match place {
        OffMapPlace::Tunis | OffMapPlace::Gabes => Some(OffMapBox::Tunisia),
        OffMapPlace::Tripoli => Some(OffMapBox::Tripoli),
        OffMapPlace::Tripolitania => Some(OffMapBox::Tripolitania),
        OffMapPlace::Nofilia => None,
    }
}

/// The off-map place a box stands for; none for a box no land unit
/// moves through (Malta, Italy, Crete).
fn place_of(b: OffMapBox) -> Option<OffMapPlace> {
    match b {
        OffMapBox::Tunisia => Some(OffMapPlace::Tunis),
        OffMapBox::Tripoli => Some(OffMapPlace::Tripoli),
        OffMapBox::Tripolitania => Some(OffMapPlace::Tripolitania),
        OffMapBox::Malta | OffMapBox::ItalySicily | OffMapBox::Crete => None,
    }
}

/// Where an off-map move starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    At(OffMapPlace),
    Transit {
        from: OffMapPlace,
        to: OffMapPlace,
        done: u8,
    },
}

/// The road hexes from Nofilia outward, Nofilia first.
fn road_from(map: &Map, start: &HexId) -> Vec<HexId> {
    let mut out = vec![start.clone()];
    let mut prev: Option<HexId> = None;
    while out.len() < COLUMN_HEXES {
        let cur = out.last().expect("never empty").clone();
        let next = map.neighbours(&cur).into_iter().find(|n| {
            prev.as_ref() != Some(n)
                && map
                    .hexside(&cur, n)
                    .is_some_and(|hs| hs.has(HexsideFeature::Road))
        });
        match next {
            Some(n) => {
                prev = Some(cur);
                out.push(n);
            }
            None => break,
        }
    }
    out
}

impl GameState {
    fn nofilia(data: &Data) -> HexId {
        data.map
            .place("nofilia")
            .expect("places.json has Nofilia")
            .hex
            .clone()
    }

    /// `Order::MoveOffMap`: the stack spends this OpStage moving toward
    /// `to` (8.8x).
    pub(crate) fn move_off_map(
        &mut self,
        data: &Data,
        player: Side,
        pieces: &[PieceId],
        to: OffMapPlace,
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::Movement)
        {
            return Err(Illegal::new(
                "8.8x",
                format!("off-map movement happens in a movement segment, not at {}", self.clock),
            ));
        }
        if player != Side::Axis {
            return Err(Illegal::new(
                "8.8x",
                "no Commonwealth land unit ever enters an off-map box",
            ));
        }
        let Some(first) = pieces.first() else {
            return Err(Illegal::new("8.8x", "no pieces to move"));
        };
        let nofilia = Self::nofilia(data);
        let mut origin: Option<Origin> = None;
        let mut cpa = u32::MAX;
        for (i, id) in pieces.iter().enumerate() {
            if pieces[..i].contains(id) {
                return Err(Illegal::new("8.8x", format!("{id} is listed twice")));
            }
            let p = self
                .pieces
                .get(id)
                .ok_or_else(|| Illegal::new("8.8x", format!("{id}: no such piece")))?;
            if p.side != player {
                return Err(Illegal::new("8.8x", format!("{id} is not {player}'s")));
            }
            self.able_to_move(p)?;
            if p.landed {
                return Err(Illegal::new(
                    "8.8x",
                    format!("{id} landed this OpStage and may not leave its box until the next"),
                ));
            }
            if p.cp_spent != Cp::ZERO {
                return Err(Illegal::new(
                    "8.8x",
                    format!("{id} has spent CP this OpStage: off-map movement takes the whole stage"),
                ));
            }
            let here = match &p.location {
                Location::Box(b) => place_of(*b).map(Origin::At),
                Location::Transit { from, to, done } => Some(Origin::Transit {
                    from: *from,
                    to: *to,
                    done: *done,
                }),
                Location::Hex(h) if *h == nofilia => Some(Origin::At(OffMapPlace::Nofilia)),
                _ => None,
            };
            let Some(here) = here else {
                return Err(Illegal::new(
                    "8.8x",
                    format!("{id} is neither in a land box, in transit, nor at Nofilia {nofilia}"),
                ));
            };
            match origin {
                None => origin = Some(here),
                Some(o) if o != here => {
                    return Err(Illegal::new(
                        "8.8x",
                        format!("{id} is not where {first} is"),
                    ));
                }
                Some(_) => {}
            }
            cpa = cpa.min(self.cpa(p).expect("able to move"));
        }
        let origin = origin.expect("at least one piece");
        let stages = |a: OffMapPlace, b: OffMapPlace| {
            data.off_map.stages(a, b, cpa).ok_or_else(|| {
                Illegal::new("8.8x", format!("the chart has no road from {a} to {b}"))
            })
        };
        let (from, done) = match origin {
            Origin::At(a) => {
                if a == to {
                    return Err(Illegal::new("8.8x", format!("the stack is at {a} already")));
                }
                (a, 1)
            }
            Origin::Transit {
                from,
                to: heading,
                done,
            } => {
                if to == heading {
                    (from, done + 1)
                } else if to == from {
                    // Turning round: as far back as it has come.
                    (heading, stages(from, heading)? - done + 1)
                } else {
                    return Err(Illegal::new(
                        "8.8x",
                        format!("a stack between {from} and {heading} goes on or turns back"),
                    ));
                }
            }
        };
        let needed = stages(from, to)?;
        let mut events = Vec::new();
        let locations: Vec<Location> = if done >= needed {
            match box_of(to) {
                Some(b) => vec![Location::Box(b); pieces.len()],
                None => self.column_from(data, &nofilia, pieces),
            }
        } else {
            vec![Location::Transit { from, to, done }; pieces.len()]
        };
        for (id, location) in pieces.iter().zip(locations) {
            if let Location::Hex(h) = &location {
                self.entered.insert(h.clone());
            }
            self.pieces.get_mut(id).expect("checked").location = location.clone();
            events.push(Event::MovedOffMap {
                piece: id.clone(),
                location,
            });
            let own = Cp::whole(self.cpa(&self.pieces[id]).expect("able to move"));
            self.charge(id, own, false, &mut events);
        }
        Ok(events)
    }

    /// Where an arriving column stands: 5 SP a hex along the road from
    /// Nofilia (8.8x), in the order given.
    fn column_from(&self, data: &Data, nofilia: &HexId, pieces: &[PieceId]) -> Vec<Location> {
        let road = road_from(&data.map, nofilia);
        let mut out = Vec::new();
        let (mut i, mut filled) = (0, 0);
        for id in pieces {
            let sp = cna_rules::stacking::points(&data.stacking, self.pieces[id].equivalent);
            if filled > 0 && filled + sp > COLUMN_SP && i + 1 < road.len() {
                i += 1;
                filled = 0;
            }
            filled += sp;
            out.push(Location::Hex(road[i].clone()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use crate::state::test_support::pass_stage;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    fn off_map(pieces: &[&str], to: OffMapPlace) -> Order {
        Order::MoveOffMap {
            pieces: pieces.iter().map(|s| pid(s)).collect(),
            to,
        }
    }

    /// On to the next OpStage's Axis movement segment, whoever holds
    /// the initiative by then.
    fn next_axis_movement(data: &Data, state: &mut GameState) {
        pass_stage(data, state);
        let holder = state.to_act().unwrap();
        state
            .apply(
                data,
                holder,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        state.apply(data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
    }

    #[test]
    fn the_road_from_nofilia() {
        let data = graziani();
        let road = road_from(&data.map, &hex("A2703"));
        assert_eq!(&road[..3], &[hex("A2703"), hex("A2704"), hex("A2604")]);
    }

    #[test]
    fn a_fast_unit_reaches_the_map_from_tripolitania_in_one_stage() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let tanks = pid("unit:it:lxiil-tank-bn");
        state.pieces.get_mut(&tanks).unwrap().location = Location::Box(OffMapBox::Tripolitania);
        let before = snapshot(&state);
        // Not the Commonwealth's; not from the map except at Nofilia; not
        // to where it is.
        let err = state
            .apply(&data, Side::Cw, off_map(&["unit:cw:1-krrc"], OffMapPlace::Tripoli))
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let err = state
            .apply(&data, Side::Axis, off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tripoli))
            .unwrap_err();
        assert_eq!(err.case, "8.8x");
        let err = state
            .apply(
                &data,
                Side::Axis,
                off_map(&["unit:it:lxiil-tank-bn"], OffMapPlace::Tripolitania),
            )
            .unwrap_err();
        assert_eq!(err.case, "8.8x");
        assert_eq!(snapshot(&state), before);
        // CPA 30: one stage to Nofilia, the whole CPA spent.
        let events = state
            .apply(&data, Side::Axis, off_map(&["unit:it:lxiil-tank-bn"], OffMapPlace::Nofilia))
            .unwrap();
        assert_eq!(
            events[0],
            Event::MovedOffMap {
                piece: tanks.clone(),
                location: Location::Hex(hex("A2703"))
            }
        );
        assert_eq!(state.pieces[&tanks].cp_spent, Cp::whole(30));
        // Not again this stage; next stage it may leave from Nofilia.
        let err = state
            .apply(
                &data,
                Side::Axis,
                off_map(&["unit:it:lxiil-tank-bn"], OffMapPlace::Tripolitania),
            )
            .unwrap_err();
        assert_eq!(err.case, "8.8x");
        next_axis_movement(&data, &mut state);
        state
            .apply(
                &data,
                Side::Axis,
                off_map(&["unit:it:lxiil-tank-bn"], OffMapPlace::Tripolitania),
            )
            .unwrap();
        assert_eq!(
            state.pieces[&tanks].location,
            Location::Box(OffMapBox::Tripolitania)
        );
        let err = state
            .apply(&data, Side::Axis, mv(&["unit:it:lxiil-tank-bn"], &["A2704"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
    }

    #[test]
    fn a_slow_unit_is_in_transit_and_may_turn_round() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let bn = pid("unit:it:xvii-libyan-bn");
        state.pieces.get_mut(&bn).unwrap().location = Location::Box(OffMapBox::Tripoli);
        // CPA 10: three stages Tripoli to Tripolitania.
        state
            .apply(
                &data,
                Side::Axis,
                off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tripolitania),
            )
            .unwrap();
        let transit = |done| Location::Transit {
            from: OffMapPlace::Tripoli,
            to: OffMapPlace::Tripolitania,
            done,
        };
        assert_eq!(state.pieces[&bn].location, transit(1));
        assert_eq!(state.pieces[&bn].cp_spent, Cp::whole(10));
        next_axis_movement(&data, &mut state);
        // Only on or back.
        let err = state
            .apply(&data, Side::Axis, off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tunis))
            .unwrap_err();
        assert_eq!(err.case, "8.8x");
        state
            .apply(
                &data,
                Side::Axis,
                off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tripolitania),
            )
            .unwrap();
        assert_eq!(state.pieces[&bn].location, transit(2));
        // Back toward Tripoli: two stages out, two back.
        next_axis_movement(&data, &mut state);
        state
            .apply(&data, Side::Axis, off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tripoli))
            .unwrap();
        assert_eq!(
            state.pieces[&bn].location,
            Location::Transit {
                from: OffMapPlace::Tripolitania,
                to: OffMapPlace::Tripoli,
                done: 2
            }
        );
        next_axis_movement(&data, &mut state);
        state
            .apply(&data, Side::Axis, off_map(&["unit:it:xvii-libyan-bn"], OffMapPlace::Tripoli))
            .unwrap();
        assert_eq!(state.pieces[&bn].location, Location::Box(OffMapBox::Tripoli));
    }

    #[test]
    fn a_big_column_is_strung_along_the_road() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        // Six Libyan battalions (6 SP) at CPA 10 from Tripolitania: three
        // stages, then 5 SP at Nofilia and 1 at A2704.
        let ids = [
            "unit:it:ix-libyan-bn",
            "unit:it:viii-libyan-bn",
            "unit:it:x-libyan-bn",
            "unit:it:xi-libyan-bn",
            "unit:it:xvii-libyan-bn",
            "unit:it:xix-libyan-bn",
        ];
        for id in ids {
            state.pieces.get_mut(&pid(id)).unwrap().location =
                Location::Box(OffMapBox::Tripolitania);
        }
        for _ in 0..2 {
            state
                .apply(&data, Side::Axis, off_map(&ids, OffMapPlace::Nofilia))
                .unwrap();
            next_axis_movement(&data, &mut state);
        }
        state
            .apply(&data, Side::Axis, off_map(&ids, OffMapPlace::Nofilia))
            .unwrap();
        for id in &ids[..5] {
            assert_eq!(state.pieces[&pid(id)].hex(), Some(&hex("A2703")));
        }
        assert_eq!(state.pieces[&pid(ids[5])].hex(), Some(&hex("A2704")));
        assert!(state.entered.contains(&hex("A2704")));
    }
}
