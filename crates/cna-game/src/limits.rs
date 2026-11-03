//! Attachment limits and shells. A parent carries, beyond its assigned
//! structure, at most what its Maximum Attachment Chart row allows
//! (19.41, 19.5, R-130), counted through nested attachments (19.4x);
//! a formation with too little of its own structure attached is a
//! shell (9.26–9.28), which stacks one level smaller.

use crate::data::Data;
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::characteristics::Equivalent;
use cna_data::max_attachments::Row;
use cna_data::oa::{Echelon, Unit, UnitId};
use cna_data::organisation::{Organisation, Row as ChartRow};
use std::collections::BTreeMap;

/// What a non-assigned attachment is, for the chart's type limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tank,
    Infantry,
    Artillery,
    Gun,
    Recce,
    Other,
}

impl GameState {
    /// The pieces attached to `unit`, nested attachments included.
    fn attached_under(&self, unit: &UnitId) -> Vec<&Piece> {
        let mut out = Vec::new();
        let mut heads = vec![unit.clone()];
        while let Some(head) = heads.pop() {
            for q in self.pieces.values() {
                if q.attached_to.as_ref() == Some(&head) {
                    out.push(q);
                    if let Some(u) = q.id.unit() {
                        heads.push(u);
                    }
                }
            }
        }
        out
    }

    /// Why attaching non-assigned `piece` to `to` would break a cap of
    /// `to` or of any parent `to` is itself attached to (19.41, 19.5,
    /// nesting); none if it fits.
    pub(crate) fn attachment_cap_refusal(
        &self,
        data: &Data,
        piece: &PieceId,
        to: &PieceId,
    ) -> Option<String> {
        let mut parent = to.clone();
        loop {
            if let Some(why) = self.one_cap_refusal(data, piece, &parent) {
                return Some(why);
            }
            let up = self.pieces[&parent].attached_to.clone()?;
            parent = PieceId::from(&up);
        }
    }

    fn one_cap_refusal(&self, data: &Data, piece: &PieceId, to: &PieceId) -> Option<String> {
        let parent = &self.pieces[to];
        let unit = to.unit()?;
        let nation = parent.nation?;
        let sheet = data.oa.unit(&unit);
        let keys: Vec<String> = if parent.kind == PieceKind::Hq {
            sheet
                .map(|u| u.attachment_parent.clone())
                .unwrap_or_default()
        } else {
            vec!["battalion".to_string()]
        };
        let turn = u32::from(self.clock.game_turn);
        let Some(row) = data.max_attachments.row(nation, &keys, turn) else {
            return Some(format!(
                "{to} has no Maximum Attachment Chart row: its O/A notes govern (19.2)"
            ));
        };
        // The extra (non-assigned) attachments under the parent, nested
        // ones included, and the new one with its own.
        let mut extra: Vec<&Piece> = self
            .attached_under(&unit)
            .into_iter()
            .filter(|q| q.parent != q.attached_to)
            .collect();
        let new = &self.pieces[piece];
        extra.push(new);
        if let Some(u) = piece.unit() {
            extra.extend(
                self.attached_under(&u)
                    .into_iter()
                    .filter(|q| q.parent != q.attached_to),
            );
        }
        let new_kind = self.attachment_kind(data, new);
        match row.unit_kind.as_deref() {
            Some("company") if new.equivalent != Some(Equivalent::Company) => {
                return Some(format!("{to} takes only companies attached (19.5)"));
            }
            Some("artillery") if new_kind != Kind::Artillery => {
                return Some(format!("{to} takes only artillery attached (19.5)"));
            }
            _ => {}
        }
        let free = match sheet.and_then(|u| u.echelon) {
            Some(Echelon::Division | Echelon::Brigade) => {
                data.max_attachments.free_company_equivalents
            }
            _ => 0,
        };
        let companies = extra
            .iter()
            .filter(|q| q.equivalent == Some(Equivalent::Company))
            .count() as u32;
        let brigades = extra
            .iter()
            .filter(|q| q.equivalent == Some(Equivalent::Brigade) || q.kind == PieceKind::Hq)
            .count() as u32;
        let units = extra.len() as u32 - companies.min(free) - brigades;
        if !fits(row, brigades, units) {
            return Some(format!(
                "{to} would carry {units} unit(s) and {brigades} brigade(s) beyond its \
                 structure, over its Maximum Attachment Chart row {} (19.5)",
                row.parent
            ));
        }
        let count = |k: Kind| {
            extra
                .iter()
                .filter(|q| self.attachment_kind(data, q) == k)
                .count() as u32
        };
        let combined = row.restriction_mode.as_deref() == Some("combined");
        let mut over = Vec::new();
        for (name, cap) in &row.restrictions {
            let kind = match name.as_str() {
                "tank" => Kind::Tank,
                "infantry" => Kind::Infantry,
                "gun" => Kind::Gun,
                "recce" => Kind::Recce,
                _ => continue,
            };
            if count(kind) > *cap {
                over.push(name.as_str());
            }
        }
        if combined {
            let cap = row.restrictions.values().copied().max().unwrap_or(0);
            if count(Kind::Tank) + count(Kind::Infantry) > cap {
                over.push("infantry or tank");
            }
        }
        (!over.is_empty()).then(|| {
            format!(
                "{to}'s Maximum Attachment Chart row {} limits {} (19.5)",
                row.parent,
                over.join(", ")
            )
        })
    }

