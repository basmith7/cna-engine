//! Raw strength of pieces (SPI 11.3x, 4.46–4.49): every TOE point carries
//! the ratings of its row, the ID code's for points the sheet lists no
//! weapon for and the weapon system's for the rest; raw points are
//! points times the rating, summed. Parenthesised ratings count only
//! when no unit in the hex has a plain one of that kind (14.15, 15.1x).

use crate::data::Data;
use crate::piece::{Piece, PieceKind};
use crate::state::GameState;
use anyhow::{Result, bail};
use cna_data::characteristics::{Mark, Rating};
use cna_data::map::HexId;
use cna_data::weapons::WeaponClass;

/// A rated activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Barrage,
    AntiArmour,
    Offence,
    Defence,
}

/// The class a barrage reads its target by (12.4x).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Infantry,
    Armour,
    Gun,
}

impl Class {
    /// The key `barrage-results.json` uses.
    pub fn key(self) -> &'static str {
        match self {
            Class::Infantry => "infantry",
            Class::Armour => "armor",
            Class::Gun => "gun",
        }
    }
}

/// Some TOE points of one piece that share a row of ratings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub points: u32,
    pub class: Class,
    /// A tank proper (15.4): not recce, not a self-propelled gun.
    pub tank: bool,
    /// Anti-aircraft guns: exempt from vulnerability losses (15.84).
    pub flak: bool,
    pub barrage: Option<Rating>,
    pub anti_armour: Option<Rating>,
    pub offence: Option<Rating>,
    pub defence: Option<Rating>,
    /// Per point; 0 for an unarmoured point.
    pub protection: u32,
    /// Per point; 0 where the row prints none.
    pub vulnerability: u32,
}

impl Part {
    pub fn rating(&self, kind: Kind) -> Option<&Rating> {
        match kind {
            Kind::Barrage => self.barrage.as_ref(),
            Kind::AntiArmour => self.anti_armour.as_ref(),
            Kind::Offence => self.offence.as_ref(),
            Kind::Defence => self.defence.as_ref(),
        }
    }

    /// The rating as a number per point: a parenthesised one only when
    /// allowed, any other mark read as its number.
    pub fn per_point(&self, kind: Kind, paren_ok: bool) -> u32 {
        let Some(r) = self.rating(kind) else { return 0 };
        if r.mark == Some(Mark::Parenthesised) && !paren_ok {
            return 0;
        }
        u32::try_from(r.value.unwrap_or(0)).unwrap_or(0)
    }

    pub fn plain(&self, kind: Kind) -> bool {
        self.rating(kind)
            .is_some_and(|r| r.mark != Some(Mark::Parenthesised) && r.value.unwrap_or(0) > 0)
    }

    pub fn armoured(&self) -> bool {
        self.protection > 0
    }

    pub fn gun(&self) -> bool {
        self.class == Class::Gun
    }
}

