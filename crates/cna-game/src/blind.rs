//! Blind barrage (SPI 12.22–12.24, R-182), the `blind_barrage` module.
//!
//! As step 2 opens, each side's targets are numbered by hex and class
//! in an order the game's RNG shuffles (the owner's numbering: the
//! engine numbers for him, as it makes the defender's patrol choices),
//! and a dummy tank formation is one more armour target. The firer names
//! `blind:<hex>:<class>:<n>` and learns nothing of which unit it is; he
//! names a unit itself only once it is identified to his side (patrol,
//! 16.51, or close assault), or a ship, which is always identified.

use crate::data::Data;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use std::collections::BTreeMap;

/// One side's numbering for a barrage step: by hex and class key, entry
/// `n - 1` is target `n`, `None` a dummy tank formation.
pub type Numbering = BTreeMap<HexId, BTreeMap<String, Vec<Option<PieceId>>>>;

/// What a fire's `target` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Named {
    /// A unit by its own id.
    Unit(PieceId),
    /// A numbered target in `hex`: the unit behind the number, or `None`
    /// for a dummy tank formation, or for a number nobody holds.
    Blind {
        hex: HexId,
        class: String,
        unit: Option<PieceId>,
        dummy: bool,
    },
}

/// The id a firer names for target `n` (1-based) of `class` in `hex`.
pub fn blind_id(hex: &HexId, class: &str, n: usize) -> PieceId {
    PieceId::from(format!("blind:{hex}:{class}:{n}"))
}

/// `(hex, class, n)` from a blind id; `None` for any other id.
pub fn parse_blind(id: &PieceId) -> Option<(HexId, String, usize)> {
    let rest = id.as_str().strip_prefix("blind:")?;
    let mut parts = rest.split(':');
    let (hex, class, n) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let hex = HexId::try_from(hex.to_string()).ok()?;
    Some((hex, class.to_string(), n.parse().ok()?))
}

impl GameState {
    /// Numbers every side's barrage targets for the step (12.23): per
    /// hex, per class, shuffled by the RNG; a dummy tank formation is an
    /// armour target. Ships are always identified and go unnumbered.
    pub(crate) fn number_targets(&mut self, data: &Data) {
        self.combat.numbering.clear();
        if !self.modules.blind_barrage {
            return;
        }
        let mut numbering: BTreeMap<Side, Numbering> = BTreeMap::new();
        for p in self.pieces.values() {
            let Some(hex) = p.hex() else { continue };
            if p.kind.is_vessel() || !self.is_target(data, p) {
                continue;
            }
            numbering
                .entry(p.side)
                .or_default()
                .entry(hex.clone())
                .or_default()
                .entry(self.class_of(data, p).key().to_string())
                .or_default()
                .push(Some(p.id.clone()));
        }
        for (side, hexes) in &self.dummy_tanks {
            for hex in hexes {
                numbering
                    .entry(*side)
                    .or_default()
                    .entry(hex.clone())
                    .or_default()
                    .entry("armor".to_string())
                    .or_default()
                    .push(None);
            }
        }
        // Fisher–Yates, in BTreeMap order, so the draws are fixed.
        for hexes in numbering.values_mut() {
            for classes in hexes.values_mut() {
                for list in classes.values_mut() {
                    for i in (1..list.len()).rev() {
                        let j = self.rng.below(i as u32 + 1) as usize;
                        list.swap(i, j);
                    }
                }
            }
        }
        self.combat.numbering = numbering;
    }

    /// What `target`, named by `player`'s fire, stands for.
    pub(crate) fn named(&self, player: Side, target: &PieceId) -> Named {
        let Some((hex, class, n)) = parse_blind(target) else {
            return Named::Unit(target.clone());
        };
        let entry = self
            .combat
            .numbering
            .get(&player.other())
            .and_then(|h| h.get(&hex))
            .and_then(|c| c.get(&class))
            .and_then(|list| n.checked_sub(1).and_then(|i| list.get(i)));
        Named::Blind {
            hex,
            class,
            unit: entry.cloned().flatten(),
            dummy: matches!(entry, Some(None)),
        }
    }

