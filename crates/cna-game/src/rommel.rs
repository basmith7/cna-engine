//! Rommel (31.0, R-135): his counter on the map lifts the Axis initiative
//! rating (7.1x), and at the start of each Game-Turn he is present a
//! two-dice 12 recalls him to Germany for the turn: out of play, he
//! neither moves nor can be caught, and comes back to the hex he left.

use crate::data::Data;
use crate::event::Event;
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::initiative::AxisSituation;
use cna_data::map::HexId;
use cna_data::oa::{Commander, Nation};
use cna_data::scenario::Side;

impl GameState {
    /// Rommel's piece on a map hex, the holding boxes excepted (7.1x).
    pub(crate) fn rommel_on_map(&self) -> Option<&Piece> {
        self.pieces
            .values()
            .find(|p| p.kind == PieceKind::Commander && p.hex().is_some())
    }

    /// Rommel commanding on the map: there, and not recalled this turn.
    pub(crate) fn rommel_at(&self, hex: &HexId) -> bool {
        !self.rommel_recalled && self.rommel_on_map().and_then(Piece::hex) == Some(hex)
    }

    /// 17.28, 31.0: Axis units sharing Rommel's hex add his
    /// `stack_morale_bonus` to their adjusted morale, past +3 if need be.
    pub(crate) fn rommel_morale_bonus(&self, data: &Data, ids: &[PieceId]) -> i32 {
        let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) else {
            return 0;
        };
        let with_him = ids.iter().any(|id| {
            let p = &self.pieces[id];
            p.side == Side::Axis && p.hex().is_some_and(|h| self.rommel_at(h))
        });
        if with_him { c.stack_morale_bonus } else { 0 }
    }

    /// The Axis initiative rating (7.1x): Rommel's row while he is on the
    /// map, his `initiative_while_recalled` while he is in Germany, else
    /// by the German combat units on the map.
    pub(crate) fn axis_initiative_rating(&self, data: &Data) -> u8 {
        if let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) {
            if self.rommel_recalled {
                return u8::try_from(c.initiative_while_recalled).unwrap_or(0);
            }
            return data
                .initiative
                .axis(AxisSituation::RommelOnMap)
                .expect("initiative-ratings.json has every situation");
        }
        let germans = self.pieces.values().any(|p| {
            p.nation == Some(Nation::De) && p.kind == PieceKind::Combat && p.hex().is_some()
        });
        let situation = if germans {
            AxisSituation::GermanLandCombatUnitsNoRommel
        } else {
            AxisSituation::NoGermanCombatUnits
        };
        data.initiative
            .axis(situation)
            .expect("initiative-ratings.json has every situation")
    }

    /// 31.0's recall roll, at a Game-Turn's start while Rommel is on the
    /// map: two dice, and his `recall_roll` total sends him to Germany for
    /// the turn. He is back the next turn unless it comes up again.
    pub(crate) fn roll_rommel_recall(&mut self, data: &Data, events: &mut Vec<Event>) {
        self.rommel_recalled = false;
        let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) else {
            return;
        };
        let target = c.recall_roll;
        let a = self.rng.d6();
        let b = self.rng.d6();
        for value in [a, b] {
            events.push(Event::DieRolled {
                purpose: "Rommel's recall".into(),
                value,
            });
        }
        if u32::from(a + b) == target {
            self.rommel_recalled = true;
            events.push(Event::RommelRecalled);
        }
    }
}

fn commander<'a>(data: &'a Data, p: &Piece) -> Option<&'a Commander> {
    data.oa.unit(&p.id.unit()?)?.commander.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::Location;
    use crate::rng::GameRng;
    use crate::state::test_support::auto_place;
    use cna_data::oa::UnitId;

    fn crusader() -> (Data, GameState) {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = GameState::new(&data, 3).unwrap();
        auto_place(&data, &mut state);
        (data, state)
    }

    fn rommel() -> PieceId {
        PieceId::from(UnitId::parse("unit:de:rommel").unwrap())
    }

    #[test]
    fn rommel_on_the_map_rates_the_axis_six_and_three_when_recalled() {
        let (data, mut state) = crusader();
        assert_eq!(state.axis_initiative_rating(&data), 6);
        state.rommel_recalled = true;
        assert_eq!(state.axis_initiative_rating(&data), 3);
        state.rommel_recalled = false;
        state.pieces.get_mut(&rommel()).unwrap().location = Location::Eliminated;
        // German combat units but no Rommel.
        assert_eq!(state.axis_initiative_rating(&data), 3);
    }

    #[test]
    fn a_twelve_recalls_rommel_and_anything_else_keeps_him() {
        let (data, state) = crusader();
        let mut recalled = 0;
        for seed in 0..400 {
            let mut s = state.clone();
            s.rng = GameRng::new(seed);
            s.rommel_recalled = true;
            let mut events = Vec::new();
            s.roll_rommel_recall(&data, &mut events);
            let dice: Vec<u8> = events
                .iter()
                .filter_map(|e| match e {
                    Event::DieRolled { value, .. } => Some(*value),
                    _ => None,
                })
                .collect();
            assert_eq!(dice.len(), 2);
            let twelve = dice == [6, 6];
            assert_eq!(s.rommel_recalled, twelve, "seed {seed}");
            assert_eq!(events.contains(&Event::RommelRecalled), twelve);
            recalled += usize::from(twelve);
        }
        assert!(recalled > 0, "no 12 in 400 seeds");
    }

    #[test]
    fn no_rommel_no_roll() {
        let (data, mut state) = crusader();
        state.pieces.get_mut(&rommel()).unwrap().location = Location::Eliminated;
        let draws = state.rng.draws();
        let mut events = Vec::new();
        state.roll_rommel_recall(&data, &mut events);
        assert!(events.is_empty());
        assert_eq!(state.rng.draws(), draws);
    }

    #[test]
    fn axis_units_with_rommel_fight_a_point_braver() {
        let (data, mut state) = crusader();
        let hex = state.pieces[&rommel()].hex().unwrap().clone();
        let with: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex) && p.kind != PieceKind::Commander)
            .map(|p| p.id.clone())
            .collect();
        let away: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.hex().is_some_and(|h| h != &hex))
            .map(|p| p.id.clone())
            .take(3)
            .collect();
        assert!(!with.is_empty() && !away.is_empty());
        assert_eq!(state.rommel_morale_bonus(&data, &with), 1);
        assert_eq!(state.rommel_morale_bonus(&data, &away), 0);
        state.rommel_recalled = true;
        assert_eq!(state.rommel_morale_bonus(&data, &with), 0);
    }

    #[test]
    fn a_recalled_rommel_does_not_move() {
        let (_, mut state) = crusader();
        state.rommel_recalled = true;
        let p = state.pieces[&rommel()].clone();
        assert_eq!(state.able_to_move(&p).unwrap_err().case, "31.0");
        state.rommel_recalled = false;
        assert!(state.able_to_move(&p).is_ok());
    }
}
