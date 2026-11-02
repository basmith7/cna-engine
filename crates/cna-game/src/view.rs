//! What each side may know (3.61, limited intelligence): the engine, not a
//! host, decides it, so the browser and the server show the same thing.
//!
//! This first part is the supply aliases. A dummy supply unit's id
//! (`supply:<side>:dummy-<n>`) gives it away, and a real one's number
//! tells its age, so an enemy's supply ids reach a viewer only as aliases
//! `supply:<side>:s<hex>`, a keyed hash of the real id under a per-game
//! secret the host keeps (never the game's seed: the dice must not tell
//! the aliases, nor the aliases the dice). A hash rather than a numbered
//! permutation keeps every alias stable while supply units arrive and
//! die during the game. A side's own ids always pass through as they are.

use crate::order::Order;
use crate::piece::PieceId;
use crate::state::GameState;
use anyhow::{Result, bail};
use cna_data::scenario::Side;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The per-game alias key and the mapping it gives.
#[derive(Debug, Clone)]
pub struct Aliases {
    key: [u8; 32],
}

const SIDES: [Side; 2] = [Side::Axis, Side::Cw];

impl Aliases {
    pub fn new(key: [u8; 32]) -> Aliases {
        Aliases { key }
    }

    /// `id` as `viewer` may see it: an enemy supply unit's id becomes its
    /// alias, anything else is unchanged. `None` (a spectator) sees both
    /// sides as an enemy.
    pub fn alias(&self, viewer: Option<Side>, id: &PieceId) -> PieceId {
        PieceId::from(self.alias_text(viewer, id.as_str()))
    }

    /// Every enemy supply id inside `text` (an id, or a sentence naming
    /// some) replaced by its alias.
    pub fn alias_text(&self, viewer: Option<Side>, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        'scan: while !rest.is_empty() {
            for side in SIDES.into_iter().filter(|s| Some(*s) != viewer) {
                let prefix = format!("supply:{side}:");
                if let Some(tail) = rest.strip_prefix(prefix.as_str()) {
                    let len = real_suffix_len(tail);
                    if len > 0 {
                        out.push_str(&self.alias_of(side, &rest[..prefix.len() + len]));
                        rest = &tail[len..];
                        continue 'scan;
                    }
                }
            }
            let c = rest.chars().next().unwrap();
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
        out
    }

    /// Every string and object key in `value` through `alias_text`.
    pub fn alias_json(&self, viewer: Option<Side>, value: &mut Value) {
        match value {
            Value::String(s) => *s = self.alias_text(viewer, s),
            Value::Array(items) => items.iter_mut().for_each(|v| self.alias_json(viewer, v)),
            Value::Object(map) => {
                let old = std::mem::take(map);
                for (k, mut v) in old {
                    self.alias_json(viewer, &mut v);
                    map.insert(self.alias_text(viewer, &k), v);
                }
            }
            _ => {}
        }
    }

    /// The real id behind an id `viewer` names: an enemy alias maps back
    /// to its piece; an enemy's real supply id is refused (a viewer never
    /// saw it, so naming it is probing); anything else passes through.
    pub fn unalias(&self, state: &GameState, viewer: Side, id: &PieceId) -> Result<PieceId> {
        let enemy = viewer.other();
        let Some(tail) = id.as_str().strip_prefix(&format!("supply:{enemy}:")) else {
            return Ok(id.clone());
        };
        if tail.strip_prefix('s').is_some_and(is_alias_hash) {
            match state
                .pieces
                .keys()
                .find(|p| self.alias(Some(viewer), p) == *id)
            {
                Some(p) => Ok(p.clone()),
                None => bail!("no such piece: {id}"),
            }
        } else if real_suffix_len(tail) == tail.len() {
            bail!("no such piece: {id}")
        } else {
            Ok(id.clone())
        }
    }

    /// `order` with every enemy alias it carries mapped back, by walking
    /// its JSON, so no `Order` variant can be missed.
    pub fn unalias_order(&self, state: &GameState, viewer: Side, order: &Order) -> Result<Order> {
        let mut v = serde_json::to_value(order)?;
        self.unalias_json(state, viewer, &mut v)?;
        Ok(serde_json::from_value(v)?)
    }

