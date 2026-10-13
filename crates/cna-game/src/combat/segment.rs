//! The six combat steps as the clock walks them (SPI 5.2 G.3; cna
//! `60-combat.md`, *The combat segment*). A step opens only when some
//! unit could act in it, else the clock passes it with `PhaseSkipped`;
//! the sealed steps wait on both players in turn, the phasing player
//! first; retreat before assault waits on the non-phasing player; close
//! assault on the phasing player.

use crate::clock::Segment;
use crate::combat::strength::Kind;
use crate::combat::{CombatSegment, SealedStep};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::order::Order;
use crate::piece::{Piece, PieceId, PieceKind, Reserve};
use crate::state::{GameState, Pending};
use cna_data::scenario::Side;

impl GameState {
    /// Whether a piece of `side` on the map stands next to an enemy piece.
    pub(crate) fn next_to_enemy(&self, data: &Data, piece: &Piece) -> bool {
        piece.hex().is_some_and(|h| {
            data.map.neighbours(h).iter().any(|n| {
                self.pieces
                    .values()
                    .any(|q| q.side != piece.side && q.hex() == Some(n))
            })
        })
    }

    /// Whether any phasing piece stands next to an enemy piece: the
    /// combat segment has something in it (11.0).
    pub(crate) fn sides_adjacent(&self, data: &Data) -> bool {
        let phasing = self.phasing();
        self.pieces
            .values()
            .filter(|p| Some(p.side) == phasing)
            .any(|p| self.next_to_enemy(data, p))
    }

    /// Whether a gun-class unit of either side stands next to the enemy
    /// (12.1x).
    fn gun_positions_needed(&self, data: &Data) -> bool {
        self.pieces
            .values()
            .filter(|p| p.kind == PieceKind::Combat && self.next_to_enemy(data, p))
            .any(|p| self.is_gun_unit(data, p))
    }

    /// Whether a unit of either side with barrage points stands next to
    /// the enemy (12.0).
    fn barrage_needed(&self, data: &Data) -> bool {
        self.pieces
            .values()
            .filter(|p| self.next_to_enemy(data, p))
            .any(|p| self.raw_of(data, p, Kind::Barrage) > 0)
    }

    /// Whether the phasing player assigned any close assault (15.2x).
    fn close_assault_needed(&self) -> bool {
        let Some(phasing) = self.phasing() else {
            return false;
        };
        self.combat
            .assignments
            .get(&phasing)
            .is_some_and(|a| a.iter().any(|x| !x.close_assault.is_empty()))
    }

    /// The order the sealed steps are answered in: the phasing player,
    /// then the other.
    fn sealed_order(&self) -> Vec<Side> {
        let phasing = self.phasing().expect("in phase G");
        vec![phasing, phasing.other()]
    }

    /// After a movement segment closes: breakdown is passed, then every
    /// combat step with something in it opens in turn.
    pub(crate) fn open_combat(&mut self, data: &Data, events: &mut Vec<Event>) {
        self.clock = self.clock.next();
        debug_assert_eq!(self.clock.segment, Some(Segment::Breakdown));
        events.push(Event::PhaseSkipped { clock: self.clock });
        self.combat = CombatSegment::default();
        self.open_step(data, Segment::PositionDetermination, events);
    }

