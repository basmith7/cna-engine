//! Dummy tank formations (SPI 16.4, cna `rules/60-combat.md`, *Dummy
//! tank formations*), behind the `dummy_tanks` module. A dummy is the
//! size of a tank battalion, worth nothing in combat, CPA 0, and has no
//! counter: its owner notes it secretly in phase C's construction segment
//! (`Order::PlaceDummyTank`) in a hex holding a friendly formation. Under
//! §32 it costs no stores and needs no supply unit (R-178); the engine
//! models no stores at all, so it costs nothing either way. 16.43's
//! limits hold: three per player at once, one per hex, never in an
//! Italian formation, even a mixed one.
//!
//! Its formation is the owner's combat units in its hex. It dies, and
//! the enemy learns it was a dummy, as soon as that hex is close-assaulted
//! or takes the enemy's anti-armour fire and is left holding no real
//! armour (16.44, 16.45); its hex's fire then frees no points for close
//! assault under 14.23. Left with no formation (the units moved off or
//! were eliminated) it is gone too, unseen by the enemy, as a dummy
//! with CPA 0 cannot follow them.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::PieceKind;
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::Nation;
use cna_data::scenario::Side;
use std::collections::BTreeSet;

/// The most dummy tank formations a player has at once (16.43).
pub const MAX_PER_PLAYER: usize = 3;

impl GameState {
    /// The hexes holding `side`'s dummy tank formations.
    pub fn dummy_tank_hexes(&self, side: Side) -> BTreeSet<HexId> {
        self.dummy_tanks.get(&side).cloned().unwrap_or_default()
    }

    /// Why `player` may not place a dummy tank formation in `hex` now, if
    /// it may not.
    fn dummy_tank_refusal(&self, data: &Data, player: Side, hex: &HexId) -> Option<Illegal> {
        let bad = |why: String| Some(Illegal::new("16.43", why));
        if !self.modules.dummy_tanks {
            return bad("this game plays without dummy tank formations".into());
        }
        if self.clock.phase != Phase::Organisation {
            return Some(Illegal::new(
                "16.42",
                format!(
                    "a dummy tank formation is placed in phase C's construction segment, not at {}",
                    self.clock
                ),
            ));
        }
        if data.map.hex(hex).is_none() {
            return bad(format!("{hex} is not a hex of the map"));
        }
        let own = self.dummy_tanks.get(&player);
        if own.is_some_and(|h| h.contains(hex)) {
            return bad(format!("{hex} already holds a dummy tank formation"));
        }
        if own.map_or(0, BTreeSet::len) >= MAX_PER_PLAYER {
            return bad(format!(
                "{player} already has {MAX_PER_PLAYER} dummy tank formations"
            ));
        }
        let friends: Vec<_> = self
            .pieces_at(hex)
            .into_iter()
            .filter(|p| p.side == player)
            .collect();
        if !friends.iter().any(|p| p.kind == PieceKind::Combat) {
            return bad(format!(
                "a dummy tank formation joins a formation: {hex} holds no {player} combat unit"
            ));
        }
        if friends.iter().any(|p| {
            matches!(p.kind, PieceKind::Combat | PieceKind::Hq) && p.nation == Some(Nation::It)
        }) {
            return bad(format!(
                "no dummy tank formation in an Italian formation, even a mixed one ({hex})"
            ));
        }
        None
    }

    /// Whether `player` may place a dummy tank formation in `hex` now,
    /// for the browser's phase C control.
    pub fn dummy_tank_placeable(&self, data: &Data, player: Side, hex: &HexId) -> bool {
        self.to_act() == Some(player) && self.dummy_tank_refusal(data, player, hex).is_none()
    }

