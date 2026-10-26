//! What a sealed answer may say (SPI 12.1x, 12.2x, 12.3x, 14.1x, 14.2x,
//! 15.2x, 15.36): each is checked against the state as it stands when
//! given, never against the other player's answer.

use crate::combat::strength::Kind;
use crate::combat::terrain::anti_armour_ban;
use crate::data::Data;
use crate::event::Illegal;
use crate::order::{Assignment, Fire};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::{HexId, HexsideFeature, Terrain};
use cna_data::scenario::Side;
use std::collections::{BTreeMap, BTreeSet};

impl GameState {
    /// A piece of the player's, on the map.
    fn own_piece(&self, player: Side, id: &PieceId, case: &'static str) -> Result<&Piece, Illegal> {
        let p = self
            .pieces
            .get(id)
            .ok_or_else(|| Illegal::new(case, format!("{id}: no such piece")))?;
        if p.side != player {
            return Err(Illegal::new(case, format!("{id} is not {player}'s")));
        }
        if p.hex().is_none() {
            return Err(Illegal::new(case, format!("{id} is not on the map")));
        }
        Ok(p)
    }

    fn no_duplicates<'a>(
        ids: impl Iterator<Item = &'a PieceId>,
        case: &'static str,
    ) -> Result<(), Illegal> {
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id) {
                return Err(Illegal::new(case, format!("{id} is listed twice")));
            }
        }
        Ok(())
    }

    /// Whether an HQ or combat unit is a barrage target (12.2x): a
    /// combat unit, or an HQ with points carrying a plain close-assault
    /// rating.
    fn is_target(&self, data: &Data, p: &Piece) -> bool {
        match p.kind {
            PieceKind::Combat => p.toe > 0,
            PieceKind::Hq => {
                p.toe > 0
                    && self
                        .parts(data, p)
                        .iter()
                        .any(|part| part.plain(Kind::Offence) || part.plain(Kind::Defence))
            }
            _ => false,
        }
    }

    /// Step 1 (12.1x): only gun-class units take a position.
    pub(crate) fn check_gun_positions(
        &self,
        data: &Data,
        player: Side,
        forward: &[PieceId],
    ) -> Result<(), Illegal> {
        Self::no_duplicates(forward.iter(), "12.1x")?;
        for id in forward {
            self.may_stand_forward(data, player, id)?;
        }
        Ok(())
    }

    /// Whether one of the player's units may stand forward (12.1x): a
    /// gun-class combat unit on the map.
    pub(crate) fn may_stand_forward(
        &self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<&Piece, Illegal> {
        let p = self.own_piece(player, id, "12.1x")?;
        if p.kind != PieceKind::Combat || !self.is_gun_unit(data, p) {
            return Err(Illegal::new(
                "12.1x",
                format!("{id} is not a gun-class unit: only artillery stands forward or back"),
            ));
        }
        Ok(p)
    }

    /// Whether one of the player's units may barrage at all (12.0): on
    /// the map with a barrage rating.
    pub(crate) fn barrage_firer(
        &self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<&Piece, Illegal> {
        let p = self.own_piece(player, id, "12.2x")?;
        if self.raw_of(data, p, Kind::Barrage) == 0 {
            return Err(Illegal::new("12.0", format!("{id} has no barrage rating")));
        }
        Ok(p)
    }

    /// Whether `firer` may barrage `target` (12.2x, 12.31): an enemy
    /// unit a barrage may target, on the map in an adjacent hex.
    pub(crate) fn barrage_target(
        &self,
        data: &Data,
        player: Side,
        firer: &Piece,
        target: &PieceId,
    ) -> Result<(), Illegal> {
        let Some(t) = self.pieces.get(target) else {
            return Err(Illegal::new("12.2x", format!("{target}: no such target")));
        };
        let (Some(from), Some(at)) = (firer.hex(), t.hex()) else {
            return Err(Illegal::new("12.2x", format!("{target} is not on the map")));
        };
        if t.side == player || !self.is_target(data, t) {
            return Err(Illegal::new(
                "12.2x",
                format!("{target} is not an enemy unit a barrage may target"),
            ));
        }
        if !data.map.adjacent(from, at) {
            return Err(Illegal::new(
                "12.31",
                format!("{target} at {at} is not adjacent to {} at {from}", firer.id),
            ));
        }
        Ok(())
    }

    /// Step 2 (12.2x, 12.3x): a unit with barrage points fires some of
    /// them at an enemy unit in an adjacent hex; a hex barrages a target
    /// once, so a back gun never shares a target with a hex-mate.
    pub(crate) fn check_barrage(
        &self,
        data: &Data,
        player: Side,
        fires: &[Fire],
    ) -> Result<(), Illegal> {
        let mut spent: BTreeMap<&PieceId, u32> = BTreeMap::new();
        // (firing hex, target) -> the firers; a back gun must be alone.
        let mut per_hex: BTreeMap<(&HexId, &PieceId), Vec<&PieceId>> = BTreeMap::new();
        for f in fires {
            let p = self.own_piece(player, &f.piece, "12.2x")?;
            if f.points == 0 {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{}: no points fired", f.piece),
                ));
            }
            self.barrage_firer(data, player, &f.piece)?;
            let total = spent.entry(&f.piece).or_default();
            *total = total.checked_add(f.points).unwrap_or(u32::MAX);
            if *total > p.toe {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{} fires {total} points but has {}", f.piece, p.toe),
                ));
            }
            self.barrage_target(data, player, p, &f.target)?;
            let from = p.hex().expect("checked");
            per_hex.entry((from, &f.target)).or_default().push(&f.piece);
        }
        for ((hex, target), firers) in per_hex {
            let backs: Vec<_> = firers
                .iter()
                .filter(|id| !self.combat.forward.contains(*id))
                .collect();
            let distinct: BTreeSet<_> = firers.iter().collect();
            if !backs.is_empty() && distinct.len() > 1 {
                return Err(Illegal::new(
                    "12.32",
                    format!(
                        "{hex} barrages {target} once: back gun {} may not share the target with a hex-mate",
                        backs[0]
                    ),
                ));
            }
        }
        // A back gun fires at one target only (12.1x).
        let mut targets: BTreeMap<&PieceId, BTreeSet<&PieceId>> = BTreeMap::new();
        for f in fires {
            targets.entry(&f.piece).or_default().insert(&f.target);
        }
        for (id, ts) in targets {
            if ts.len() > 1
                && !self.combat.forward.contains(id)
                && self.is_gun_unit(data, &self.pieces[id])
            {
                return Err(Illegal::new(
                    "12.1x",
                    format!("{id} is back: it may not split its fire across targets"),
                ));
            }
        }
        Ok(())
    }

    /// Step 4 (14.1x, 14.2x, 15.2x): each unit splits its points between
    /// the roles, the phasing player's against adjacent enemy hexes.
    pub(crate) fn check_assignments(
        &self,
        data: &Data,
        player: Side,
        assignments: &[Assignment],
    ) -> Result<(), Illegal> {
        Self::no_duplicates(assignments.iter().map(|a| &a.piece), "14.1x")?;
        let phasing = self.phasing() == Some(player);
        for a in assignments {
            let p = self.may_commit(data, player, &a.piece)?;
            let from = p.hex().expect("checked").clone();
            let total: u64 = a
                .anti_armour
                .iter()
                .chain(&a.close_assault)
                .map(|al| u64::from(al.points))
                .sum();
            if total > u64::from(p.toe) {
                return Err(Illegal::new(
                    "14.0",
                    format!("{} commits {total} points but has {}", a.piece, p.toe),
                ));
            }
            if a.anti_armour
                .iter()
                .chain(&a.close_assault)
                .any(|al| al.points == 0)
            {
                return Err(Illegal::new(
                    "14.0",
                    format!("{}: an allocation of no points", a.piece),
                ));
            }
            if !a.anti_armour.is_empty() && self.raw_of(data, p, Kind::AntiArmour) == 0 {
                return Err(Illegal::new(
                    "14.0",
                    format!("{} has no anti-armour rating", a.piece),
                ));
            }
            for al in a.anti_armour.iter().chain(&a.close_assault) {
                match (&al.target, phasing) {
                    (None, true) => {
                        return Err(Illegal::new(
                            "14.12",
                            format!(
                                "{}: the phasing player names the hex it fires at or assaults",
                                a.piece
                            ),
                        ));
                    }
                    (Some(h), false) => {
                        return Err(Illegal::new(
                            "14.12",
                            format!(
                                "{}: the non-phasing player defends its own hex, not {h}",
                                a.piece
                            ),
                        ));
                    }
                    (Some(_), true) | (None, false) => {}
                }
            }
            for al in &a.anti_armour {
                if let Some(h) = &al.target {
                    self.anti_armour_target(data, p, &from, h)?;
                }
            }
            for al in &a.close_assault {
                if let Some(h) = &al.target {
                    self.assault_target(data, p, &from, h)?;
                }
            }
            // A unit released from reserve makes one assault at most for
            // the rest of the stage (18.23, 18.24).
            if let Some(released) = p.released {
                let targets: BTreeSet<&HexId> = a
                    .close_assault
                    .iter()
                    .filter_map(|al| al.target.as_ref())
                    .collect();
                if p.released_assaulted && !targets.is_empty() {
                    return Err(Illegal::new(
                        "18.2x",
                        format!(
                            "{} was released from Reserve {} and has made its one close assault this stage",
                            a.piece, released
                        ),
                    ));
                }
                if targets.len() > 1 {
                    return Err(Illegal::new(
                        "18.2x",
                        format!(
                            "{} was released from Reserve {}: one close assault at most",
                            a.piece, released
                        ),
                    ));
                }
            }
            // Several hexes from one unit: each adjacent to the unit's
            // hex and to each other (15.21).
            let targets: Vec<&HexId> = a
                .close_assault
                .iter()
                .filter_map(|al| al.target.as_ref())
                .collect();
            for (i, x) in targets.iter().enumerate() {
                for y in &targets[i + 1..] {
                    if x != y && !data.map.adjacent(x, y) {
                        return Err(Illegal::new(
                            "15.21",
                            format!(
                                "{} assaults {x} and {y}, which are not adjacent to each other",
                                a.piece
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether one of the player's units may commit points in step 4 at
    /// all (14.1x, 15.1x, 12.44, 13.28, 6.26, 12.1x): a combat unit or
    /// HQ on the map with TOE, not pinned, not retreated this segment,
    /// not collapsed, and not a back gun.
    pub(crate) fn may_commit(
        &self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<&Piece, Illegal> {
        let p = self.own_piece(player, id, "14.1x")?;
        if !matches!(p.kind, PieceKind::Combat | PieceKind::Hq) || p.toe == 0 {
            return Err(Illegal::new(
                "15.1x",
                format!("{id} has no TOE points to commit"),
            ));
        }
        if self.combat.pinned.contains(id) {
            return Err(Illegal::new(
                "12.44",
                format!("{id} is pinned: it may neither fire anti-armour nor close assault"),
            ));
        }
        if self.combat.retreated.contains(id) {
            return Err(Illegal::new(
                "13.28",
                format!("{id} retreated before assault: it adds nothing this segment"),
            ));
        }
        if p.cohesion.collapsed(&self.ruleset) {
            return Err(Illegal::new(
                "6.26",
                format!(
                    "{id} is at cohesion {}: it cannot attack or defend",
                    p.cohesion.level
                ),
            ));
        }
        if self.is_gun_unit(data, p) && !self.combat.forward.contains(id) {
            return Err(Illegal::new(
                "12.1x",
                format!("{id} is back: a back gun neither fires anti-armour nor close assaults"),
            ));
        }
        Ok(p)
    }

    /// Whether the phasing unit `p` at `from` may fire anti-armour at
    /// `h` (15.0, 14.34).
    pub(crate) fn anti_armour_target(
        &self,
        data: &Data,
        p: &Piece,
        from: &HexId,
        h: &HexId,
    ) -> Result<(), Illegal> {
        self.check_target_hex(data, p, from, h)?;
        self.check_anti_armour_hexside(data, from, h)
    }

    /// Whether the phasing unit `p` at `from` may close assault `h`
    /// (15.0, 15.36).
    pub(crate) fn assault_target(
        &self,
        data: &Data,
        p: &Piece,
        from: &HexId,
        h: &HexId,
    ) -> Result<(), Illegal> {
        self.check_target_hex(data, p, from, h)?;
        self.check_assault_hexside(data, p, from, h)
    }

    /// An adjacent enemy-held hex.
    fn check_target_hex(
        &self,
        data: &Data,
        p: &Piece,
        from: &HexId,
        h: &HexId,
    ) -> Result<(), Illegal> {
        if !data.map.adjacent(from, h) {
            return Err(Illegal::new(
                "15.0",
                format!("{h} is not adjacent to {} at {from}", p.id),
            ));
        }
        if !self
            .pieces
            .values()
            .any(|q| q.side != p.side && q.hex() == Some(h))
        {
            return Err(Illegal::new("15.0", format!("{h} holds no enemy unit")));
        }
        Ok(())
    }

    /// 14.34: no anti-armour fire crosses a ridge or goes down a slope
    /// without a road or track, down an escarpment without a track, or
    /// up an escarpment at all. The ban binds the non-phasing side's
    /// return fire too (`resolve_anti_armour`).
    fn check_anti_armour_hexside(
        &self,
        data: &Data,
        from: &HexId,
        to: &HexId,
    ) -> Result<(), Illegal> {
        let ban = data
            .map
            .hexside(from, to)
            .and_then(|side| anti_armour_ban(side, from, to));
        if let Some(ban) = ban {
            return Err(Illegal::new(
                "14.34",
                format!("no anti-armour fire from {from} into {to}, {ban}"),
            ));
        }
        Ok(())
    }

    /// 15.36: a motorised unit never assaults up an escarpment or into
    /// salt marsh.
    fn check_assault_hexside(
        &self,
        data: &Data,
        p: &Piece,
        from: &HexId,
        to: &HexId,
    ) -> Result<(), Illegal> {
        if !self.mobility(data, p).motorised() {
            return Ok(());
        }
        let hs = data.map.hexside(from, to);
        if hs.is_some_and(|h| h.has(HexsideFeature::Escarpment) && h.climbs(from, to)) {
            return Err(Illegal::new(
                "15.36",
                format!(
                    "{}: a motorised unit never assaults up the escarpment into {to}",
                    p.id
                ),
            ));
        }
        if data
            .map
            .hex(to)
            .is_some_and(|h| h.terrain == Terrain::SaltMarsh)
        {
            return Err(Illegal::new(
                "15.36",
                format!(
                    "{}: a motorised unit never assaults into salt marsh at {to}",
                    p.id
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment::test_support::at_sollum;
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::Allocation;

    #[test]
    fn anti_armour_fire_across_a_banned_hexside_is_refused_at_assignment() {
        // 14.34. A3933|A4033: an escarpment with no track, A4033 above;
        // banned climbed and descended. C4020|C4121: an escarpment with
        // a track, C4020 above; open down it, banned up it. C4213|C4313:
        // a bare ridge. C4021|C4020: up a slope by road, never banned.
        let data = Data::load("grazianis-offensive").unwrap();
        let state = at_sollum(&data);
        let check =
            |from: &str, to: &str| state.check_anti_armour_hexside(&data, &hex(from), &hex(to));
        let up = check("A3933", "A4033").unwrap_err();
        assert_eq!(up.case, "14.34");
        assert!(up.reason.contains("escarpment"), "{}", up.reason);
        let down = check("A4033", "A3933").unwrap_err();
        assert_eq!(down.case, "14.34");
        assert!(down.reason.contains("no track"), "{}", down.reason);
        assert!(check("C4020", "C4121").is_ok());
        assert_eq!(check("C4121", "C4020").unwrap_err().case, "14.34");
        let ridge = check("C4213", "C4313").unwrap_err();
        assert_eq!(ridge.case, "14.34");
        assert!(ridge.reason.contains("ridge"), "{}", ridge.reason);
        assert!(check("C4021", "C4020").is_ok());
    }

    #[test]
    fn a_motorised_unit_never_assaults_up_an_escarpment_or_into_salt_marsh() {
        // 15.36, judged hexside by hexside: A3933|A4033 is an escarpment
        // with A4033 above. Foot units are not bound.
        let data = Data::load("grazianis-offensive").unwrap();
        let state = at_sollum(&data);
        let (motor, foot): (Vec<&Piece>, Vec<&Piece>) = state
            .pieces
            .values()
            .filter(|p| p.kind == PieceKind::Combat && state.cpa(p).is_some())
            .partition(|p| state.mobility(&data, p).motorised());
        let (motor, foot) = (motor[0], foot[0]);
        let check = |p: &Piece, from: &str, to: &str| {
            state.check_assault_hexside(&data, p, &hex(from), &hex(to))
        };
        let up = check(motor, "A3933", "A4033").unwrap_err();
        assert_eq!(up.case, "15.36");
        assert!(up.reason.contains("escarpment"), "{}", up.reason);
        assert!(check(motor, "A4033", "A3933").is_ok());
        assert!(check(foot, "A3933", "A4033").is_ok());
        let marsh = data
            .map
            .hexes()
            .find(|h| h.terrain == Terrain::SaltMarsh)
            .unwrap();
        let next = data.map.neighbours(&marsh.id)[0].clone();
        let into = check(motor, next.as_str(), marsh.id.as_str()).unwrap_err();
        assert_eq!(into.case, "15.36");
        assert!(into.reason.contains("salt marsh"), "{}", into.reason);
        assert!(check(foot, next.as_str(), marsh.id.as_str()).is_ok());
    }

    #[test]
    fn a_collapsed_unit_may_not_be_assigned_to_an_assault() {
        // 6.26: at the collapse level a unit neither attacks nor defends.
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_sollum(&data);
        let krrc = pid("unit:cw:1-krrc");
        let assignment = || Assignment {
            piece: krrc.clone(),
            anti_armour: vec![],
            close_assault: vec![Allocation {
                target: Some(hex("C4020")),
                points: 2,
            }],
        };
        state
            .check_assignments(&data, Side::Cw, &[assignment()])
            .unwrap();
        state.pieces.get_mut(&krrc).unwrap().cohesion.level = -cna_rules::cohesion::COLLAPSE;
        let err = state
            .check_assignments(&data, Side::Cw, &[assignment()])
            .unwrap_err();
        assert_eq!(err.case, "6.26");
    }

    #[test]
    fn a_plot_whose_points_overflow_is_illegal_not_a_panic() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = at_sollum(&data);
        let fire = |points| Fire {
            piece: pid("unit:cw:4-rha"),
            points,
            target: pid("unit:it:viii-libyan-bn"),
        };
        let err = state
            .check_barrage(&data, Side::Cw, &[fire(6), fire(u32::MAX - 5)])
            .unwrap_err();
        assert_eq!(err.case, "12.2x");
        let huge = Allocation {
            target: Some(hex("C4020")),
            points: 1 << 31,
        };
        let err = state
            .check_assignments(
                &data,
                Side::Cw,
                &[Assignment {
                    piece: pid("unit:cw:1-krrc"),
                    anti_armour: vec![],
                    close_assault: vec![huge.clone(), huge],
                }],
            )
            .unwrap_err();
        assert_eq!(err.case, "14.0");
    }
}
