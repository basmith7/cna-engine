//! What a sealed answer may contain (M3 spec, *Sealed options*): the
//! gun-class units that may stand forward, the firers and the enemy
//! units each may barrage, the units with TOE to commit and the hexes
//! each role may name. Built from the same per-item predicates
//! `check_gun_positions`, `check_barrage` and `check_assignments` call
//! (`plots.rs`), so one owner decides what a panel offers and what
//! `apply` accepts. Points are the player's to type in and `apply`'s to
//! judge.

use crate::combat::SealedStep;
use crate::combat::strength::Kind;
use crate::data::Data;
use crate::event::Illegal;
use crate::piece::PieceId;
use crate::state::{GameState, Pending};
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

/// The choices open to one side in the sealed step now pending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SealedOptions {
    /// Step 1 (12.1x): the gun-class units that may stand forward.
    GunPositions { guns: Vec<Gun> },
    /// Step 2 (12.2x): the units with barrage points that have a target
    /// in reach, and those targets.
    Barrage { firers: Vec<Firer> },
    /// Step 4 (14.1x, 15.2x): the units with TOE to commit and the
    /// roles each has; `phasing` says whether each allocation names a
    /// target hex (14.12).
    AssignForces {
        phasing: bool,
        units: Vec<Committer>,
    },
}

/// A gun-class unit that may take a position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gun {
    pub piece: PieceId,
    /// Standing next to an enemy unit: the position matters now; the
    /// rest may take one all the same.
    pub next_to_enemy: bool,
}

/// A unit that may barrage, with the most points it can fire and the
/// enemy units in adjacent hexes it may fire at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Firer {
    pub piece: PieceId,
    /// Its TOE: the most points it may fire in all (12.2x).
    pub points: u32,
    /// Standing forward: it may split its fire; a back gun fires at one
    /// target and never shares a target with a hex-mate (12.1x, 12.32).
    pub forward: bool,
    pub targets: Vec<PieceId>,
}

/// A unit that may commit TOE points to the roles of step 4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committer {
    pub piece: PieceId,
    /// Its TOE: the most points both roles may take together (14.0).
    pub points: u32,
    /// It has an anti-armour rating (14.0).
    pub anti_armour: bool,
    /// The phasing side only: the adjacent enemy hexes anti-armour fire
    /// may be assigned against (14.34); empty for the non-phasing side,
    /// which defends its own hex.
    pub anti_armour_targets: Vec<HexId>,
    /// The phasing side only: the adjacent enemy hexes it may assault
    /// (15.0, 15.36); empty for the non-phasing side.
    pub close_assault_targets: Vec<HexId>,
    /// Released from reserve this OpStage: one close assault at most
    /// (18.2x).
    pub one_assault: bool,
}

