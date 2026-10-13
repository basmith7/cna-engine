//! The movement-combat cycle of phase G (SPI 8.2x, 9.14, 18.1x): the
//! phasing player ends a movement segment with `EndSegment`, which
//! enforces the stacking ceiling (9.14), walks the clock through the
//! segments this slice does not model to reserve release, and from there
//! `EndSegment` opens the next cycle while `Pass` ends the phase.

use crate::clock::{Phase, Segment};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::UnitId;
use cna_data::scenario::Side;
use std::collections::BTreeSet;

/// SP of a formation stacked with its HQ (9.4): a division, or a
/// brigade equivalent.
const DIVISION_SP: u32 = 5;
const BRIGADE_SP: u32 = 2;

impl GameState {
    /// The unit this piece is under, as a piece, if that piece is on
    /// `hex`.
    fn parent_on<'a>(&'a self, piece: &Piece, hex: &HexId) -> Option<&'a Piece> {
        let parent = piece.parent.as_ref()?;
        let p = self.pieces.get(&PieceId::from(parent))?;
        (p.hex() == Some(hex)).then_some(p)
    }

    /// Whether any HQ is under this unit (a division has brigades or
    /// regiments under it; a brigade has only battalions, 9.21–9.24).
    fn has_sub_formation(&self, unit: &UnitId) -> bool {
        let mut heads = vec![unit.clone()];
        while let Some(head) = heads.pop() {
            for q in self.pieces.values() {
                if q.parent.as_ref() == Some(&head) {
                    if q.kind == PieceKind::Hq {
                        return true;
                    }
                    if let Some(u) = q.id.unit() {
                        heads.push(u);
                    }
                }
            }
        }
        false
    }

    /// A unit that takes no stacking space here (9.16): a garrison unit
    /// in the hex it garrisons, or an anti-air unit in a major city.
    fn stacks_free(&self, data: &Data, piece: &Piece, hex: &HexId) -> bool {
        if piece.garrison_of.as_ref() == Some(hex) {
            return true;
        }
        let major_city = data
            .map
            .hex(hex)
            .is_some_and(|h| h.terrain == cna_data::map::Terrain::MajorCity);
        major_city
            && piece
                .nation
                .zip(piece.id_code.as_deref())
                .and_then(|(n, code)| data.chars.row(n, code))
                .is_some_and(|r| r.unit_type.contains("Anti-air"))
    }

    /// Stacking points in a hex (9.1x): a unit stacked with an HQ it is
    /// under counts as part of that HQ's formation, a division 5 SP and a
    /// brigade equivalent 2 (9.13, 9.21, 9.4); an HQ alone is 0 (9.25);
    /// anything else its equivalent's value, or nothing by 9.16.
    pub(crate) fn stacked(&self, data: &Data, hex: &HexId) -> u32 {
        let in_hex: Vec<&Piece> = self
            .pieces
            .values()
            .filter(|p| p.hex() == Some(hex) && !self.stacks_free(data, p, hex))
            .collect();
        let mut formations: BTreeSet<PieceId> = BTreeSet::new();
        let mut sp = 0;
        for p in &in_hex {
            // Climb to the topmost HQ on this hex the piece is under.
            let mut top: Option<&Piece> = None;
            let mut cur = *p;
            while let Some(parent) = self.parent_on(cur, hex) {
                top = Some(parent);
                cur = parent;
            }
            match top {
                Some(hq) => {
                    formations.insert(hq.id.clone());
                }
                None if p.kind == PieceKind::Hq => {}
                None => sp += cna_rules::stacking::points(&data.stacking, p.equivalent),
            }
        }
        for hq in formations {
            let unit = self.pieces[&hq]
                .id
                .unit()
                .expect("an HQ comes from a sheet");
            sp += if self.has_sub_formation(&unit) {
                DIVISION_SP
            } else {
                BRIGADE_SP
            };
        }
        sp
    }

    /// Whether a hex is over its terrain's ceiling (9.14).
    pub(crate) fn over_ceiling_at(&self, data: &Data, hex: &HexId) -> Option<Illegal> {
        let terrain = data.map.hex(hex).expect("pieces stand on hexes").terrain;
        let ceiling = data.terrain.ceiling(terrain.name()).unwrap_or(0);
        let sp = self.stacked(data, hex);
        (sp > ceiling).then(|| {
            Illegal::new(
                "9.14",
                format!(
                    "{hex} holds {sp} SP, over the {} ceiling of {ceiling}",
                    terrain.name()
                ),
            )
        })
    }

    /// The first hex a piece moved into this segment, in id order, that
    /// is over its ceiling (9.14, 9.31). A hex the set-up overfilled is
    /// not a violation until something ends a move there.
    fn over_ceiling(&self, data: &Data) -> Option<Illegal> {
        self.entered
            .iter()
            .find_map(|hex| self.over_ceiling_at(data, hex))
    }

    /// Every hex over its ceiling, for tests.
    #[cfg(test)]
    pub(crate) fn overfilled(&self, data: &Data) -> Vec<HexId> {
        let hexes: BTreeSet<&HexId> = self.pieces.values().filter_map(Piece::hex).collect();
        hexes
            .into_iter()
            .filter(|h| self.over_ceiling_at(data, h).is_some())
            .cloned()
            .collect()
    }

    /// Whether an enemy combat unit of `side` stands within two hexes of
    /// `hex` (8.2x).
    fn enemy_combat_within_two(&self, data: &Data, hex: &HexId, side: Side) -> bool {
        let near = data.map.within(hex, 2);
        self.pieces.values().any(|p| {
            p.side != side
                && p.kind == PieceKind::Combat
                && p.hex().is_some_and(|h| near.contains(h))
        })
    }

    /// What ending a movement segment requires and records: the ceiling
    /// (9.14), and who may move in a later segment (8.2x).
    fn close_movement_segment(&mut self, data: &Data) -> Result<(), Illegal> {
        if let Some(e) = self.over_ceiling(data) {
            return Err(e);
        }
        let phasing = self.phasing();
        self.continuing = self
            .pieces
            .values()
            .filter(|p| Some(p.side) == phasing)
            .filter(|p| {
                p.hex()
                    .is_some_and(|h| self.enemy_combat_within_two(data, h, p.side))
            })
            .map(|p| p.id.clone())
            .collect();
        self.cycle += 1;
        self.entered.clear();
        Ok(())
    }

    /// Whether a piece may move in this movement segment under the
    /// two-hex rule (8.2x): always in the first segment of the phase,
    /// later only if it ended the last one within two hexes of an enemy
    /// combat unit.
    pub(crate) fn may_continue(&self, id: &PieceId) -> bool {
        self.cycle == 0 || self.continuing.contains(id)
    }

    /// `Order::EndSegment` in phase G.
    pub(crate) fn end_segment(&mut self, data: &Data) -> Result<Vec<Event>, Illegal> {
        let mut events = Vec::new();
        match (self.clock.phase, self.clock.segment) {
            (Phase::MovementCombat, Some(Segment::Movement)) => {
                self.close_movement_segment(data)?;
                self.clock = self.clock.next();
                while self.clock.segment != Some(Segment::ReserveRelease) {
                    events.push(Event::PhaseSkipped { clock: self.clock });
                    self.clock = self.clock.next();
                }
                events.push(Event::PhaseStarted { clock: self.clock });
            }
            (Phase::MovementCombat, Some(Segment::ReserveRelease)) => {
                self.close_release_segment(&mut events);
                self.clock.segment = Some(Segment::Movement);
                events.push(Event::PhaseStarted { clock: self.clock });
                self.open_movement_segment(data, &mut events);
            }
            _ => {
                return Err(Illegal::new(
                    "5.2",
                    format!("no segment to end at {}", self.clock),
                ));
            }
        }
        Ok(events)
    }

    /// What `Pass` must settle before phase G ends: from a movement
    /// segment the same as ending it; from a release segment the flip
    /// to Reserve II (18.2x).
    pub(crate) fn before_leaving_g(
        &mut self,
        data: &Data,
        events: &mut Vec<Event>,
    ) -> Result<(), Illegal> {
        match (self.clock.phase, self.clock.segment) {
            (Phase::MovementCombat, Some(Segment::Movement)) => {
                self.close_movement_segment(data)?;
            }
            (Phase::MovementCombat, Some(Segment::ReserveRelease)) => {
                self.close_release_segment(events);
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use cna_data::scenario::Side;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    #[test]
    fn a_division_with_its_hq_is_five_sp_and_an_hq_alone_is_nothing() {
        let data = graziani();
        let state = moving(&data, 3, Side::Axis);
        // Gruppo Maletti at C3617: a division HQ, three regiment HQs, eight
        // battalions, two artillery battalions: 5 SP as a formation.
        assert_eq!(state.stacked(&data, &hex("C3617")), 5);
        // The 1st CCNN at C4218 with its attached tank battalion: 5.
        assert_eq!(state.stacked(&data, &hex("C4218")), 5);
        // The 16th Infantry Brigade at E1829: 2.
        assert_eq!(state.stacked(&data, &hex("E1829")), 2);
        // Four independent battalions at C3922: 4.
        assert_eq!(state.stacked(&data, &hex("C3922")), 4);
        // Bardia (major city, ceiling 8): the Trivioli regiment (2); its
        // garrison sheet and the AA group take no space there (9.16).
        assert_eq!(state.stacked(&data, &hex("C4321")), 2);
        // The Fort Maddalena garrison battalion is from the minor-garrison
        // sheet, which names no hex: 1 SP like any battalion.
        assert_eq!(state.stacked(&data, &hex("C3019")), 1);
        // The set-up overfills Cairo (the 6th Australian Division and four
        // battalions, 9 in a major city's 8) and nothing else; that binds
        // only once a piece ends a move there (9.31).
        assert_eq!(state.overfilled(&data), [hex("E1730")]);
        assert_eq!(state.stacked(&data, &hex("E1730")), 9);
    }

    #[test]
    fn a_hex_over_the_ceiling_blocks_the_end_of_the_segment() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // The four battalions at C3922 (4 SP) join 1 RNF and 3 RHA at
        // C3721 (2 SP): 6, the clear ceiling. Mid-move nothing is checked.
        let stack = [
            "unit:cw:3-cldgds",
            "unit:cw:1-krrc",
            "unit:cw:4-rha",
            "unit:cw:7-med",
        ];
        state
            .apply(&data, Side::Cw, mv(&stack, &["C3821", "C3721"]))
            .unwrap();
        assert_eq!(state.stacked(&data, &hex("C3721")), 6);
        // 1 RTR (1 SP) from C3520 by track: 7.
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-rtr"], &["C3620", "C3721"]))
            .unwrap();
        assert_eq!(state.stacked(&data, &hex("C3721")), 7);
        let before = snapshot(&state);
        let err = state.apply(&data, Side::Cw, Order::EndSegment).unwrap_err();
        assert_eq!(err.case, "9.14");
        assert!(err.reason.contains("C3721"), "{}", err.reason);
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "9.14");
        assert_eq!(snapshot(&state), before);
        // Back out and the segment can end.
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-rtr"], &["C3620"]))
            .unwrap();
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
    }

    #[test]
    fn end_segment_walks_to_reserve_release_and_opens_the_next_cycle() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        assert_eq!(state.cycle, 0);
        let events = state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        let skipped: Vec<Segment> = events
            .iter()
            .filter_map(|e| match e {
                Event::PhaseSkipped { clock } => clock.segment,
                _ => None,
            })
            .collect();
        assert_eq!(
            skipped,
            [
                Segment::Breakdown,
                Segment::PositionDetermination,
                Segment::Barrage,
                Segment::RetreatBeforeAssault,
                Segment::ForceAssignment,
                Segment::AntiArmour,
                Segment::CloseAssault,
            ]
        );
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
        assert_eq!(state.cycle, 1);
        assert_eq!(state.to_act(), Some(Side::Cw));
        // No moving in reserve release.
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        // Another cycle.
        let events = state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        assert_eq!(events, [Event::PhaseStarted { clock: state.clock }]);
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        assert_eq!(state.clock.half, Some(crate::clock::Half::A));
        assert_eq!(state.cycle, 1);
        // Pass from reserve release ends the phase; Player B's F follows.
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
        assert_eq!(state.clock.half, Some(crate::clock::Half::B));
        assert_eq!(state.cycle, 0);
        // EndSegment outside G is nothing.
        let err = state
            .apply(&data, Side::Axis, Order::EndSegment)
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let _ = pid;
    }
}
