//! Zones of control and contact in the game state (SPI 10.1x, 10.2x,
//! 8.6x, R-003): which hexes could exert one, the non-phasing player's
//! declarations (10.16), the contact fixed at the start of every movement
//! segment (R-003), and the capture of non-combat units left alone in an
//! enemy ZOC (10.29).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceKind};
use crate::state::{GameState, Pending};
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::movement::{Step, step_cost};
use std::collections::BTreeSet;

/// What an enemy ZOC does to a hex, as far as is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Zoc {
    /// An enemy hex exerts a ZOC into it.
    Exerts,
    /// No enemy ZOC reaches it.
    Clear,
    /// These non-phasing hexes could exert one and have not said (10.16).
    Undeclared(Vec<HexId>),
}

impl GameState {
    /// The player working through phases F–L, if any.
    pub fn phasing(&self) -> Option<Side> {
        match self.clock.half {
            None => None,
            Some(crate::clock::Half::A) => self.player_a,
            Some(crate::clock::Half::B) => self.player_a.map(Side::other),
        }
    }

    /// Raw close-assault defence points in a hex (4.4x): TOE points
    /// times each combat unit's defence rating, tanks and guns by their
    /// weapon systems (4.47–4.49); a parenthesised rating counts only
    /// when no unit in the hex has a plain one.
    pub(crate) fn raw_defence(&self, data: &Data, hex: &HexId) -> u32 {
        self.pieces
            .values()
            .filter(|p| p.hex() == Some(hex) && p.kind == PieceKind::Combat)
            .filter(|p| !p.cohesion.collapsed(&self.ruleset))
            .map(|p| self.raw_of(data, p, crate::combat::strength::Kind::Defence))
            .sum()
    }

    /// Whether the units in a hex are enough to exert a ZOC (10.1x):
    /// more than 1 SP of combat units, not collapsed, with 10 raw
    /// defence points.
    pub(crate) fn could_exert(&self, data: &Data, hex: &HexId) -> bool {
        let sp: u32 = self
            .pieces
            .values()
            .filter(|p| p.hex() == Some(hex) && p.kind == PieceKind::Combat)
            .filter(|p| !p.cohesion.collapsed(&self.ruleset))
            .map(|p| cna_rules::stacking::points(&data.stacking, p.equivalent))
            .sum();
        cna_rules::zoc::exerts(sp, self.raw_defence(data, hex))
    }