    fn attachment_kind(&self, data: &Data, p: &Piece) -> Kind {
        let Some(t) = p
            .nation
            .zip(p.id_code.as_deref())
            .and_then(|(n, code)| data.chars.row(n, code))
            .map(|r| r.unit_type.as_str())
        else {
            return Kind::Other;
        };
        if t.starts_with("Tank") {
            Kind::Tank
        } else if t.starts_with("Infantry") {
            Kind::Infantry
        } else if t.starts_with("Artillery") {
            Kind::Artillery
        } else if t.starts_with("Anti-tank") {
            Kind::Gun
        } else if t.starts_with("Rec") {
            Kind::Recce
        } else {
            Kind::Other
        }
    }

    /// Whether a formation is a shell (9.26–9.28): a division with half
    /// or fewer of its brigade equivalents attached, a brigade with under
    /// two-thirds of its battalions (a two-battalion brigade with both
    /// attached, one not a shell, is not), a battalion under half its
    /// printed TOE (artillery a quarter). Attached shells do not count
    /// toward the parent's share.
    ///
    /// The whole is the HQ's Formation Organisation Chart row (19.3): the
    /// row its OA `organisation` names for this nation and game-turn,
    /// counting the components R-141 marks `counts_for_shell`. An HQ the
    /// chart has no row for (the O/A notes govern, or none covers the
    /// turn) falls back to the units the OA assigns it. What counts
    /// toward the share is still the assigned units attached to it.
    pub fn is_shell(&self, data: &Data, id: &PieceId) -> bool {
        self.shell_at_depth(data, id, 0)
    }