    /// Opens `segment`, or passes it and the ones after it that have
    /// nothing to do, until a step waits for a player.
    pub(crate) fn open_step(&mut self, data: &Data, segment: Segment, events: &mut Vec<Event>) {
        self.clock.segment = Some(segment);
        let fighting = self.sides_adjacent(data);
        let sealed = |state: &mut GameState, step: SealedStep, events: &mut Vec<Event>| {
            state.pending = Some(Pending::Sealed {
                step,
                waiting: state.sealed_order(),
            });
            events.push(Event::PhaseStarted { clock: state.clock });
        };
        match segment {
            Segment::PositionDetermination => {
                if fighting && self.gun_positions_needed(data) {
                    sealed(self, SealedStep::GunPositions, events);
                    return;
                }
            }
            Segment::Barrage => {
                if fighting && self.barrage_needed(data) {
                    sealed(self, SealedStep::Barrage, events);
                    return;
                }
            }
            Segment::RetreatBeforeAssault => {
                if fighting {
                    self.pending = Some(Pending::RetreatBeforeAssault {
                        side: self.phasing().expect("in phase G").other(),
                    });
                    events.push(Event::PhaseStarted { clock: self.clock });
                    return;
                }
            }
            Segment::ForceAssignment => {
                if fighting {
                    sealed(self, SealedStep::AssignForces, events);
                    return;
                }
            }
            Segment::AntiArmour => {
                self.resolve_anti_armour(data, events);
            }
            Segment::CloseAssault => {
                if self.close_assault_needed() {
                    events.push(Event::PhaseStarted { clock: self.clock });
                    return;
                }
                if self.end_close_assault(data, events) {
                    return;
                }
            }
            Segment::ReserveRelease => {
                self.combat = CombatSegment::default();
                events.push(Event::PhaseStarted { clock: self.clock });
                return;
            }
            Segment::Movement | Segment::Breakdown => unreachable!("not combat steps"),
        }
        events.push(Event::PhaseSkipped { clock: self.clock });
        let next = self
            .clock
            .next()
            .segment
            .expect("segments to reserve release");
        self.open_step(data, next, events);
    }

    /// A unit released from Reserve II earns 1 DP for any voluntary
    /// barrage, anti-armour fire or close assault (18.2x).
    fn reserve_ii_dp(&mut self, ids: &[PieceId], events: &mut Vec<Event>) {
        for id in ids {
            let p = self.pieces.get_mut(id).expect("assigned");
            if p.released == Some(Reserve::II) {
                p.cohesion.earn_dp(1);
                events.push(Event::Disorganised {
                    piece: id.clone(),
                    dp: 1,
                    level: p.cohesion.level,
                });
            }
        }
    }

    /// A sealed answer: checked against the state as it stands, kept,
    /// and acknowledged only as given; the step resolves and is revealed
    /// when the last answer is in.
    pub(crate) fn sealed(
        &mut self,
        data: &Data,
        player: Side,
        order: &Order,
    ) -> Result<Vec<Event>, Illegal> {
        let Some(Pending::Sealed { step, waiting }) = self.pending.clone() else {
            return Err(Illegal::new("11.0", "no sealed combat step is pending"));
        };
        match waiting.first() {
            Some(next) if *next == player => {}
            Some(next) => {
                return Err(Illegal::new(
                    "11.0",
                    format!("{next} answers before {player}"),
                ));
            }
            None => return Err(Illegal::new("11.0", "nobody is waiting to answer")),
        }
        match order {
            Order::GunPositions { forward } => {
                self.check_gun_positions(data, player, forward)?;
                self.combat.forward.extend(forward.iter().cloned());
            }
            Order::Barrage { fires } => {
                self.check_barrage(data, player, fires)?;
                self.combat.plots.insert(player, fires.clone());
            }
            Order::AssignForces { assignments } => {
                self.check_assignments(data, player, assignments)?;
                self.combat.assignments.insert(player, assignments.clone());
            }
            _ => unreachable!("matched by the caller"),
        }
        let mut events = vec![Event::Sealed { side: player, step }];
        let left: Vec<Side> = waiting[1..].to_vec();
        if !left.is_empty() {
            self.pending = Some(Pending::Sealed {
                step,
                waiting: left,
            });
            return Ok(events);
        }
        self.pending = None;
        self.reveal(data, step, &mut events);
        Ok(events)
    }

