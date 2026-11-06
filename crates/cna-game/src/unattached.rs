//! Unattached motorisation points at the start (61.72, R-160), with the
//! `unattached_motorisation` and `abstract_supply` modules on: each of
//! the scenario's `unattached` rows becomes a quota its side divides, as
//! it likes, among the row's hexes with `Order::PlaceMotorisation` during
//! its set-up, which does not pass on until every quota is spent. The
//! points join `motorisation` in the hex they are placed in (32.5x).
//! Rommel's Arrival is the only scenario with such rows.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::place::area;
use crate::state::{GameState, Pending};
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

/// Points a side has yet to place, and the hexes they may go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotorisationQuota {
    pub hexes: Vec<HexId>,
    pub points: u32,
}

impl GameState {
    /// The scenario's unattached rows as quotas, with the modules on.
    pub(crate) fn start_unattached_motorisation(&mut self, data: &Data) -> anyhow::Result<()> {
        if !(self.modules.unattached_motorisation && self.modules.abstract_supply) {
            return Ok(());
        }
        let rows = data
            .scenario
            .abstractions
            .as_ref()
            .and_then(|a| a.air_and_logistics.as_ref())
            .and_then(|a| a.motorisation.as_ref())
            .map(|m| m.unattached.clone())
            .unwrap_or_default();
        for (i, row) in rows.into_iter().enumerate() {
            let mut hexes = area(data, &row.placement);
            hexes.sort();
            hexes.dedup();
            if hexes.is_empty() {
                anyhow::bail!("unattached motorisation row {}: no hex on the map", i + 1);
            }
            if row.count > 0 {
                self.motorisation_to_place
                    .entry(row.side)
                    .or_default()
                    .push(MotorisationQuota {
                        hexes,
                        points: row.count,
                    });
            }
        }
        Ok(())
    }

    /// The hexes `side` may place motorisation points in now, in id
    /// order, with the most it may place in each: empty when that side is
    /// not setting up.
    pub fn motorisation_placements(&self, side: Side) -> Vec<(HexId, u32)> {
        if self.pending != Some(Pending::SetUp { side }) {
            return Vec::new();
        }
        let mut v: std::collections::BTreeMap<HexId, u32> = Default::default();
        for q in self.motorisation_to_place.get(&side).into_iter().flatten() {
            for h in &q.hexes {
                let most = v.entry(h.clone()).or_default();
                *most = (*most).max(q.points);
            }
        }
        v.into_iter().collect()
    }