    fn shell_at_depth(&self, data: &Data, id: &PieceId, depth: u8) -> bool {
        let Some(p) = self.pieces.get(id) else {
            return false;
        };
        let Some(unit) = id.unit() else {
            return false;
        };
        if depth > 4 {
            return false;
        }
        let sheet = data.oa.unit(&unit);
        let chart = sheet
            .and_then(|u| u.organisation.as_deref())
            .and_then(|key| {
                let turn = u32::from(self.clock.game_turn);
                data.organisation.row(unit.nation(), key, turn)
            });
        let is_shell = |q: &Piece| self.shell_at_depth(data, &q.id, depth + 1);
        // With a chart row (R-141), the counted slots its attached
        // counters fill (R-142): (filled by whole units, filled by all).
        let filled = |row: &ChartRow| -> (u32, u32) {
            let attached: Vec<(&Unit, bool)> = self
                .pieces
                .values()
                .filter(|q| q.attached_to.as_ref() == Some(&unit))
                .filter_map(|q| Some((data.oa.unit(&q.id.unit()?)?, is_shell(q))))
                .collect();
            let whole: Vec<&Unit> = attached
                .iter()
                .filter(|(_, s)| !s)
                .map(|(u, _)| *u)
                .collect();
            let all: Vec<&Unit> = attached.iter().map(|(u, _)| *u).collect();
            (
                slots_filled(&data.organisation, row, &whole),
                slots_filled(&data.organisation, row, &all),
            )
        };
        let assigned = |pred: &dyn Fn(&Piece) -> bool| -> (u32, u32, u32) {
            let (mut total, mut whole, mut shells) = (0, 0, 0);
            for q in self.pieces.values() {
                if q.parent.as_ref() == Some(&unit) && pred(q) {
                    total += 1;
                    if q.attached_to.as_ref() == Some(&unit) {
                        if is_shell(q) {
                            shells += 1;
                        } else {
                            whole += 1;
                        }
                    }
                }
            }
            (total, whole, shells)
        };
        match (p.kind, sheet.and_then(|u| u.echelon)) {
            (PieceKind::Hq, Some(Echelon::Division)) => {
                let (total, whole) = match chart {
                    Some(row) => (row.shell_count(), filled(row).0),
                    None => {
                        let (total, whole, _) = assigned(&|q| {
                            q.kind == PieceKind::Hq || q.equivalent == Some(Equivalent::Brigade)
                        });
                        (total, whole)
                    }
                };
                total > 0 && 2 * whole <= total
            }
            (PieceKind::Hq, Some(Echelon::Brigade)) => {
                let (total, whole, all) = match chart {
                    Some(row) => {
                        let (whole, all) = filled(row);
                        (row.shell_count(), whole, all)
                    }
                    None => {
                        let (total, whole, shells) = assigned(&|q| {
                            q.kind == PieceKind::Combat
                                && q.equivalent == Some(Equivalent::Battalion)
                        });
                        (total, whole, whole + shells)
                    }
                };
                if total == 2 && all == 2 && whole >= 1 {
                    return false;
                }
                total > 0 && 3 * whole < 2 * total
            }
            (PieceKind::Combat, _) if p.equivalent == Some(Equivalent::Battalion) => {
                // Printed strength: the sheet's TOE, or the ID code's row.
                let printed = sheet
                    .map(|u| {
                        let row = u
                            .id_code
                            .as_deref()
                            .and_then(|code| data.chars.row(u.id.nation(), code));
                        crate::setup::toe_points(u, row)
                    })
                    .unwrap_or(0);
                if printed == 0 {
                    return false;
                }
                let artillery = self.attachment_kind(data, p) == Kind::Artillery;
                if artillery {
                    4 * p.toe < printed
                } else {
                    2 * p.toe < printed
                }
            }
            _ => false,
        }
    }
}

/// The most of `row`'s counted slots (9.26, R-141) that `counters` can
/// fill at once (R-142): each counter fills one slot, a slot it names in
/// `fills`, or with others of its kind one it names in `fills_part`,
/// filled once the chart's `part_slots` minimum of them are in it.
fn slots_filled(org: &Organisation, row: &ChartRow, counters: &[&Unit]) -> u32 {
    let slots: Vec<Vec<&str>> = row
        .components
        .iter()
        .filter(|c| c.counts_for_shell)
        .flat_map(|c| {
            let accepts: Vec<&str> = c.unit.iter().chain(&c.any_of).map(String::as_str).collect();
            std::iter::repeat_n(accepts, c.count as usize)
        })
        .collect();
    // Counters beyond 64 never stand under one HQ; ignore the rest.
    let counters = &counters[..counters.len().min(64)];
    let mut memo = BTreeMap::new();
    best_fill(org, &slots, counters, 0, 0, &mut memo)
}

fn best_fill(
    org: &Organisation,
    slots: &[Vec<&str>],
    counters: &[&Unit],
    slot: usize,
    used: u64,
    memo: &mut BTreeMap<(usize, u64), u32>,
) -> u32 {
    let Some(accepts) = slots.get(slot) else {
        return 0;
    };
    if let Some(&b) = memo.get(&(slot, used)) {
        return b;
    }
    let free = |i: usize| used & (1 << i) == 0;
    let names = |list: &[String], s: &str| list.iter().any(|f| f == s);
    let mut best = best_fill(org, slots, counters, slot + 1, used, memo);
    let left = (slots.len() - slot) as u32;
    for (i, c) in counters.iter().enumerate() {
        if best == left {
            break;
        }
        if free(i) && accepts.iter().any(|s| names(&c.fills, s)) {
            best = best.max(1 + best_fill(org, slots, counters, slot + 1, used | 1 << i, memo));
        }
    }
    for s in accepts {
        let Some(part) = org.part_slots.get(*s) else {
            continue;
        };
        let parts: Vec<usize> = (0..counters.len())
            .filter(|&i| free(i) && names(&counters[i].fills_part, s))
            .collect();
        // The fewest that fill it: more in one slot fill nothing more.
        for combo in combinations(&parts, part.min as usize) {
            if best == left {
                break;
            }
            let mask = combo.iter().fold(used, |m, i| m | 1 << i);
            best = best.max(1 + best_fill(org, slots, counters, slot + 1, mask, memo));
        }
    }
    memo.insert((slot, used), best);
    best
}

