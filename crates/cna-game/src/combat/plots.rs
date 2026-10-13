//! What a sealed answer may say (SPI 12.1x, 12.2x, 12.3x, 14.1x, 14.2x,
//! 15.2x, 15.36): each is checked against the state as it stands when
//! given, never against the other player's answer.

use crate::combat::strength::Kind;
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
            let p = self.own_piece(player, id, "12.1x")?;
            if p.kind != PieceKind::Combat || !self.is_gun_unit(data, p) {
                return Err(Illegal::new(
                    "12.1x",
                    format!("{id} is not a gun-class unit: only artillery stands forward or back"),
                ));
            }
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
            if self.raw_of(data, p, Kind::Barrage) == 0 {
                return Err(Illegal::new(
                    "12.0",
                    format!("{} has no barrage rating", f.piece),
                ));
            }
            let total = spent.entry(&f.piece).or_default();
            *total += f.points;
            if *total > p.toe {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{} fires {total} points but has {}", f.piece, p.toe),
                ));
            }
            let Some(t) = self.pieces.get(&f.target) else {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{}: no such target", f.target),
                ));
            };
            let (Some(from), Some(at)) = (p.hex(), t.hex()) else {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{} is not on the map", f.target),
                ));
            };
            if t.side == player || !self.is_target(data, t) {
                return Err(Illegal::new(
                    "12.2x",
                    format!("{} is not an enemy unit a barrage may target", f.target),
                ));
            }
            if !data.map.adjacent(from, at) {
                return Err(Illegal::new(
                    "12.31",
                    format!(
                        "{} at {at} is not adjacent to {} at {from}",
                        f.target, f.piece
                    ),
                ));
            }
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
            let p = self.own_piece(player, &a.piece, "14.1x")?;
            let from = p.hex().expect("checked").clone();
            if !matches!(p.kind, PieceKind::Combat | PieceKind::Hq) || p.toe == 0 {
                return Err(Illegal::new(
                    "15.1x",
                    format!("{} has no TOE points to commit", a.piece),
                ));
            }
            if self.combat.pinned.contains(&a.piece) {
                return Err(Illegal::new(
                    "12.44",
                    format!(
                        "{} is pinned: it may neither fire anti-armour nor close assault",
                        a.piece
                    ),
                ));
            }
            if self.combat.retreated.contains(&a.piece) {
                return Err(Illegal::new(
                    "13.28",
                    format!(
                        "{} retreated before assault: it adds nothing this segment",
                        a.piece
                    ),
                ));
            }
            if p.cohesion.collapsed(&self.ruleset) {
                return Err(Illegal::new(
                    "6.26",
                    format!(
                        "{} is at cohesion {}: it cannot attack or defend",
                        a.piece, p.cohesion.level
                    ),
                ));
            }
            if self.is_gun_unit(data, p) && !self.combat.forward.contains(&a.piece) {
                return Err(Illegal::new(
                    "12.1x",
                    format!(
                        "{} is back: a back gun neither fires anti-armour nor close assaults",
                        a.piece
                    ),
                ));
            }
            let total: u32 = a
                .anti_armour
                .iter()
                .chain(&a.close_assault)
                .map(|al| al.points)
                .sum();
            if total > p.toe {
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
                    (Some(h), true) => self.check_target_hex(data, p, &from, h)?,
                    (None, false) => {}
                }
            }
            for al in &a.anti_armour {
                if let Some(h) = &al.target {
                    self.check_anti_armour_hexside(data, &from, h)?;
                }
            }
            for al in &a.close_assault {
                if let Some(h) = &al.target {
                    self.check_assault_hexside(data, p, &from, h)?;
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

    /// 14.35: no anti-armour fire across a ridge without road or track,
    /// down a slope without road or track, or down an escarpment without
    /// a track.
    fn check_anti_armour_hexside(
        &self,
        data: &Data,
        from: &HexId,
        to: &HexId,
    ) -> Result<(), Illegal> {
        let hs = data.map.hexside(from, to);
        let has = |f| hs.is_some_and(|h| h.has(f));
        let climbs = hs.is_some_and(|h| h.climbs(from, to));
        let road_or_track = has(HexsideFeature::Road)
            || has(HexsideFeature::Track)
            || has(HexsideFeature::UnfinishedRoad);
        let track = has(HexsideFeature::Track) || has(HexsideFeature::UnfinishedRoad);
        let blocked = (has(HexsideFeature::Ridge) && !road_or_track)
            || (has(HexsideFeature::Slope) && !climbs && !road_or_track)
            || (has(HexsideFeature::Escarpment) && !climbs && !track);
        if blocked {
            return Err(Illegal::new(
                "14.35",
                format!("no anti-armour fire crosses the {from}|{to} hexside"),
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