    /// Whether `player` may name `target` itself in a barrage (12.24):
    /// always without the module; with it, a ship or a unit identified
    /// to `player`.
    pub(crate) fn may_name(&self, player: Side, target: &PieceId) -> bool {
        !self.modules.blind_barrage
            || self.pieces.get(target).is_some_and(|p| p.kind.is_vessel())
            || self
                .identified
                .get(&player)
                .is_some_and(|s| s.contains(target))
    }

    /// The blind ids `player` may fire at in `hex` (an enemy's): one per
    /// numbered target of the step.
    pub(crate) fn blind_targets(&self, player: Side, hex: &HexId) -> Vec<PieceId> {
        let Some(classes) = self
            .combat
            .numbering
            .get(&player.other())
            .and_then(|h| h.get(hex))
        else {
            return Vec::new();
        };
        classes
            .iter()
            .flat_map(|(class, list)| (1..=list.len()).map(move |n| blind_id(hex, class, n)))
            .collect()
    }

    /// The blind id the step gave `unit`, for tests and scripts that
    /// play the firer with the owner's knowledge.
    pub fn blind_id_of(&self, unit: &PieceId) -> Option<PieceId> {
        let p = self.pieces.get(unit)?;
        let hexes = self.combat.numbering.get(&p.side)?;
        for (hex, classes) in hexes {
            for (class, list) in classes {
                if let Some(i) = list.iter().position(|e| e.as_ref() == Some(unit)) {
                    return Some(blind_id(hex, class, i + 1));
                }
            }
        }
        None
    }