/// Every `k`-element subset of `items`, in order.
fn combinations(items: &[usize], k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![Vec::new()];
    }
    if items.len() < k {
        return Vec::new();
    }
    let mut out: Vec<Vec<usize>> = combinations(&items[1..], k - 1)
        .into_iter()
        .map(|mut rest| {
            rest.insert(0, items[0]);
            rest
        })
        .collect();
    out.extend(combinations(&items[1..], k));
    out
}

/// Whether `brigades` brigade-equivalents and `units` battalion-
/// equivalents fit a row: its alternatives where it has them, else
/// units only (the chart counts battalion-equivalents).
fn fits(row: &Row, brigades: u32, units: u32) -> bool {
    if row.alternatives.is_empty() {
        brigades == 0 && units <= row.max_units
    } else {
        row.alternatives
            .iter()
            .any(|a| brigades <= a.brigades && units <= a.units)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::oa::Nation;

    fn data() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn filled(data: &Data, nation: Nation, row: &str, turn: u32, ids: &[&str]) -> u32 {
        let row = data.organisation.row(nation, row, turn).unwrap();
        let units: Vec<&Unit> = ids
            .iter()
            .map(|id| data.oa.unit(&UnitId::parse(id).unwrap()).unwrap())
            .collect();
        slots_filled(&data.organisation, row, &units)
    }

    #[test]
    fn three_artillery_counters_fill_the_german_artillery_group_r142() {
        let data = data();
        let arty = [
            "unit:de:33-arty-regt",
            "unit:de:140-arty-regt",
            "unit:de:i-115-arty-bn",
        ];
        let at = |ids: &[&str]| filled(&data, Nation::De, "15-panzer-division", 30, ids);
        assert_eq!(at(&arty), 1);
        assert_eq!(at(&arty[..2]), 0, "two of three fill nothing");
        // A regiment HQ fills the group alone.
        assert_eq!(at(&["unit:de:220-arty-regt-hq"]), 1);
    }

    #[test]
    fn surplus_battalions_fill_no_other_slot_r142() {
        // The 1st Free French Brigade's five infantry battalions fill its
        // three infantry slots; its artillery fills the fourth.
        let data = data();
        let inf = [
            "unit:cw:2-lgetr",
            "unit:cw:3-lgetr",
            "unit:cw:2-mrch",
            "unit:cw:1-marine",
            "unit:cw:1-pacf",
        ];
        let at = |ids: &[&str]| filled(&data, Nation::Cw, "infantry-brigade-3sp", 30, ids);
        assert_eq!(at(&inf), 3);
        let mut with_arty = inf.to_vec();
        with_arty.push("unit:cw:1-ff-arty");
        assert_eq!(at(&with_arty), 4);
    }

    #[test]
    fn a_company_fills_only_the_any_unit_slot_r142() {
        // The Kopanski MG Company fills no counted slot of the Polish
        // brigade; a battalion in its place would.
        let data = data();
        let at = |ids: &[&str]| filled(&data, Nation::Cw, "infantry-brigade-3sp", 30, ids);
        let pol = [
            "unit:cw:1-pol",
            "unit:cw:2-pol",
            "unit:cw:3-pol",
            "unit:cw:1-pol-arty",
        ];
        assert_eq!(at(&pol), 4);
        let mut with_mg = pol.to_vec();
        with_mg.push("unit:cw:kopanski-mg");
        assert_eq!(at(&with_mg), 4);
        assert_eq!(at(&[]), 0);
    }
}
