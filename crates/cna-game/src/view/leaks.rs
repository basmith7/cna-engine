//! Refusals as a viewer sees them, and the leak predicate the engine's,
//! the server's and the browser tests share.
//!
//! Refusals were audited for 3.61 (2026-11-02): of the `Illegal` reasons
//! that interpolate a piece or a number, the numbers are the ordering
//! side's own (its TOE, CP, fuel owed, guard and prisoner points, its
//! stacking), so only the ids need the alias layer. The one judgment on
//! an enemy unit's internals is the barrage target check (`toe > 0`,
//! 12.2x), which a unit at 0 TOE no longer on the board cannot tell.

use super::{Aliases, ENEMY_PIECE_REMOVES};
use crate::event::Illegal;
use crate::piece::PieceKind;
use crate::state::GameState;
use cna_data::scenario::Side;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// `illegal` as `viewer` may read it: `{"case", "reason"}` with every
/// enemy supply id in the reason aliased unless information is open.
pub fn refusal_for(
    aliases: &Aliases,
    viewer: Option<Side>,
    illegal: &Illegal,
    open: bool,
) -> Value {
    let reason = if open {
        illegal.reason.clone()
    } else {
        aliases.alias_text(viewer, &illegal.reason)
    };
    json!({ "case": illegal.case, "reason": reason })
}

/// Fields that state a unit's internals wherever they sit beside its id.
const INTERNALS: &[&str] = &["toe", "weapons", "cohesion", "cp_spent", "morale"];

/// What `json`, meant for `about`'s enemy under limited intelligence,
/// holds that 3.61 hides from it about `about`'s pieces: a real supply
/// id (a dummy's gives it away), a removed piece field, a dummy's kind,
/// a placement still to be made, an unrevealed sealed answer, an
/// internal beside a unit's id, whether an unrevealed minefield is real,
/// or its set-up works still to place. Empty when clean.
pub fn leaks(state: &GameState, json: &Value, about: Side) -> Vec<String> {
    let mut found = Vec::new();
    let side = about.to_string();
    if let Some(combat) = json.get("combat") {
        for key in ["plots", "assignments"] {
            if combat.get(key).and_then(|m| m.get(&side)).is_some() {
                found.push(format!("combat.{key}.{side}"));
            }
        }
    }
    let units: BTreeSet<&str> = state
        .pieces
        .values()
        .filter(|p| p.side == about && !p.kind.is_supply())
        .map(|p| p.id.as_str())
        .collect();
    walk(&units, json, about, "", &mut found);
    // 24.31: whether an unrevealed field is real, and the owner's count
    // of real fields placed at set-up.
    if let Some(m) = json.get("minefields").and_then(Value::as_object) {
        for (hex, f) in m {
            if f.get("side").and_then(Value::as_str) == Some(side.as_str())
                && f.get("revealed").and_then(Value::as_bool) != Some(true)
                && f.get("real").is_some()
            {
                found.push(format!("minefields.{hex}.real"));
            }
        }
    }
    // 61.72: the owner's unattached motorisation points still to place.
    if json
        .get("motorisation_to_place")
        .and_then(|m| m.get(side.as_str()))
        .is_some()
    {
        found.push("motorisation_to_place".into());
    }
    if let Some(w) = json.get("works_to_place").and_then(Value::as_array) {
        for (i, q) in w.iter().enumerate() {
            if q.get("side").and_then(Value::as_str) == Some(side.as_str()) {
                found.push(format!("works_to_place[{i}]"));
            }
        }
    }
    found
}

fn walk(units: &BTreeSet<&str>, v: &Value, about: Side, at: &str, found: &mut Vec<String>) {
    match v {
        Value::String(s) => {
            if names_real_supply(s, about) {
                found.push(format!("{at}: {s}"));
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk(units, item, about, &format!("{at}[{i}]"), found);
            }
        }
        Value::Object(map) => {
            for k in map.keys().filter(|k| names_real_supply(k, about)) {
                found.push(format!("{at}: key {k}"));
            }
            let is_about_piece = map.get("side").and_then(Value::as_str)
                == Some(&about.to_string())
                && map.contains_key("id");
            if is_about_piece {
                for k in ENEMY_PIECE_REMOVES.iter().filter(|k| map.contains_key(**k)) {
                    found.push(format!("{at}.{k}"));
                }
                if map.get("kind") == Some(&json!(PieceKind::DummySupplyUnit)) {
                    found.push(format!("{at}.kind"));
                }
                if map
                    .get("location")
                    .is_some_and(|l| l.get("pending").is_some())
                {
                    found.push(format!("{at}.location"));
                }
            }
            let names_unit = map
                .values()
                .any(|x| x.as_str().is_some_and(|s| units.contains(s)));
            // 16.5x: a patrol's description states the unit's TOE; that
            // event alone may carry it.
            let described = at.contains(".patrolled.described[");
            if names_unit && !is_about_piece && !described {
                for k in INTERNALS.iter().filter(|k| map.contains_key(**k)) {
                    found.push(format!("{at}.{k}"));
                }
            }
            for (k, x) in map {
                walk(units, x, about, &format!("{at}.{k}"), found);
            }
        }
        _ => {}
    }
}

/// `text` holds `supply:<about>:<n>` or `supply:<about>:dummy-<n>`.
fn names_real_supply(text: &str, about: Side) -> bool {
    let prefix = format!("supply:{about}:");
    text.match_indices(&prefix).any(|(i, _)| {
        let tail = &text[i + prefix.len()..];
        let digits = tail.strip_prefix("dummy-").unwrap_or(tail);
        digits.bytes().next().is_some_and(|b| b.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::state::Modules;
    use crate::view::view_for;

    #[test]
    fn a_refusal_names_enemy_supply_only_by_alias() {
        let a = Aliases::new([1; 32]);
        let i = Illegal::new(
            "12.2x",
            "supply:axis:dummy-2 is not an enemy unit a barrage may target",
        );
        let seen = refusal_for(&a, Some(Side::Cw), &i, false);
        assert_eq!(seen["case"], json!("12.2x"));
        assert!(!seen["reason"].as_str().unwrap().contains("dummy"));
        assert_eq!(
            refusal_for(&a, Some(Side::Axis), &i, false)["reason"],
            json!(i.reason)
        );
        assert_eq!(
            refusal_for(&a, Some(Side::Cw), &i, true)["reason"],
            json!(i.reason)
        );
    }

    #[test]
    fn leaks_finds_what_the_full_state_holds_and_not_the_view() {
        let data = Data::load("grazianis-offensive").unwrap();
        let s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        let full = serde_json::to_value(&s).unwrap();
        let found = leaks(&s, &full, Side::Axis);
        assert!(
            found.iter().any(|f| f.contains("supply:axis:dummy")),
            "{found:?}"
        );
        assert!(found.iter().any(|f| f.ends_with(".toe")), "{found:?}");
        assert!(found.iter().any(|f| f.ends_with(".location")), "{found:?}");
        let v = view_for(&s, &data, &Aliases::new([1; 32]), Some(Side::Cw), false).unwrap();
        assert_eq!(leaks(&s, &v, Side::Axis), Vec::<String>::new());
        let unit = s
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && !p.kind.is_supply())
            .unwrap();
        let event = json!({"losses": {"piece": unit.id.as_str(), "toe": 3}});
        assert_eq!(
            leaks(&s, &event, Side::Axis),
            vec![".losses.toe".to_owned()]
        );
    }
}