fn per_point(r: Option<&Rating>) -> u32 {
    r.and_then(|r| r.value)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

/// The rows a piece's points are rated by: the ID code's for the points
/// no weapon is listed for, a weapon row for each listed entry. Fails
/// on a weapon name that is neither in `weapon-systems.json` nor an ID
/// code of the nation.
pub fn parts(data: &Data, piece: &Piece, game_turn: u8) -> Result<Vec<Part>> {
    let mut out = Vec::new();
    let (Some(nation), Some(code)) = (piece.nation, piece.id_code.as_deref()) else {
        return Ok(out);
    };
    let row = data.chars.row(nation, code);
    let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
    let unlisted = piece.toe.saturating_sub(listed);
    let from_row = |points: u32, r: &cna_data::characteristics::Row| {
        let t = r.unit_type.as_str();
        let gun = t.contains("Artillery") || t.contains("Anti-tank") || t.contains("Anti-air");
        let protection = per_point(r.armour_protection.as_ref());
        Part {
            points,
            class: if gun {
                Class::Gun
            } else if protection > 0 || t.contains("Tank") {
                Class::Armour
            } else {
                Class::Infantry
            },
            tank: t.contains("Tank"),
            flak: t.contains("Anti-air"),
            barrage: r.barrage.clone(),
            anti_armour: r.anti_armour.clone(),
            offence: r.close_assault_offence.clone(),
            defence: r.close_assault_defence.clone(),
            protection,
            vulnerability: per_point(r.vulnerability.as_ref()),
        }
    };
    if unlisted > 0
        && let Some(r) = row
    {
        out.push(from_row(unlisted, r));
    }
    for w in &piece.weapons {
        if w.points == 0 {
            continue;
        }
        match data.weapons.resolve(nation, &w.weapon) {
            Some(r) => out.push(Part {
                points: w.points,
                class: match r.class {
                    WeaponClass::Tank => Class::Armour,
                    _ => Class::Gun,
                },
                tank: r.class == WeaponClass::Tank,
                flak: r.class == WeaponClass::AntiAir,
                barrage: r.barrage.clone(),
                anti_armour: r.anti_armour_at(game_turn).cloned(),
                offence: r.close_assault_offence.clone(),
                defence: r.close_assault_defence.clone(),
                protection: per_point(r.armour_protection.as_ref()),
                vulnerability: per_point(r.vulnerability.as_ref()),
            }),
            None => match data.chars.row(nation, &w.weapon) {
                Some(r) => out.push(from_row(w.points, r)),
                None => bail!(
                    "{}: {:?} is neither a weapon system nor a {nation} ID code",
                    piece.id,
                    w.weapon
                ),
            },
        }
    }
    Ok(out)
}

/// Raw points in `kind` of `points` TOE points of a piece, drawn from
/// its parts in order (the first part first).
pub fn raw(parts: &[Part], kind: Kind, points: u32, paren_ok: bool) -> u32 {
    let mut left = points;
    let mut sum = 0;
    for p in parts {
        let take = p.points.min(left);
        sum += take * p.per_point(kind, paren_ok);
        left -= take;
        if left == 0 {
            break;
        }
    }
    sum
}

impl GameState {
    /// A piece's rated parts; a piece whose weapons were checked at
    /// set-up never fails.
    pub fn parts(&self, data: &Data, piece: &Piece) -> Vec<Part> {
        parts(data, piece, self.clock.game_turn).unwrap_or_default()
    }

    /// Whether a combat unit in the hex has a plain rating of this kind,
    /// which bars every parenthesised one there (14.15, 15.1x).
    pub(crate) fn plain_in_hex(&self, data: &Data, hex: &HexId, kind: Kind) -> bool {
        self.pieces
            .values()
            .filter(|p| p.hex() == Some(hex) && p.kind == PieceKind::Combat)
            .any(|p| self.parts(data, p).iter().any(|part| part.plain(kind)))
    }

    /// Raw points of a piece's whole TOE in `kind`, the parenthesised
    /// rule judged by its hex.
    pub fn raw_of(&self, data: &Data, piece: &Piece, kind: Kind) -> u32 {
        let paren_ok = piece
            .hex()
            .is_some_and(|h| !self.plain_in_hex(data, h, kind));
        raw(&self.parts(data, piece), kind, piece.toe, paren_ok)
    }

    /// The class a barrage against this piece reads (12.4x): armour if
    /// any point is armoured, a gun if any is a gun, else infantry.
    pub fn class_of(&self, data: &Data, piece: &Piece) -> Class {
        self.parts(data, piece)
            .iter()
            .map(|p| p.class)
            .max()
            .unwrap_or(Class::Infantry)
    }

    /// A gun-class unit (12.1x): every rated point is a gun.
    pub fn is_gun_unit(&self, data: &Data, piece: &Piece) -> bool {
        let parts = self.parts(data, piece);
        !parts.is_empty() && parts.iter().all(Part::gun)
    }

    /// Checks every piece's weapons resolve, for a new game.
    pub(crate) fn check_weapons(&self, data: &Data) -> Result<()> {
        for p in self.pieces.values() {
            parts(data, p, self.clock.game_turn)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::state::test_support::set_up;
    use cna_rules::strength::actual;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    #[test]
    fn three_graziani_units_by_hand() {
        let data = graziani();
        let state = set_up(&data, 3);
        // 1 KRRC: ID code n, 6 points, offence 1, defence 2.
        let krrc = &state.pieces[&pid("unit:cw:1-krrc")];
        assert_eq!(state.raw_of(&data, krrc, Kind::Offence), 6);
        assert_eq!(state.raw_of(&data, krrc, Kind::Defence), 12);
        assert_eq!(state.raw_of(&data, krrc, Kind::Barrage), 0);
        assert_eq!(state.class_of(&data, krrc), Class::Infantry);
        assert!(!state.is_gun_unit(&data, krrc));
        // 4 RHA: six 18/25-pounders, barrage 8, anti-armour 2,
        // vulnerability 5: 48 raw barrage is 5 actual.
        let rha = &state.pieces[&pid("unit:cw:4-rha")];
        assert_eq!(rha.toe, 6);
        let parts = state.parts(&data, rha);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].vulnerability, 5);
        assert_eq!(state.raw_of(&data, rha, Kind::Barrage), 48);
        assert_eq!(actual(state.raw_of(&data, rha, Kind::Barrage)), 5);
        assert_eq!(state.raw_of(&data, rha, Kind::AntiArmour), 12);
        assert_eq!(state.raw_of(&data, rha, Kind::Defence), 6);
        assert_eq!(state.class_of(&data, rha), Class::Gun);
        assert!(state.is_gun_unit(&data, rha));
        // LXII(L) tank battalion: nine CV33, offence 1, defence 2,
        // protection 1, anti-armour 0.
        let tanks = &state.pieces[&pid("unit:it:lxiil-tank-bn")];
        let parts = state.parts(&data, tanks);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].tank && parts[0].armoured());
        assert_eq!(parts[0].protection, 1);
        assert_eq!(state.raw_of(&data, tanks, Kind::Offence), 9);
        assert_eq!(state.raw_of(&data, tanks, Kind::Defence), 18);
        assert_eq!(state.raw_of(&data, tanks, Kind::AntiArmour), 0);
        assert_eq!(state.class_of(&data, tanks), Class::Armour);
        // Points are drawn from the parts in order.
        assert_eq!(raw(&parts, Kind::Defence, 4, false), 8);
    }

    #[test]
    fn a_parenthesised_rating_counts_only_without_a_plain_one() {
        let data = graziani();
        let mut state = set_up(&data, 3);
        // The 1st Libyan Division HQ (ID code b: defence (1), 1 point)
        // at C4020 with six battalions: nothing. Alone: 1.
        let hq = pid("unit:it:1st-libyan-infantry-hq");
        let h = state.pieces[&hq].clone();
        assert_eq!(h.hex(), Some(&hex("C4020")));
        assert!(h.toe > 0, "{}", h.toe);
        assert_eq!(state.raw_of(&data, &h, Kind::Defence), 0);
        let others: Vec<_> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex("C4020")) && p.id != hq)
            .map(|p| p.id.clone())
            .collect();
        for id in others {
            state.pieces.get_mut(&id).unwrap().location = crate::piece::Location::Eliminated;
        }
        assert_eq!(state.raw_of(&data, &h, Kind::Defence), h.toe);
    }

    #[test]
    fn every_oa_weapon_name_resolves() {
        let data = graziani();
        let state = set_up(&data, 3);
        state.check_weapons(&data).unwrap();
        // The whole OA, not just the scenario: a name is a weapon system
        // or an ID code (the RECAM HQ's `vv`).
        for u in data.oa.units() {
            for w in &u.toe {
                let nation = u.id.nation();
                assert!(
                    data.weapons.resolve(nation, &w.weapon).is_some()
                        || data.chars.row(nation, &w.weapon).is_some(),
                    "{}: {:?}",
                    u.id,
                    w.weapon
                );
            }
        }
    }
}