impl GameState {
    /// The choices open to `side` in the sealed step now pending; `Err`
    /// with the case `apply` would give when `side` may not answer now.
    pub fn sealed_options(&self, data: &Data, side: Side) -> Result<SealedOptions, Illegal> {
        let Some(Pending::Sealed { step, .. }) = &self.pending else {
            return Err(Illegal::new("11.0", "no sealed combat step is pending"));
        };
        match self.to_act() {
            None => return Err(Illegal::new("60.22", "the game is over")),
            Some(s) if s != side => {
                return Err(Illegal::new(
                    "5.2",
                    format!("{side} may not act: waiting on {s} at {}", self.clock),
                ));
            }
            Some(_) => {}
        }
        let own: Vec<&PieceId> = self
            .pieces
            .values()
            .filter(|p| p.side == side && p.hex().is_some())
            .map(|p| &p.id)
            .collect();
        Ok(match step {
            SealedStep::GunPositions => SealedOptions::GunPositions {
                guns: own
                    .iter()
                    .filter_map(|id| {
                        let p = self.may_stand_forward(data, side, id).ok()?;
                        Some(Gun {
                            piece: (*id).clone(),
                            next_to_enemy: self.next_to_enemy(data, p),
                        })
                    })
                    .collect(),
            },
            SealedStep::Barrage => SealedOptions::Barrage {
                firers: own
                    .iter()
                    .filter_map(|id| {
                        let p = self.barrage_firer(data, side, id).ok()?;
                        let targets: Vec<PieceId> = self
                            .pieces
                            .values()
                            .filter(|t| self.barrage_target(data, side, p, &t.id).is_ok())
                            .map(|t| t.id.clone())
                            .collect();
                        // A firer with nobody in reach is no choice.
                        if targets.is_empty() {
                            return None;
                        }
                        Some(Firer {
                            piece: (*id).clone(),
                            points: p.toe,
                            forward: self.combat.forward.contains(id),
                            targets,
                        })
                    })
                    .collect(),
            },
            SealedStep::AssignForces => {
                let phasing = self.phasing() == Some(side);
                let units = own
                    .iter()
                    .filter_map(|id| {
                        let p = self.may_commit(data, side, id).ok()?;
                        let from = p.hex().expect("on the map");
                        let anti_armour = self.raw_of(data, p, Kind::AntiArmour) > 0;
                        let around = data.map.neighbours(from);
                        let (anti_armour_targets, close_assault_targets) = if phasing {
                            (
                                around
                                    .iter()
                                    .filter(|_| anti_armour)
                                    .filter(|h| self.anti_armour_target(data, p, from, h).is_ok())
                                    .cloned()
                                    .collect(),
                                around
                                    .iter()
                                    .filter(|h| self.assault_target(data, p, from, h).is_ok())
                                    .cloned()
                                    .collect(),
                            )
                        } else {
                            (Vec::new(), Vec::new())
                        };
                        Some(Committer {
                            piece: (*id).clone(),
                            points: p.toe,
                            anti_armour,
                            anti_armour_targets,
                            close_assault_targets,
                            one_assault: p.released.is_some(),
                        })
                    })
                    .collect();
                SealedOptions::AssignForces { phasing, units }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment::test_support::at_sollum;
    use super::*;
    use crate::log::{Log, replay};
    use crate::movement::test_support::pid;
    use crate::order::{Allocation, Assignment, Fire, Order};
    use std::collections::BTreeSet;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn accepted(state: &GameState, data: &Data, side: Side, order: Order) -> Result<(), Illegal> {
        state.clone().apply(data, side, order).map(|_| ())
    }

    fn own_on_map(state: &GameState, side: Side) -> Vec<PieceId> {
        state
            .pieces
            .values()
            .filter(|p| p.side == side && p.hex().is_some())
            .map(|p| p.id.clone())
            .collect()
    }

    /// Every offered item, sent alone with one point, is accepted by
    /// `apply` on a clone; every own on-map unit, enemy unit or hex not
    /// offered is refused.
    fn check_options(data: &Data, state: &GameState) {
        let side = state.to_act().unwrap();
        let options = state.sealed_options(data, side).unwrap();
        let own = own_on_map(state, side);
        match options {
            SealedOptions::GunPositions { guns } => {
                assert!(!guns.is_empty());
                for id in &own {
                    let r = accepted(
                        state,
                        data,
                        side,
                        Order::GunPositions {
                            forward: vec![id.clone()],
                        },
                    );
                    let gun = guns.iter().find(|g| &g.piece == id);
                    assert_eq!(r.is_ok(), gun.is_some(), "{id}: {r:?}");
                    if let Some(g) = gun {
                        assert_eq!(
                            g.next_to_enemy,
                            state.next_to_enemy(data, &state.pieces[id])
                        );
                    }
                }
                assert!(guns.iter().any(|g| g.next_to_enemy));
            }
            SealedOptions::Barrage { firers } => {
                assert!(!firers.is_empty());
                let enemies: Vec<PieceId> = own_on_map(state, side.other());
                for id in &own {
                    let firer = firers.iter().find(|f| &f.piece == id);
                    for t in &enemies {
                        let r = accepted(
                            state,
                            data,
                            side,
                            Order::Barrage {
                                fires: vec![Fire {
                                    piece: id.clone(),
                                    points: 1,
                                    target: t.clone(),
                                }],
                            },
                        );
                        let offered = firer.is_some_and(|f| f.targets.contains(t));
                        assert_eq!(r.is_ok(), offered, "{id} at {t}: {r:?}");
                    }
                    if let Some(f) = firer {
                        assert!(!f.targets.is_empty(), "{id}");
                        assert_eq!(f.points, state.pieces[id].toe);
                        assert_eq!(f.forward, state.combat.forward.contains(id));
                    }
                }
            }
            SealedOptions::AssignForces { phasing, units } => {
                assert!(!units.is_empty());
                assert_eq!(phasing, state.phasing() == Some(side));
                for id in &own {
                    let unit = units.iter().find(|u| &u.piece == id);
                    let from = state.pieces[id].hex().unwrap().clone();
                    let hexes: Vec<Option<HexId>> = if phasing {
                        data.map.neighbours(&from).into_iter().map(Some).collect()
                    } else {
                        vec![None]
                    };
                    for h in &hexes {
                        for anti_armour in [false, true] {
                            let alloc = vec![Allocation {
                                target: h.clone(),
                                points: 1,
                            }];
                            let order = Order::AssignForces {
                                assignments: vec![Assignment {
                                    piece: id.clone(),
                                    anti_armour: if anti_armour { alloc.clone() } else { vec![] },
                                    close_assault: if anti_armour { vec![] } else { alloc },
                                }],
                            };
                            let r = accepted(state, data, side, order);
                            let offered = unit.is_some_and(|u| match (anti_armour, h) {
                                (true, Some(h)) => u.anti_armour_targets.contains(h),
                                (false, Some(h)) => u.close_assault_targets.contains(h),
                                (true, None) => u.anti_armour,
                                (false, None) => true,
                            });
                            assert_eq!(r.is_ok(), offered, "{id} {anti_armour} {h:?}: {r:?}");
                        }
                    }
                    if let Some(u) = unit {
                        assert_eq!(u.points, state.pieces[id].toe);
                        assert_eq!(u.one_assault, state.pieces[id].released.is_some());
                    }
                }
            }
        }
    }

    #[test]
    fn the_options_are_exactly_what_apply_accepts_at_sollum() {
        let data = graziani();
        let mut state = at_sollum(&data);
        // Step 1, both sides.
        check_options(&data, &state);
        state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:4-rha")],
                },
            )
            .unwrap();
        check_options(&data, &state);
        state
            .apply(&data, Side::Axis, Order::GunPositions { forward: vec![] })
            .unwrap();
        // Step 2, both sides: 4 RHA forward, 7 Med back.
        check_options(&data, &state);
        state
            .apply(&data, Side::Cw, Order::Barrage { fires: vec![] })
            .unwrap();
        check_options(&data, &state);
        state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        // Step 3 is no sealed step.
        assert_eq!(
            state.sealed_options(&data, Side::Axis).unwrap_err().case,
            "11.0"
        );
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        // Step 4, the phasing side names hexes, the other does not.
        check_options(&data, &state);
        assert_eq!(
            state.sealed_options(&data, Side::Axis).unwrap_err().case,
            "5.2"
        );
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        check_options(&data, &state);
    }

    #[test]
    fn a_pinned_unit_and_a_back_gun_have_no_roles() {
        let data = graziani();
        let mut state = at_sollum(&data);
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (Side::Axis, Order::GunPositions { forward: vec![] }),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(&data, side, order).unwrap();
        }
        state.combat.pinned.insert(pid("unit:cw:1-krrc"));
        check_options(&data, &state);
        let SealedOptions::AssignForces { units, .. } =
            state.sealed_options(&data, Side::Cw).unwrap()
        else {
            panic!("step 4");
        };
        let ids: BTreeSet<&PieceId> = units.iter().map(|u| &u.piece).collect();
        assert!(!ids.contains(&pid("unit:cw:1-krrc")), "pinned (12.44)");
        assert!(!ids.contains(&pid("unit:cw:4-rha")), "a back gun (12.1x)");
        assert!(!ids.contains(&pid("unit:cw:7-med")));
    }

    #[test]
    fn the_golden_game_answers_with_offered_items_only() {
        let data = graziani();
        let log: Log = serde_json::from_str(
            &std::fs::read_to_string(cna_data::repo_root().join("tests/games/graziani-short.json"))
                .unwrap(),
        )
        .unwrap();
        for n in 66..=72 {
            let mut prefix = log.clone();
            prefix.orders.truncate(n);
            let state = replay(&data, &prefix).unwrap();
            let o = &log.orders[n];
            if o.order == Order::Pass {
                // Order 70: retreat before assault is no sealed step.
                assert_eq!(
                    state.sealed_options(&data, o.player).unwrap_err().case,
                    "11.0"
                );
                continue;
            }
            let options = state.sealed_options(&data, o.player).unwrap();
            match (&o.order, &options) {
                (Order::GunPositions { forward }, SealedOptions::GunPositions { guns }) => {
                    assert!(
                        forward.iter().all(|id| guns.iter().any(|g| &g.piece == id)),
                        "{n}: {forward:?} in {guns:?}"
                    );
                }
                (Order::Barrage { fires }, SealedOptions::Barrage { firers }) => {
                    for f in fires {
                        let firer = firers
                            .iter()
                            .find(|x| x.piece == f.piece)
                            .unwrap_or_else(|| panic!("{n}: {}", f.piece));
                        assert!(firer.targets.contains(&f.target), "{n}: {f:?}");
                        assert!(f.points <= firer.points);
                    }
                }
                (
                    Order::AssignForces { assignments },
                    SealedOptions::AssignForces { phasing, units },
                ) => {
                    for a in assignments {
                        let u = units
                            .iter()
                            .find(|x| x.piece == a.piece)
                            .unwrap_or_else(|| panic!("{n}: {}", a.piece));
                        for al in &a.close_assault {
                            match &al.target {
                                Some(h) => assert!(
                                    *phasing && u.close_assault_targets.contains(h),
                                    "{n}: {al:?}"
                                ),
                                None => assert!(!phasing),
                            }
                        }
                        for al in &a.anti_armour {
                            assert!(u.anti_armour);
                            if let Some(h) = &al.target {
                                assert!(u.anti_armour_targets.contains(h));
                            }
                        }
                    }
                }
                _ => panic!("order {n} and the options disagree: {options:?}"),
            }
        }
    }
}