    /// Whether a ZOC from `src` reaches the adjacent `hex` (10.21): not
    /// across a major river, lake or escarpment, nor into a hex no
    /// combat unit in `src` could enter from there.
    pub(crate) fn reaches(&self, data: &Data, src: &HexId, hex: &HexId) -> bool {
        let hexside = data.map.hexside(src, hex);
        if !cna_rules::zoc::crosses(hexside) {
            return false;
        }
        let (Some(from), Some(to)) = (data.map.hex(src), data.map.hex(hex)) else {
            return false;
        };
        let step = Step {
            from,
            to,
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(src, hex)),
            weather: self.weather_at(hex),
        };
        self.pieces
            .values()
            .filter(|p| p.hex() == Some(src) && p.kind == PieceKind::Combat)
            .any(|p| step_cost(&data.terrain, self.mobility(data, p), &step).is_ok())
    }

    /// The side holding a hex, if any.
    pub(crate) fn holder(&self, hex: &HexId) -> Option<Side> {
        self.pieces
            .values()
            .find(|p| p.hex() == Some(hex))
            .map(|p| p.side)
    }

    /// The enemy ZOC over `hex` for a unit of `side` (10.2x): the phasing
    /// player's eligible hexes always exert; the non-phasing player's
    /// only when declared (10.16).
    pub(crate) fn zoc_at(&self, data: &Data, hex: &HexId, side: Side) -> Zoc {
        let phasing = self.phasing();
        let mut undeclared = Vec::new();
        for n in data.map.neighbours(hex) {
            if self.holder(&n) != Some(side.other())
                || !self.could_exert(data, &n)
                || !self.reaches(data, &n, hex)
            {
                continue;
            }
            if phasing == Some(side.other()) {
                return Zoc::Exerts;
            }
            match self.zoc.get(&n) {
                Some(true) => return Zoc::Exerts,
                Some(false) => {}
                None => undeclared.push(n),
            }
        }
        if undeclared.is_empty() {
            Zoc::Clear
        } else {
            Zoc::Undeclared(undeclared)
        }
    }

    /// The enemy hexes whose declared ZOC covers `hex` for a unit of
    /// `side` (10.16, 10.3x): the non-phasing player's declarations, so
    /// a hex it was never asked about exerts nothing.
    pub(crate) fn zoc_sources(&self, data: &Data, hex: &HexId, side: Side) -> Vec<HexId> {
        data.map
            .neighbours(hex)
            .into_iter()
            .filter(|n| {
                self.holder(n) == Some(side.other())
                    && self.could_exert(data, n)
                    && self.reaches(data, n, hex)
                    && self.zoc.get(n) == Some(&true)
            })
            .collect()
    }

    /// Whether a combat unit of `side` other than those in `except`
    /// stands in `hex`: it cancels an enemy ZOC there for movement
    /// (10.26).
    pub(crate) fn friendly_combat_in(
        &self,
        hex: &HexId,
        side: Side,
        except: &[crate::piece::PieceId],
    ) -> bool {
        self.pieces.values().any(|p| {
            p.side == side
                && p.kind == PieceKind::Combat
                && p.hex() == Some(hex)
                && !except.contains(&p.id)
        })
    }

    /// Opens a movement segment: the declarations the non-phasing player
    /// owes for its hexes next to phasing units (10.16), else contact is
    /// fixed at once (R-003).
    pub(crate) fn open_movement_segment(&mut self, data: &Data, events: &mut Vec<Event>) {
        self.zoc.clear();
        self.moved.clear();
        self.scuttled.clear();
        self.shifted.clear();
        let Some(phasing) = self.phasing() else {
            return;
        };
        let mut hexes = BTreeSet::new();
        for p in self.pieces.values().filter(|p| p.side == phasing) {
            let Some(h) = p.hex() else { continue };
            for n in data.map.neighbours(h) {
                if self.holder(&n) == Some(phasing.other())
                    && self.could_exert(data, &n)
                    && self.reaches(data, &n, h)
                {
                    hexes.insert(n);
                }
            }
        }
        if hexes.is_empty() {
            self.fix_contact(data, events);
        } else {
            self.pending = Some(Pending::DeclareZoc {
                side: phasing.other(),
                hexes: hexes.into_iter().collect(),
                at_start: true,
            });
        }
    }

    /// `Order::DeclareZoc`: the hexes named exert, the rest of those
    /// asked about do not (10.16).
    pub(crate) fn declare_zoc(
        &mut self,
        data: &Data,
        player: Side,
        exerting: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        let Some(Pending::DeclareZoc {
            side,
            hexes,
            at_start,
        }) = self.pending.clone()
        else {
            return Err(Illegal::new("10.16", "no ZOC declaration is pending"));
        };
        if side != player {
            return Err(Illegal::new(
                "10.16",
                format!("{side} declares, not {player}"),
            ));
        }
        for h in exerting {
            if !hexes.contains(h) {
                return Err(Illegal::new(
                    "10.16",
                    format!("{h} is not one of the hexes asked about ({hexes:?})"),
                ));
            }
        }
        let mut events = Vec::new();
        for h in &hexes {
            let exerts = exerting.contains(h);
            self.zoc.insert(h.clone(), exerts);
            events.push(Event::ZocDeclared {
                hex: h.clone(),
                exerts,
            });
        }
        self.pending = None;
        if at_start {
            self.fix_contact(data, &mut events);
        }
        Ok(events)
    }

    /// Contact at the start of a movement segment (R-003): every unit in
    /// an enemy ZOC is in contact until it breaks off. A non-combat piece
    /// with no strength, alone in an enemy ZOC during the enemy's phase,
    /// is captured (10.29): removed, or under abstract supply a real
    /// supply unit goes over to the captor whole (32.1x).
    fn fix_contact(&mut self, data: &Data, events: &mut Vec<Event>) {
        let phasing = self.phasing();
        let ids: Vec<_> = self.pieces.keys().cloned().collect();
        for id in ids {
            let (hex, side) = {
                let p = &self.pieces[&id];
                let Some(h) = p.hex() else { continue };
                (h.clone(), p.side)
            };
            let contact = self.zoc_at(data, &hex, side) == Zoc::Exerts;
            let p = self.pieces.get_mut(&id).expect("listed");
            p.contact = contact;
            if contact
                && Some(side) != phasing
                && p.kind != PieceKind::Combat
                && p.toe == 0
                && !self.friendly_combat_in(&hex, side, &[])
            {
                let module = self.modules.abstract_supply;
                let p = self.pieces.get_mut(&id).expect("listed");
                p.contact = false;
                if module && p.kind == PieceKind::SupplyUnit {
                    // The captor may use it at once (32.1x), its points
                    // with it (32.5x).
                    p.side = side.other();
                } else {
                    p.location = Location::Eliminated;
                }
                events.push(Event::Captured {
                    piece: id.clone(),
                    hex: hex.clone(),
                });
            }
        }
    }

    /// After a stack left `from`: enemy units next to it stay in contact
    /// only while still in a ZOC of the mover's side (8.6x).
    pub(crate) fn release_contact_around(&mut self, data: &Data, from: &HexId, mover: Side) {
        let ids: Vec<_> = self
            .pieces
            .iter()
            .filter(|(_, p)| p.side != mover && p.contact)
            .filter(|(_, p)| p.hex().is_some_and(|h| data.map.adjacent(h, from)))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            let hex = self.pieces[&id].hex().expect("adjacent").clone();
            let side = self.pieces[&id].side;
            if self.zoc_at(data, &hex, side) != Zoc::Exerts {
                self.pieces.get_mut(&id).expect("listed").contact = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Segment;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use crate::piece::PieceId;
    use cna_data::cp::Cp;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    #[test]
    fn eligibility_counts_combat_sp_and_defence_points() {
        let data = graziani();
        let state = moving(&data, 3, Side::Cw);
        // The 1st Libyan Division at C4020: six battalions of 5 TOE at
        // defence 1 (30 points) and eight 75/27 guns at 1, 6 SP of combat
        // units.
        assert!(state.could_exert(&data, &hex("C4020")));
        assert_eq!(state.raw_defence(&data, &hex("C4020")), 38);
        // C3922: 3 Coldstream Guards and 1 KRRC, 6 TOE at 2 each, plus 4
        // RHA's six 18/25-pounders at 1 (7 Med's 60-pounders defend at 0).
        assert_eq!(state.raw_defence(&data, &hex("C3922")), 30);
        assert!(state.could_exert(&data, &hex("C3922")));
        // The Fort Maddalena garrison: 2 TOE at 3, one battalion: neither.
        assert_eq!(state.raw_defence(&data, &hex("C3019")), 6);
        assert!(!state.could_exert(&data, &hex("C3019")));
        // 11 Hussars alone at C3020: 8 x 3 = 24 points but 1 SP.
        assert!(!state.could_exert(&data, &hex("C3020")));
        // An empty hex.
        assert!(!state.could_exert(&data, &hex("C3923")));
    }

    #[test]
    fn reach_stops_at_escarpments_and_where_no_unit_could_enter() {
        let data = graziani();
        let state = moving(&data, 3, Side::Cw);
        // C4020's ZOC reaches C4021 (road, slope) and C3921 (track) but not
        // C4121 (escarpment).
        assert!(state.reaches(&data, &hex("C4020"), &hex("C4021")));
        assert!(state.reaches(&data, &hex("C4020"), &hex("C3921")));
        assert!(!state.reaches(&data, &hex("C4020"), &hex("C4121")));
        // Nothing enters the sea.
        assert!(!state.reaches(&data, &hex("C3922"), &hex("C4022")));
    }

    #[test]
    fn the_non_phasing_player_declares_before_a_unit_moves_adjacent() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        assert_eq!(state.pending, None);
        // 1 KRRC heads for C4021 (Sollum), next to the 1st Libyan Division
        // at C4020. The move stops short and the Axis must declare.
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc"], &["C4021", "C4121"]),
            )
            .unwrap();
        assert_eq!(
            state.pieces[&pid("unit:cw:1-krrc")].hex(),
            Some(&hex("C3922"))
        );
        assert_eq!(
            events,
            [Event::MoveInterrupted {
                pieces: vec![pid("unit:cw:1-krrc")],
                at: hex("C3922"),
                before: hex("C4021"),
                declare: vec![hex("C4020")],
            }]
        );
        assert_eq!(
            state.pending,
            Some(Pending::DeclareZoc {
                side: Side::Axis,
                hexes: vec![hex("C4020")],
                at_start: false,
            })
        );
        assert_eq!(state.to_act(), Some(Side::Axis));
        // Only the Axis may answer, and only about those hexes.
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C3920")],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "10.16");
        let err = state.apply(&data, Side::Axis, Order::Pass).unwrap_err();
        assert_eq!(err.case, "10.16");
        assert_eq!(snapshot(&state), before);
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C4020")],
                },
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::ZocDeclared {
                hex: hex("C4020"),
                exerts: true
            }]
        );
        assert_eq!(state.pending, None);
        assert_eq!(state.to_act(), Some(Side::Cw));
        // Entering the ZOC costs nothing and ends the move: a path on
        // past it is refused, and so is any later move this segment.
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc"], &["C4021", "C4121"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "10.2x");
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap();
        // The motorised guns at C4020 are offered a reaction (8.5x) and
        // the Axis declines.
        assert!(matches!(state.pending, Some(Pending::Reaction { .. })));
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        let krrc = &state.pieces[&pid("unit:cw:1-krrc")];
        assert_eq!(krrc.hex(), Some(&hex("C4021")));
        assert_eq!(krrc.cp_spent, Cp::ZERO);
        assert!(!krrc.contact, "not in contact until a segment starts");
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4121"]))
            .unwrap_err();
        assert_eq!(err.case, "10.2x");
        // Under a declared ZOC, 1 KRRC must attack C4020 or retreat
        // (10.3x): it probes with two points (basic −3, enough to hold
        // the hex off) for 2 CP. The next segment then opens with the
        // Axis asked again (the declarations are per segment), 1 KRRC is
        // in contact and pays 2 CP to leave, on top of the terrain.
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (Side::Axis, Order::GunPositions { forward: vec![] }),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
            (
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![crate::order::Assignment {
                        piece: pid("unit:cw:1-krrc"),
                        anti_armour: vec![],
                        close_assault: vec![crate::order::Allocation {
                            target: Some(hex("C4020")),
                            points: 2,
                        }],
                    }],
                },
            ),
            (
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            ),
            (
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            ),
        ] {
            state.apply(&data, side, order).unwrap();
        }
        if matches!(state.pending, Some(Pending::ForcedRetreat { .. })) {
            state.apply(&data, Side::Axis, Order::Pass).unwrap();
        }
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(2));
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        assert_eq!(
            state.pending,
            Some(Pending::DeclareZoc {
                side: Side::Axis,
                hexes: vec![hex("C4020")],
                at_start: true,
            })
        );
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C4020")],
                },
            )
            .unwrap();
        assert!(state.pieces[&pid("unit:cw:1-krrc")].contact);
        // ZOC to ZOC: C3921 is next to C4020 too (and to C3920, which need
        // not be asked: C4020 settles it).
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3921"]))
            .unwrap_err();
        assert_eq!(err.case, "10.2x");
        assert!(err.reason.contains("ZOC-to-ZOC"), "{}", err.reason);
        // Back down to C3922: 2 to break contact, then (2 + 4) / 2 down
        // the escarpment by the unfinished road.
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3922"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::ContactBroken {
                piece: pid("unit:cw:1-krrc"),
                cost: Cp::whole(2)
            }
        );
        let krrc = &state.pieces[&pid("unit:cw:1-krrc")];
        assert_eq!(krrc.cp_spent, Cp::whole(7));
        assert!(!krrc.contact);
    }

    #[test]
    fn a_friendly_combat_unit_cancels_the_zoc_for_entry() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C4020")],
                },
            )
            .unwrap();
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap();
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        // 3 Coldstream Guards pass through C4021 (1 KRRC is there) on to
        // C4121, which no ZOC reaches: up the escarpment by the unfinished
        // road, a track, (2 + 6) / 2, then 2. At C4021 the two battalions
        // exert a ZOC over C4020, so its guns may not react (8.5x); at
        // C4121 the 63rd Cirene's vehicles at C4120 are offered one.
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:3-cldgds"], &["C4021", "C4121"]),
            )
            .unwrap();
        assert!(matches!(
            events.last(),
            Some(Event::ReactionOffered { at, .. }) if at == &hex("C4121")
        ));
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        let guards = &state.pieces[&pid("unit:cw:3-cldgds")];
        assert_eq!(guards.hex(), Some(&hex("C4121")));
        assert_eq!(guards.cp_spent, Cp::whole(6));
    }

    #[test]
    fn a_non_combat_unit_may_not_enter_an_unoccupied_enemy_zoc() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let hq = pid("unit:cw:7th-armored-div-hq");
        state.pieces.get_mut(&hq).unwrap().location = Location::Hex(hex("C3922"));
        state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:7th-armored-div-hq"], &["C4021"]),
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C4020")],
                },
            )
            .unwrap();
        let before = snapshot(&state);
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:7th-armored-div-hq"], &["C4021"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "10.29");
        assert_eq!(snapshot(&state), before);
    }

    #[test]
    fn a_lone_supply_unit_in_an_enemy_zoc_is_captured_when_the_enemy_s_segment_opens() {
        let data = graziani();
        let mut state = crate::state::test_support::set_up(&data, 3);
        let supply = PieceId::supply(Side::Cw, 1);
        state.pieces.get_mut(&supply).unwrap().location = Location::Hex(hex("C4021"));
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        let events = state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        assert!(events.contains(&Event::Captured {
            piece: supply.clone(),
            hex: hex("C4021")
        }));
        assert_eq!(state.pieces[&supply].location, Location::Eliminated);
    }

    #[test]
    fn under_abstract_supply_a_captured_supply_unit_changes_sides_with_its_load_and_points_32_1x() {
        use crate::state::Modules;
        let data = graziani();
        let mut state = GameState::with_modules(&data, 3, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(&data, &mut state);
        let supply = PieceId::supply(Side::Cw, 1);
        let p = state.pieces.get_mut(&supply).unwrap();
        p.location = Location::Hex(hex("C4021"));
        p.motorised = 30;
        let load = p.load;
        assert!(load.is_some());
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        let mut events = Vec::new();
        while state.clock.segment != Some(Segment::Movement) {
            let side = state.to_act().unwrap();
            events.extend(state.apply(&data, side, Order::Pass).unwrap());
        }
        assert!(events.contains(&Event::Captured {
            piece: supply.clone(),
            hex: hex("C4021")
        }));
        let p = &state.pieces[&supply];
        assert_eq!(p.side, Side::Axis);
        assert_eq!(p.location, Location::Hex(hex("C4021")));
        assert_eq!(p.load, load);
        assert_eq!(p.motorised, 30);
    }
}