    /// `Order::PlaceMotorisation`: the side setting up puts `points` of
    /// its unattached motorisation points in `hex` (61.72, R-160).
    pub(crate) fn place_motorisation(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
        points: u32,
    ) -> Result<Vec<Event>, Illegal> {
        if self.pending != Some(Pending::SetUp { side: player }) {
            return Err(Illegal::new("59.2", format!("{player} is not setting up")));
        }
        if points == 0 {
            return Err(Illegal::new("61.72", "place at least one point"));
        }
        let quotas = self.motorisation_to_place.get_mut(&player);
        let Some(q) = quotas.and_then(|qs| qs.iter_mut().find(|q| q.hexes.contains(hex))) else {
            return Err(Illegal::new(
                "61.72",
                format!("{player} has no unattached motorisation points to place in {hex} (R-160)"),
            ));
        };
        if points > q.points {
            return Err(Illegal::new(
                "61.72",
                format!("only {} points are left to place in {hex}", q.points),
            ));
        }
        q.points -= points;
        if let Some(qs) = self.motorisation_to_place.get_mut(&player) {
            qs.retain(|q| q.points > 0);
            if qs.is_empty() {
                self.motorisation_to_place.remove(&player);
            }
        }
        *self
            .motorisation
            .entry(player)
            .or_default()
            .entry(hex.to_string())
            .or_default() += points;
        Ok(self.advance_set_up(data))
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::order::Order;
    use crate::state::test_support::{place_only_pieces, place_pieces};
    use crate::state::{GameState, Modules, Pending};
    use crate::view::{Aliases, leaks, view_for};
    use cna_data::map::HexId;
    use cna_data::scenario::Side;

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn rommel() -> Data {
        Data::load("rommels-arrival").unwrap()
    }

    fn points(state: &GameState, side: Side, h: &str) -> u32 {
        state
            .motorisation
            .get(&side)
            .and_then(|m| m.get(h))
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn rommels_arrival_sides_divide_their_unattached_points_61_72() {
        let data = rommel();
        let mut state = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        let axis = &state.motorisation_to_place[&Side::Axis];
        assert_eq!(axis.len(), 1);
        assert_eq!(axis[0].points, 290);
        assert_eq!(axis[0].hexes.len(), 20);
        let cw: Vec<u32> = state.motorisation_to_place[&Side::Cw]
            .iter()
            .map(|q| q.points)
            .collect();
        assert_eq!(cw, vec![150, 90]);

        place_only_pieces(&data, &mut state, Side::Axis);
        assert_eq!(state.pending, Some(Pending::SetUp { side: Side::Axis }));
        // Off the coast road, or more than is left: refused.
        let off = state.apply(
            &data,
            Side::Axis,
            Order::PlaceMotorisation {
                hex: hex("A1807"),
                points: 30,
            },
        );
        assert_eq!(off.unwrap_err().case, "61.72");
        let much = state.apply(
            &data,
            Side::Axis,
            Order::PlaceMotorisation {
                hex: hex("A1816"),
                points: 291,
            },
        );
        assert_eq!(much.unwrap_err().case, "61.72");
        let zero = state.apply(
            &data,
            Side::Axis,
            Order::PlaceMotorisation {
                hex: hex("A1816"),
                points: 0,
            },
        );
        assert!(zero.is_err());
        // Pass is refused while points are left.
        assert!(state.apply(&data, Side::Axis, Order::Pass).is_err());

        state
            .apply(
                &data,
                Side::Axis,
                Order::PlaceMotorisation {
                    hex: hex("A1816"),
                    points: 260,
                },
            )
            .unwrap();
        assert_eq!(state.pending, Some(Pending::SetUp { side: Side::Axis }));
        assert_eq!(state.motorisation_placements(Side::Axis).len(), 20);
        // Set-up waits on the 30 left.
        assert!(state.apply(&data, Side::Axis, Order::Pass).is_err());
        state
            .apply(
                &data,
                Side::Axis,
                Order::PlaceMotorisation {
                    hex: hex("A2703"),
                    points: 30,
                },
            )
            .unwrap();
        assert_eq!(points(&state, Side::Axis, "A1816"), 260);
        assert_eq!(points(&state, Side::Axis, "A2703"), 30);
        assert!(!state.motorisation_to_place.contains_key(&Side::Axis));
        // The Axis is done: the Commonwealth sets up.
        assert_eq!(state.pending, Some(Pending::SetUp { side: Side::Cw }));
        assert!(state.motorisation_placements(Side::Axis).is_empty());

        // Cairo takes only the 150 row's points; Tobruk only the 90's.
        let cairo = state.apply(
            &data,
            Side::Cw,
            Order::PlaceMotorisation {
                hex: hex("E1730"),
                points: 151,
            },
        );
        assert!(cairo.is_err());
        state
            .apply(
                &data,
                Side::Cw,
                Order::PlaceMotorisation {
                    hex: hex("E1730"),
                    points: 150,
                },
            )
            .unwrap();
        // Tobruk's own trucks already give it points (59.63); these add.
        let tobruk = points(&state, Side::Cw, "C4807");
        state
            .apply(
                &data,
                Side::Cw,
                Order::PlaceMotorisation {
                    hex: hex("C4807"),
                    points: 90,
                },
            )
            .unwrap();
        assert_eq!(points(&state, Side::Cw, "C4807"), tobruk + 90);
        assert!(state.motorisation_to_place.is_empty());
        place_pieces(&data, &mut state, Side::Cw);
    }

    #[test]
    fn the_enemy_sees_none_of_the_points_left_to_place_3_61() {
        let data = rommel();
        let state = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        let aliases = Aliases::new([1; 32]);
        let v = view_for(&state, &data, &aliases, Some(Side::Cw), false).unwrap();
        assert!(v["motorisation_to_place"].get("axis").is_none());
        assert!(v["motorisation_to_place"].get("cw").is_some());
        assert!(leaks(&state, &v, Side::Axis).is_empty());
    }

    #[test]
    fn with_the_module_off_no_points_wait() {
        let data = rommel();
        let modules = Modules {
            unattached_motorisation: false,
            ..Modules::ALL
        };
        let state = GameState::with_modules(&data, 1, modules).unwrap();
        assert!(state.motorisation_to_place.is_empty());
    }
}
