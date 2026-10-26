//! Rommel (31.0, R-135): his counter on the map lifts the Axis initiative
//! rating (7.1x), and at the start of each Game-Turn he is present a
//! two-dice 12 recalls him to Germany for the turn: out of play, he
//! neither moves nor can be caught, and comes back to the hex he left.

use crate::data::Data;
use crate::event::Event;
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
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

    /// R-137: as an OpStage begins, every Axis unit in the hex of a
    /// commanding Rommel borrows his `escort_cpa_bonus`; everyone else
    /// has none.
    pub(crate) fn mark_rommel_escort(&mut self, data: &Data) {
        let bonus = self
            .rommel_on_map()
            .filter(|_| !self.rommel_recalled)
            .and_then(|r| Some((r.hex()?.clone(), commander(data, r)?.escort_cpa_bonus)));
        for p in self.pieces.values_mut() {
            p.rommel_cpa = match &bonus {
                Some((hex, n))
                    if p.side == Side::Axis
                        && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                        && p.hex() == Some(hex) =>
                {
                    *n
                }
                _ => 0,
            };
        }
    }

    /// R-137: a unit no longer in a commanding Rommel's hex loses his
    /// bonus at once and earns the DP it spared: those for CP spent past
    /// its own CPA but inside the raised one (6.21).
    pub(crate) fn part_from_rommel(&mut self, events: &mut Vec<Event>) {
        let hex = self
            .rommel_on_map()
            .filter(|_| !self.rommel_recalled)
            .and_then(Piece::hex)
            .cloned();
        let parted: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.rommel_cpa > 0 && (hex.is_none() || p.hex() != hex.as_ref()))
            .map(|p| p.id.clone())
            .collect();
        for id in parted {
            let p = &self.pieces[&id];
            let over = |cpa: Option<u32>| {
                p.cp_spent
                    .saturating_sub(Cp::whole(cpa.unwrap_or(0)))
                    .floor()
            };
            let dp = over(self.base_cpa(p)) - over(self.cpa(p));
            let p = self.pieces.get_mut(&id).expect("listed");
            p.rommel_cpa = 0;
            if dp > 0 {
                p.cohesion.earn_dp(dp);
                events.push(Event::Disorganised {
                    piece: id,
                    dp,
                    level: p.cohesion.level,
                });
            }
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

    /// The Axis units in Rommel's hex, him excepted.
    fn with_him(state: &GameState) -> Vec<PieceId> {
        let hex = state.pieces[&rommel()].hex().unwrap().clone();
        state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex) && p.kind != PieceKind::Commander)
            .map(|p| p.id.clone())
            .collect()
    }

    // R-137: every unit in his hex as an OpStage begins moves on CPA +5;
    // nobody else, and not Rommel himself.
    #[test]
    fn units_with_rommel_at_a_stages_start_move_five_further() {
        let (_, state) = crusader();
        let with = with_him(&state);
        assert!(!with.is_empty());
        for id in &with {
            let p = &state.pieces[id];
            assert_eq!(p.rommel_cpa, 5, "{id}");
            assert_eq!(state.cpa(p), state.base_cpa(p).map(|c| c + 5), "{id}");
        }
        let away = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.hex().is_some() && !with.contains(&p.id))
            .unwrap();
        assert_eq!(away.rommel_cpa, 0);
        assert_eq!(state.cpa(away), state.base_cpa(away));
        assert_eq!(state.pieces[&rommel()].rommel_cpa, 0);
    }

    // R-137: a recalled Rommel gives nobody the bonus.
    #[test]
    fn a_recalled_rommel_gives_no_bonus() {
        let (data, mut state) = crusader();
        state.rommel_recalled = true;
        state.mark_rommel_escort(&data);
        assert!(state.pieces.values().all(|p| p.rommel_cpa == 0));
    }

    // R-137: parting loses the bonus at once and charges the DP it had
    // spared, one per CP spent past the unit's own CPA, at most five.
    #[test]
    fn parting_from_rommel_charges_the_spared_dp() {
        let (_, mut state) = crusader();
        let id = with_him(&state)
            .into_iter()
            .find(|id| state.base_cpa(&state.pieces[id]).is_some())
            .unwrap();
        let base = state.base_cpa(&state.pieces[&id]).unwrap();
        let elsewhere = state
            .pieces
            .values()
            .find_map(|p| {
                p.hex()
                    .filter(|h| *h != state.pieces[&rommel()].hex().unwrap())
            })
            .unwrap()
            .clone();
        let mut events = Vec::new();
        // Spent three past its own CPA, inside the raised one: no DP yet.
        state.charge(&id, Cp::whole(base + 3), false, &mut events);
        assert!(events.is_empty(), "{events:?}");
        let level = state.pieces[&id].cohesion.level;
        // Still together: nothing happens.
        state.part_from_rommel(&mut events);
        assert_eq!(state.pieces[&id].rommel_cpa, 5);
        state.pieces.get_mut(&id).unwrap().location = Location::Hex(elsewhere);
        state.part_from_rommel(&mut events);
        let p = &state.pieces[&id];
        assert_eq!(p.rommel_cpa, 0);
        assert_eq!(state.cpa(p), Some(base));
        assert_eq!(p.cohesion.level, level - 3);
        assert!(events.contains(&Event::Disorganised {
            piece: id.clone(),
            dp: 3,
            level: level - 3,
        }));
    }

    // R-137: 8.17's cap and whether a unit is motorised go by its own CPA.
    #[test]
    fn the_foot_cap_and_mobility_ignore_the_bonus() {
        let (data, mut state) = crusader();
        let foot = state
            .pieces
            .values()
            .find(|p| p.kind == PieceKind::Combat && state.base_cpa(p) == Some(10))
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&foot).unwrap().rommel_cpa = 5;
        let p = &state.pieces[&foot];
        assert_eq!(state.cpa(p), Some(15));
        assert_eq!(
            state.mobility(&data, p),
            cna_rules::movement::Mobility::Foot
        );
        let (cap, case, _) = state.voluntary_cap(&foot).unwrap();
        assert_eq!((cap, case), (Cp::whole(15), "8.1x"));
    }

    // 31.0: Rommel may react when an enemy unit comes next to him, like
    // a motorised combat unit; not while he is in Germany.
    #[test]
    fn rommel_is_offered_a_reaction() {
        let (data, mut state) = crusader();
        let at = state.pieces[&rommel()].hex().unwrap().clone();
        let (enemy, next) = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Combat && p.hex().is_some())
            .find_map(|p| {
                let free = data
                    .map
                    .neighbours(&at)
                    .into_iter()
                    .find(|n| state.pieces.values().all(|q| q.hex() != Some(n)))?;
                Some((p.id.clone(), free))
            })
            .unwrap();
        state.pieces.get_mut(&enemy).unwrap().location = Location::Hex(next.clone());
        let offered = state.reactors_next_to(&data, &next, std::slice::from_ref(&enemy), false);
        assert!(offered.contains(&rommel()), "{offered:?}");
        state.rommel_recalled = true;
        let offered = state.reactors_next_to(&data, &next, &[enemy], false);
        assert!(!offered.contains(&rommel()), "{offered:?}");
    }
}