    /// The hexes `player` may place a dummy tank formation in now, in id
    /// order: none when it is not `player`'s turn to act.
    pub fn dummy_tank_hexes_placeable(&self, data: &Data, player: Side) -> Vec<HexId> {
        if self.to_act() != Some(player) || !self.modules.dummy_tanks {
            return Vec::new();
        }
        let hexes: BTreeSet<HexId> = self
            .pieces
            .values()
            .filter(|p| p.side == player && p.kind == PieceKind::Combat)
            .filter_map(|p| p.hex().cloned())
            .collect();
        hexes
            .into_iter()
            .filter(|h| self.dummy_tank_refusal(data, player, h).is_none())
            .collect()
    }

    /// `Order::PlaceDummyTank` (16.42): noted secretly in `hex`.
    pub(crate) fn place_dummy_tank(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        if let Some(refused) = self.dummy_tank_refusal(data, player, hex) {
            return Err(refused);
        }
        self.dummy_tanks
            .entry(player)
            .or_default()
            .insert(hex.clone());
        Ok(vec![Event::DummyTankPlaced {
            side: player,
            hex: hex.clone(),
        }])
    }

    fn take_dummy_tank(&mut self, side: Side, hex: &HexId) -> bool {
        let Some(hexes) = self.dummy_tanks.get_mut(&side) else {
            return false;
        };
        let taken = hexes.remove(hex);
        if hexes.is_empty() {
            self.dummy_tanks.remove(&side);
        }
        taken
    }

    /// Whether `side` has a combat unit with armoured TOE in `hex`.
    pub(crate) fn holds_real_armour(&self, data: &Data, side: Side, hex: &HexId) -> bool {
        self.pieces.values().any(|p| {
            p.side == side
                && p.hex() == Some(hex)
                && p.toe > 0
                && self
                    .parts(data, p)
                    .iter()
                    .any(crate::combat::strength::Part::armoured)
        })
    }

    /// Whether `side` holds a dummy tank formation in `hex`.
    pub(crate) fn has_dummy_tank(&self, side: Side, hex: &HexId) -> bool {
        self.dummy_tanks.get(&side).is_some_and(|h| h.contains(hex))
    }

    /// The enemy has shown `side`'s dummy in `hex` for what it is, by a
    /// close assault or anti-armour fire (16.44, 16.45): it dies, in
    /// public.
    pub(crate) fn expose_dummy_tank(&mut self, side: Side, hex: &HexId, events: &mut Vec<Event>) {
        if self.take_dummy_tank(side, hex) {
            events.push(Event::DummyTankExposed {
                side,
                hex: hex.clone(),
            });
        }
    }