    /// Both answers are in: show them, resolve the step, open the next.
    fn reveal(&mut self, data: &Data, step: SealedStep, events: &mut Vec<Event>) {
        let next = match step {
            SealedStep::GunPositions => {
                for side in self.sealed_order() {
                    let forward: Vec<_> = self
                        .combat
                        .forward
                        .iter()
                        .filter(|id| self.pieces[*id].side == side)
                        .cloned()
                        .collect();
                    events.push(Event::GunPositionsSet { side, forward });
                }
                Segment::Barrage
            }
            SealedStep::Barrage => {
                for side in self.sealed_order() {
                    let fires = self.combat.plots.get(&side).cloned().unwrap_or_default();
                    let firers: Vec<PieceId> = fires.iter().map(|f| f.piece.clone()).collect();
                    events.push(Event::BarragePlotted { side, fires });
                    self.reserve_ii_dp(&firers, events);
                }
                self.resolve_barrages(data, events);
                Segment::RetreatBeforeAssault
            }
            SealedStep::AssignForces => {
                for side in self.sealed_order() {
                    let assignments = self
                        .combat
                        .assignments
                        .get(&side)
                        .cloned()
                        .unwrap_or_default();
                    let active: Vec<PieceId> = assignments
                        .iter()
                        .filter(|a| !a.anti_armour.is_empty() || !a.close_assault.is_empty())
                        .map(|a| a.piece.clone())
                        .collect();
                    events.push(Event::ForcesAssigned { side, assignments });
                    self.reserve_ii_dp(&active, events);
                }
                if let Some(phasing) = self.phasing() {
                    self.record_probes(phasing);
                }
                Segment::AntiArmour
            }
        };
        self.open_step(data, next, events);
    }

    /// `Pass` while retreat before assault is pending: the step is over.
    pub(crate) fn end_retreat_before_assault(
        &mut self,
        data: &Data,
    ) -> Result<Vec<Event>, Illegal> {
        self.pending = None;
        let mut events = Vec::new();
        self.open_step(data, Segment::ForceAssignment, &mut events);
        Ok(events)
    }

    /// `Pass` or `EndSegment` in the close assault step: every assigned
    /// assault must have been resolved (15.2x); then the must-attack
    /// rule is checked and reserve release opens.
    pub(crate) fn end_close_assault_step(&mut self, data: &Data) -> Result<Vec<Event>, Illegal> {
        if let Some(phasing) = self.phasing()
            && let Some(h) = self.unresolved_assaults(phasing).first()
        {
            return Err(Illegal::new(
                "15.2x",
                format!("the assault assigned against {h} has not been resolved"),
            ));
        }
        let mut events = Vec::new();
        if !self.end_close_assault(data, &mut events) {
            self.open_step(data, Segment::ReserveRelease, &mut events);
        }
        Ok(events)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::movement::test_support::{hex, moving};
    use crate::order::Order;
    use crate::piece::Location;

    /// Commonwealth phasing, GT1 OpStage 1: 1 KRRC, 4 RHA and 7 Med put
    /// at C4021 (Sollum) next to the 1st Libyan Division at C4020, and
    /// the movement segment ended so the combat steps open.
    pub fn at_sollum(data: &Data) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        for id in ["unit:cw:1-krrc", "unit:cw:4-rha", "unit:cw:7-med"] {
            state
                .pieces
                .get_mut(&crate::movement::test_support::pid(id))
                .unwrap()
                .location = Location::Hex(hex("C4021"));
        }
        state.apply(data, Side::Cw, Order::EndSegment).unwrap();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::at_sollum;
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Allocation, Assignment, Fire};

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    fn fire(piece: &str, points: u32, target: &str) -> Fire {
        Fire {
            piece: pid(piece),
            points,
            target: pid(target),
        }
    }

    fn assign(piece: &str, aa: &[(Option<&str>, u32)], ca: &[(Option<&str>, u32)]) -> Assignment {
        let alloc = |v: &[(Option<&str>, u32)]| {
            v.iter()
                .map(|(t, p)| Allocation {
                    target: t.map(hex),
                    points: *p,
                })
                .collect()
        };
        Assignment {
            piece: pid(piece),
            anti_armour: alloc(aa),
            close_assault: alloc(ca),
        }
    }