    /// `pieces` become identified to `side` (12.24): a patrol described
    /// them, or they fought it in a close assault.
    pub(crate) fn identify(&mut self, side: Side, pieces: impl IntoIterator<Item = PieceId>) {
        if !self.modules.blind_barrage {
            return;
        }
        let known = self.identified.entry(side).or_default();
        for id in pieces {
            if self.pieces.get(&id).is_some_and(|p| p.side != side)
                && matches!(self.pieces[&id].kind, PieceKind::Combat | PieceKind::Hq)
            {
                known.insert(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::options::SealedOptions;
    use crate::combat::segment::test_support::at_sollum;
    use crate::event::Event;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Fire, Order};
    use crate::view::{Aliases, view_for};

    /// At Sollum with the module on, gun positions given: step 2 open.
    fn barrage_step(data: &Data, dummy_at: Option<&str>) -> GameState {
        let mut state = at_sollum(data);
        state.modules.blind_barrage = true;
        if let Some(h) = dummy_at {
            state
                .dummy_tanks
                .entry(Side::Axis)
                .or_default()
                .insert(hex(h));
        }
        for side in [Side::Cw, Side::Axis] {
            let forward = if side == Side::Cw {
                vec![pid("unit:cw:4-rha")]
            } else {
                vec![]
            };
            state
                .apply(data, side, Order::GunPositions { forward })
                .unwrap();
        }
        state
    }

    fn fire(target: PieceId) -> Order {
        Order::Barrage {
            fires: vec![Fire {
                piece: pid("unit:cw:4-rha"),
                points: 4,
                target,
            }],
        }
    }

    #[test]
    fn a_numbered_fire_lands_on_the_unit_behind_the_number() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = barrage_step(&data, None);
        let target = pid("unit:it:viii-libyan-bn");
        let name = state.blind_id_of(&target).expect("numbered");
        assert!(name.as_str().starts_with("blind:C4020:infantry:"), "{name}");
        // The firer is offered numbers, never an unidentified unit.
        let SealedOptions::Barrage { firers } = state.sealed_options(&data, Side::Cw).unwrap()
        else {
            panic!("barrage step")
        };
        let rha = firers
            .iter()
            .find(|f| f.piece == pid("unit:cw:4-rha"))
            .unwrap();
        assert!(rha.targets.contains(&name));
        assert!(
            rha.targets.iter().all(|t| t.as_str().starts_with("blind:")),
            "{:?}",
            rha.targets
        );
        // Each side's view holds only its own numbering.
        let aliases = Aliases::new([0; 32]);
        let cw = view_for(&state, &data, &aliases, Some(Side::Cw), false).unwrap();
        let axis = view_for(&state, &data, &aliases, Some(Side::Axis), false).unwrap();
        assert!(cw["combat"]["numbering"].get("axis").is_none());
        assert!(axis["combat"]["numbering"]["axis"]["C4020"].is_object());

        state.apply(&data, Side::Cw, fire(name.clone())).unwrap();
        let before = state.pieces[&target].toe;
        let events = state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        let result = events
            .iter()
            .find_map(|e| match e {
                Event::Barrage {
                    target: t, result, ..
                } => {
                    assert_eq!(t, &name, "the event keeps the name fired at");
                    Some(*result)
                }
                _ => None,
            })
            .expect("a barrage");
        use cna_rules::barrage::BarrageResult;
        match result {
            BarrageResult::Lose(n) => assert_eq!(state.pieces[&target].toe, before - n),
            BarrageResult::Pinned => assert!(state.combat.pinned.contains(&target)),
            _ => assert_eq!(state.pieces[&target].toe, before),
        }
        assert!(
            state.combat.numbering.is_empty(),
            "the numbering lasts one step"
        );
    }

    #[test]
    fn a_unit_is_named_only_once_identified() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = barrage_step(&data, None);
        let target = pid("unit:it:viii-libyan-bn");
        let refused = state
            .clone()
            .apply(&data, Side::Cw, fire(target.clone()))
            .unwrap_err();
        assert_eq!(refused.case, "12.24", "{refused:?}");
        // A number nobody holds is refused by its name alone.
        let refused = state
            .clone()
            .apply(
                &data,
                Side::Cw,
                fire(blind_id(&hex("C4020"), "infantry", 99)),
            )
            .unwrap_err();
        assert_eq!(refused.case, "12.23");
        assert!(!refused.reason.contains("unit:"), "{}", refused.reason);
        // Once a patrol or an assault has identified it, it may be named.
        state.identify(Side::Cw, [target.clone()]);
        state.apply(&data, Side::Cw, fire(target)).unwrap();
    }

    #[test]
    fn a_dummy_tank_formation_is_a_numbered_armour_target_that_loses_nothing() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = barrage_step(&data, Some("C4020"));
        let armour = &state.combat.numbering[&Side::Axis][&hex("C4020")]["armor"];
        let n = armour.iter().position(Option::is_none).expect("the dummy") + 1;
        let toe: Vec<u32> = state.pieces.values().map(|p| p.toe).collect();
        state
            .apply(&data, Side::Cw, fire(blind_id(&hex("C4020"), "armor", n)))
            .unwrap();
        let events = state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        assert!(events.iter().any(|e| matches!(e, Event::Barrage { .. })));
        assert!(!events.iter().any(|e| matches!(e, Event::Losses { .. })));
        assert_eq!(
            toe,
            state.pieces.values().map(|p| p.toe).collect::<Vec<_>>()
        );
        assert!(state.combat.pinned.is_empty());
    }

    #[test]
    fn without_the_module_nothing_is_numbered() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = barrage_step(&data, None);
        state.modules.blind_barrage = false;
        state.combat.numbering.clear();
        state
            .apply(&data, Side::Cw, fire(pid("unit:it:viii-libyan-bn")))
            .unwrap();
    }

    #[test]
    fn blind_ids_round_trip() {
        let id = blind_id(&hex("C4020"), "gun", 2);
        assert_eq!(parse_blind(&id), Some((hex("C4020"), "gun".into(), 2)));
        for bad in [
            "unit:it:x",
            "blind:C4020:gun",
            "blind:C4020:gun:x",
            "blind:Z9:gun:1",
            "blind:C4020:gun:1:2",
        ] {
            assert_eq!(parse_blind(&PieceId::from(bad.to_string())), None, "{bad}");
        }
    }
}