    /// After every order: a dummy whose hex holds no friendly combat unit
    /// has lost its formation and is gone, seen only by its owner.
    pub(crate) fn drop_orphan_dummy_tanks(&mut self, events: &mut Vec<Event>) {
        let orphans: Vec<(Side, HexId)> = self
            .dummy_tanks
            .iter()
            .flat_map(|(side, hexes)| hexes.iter().map(move |h| (*side, h.clone())))
            .filter(|(side, h)| {
                !self
                    .pieces_at(h)
                    .iter()
                    .any(|p| p.side == *side && p.kind == PieceKind::Combat)
            })
            .collect();
        for (side, hex) in orphans {
            self.take_dummy_tank(side, &hex);
            events.push(Event::DummyTankLost { side, hex });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::piece::{Location, PieceId};
    use crate::state::Modules;
    use crate::state::test_support::auto_place;
    use crate::view::{Aliases, events_for, view_for};

    fn data() -> Data {
        crate::raiders::tests::graziani()
    }

    /// Dummies on, and construction to open phase C (as `abstract_supply`
    /// does in a new game).
    fn on() -> Modules {
        Modules {
            dummy_tanks: true,
            construction: true,
            ..Modules::default()
        }
    }

    /// Plays from set-up to phase C with `side` to act.
    fn at_phase_c(data: &Data, modules: Modules, side: Side) -> GameState {
        let mut state = GameState::with_modules(data, 7, modules).unwrap();
        auto_place(data, &mut state);
        for _ in 0..20 {
            if state.clock.phase == Phase::Organisation && state.to_act() == Some(side) {
                return state;
            }
            let p = state.to_act().unwrap();
            let order = if state.clock.phase == Phase::InitiativeDeclaration {
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                }
            } else {
                Order::Pass
            };
            state.apply(data, p, order).unwrap();
        }
        panic!("no phase C for {side} at {}", state.clock);
    }

    /// Hexes holding a combat unit of `side`, in id order.
    fn combat_hexes(state: &GameState, side: Side) -> Vec<HexId> {
        let hexes: BTreeSet<HexId> = state
            .pieces
            .values()
            .filter(|p| p.side == side && p.kind == PieceKind::Combat)
            .filter_map(|p| p.hex().cloned())
            .collect();
        hexes.into_iter().collect()
    }

    fn place(
        state: &mut GameState,
        data: &Data,
        side: Side,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        state.apply(data, side, Order::PlaceDummyTank { hex: hex.clone() })
    }

    fn refusal(state: &mut GameState, data: &Data, side: Side, hex: &HexId) -> String {
        let before = serde_json::to_string(&*state).unwrap();
        let e = place(state, data, side, hex).unwrap_err();
        assert_eq!(
            serde_json::to_string(&*state).unwrap(),
            before,
            "refused unchanged"
        );
        e.reason
    }

    #[test]
    fn placed_in_phase_c_at_no_cost_16_42_r178() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let hex = combat_hexes(&state, Side::Cw)[0].clone();
        assert!(state.dummy_tank_placeable(&data, Side::Cw, &hex));
        let events = place(&mut state, &data, Side::Cw, &hex).unwrap();
        assert_eq!(
            events,
            [Event::DummyTankPlaced {
                side: Side::Cw,
                hex: hex.clone()
            }]
        );
        assert_eq!(state.dummy_tank_hexes(Side::Cw), BTreeSet::from([hex]));
    }