    #[test]
    fn gun_positions_are_sealed_until_both_are_in() {
        let data = graziani();
        let mut state = at_sollum(&data);
        assert_eq!(state.clock.segment, Some(Segment::PositionDetermination));
        assert_eq!(
            state.pending,
            Some(Pending::Sealed {
                step: SealedStep::GunPositions,
                waiting: vec![Side::Cw, Side::Axis],
            })
        );
        assert_eq!(state.to_act(), Some(Side::Cw));
        let before = snapshot(&state);
        // The Axis is not up; an infantry battalion takes no position;
        // another step's order is refused; a duplicate too.
        let err = state
            .apply(&data, Side::Axis, Order::GunPositions { forward: vec![] })
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:1-krrc")],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "12.1x");
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:4-rha"), pid("unit:cw:4-rha")],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "12.1x");
        let err = state
            .apply(&data, Side::Cw, Order::Barrage { fires: vec![] })
            .unwrap_err();
        assert_eq!(err.case, "11.0");
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "11.0");
        assert_eq!(snapshot(&state), before);
        // 4 RHA forward, 7 Med back: acknowledged, not revealed.
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:4-rha")],
                },
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::Sealed {
                side: Side::Cw,
                step: SealedStep::GunPositions
            }]
        );
        assert_eq!(state.to_act(), Some(Side::Axis));
        let events = state
            .apply(&data, Side::Axis, Order::GunPositions { forward: vec![] })
            .unwrap();
        assert_eq!(
            events[..3],
            [
                Event::Sealed {
                    side: Side::Axis,
                    step: SealedStep::GunPositions
                },
                Event::GunPositionsSet {
                    side: Side::Cw,
                    forward: vec![pid("unit:cw:4-rha")]
                },
                Event::GunPositionsSet {
                    side: Side::Axis,
                    forward: vec![]
                },
            ]
        );
        assert_eq!(state.clock.segment, Some(Segment::Barrage));
        assert_eq!(
            state.pending,
            Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting: vec![Side::Cw, Side::Axis],
            })
        );
    }

    #[test]
    fn a_barrage_plot_is_checked_against_the_board() {
        let data = graziani();
        let mut state = at_sollum(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:4-rha")],
                },
            )
            .unwrap();
        state
            .apply(&data, Side::Axis, Order::GunPositions { forward: vec![] })
            .unwrap();
        let before = snapshot(&state);
        let bn = "unit:it:viii-libyan-bn";
        let cases: Vec<(Vec<Fire>, &str)> = vec![
            // No barrage rating; no points; over the TOE.
            (vec![fire("unit:cw:1-krrc", 2, bn)], "12.0"),
            (vec![fire("unit:cw:4-rha", 0, bn)], "12.2x"),
            (
                vec![fire("unit:cw:4-rha", 4, bn), fire("unit:cw:4-rha", 3, bn)],
                "12.2x",
            ),
            // Own unit, no such unit, a unit not adjacent.
            (vec![fire("unit:cw:4-rha", 2, "unit:cw:1-krrc")], "12.2x"),
            (vec![fire("unit:cw:4-rha", 2, "unit:it:nobody")], "12.2x"),
            (
                vec![fire(
                    "unit:cw:4-rha",
                    2,
                    "unit:it:fort-maddalenna-garrison-bn",
                )],
                "12.31",
            ),
            // A back gun splits no fire, and shares no target with a
            // hex-mate (one barrage per firing hex per target).
            (
                vec![
                    fire("unit:cw:7-med", 3, bn),
                    fire("unit:cw:7-med", 3, "unit:it:ix-libyan-bn"),
                ],
                "12.1x",
            ),
            (
                vec![fire("unit:cw:7-med", 6, bn), fire("unit:cw:4-rha", 6, bn)],
                "12.32",
            ),
        ];
        for (fires, case) in cases {
            let err = state
                .apply(
                    &data,
                    Side::Cw,
                    Order::Barrage {
                        fires: fires.clone(),
                    },
                )
                .unwrap_err();
            assert_eq!(err.case, case, "{fires:?}: {}", err.reason);
            assert_eq!(snapshot(&state), before);
        }
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Barrage {
                    fires: vec![
                        fire("unit:cw:4-rha", 4, bn),
                        fire("unit:cw:7-med", 6, "unit:it:ix-libyan-bn"),
                    ],
                },
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::Sealed {
                side: Side::Cw,
                step: SealedStep::Barrage
            }]
        );
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn force_assignment_is_checked_for_both_players() {
        let data = graziani();
        let mut state = at_sollum(&data);
        crate::state::test_support::drain_combat(&data, &mut state);
        // The drain answers everything with nothing; rebuild to the
        // assignment step by hand.
        let mut state = at_sollum(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::GunPositions {
                    forward: vec![pid("unit:cw:4-rha")],
                },
            )
            .unwrap();
        state
            .apply(&data, Side::Axis, Order::GunPositions { forward: vec![] })
            .unwrap();
        state
            .apply(&data, Side::Cw, Order::Barrage { fires: vec![] })
            .unwrap();
        state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        assert_eq!(
            state.pending,
            Some(Pending::RetreatBeforeAssault { side: Side::Axis })
        );
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::ForceAssignment));
        assert_eq!(state.to_act(), Some(Side::Cw));
        let before = snapshot(&state);
        let c4020 = Some("C4020");
        let cases: Vec<(Vec<Assignment>, &str)> = vec![
            // The phasing player names a hex; an adjacent enemy one.
            (vec![assign("unit:cw:1-krrc", &[], &[(None, 6)])], "14.12"),
            (
                vec![assign("unit:cw:1-krrc", &[], &[(Some("C3921"), 6)])],
                "15.0",
            ),
            (
                vec![assign("unit:cw:1-krrc", &[], &[(Some("C4121"), 6)])],
                "15.0",
            ),
            // Over the TOE; no points; no anti-armour rating; a back gun.
            (vec![assign("unit:cw:1-krrc", &[], &[(c4020, 7)])], "14.0"),
            (vec![assign("unit:cw:1-krrc", &[], &[(c4020, 0)])], "14.0"),
            (vec![assign("unit:cw:1-krrc", &[(c4020, 2)], &[])], "14.0"),
            (vec![assign("unit:cw:7-med", &[], &[(c4020, 6)])], "12.1x"),
            // Listed twice; not the player's.
            (
                vec![
                    assign("unit:cw:1-krrc", &[], &[(c4020, 3)]),
                    assign("unit:cw:1-krrc", &[], &[(c4020, 3)]),
                ],
                "14.1x",
            ),
            (
                vec![assign("unit:it:viii-libyan-bn", &[], &[(c4020, 1)])],
                "14.1x",
            ),
        ];
        for (assignments, case) in cases {
            let err = state
                .apply(
                    &data,
                    Side::Cw,
                    Order::AssignForces {
                        assignments: assignments.clone(),
                    },
                )
                .unwrap_err();
            assert_eq!(err.case, case, "{assignments:?}: {}", err.reason);
            assert_eq!(snapshot(&state), before);
        }
        // 1 KRRC assaults, 4 RHA fires anti-armour with 2 points and
        // assaults with 4.
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![
                        assign("unit:cw:1-krrc", &[], &[(c4020, 6)]),
                        assign("unit:cw:4-rha", &[(c4020, 2)], &[(c4020, 4)]),
                    ],
                },
            )
            .unwrap();
        // The Axis defends its own hex: no target named.
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign("unit:it:viii-libyan-bn", &[], &[(Some("C4021"), 5)])],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "14.12");
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign("unit:it:viii-libyan-bn", &[], &[(None, 5)])],
                },
            )
            .unwrap();
        assert!(matches!(
            events[1],
            Event::ForcesAssigned { side: Side::Cw, ref assignments } if assignments.len() == 2
        ));
        assert!(matches!(
            events[2],
            Event::ForcesAssigned {
                side: Side::Axis,
                ..
            }
        ));
        // Anti-armour resolves on its own; close assault waits on the
        // phasing player.
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
        assert_eq!(state.pending, None);
        assert_eq!(state.to_act(), Some(Side::Cw));
        // Passing with the assault unresolved is refused.
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "15.2x");
    }

    #[test]
    fn a_released_unit_assaults_once_and_reserve_ii_pays_a_dp() {
        let data = graziani();
        let mut state = at_sollum(&data);
        // A second enemy hex next to both: XVII Libyan at C3921.
        state
            .pieces
            .get_mut(&pid("unit:it:xvii-libyan-bn"))
            .unwrap()
            .location = crate::piece::Location::Hex(hex("C3921"));
        state
            .pieces
            .get_mut(&pid("unit:cw:1-krrc"))
            .unwrap()
            .released = Some(Reserve::II);
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (Side::Axis, Order::GunPositions { forward: vec![] }),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(&data, side, order).unwrap();
        }
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-krrc",
                        &[],
                        &[(Some("C4020"), 3), (Some("C3921"), 3)],
                    )],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign("unit:cw:1-krrc", &[], &[(Some("C4020"), 3)])],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        assert!(events.contains(&Event::Disorganised {
            piece: pid("unit:cw:1-krrc"),
            dp: 1,
            level: -1,
        }));
    }
}
