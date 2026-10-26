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
use cna_data::oa::{Echelon, UnitId};

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
    /// or fewer of its assigned brigade equivalents attached, a brigade
    /// with under two-thirds of its assigned battalions (a two-battalion
    /// brigade with both attached, one not a shell, is not), a battalion
    /// under half its printed TOE (artillery a quarter). Attached shells
    /// do not count toward the parent's share.
    pub(crate) fn is_shell(&self, data: &Data, id: &PieceId) -> bool {
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
        let assigned = |pred: &dyn Fn(&Piece) -> bool| -> (u32, u32, u32) {
            let (mut total, mut whole, mut shells) = (0, 0, 0);
            for q in self.pieces.values() {
                if q.parent.as_ref() == Some(&unit) && pred(q) {
                    total += 1;
                    if q.attached_to.as_ref() == Some(&unit) {
                        if self.shell_at_depth(data, &q.id, depth + 1) {
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
                let (total, whole, _) = assigned(&|q| {
                    q.kind == PieceKind::Hq || q.equivalent == Some(Equivalent::Brigade)
                });
                total > 0 && 2 * whole <= total
            }
            (PieceKind::Hq, Some(Echelon::Brigade)) => {
                let (total, whole, shells) = assigned(&|q| {
                    q.kind == PieceKind::Combat && q.equivalent == Some(Equivalent::Battalion)
                });
                if total == 2 && whole + shells == 2 && whole >= 1 {
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