    #[test]
    fn three_per_player_one_per_hex_16_43() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let hexes = combat_hexes(&state, Side::Cw);
        assert!(hexes.len() > MAX_PER_PLAYER, "{hexes:?}");
        place(&mut state, &data, Side::Cw, &hexes[0]).unwrap();
        assert!(refusal(&mut state, &data, Side::Cw, &hexes[0]).contains("already holds"));
        for h in &hexes[1..MAX_PER_PLAYER] {
            place(&mut state, &data, Side::Cw, h).unwrap();
        }
        let fourth = &hexes[MAX_PER_PLAYER];
        assert!(!state.dummy_tank_placeable(&data, Side::Cw, fourth));
        assert!(refusal(&mut state, &data, Side::Cw, fourth).contains("already has 3"));
    }

    #[test]
    fn never_in_an_italian_formation_even_a_mixed_one_16_43() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Axis);
        let hex = combat_hexes(&state, Side::Axis)[0].clone();
        assert!(refusal(&mut state, &data, Side::Axis, &hex).contains("Italian"));
        let axis: Vec<PieceId> = state
            .pieces_at(&hex)
            .iter()
            .filter(|p| p.side == Side::Axis)
            .map(|p| p.id.clone())
            .collect();
        // Every unit there German: placed.
        let mut german = state.clone();
        for id in &axis {
            german.pieces.get_mut(id).unwrap().nation = Some(Nation::De);
        }
        place(&mut german, &data, Side::Axis, &hex).unwrap();
        // One Italian unit among Germans: a mixed formation, refused.
        let mut mixed = state.clone();
        for id in &axis {
            mixed.pieces.get_mut(id).unwrap().nation = Some(Nation::De);
        }
        let italian = axis
            .iter()
            .find(|id| state.pieces[*id].kind == PieceKind::Combat)
            .unwrap();
        mixed.pieces.get_mut(italian).unwrap().nation = Some(Nation::It);
        assert!(refusal(&mut mixed, &data, Side::Axis, &hex).contains("Italian"));
        // An Italian supply unit beside German units is no Italian
        // formation: placed.
        let mut supplied = german.clone();
        supplied.dummy_tanks.clear();
        let piece = supplied.pieces.get_mut(italian).unwrap();
        piece.kind = PieceKind::SupplyUnit;
        piece.nation = Some(Nation::It);
        assert!(
            axis.iter()
                .any(|id| *id != *italian && state.pieces[id].kind == PieceKind::Combat)
        );
        place(&mut supplied, &data, Side::Axis, &hex).unwrap();
    }

    #[test]
    fn only_with_a_formation_in_phase_c_and_with_the_module() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let axis_only = combat_hexes(&state, Side::Axis)[0].clone();
        assert!(
            refusal(&mut state, &data, Side::Cw, &axis_only).contains("holds no cw combat unit")
        );
        let hex = combat_hexes(&state, Side::Cw)[0].clone();
        let mut off = at_phase_c(
            &data,
            Modules {
                dummy_tanks: false,
                ..on()
            },
            Side::Cw,
        );
        assert!(refusal(&mut off, &data, Side::Cw, &hex).contains("without"));
        state.clock.phase = Phase::MovementCombat;
        let e = state.dummy_tank_refusal(&data, Side::Cw, &hex).unwrap();
        assert!(e.reason.contains("phase C"), "{e:?}");
    }

    #[test]
    fn the_enemy_never_sees_one_placed() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let hex = combat_hexes(&state, Side::Cw)[0].clone();
        let events = place(&mut state, &data, Side::Cw, &hex).unwrap();
        let aliases = Aliases::new([1; 32]);
        let axis = view_for(&state, &data, &aliases, Some(Side::Axis), false).unwrap();
        assert!(axis.get("dummy_tanks").is_none(), "{}", axis["dummy_tanks"]);
        assert!(events_for(&state, &aliases, &events, Some(Side::Axis), false).is_empty());
        let cw = view_for(&state, &data, &aliases, Some(Side::Cw), false).unwrap();
        assert_eq!(cw["dummy_tanks"]["cw"][0], hex.to_string());
        assert_eq!(
            events_for(&state, &aliases, &events, Some(Side::Cw), false).len(),
            1
        );
    }

    #[test]
    fn lost_unseen_when_its_formation_leaves() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let hex = combat_hexes(&state, Side::Cw)[0].clone();
        place(&mut state, &data, Side::Cw, &hex).unwrap();
        let ids: Vec<PieceId> = state
            .pieces_at(&hex)
            .iter()
            .filter(|p| p.side == Side::Cw)
            .map(|p| p.id.clone())
            .collect();
        for id in &ids {
            state.put(id, Location::Eliminated);
        }
        let mut events = Vec::new();
        state.drop_orphan_dummy_tanks(&mut events);
        assert_eq!(
            events,
            [Event::DummyTankLost {
                side: Side::Cw,
                hex: hex.clone()
            }]
        );
        assert!(state.dummy_tanks.is_empty());
        let aliases = Aliases::new([1; 32]);
        assert!(events_for(&state, &aliases, &events, Some(Side::Axis), false).is_empty());
    }

    #[test]
    fn exposed_in_public() {
        let data = data();
        let mut state = at_phase_c(&data, on(), Side::Cw);
        let hex = combat_hexes(&state, Side::Cw)[0].clone();
        place(&mut state, &data, Side::Cw, &hex).unwrap();
        let mut events = Vec::new();
        state.expose_dummy_tank(Side::Cw, &hex, &mut events);
        state.expose_dummy_tank(Side::Cw, &hex, &mut events);
        assert_eq!(events.len(), 1, "once");
        let aliases = Aliases::new([1; 32]);
        assert_eq!(
            events_for(&state, &aliases, &events, Some(Side::Axis), false).len(),
            1
        );
        assert!(!state.has_dummy_tank(Side::Cw, &hex));
    }
}