    fn unalias_json(&self, state: &GameState, viewer: Side, value: &mut Value) -> Result<()> {
        match value {
            Value::String(s) => {
                *s = self
                    .unalias(state, viewer, &PieceId::from(s.clone()))?
                    .as_str()
                    .to_owned();
            }
            Value::Array(items) => {
                for v in items {
                    self.unalias_json(state, viewer, v)?;
                }
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    self.unalias_json(state, viewer, v)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn alias_of(&self, side: Side, real: &str) -> String {
        let mut h = Sha256::new();
        h.update(self.key);
        h.update(real.as_bytes());
        let d = h.finalize();
        let hex: String = d[..8].iter().map(|b| format!("{b:02x}")).collect();
        format!("supply:{side}:s{hex}")
    }
}

/// The length of a real supply id's suffix at the start of `tail`:
/// `<n>` or `dummy-<n>`, ending where the digits do; 0 if neither.
fn real_suffix_len(tail: &str) -> usize {
    let (skip, digits) = match tail.strip_prefix("dummy-") {
        Some(d) => (6, d),
        None => (0, tail),
    };
    let n = digits.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 { 0 } else { skip + n }
}

fn is_alias_hash(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::order::Fire;
    use crate::state::Modules;
    use std::collections::BTreeSet;

    fn state() -> GameState {
        let data = Data::load("grazianis-offensive").unwrap();
        GameState::with_modules(&data, 7, Modules::ALL).unwrap()
    }

    fn supply(state: &GameState, side: Side) -> Vec<PieceId> {
        state
            .pieces
            .values()
            .filter(|p| p.side == side && p.kind.is_supply())
            .map(|p| p.id.clone())
            .collect()
    }

    #[test]
    fn enemy_supply_ids_are_aliased_and_own_ones_are_not() {
        let s = state();
        let a = Aliases::new([3; 32]);
        let axis = supply(&s, Side::Axis);
        assert!(
            axis.iter().any(|p| p.as_str().contains("dummy")),
            "{axis:?}"
        );
        let seen: BTreeSet<_> = axis.iter().map(|p| a.alias(Some(Side::Cw), p)).collect();
        assert_eq!(seen.len(), axis.len(), "a bijection");
        for (real, alias) in axis
            .iter()
            .zip(axis.iter().map(|p| a.alias(Some(Side::Cw), p)))
        {
            assert!(alias.as_str().starts_with("supply:axis:s"), "{alias}");
            assert!(!alias.as_str().contains("dummy"));
            assert_eq!(a.alias(Some(Side::Axis), real), *real, "own ids pass");
            assert_eq!(a.unalias(&s, Side::Cw, &alias).unwrap(), *real);
            assert_eq!(a.alias(None, real), alias, "a spectator sees aliases");
        }
        let unit = s.pieces.values().find(|p| !p.kind.is_supply()).unwrap();
        assert_eq!(a.alias(Some(Side::Cw), &unit.id), unit.id);
        assert_eq!(a.unalias(&s, Side::Cw, &unit.id).unwrap(), unit.id);
    }

    #[test]
    fn aliases_hang_on_the_key_alone() {
        let s = state();
        let id = &supply(&s, Side::Axis)[0];
        let a = Aliases::new([3; 32]);
        let reloaded = GameState::from_json(
            &s.to_json().unwrap(),
            &Data::load("grazianis-offensive").unwrap(),
        )
        .unwrap();
        let alias = a.alias(Some(Side::Cw), id);
        assert_eq!(a.unalias(&reloaded, Side::Cw, &alias).unwrap(), *id);
        assert_ne!(Aliases::new([4; 32]).alias(Some(Side::Cw), id), alias);
    }

    #[test]
    fn an_enemy_real_id_or_unknown_alias_is_refused() {
        let s = state();
        let a = Aliases::new([3; 32]);
        for bad in [
            "supply:axis:dummy-1",
            "supply:axis:1",
            "supply:axis:s0123456789abcdef",
        ] {
            assert!(
                a.unalias(&s, Side::Cw, &PieceId::from(bad.to_owned()))
                    .is_err(),
                "{bad}"
            );
        }
        let own = PieceId::from("supply:axis:dummy-1".to_owned());
        assert_eq!(a.unalias(&s, Side::Axis, &own).unwrap(), own);
    }

    #[test]
    fn text_and_json_alias_every_enemy_id_inside() {
        let a = Aliases::new([3; 32]);
        let d = PieceId::from("supply:axis:dummy-12".to_owned());
        let alias = a.alias(Some(Side::Cw), &d);
        assert_eq!(
            a.alias_text(Some(Side::Cw), "[supply:axis:dummy-12, supply:cw:3] gone"),
            format!("[{alias}, supply:cw:3] gone")
        );
        let mut v = serde_json::json!({"supply:axis:dummy-12": ["supply:axis:dummy-12", 3]});
        a.alias_json(Some(Side::Cw), &mut v);
        assert_eq!(
            v,
            serde_json::json!({ alias.as_str(): [alias.as_str(), 3] })
        );
    }

    #[test]
    fn an_order_naming_an_alias_unaliases_to_the_real_id() {
        let s = state();
        let a = Aliases::new([3; 32]);
        let target = supply(&s, Side::Axis)[0].clone();
        let shooter = s
            .pieces
            .values()
            .find(|p| p.side == Side::Cw)
            .unwrap()
            .id
            .clone();
        let order = |t: PieceId| Order::Barrage {
            fires: vec![Fire {
                piece: shooter.clone(),
                points: 1,
                target: t,
            }],
        };
        let seen = order(a.alias(Some(Side::Cw), &target));
        assert_eq!(
            a.unalias_order(&s, Side::Cw, &seen).unwrap(),
            order(target.clone())
        );
        assert!(a.unalias_order(&s, Side::Cw, &order(target)).is_err());
    }
}
