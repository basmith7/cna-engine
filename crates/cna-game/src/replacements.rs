//! Replacement points from repair (22.5, 20.4x): repaired TOE points
//! wait in the hex they were repaired in, and a unit there absorbs them
//! in phase C at a CP per two points, never above its printed strength
//! (19.6x).

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::oa::ToeEntry;
use cna_data::scenario::Side;
use cna_data::weapons::WeaponClass;

impl GameState {
    /// Whether `weapon`'s points fit `piece` (19.6x): a recce vehicle
    /// class fits a recce unit of that class (R-125); a model the unit
    /// lists fits it; any tank fits a unit with tanks.
    pub(crate) fn fits(&self, data: &Data, piece: &Piece, weapon: &str) -> bool {
        let unit = piece.id.unit().and_then(|u| data.oa.unit(&u));
        if unit
            .and_then(|u| u.vehicle)
            .is_some_and(|v| v.as_str() == weapon)
        {
            return true;
        }
        if piece.weapons.iter().any(|w| w.weapon == weapon) {
            return true;
        }
        let tank = piece
            .nation
            .and_then(|n| data.weapons.resolve(n, weapon))
            .is_some_and(|r| r.class == WeaponClass::Tank);
        tank && self.parts(data, piece).iter().any(|p| p.tank)
    }

    /// The most TOE points `piece` may hold (19.61): its sheet's printed
    /// strength, or the characteristics row's maximum where that is more
    /// (a unit that arrived under strength may be built up to it).
    pub(crate) fn toe_ceiling(&self, data: &Data, piece: &Piece) -> u32 {
        let Some(unit) = piece.id.unit().and_then(|u| data.oa.unit(&u)) else {
            return piece.toe;
        };
        let row = unit
            .id_code
            .as_deref()
            .and_then(|code| data.chars.row(unit.id.nation(), code));
        let max = row
            .and_then(|r| r.max_toe.as_ref())
            .and_then(|m| m.value)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);
        crate::setup::toe_points(unit, row).max(max).max(piece.toe)
    }

    /// `Order::Absorb` (19.61, 20.41): in phase C `piece` takes `points`
    /// of the player's repaired `weapon` points waiting in its hex, a CP
    /// per two (rounded up) from it and from its parent if the parent
    /// stands there too.
    pub(crate) fn absorb_replacements(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        weapon: &str,
        points: u32,
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::Organisation {
            return Err(Illegal::new(
                "20.41",
                format!(
                    "replacement points are absorbed in phase C, not at {}",
                    self.clock
                ),
            ));
        }
        let p = self
            .pieces
            .get(piece)
            .filter(|p| p.side == player && matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
            .ok_or_else(|| {
                Illegal::new("20.41", format!("{piece} is not one of {player}'s units"))
            })?;
        let Some(hex) = p.hex().cloned() else {
            return Err(Illegal::new("20.41", format!("{piece} is not on the map")));
        };
        let waiting = self
            .repaired
            .get(&player)
            .and_then(|m| m.get(&hex))
            .and_then(|m| m.get(weapon))
            .copied()
            .unwrap_or(0);
        if points == 0 || points > waiting {
            return Err(Illegal::new(
                "20.41",
                format!("{waiting} repaired {weapon} points wait in {hex}, not {points}"),
            ));
        }
        if !self.fits(data, p, weapon) {
            return Err(Illegal::new(
                "19.6x",
                format!("{weapon} points do not rebuild {piece}"),
            ));
        }
        let ceiling = self.toe_ceiling(data, p);
        if p.toe + points > ceiling {
            return Err(Illegal::new(
                "19.61",
                format!(
                    "{piece} would hold {} TOE points: at most {ceiling}",
                    p.toe + points
                ),
            ));
        }
        let parent = p
            .parent
            .as_ref()
            .map(PieceId::from)
            .filter(|id| self.pieces.get(id).is_some_and(|q| q.hex() == Some(&hex)));
        // A model joins the unit's weapon list; a recce class's points
        // are unlisted, rated by the unit's ID code.
        let listed = p
            .nation
            .is_some_and(|n| data.weapons.resolve(n, weapon).is_some());
        let cp = Cp::whole(points.div_ceil(2));
        let mut events = vec![Event::Absorbed {
            piece: piece.clone(),
            weapon: weapon.to_string(),
            points,
            cp,
        }];
        let unit = self.pieces.get_mut(piece).expect("checked");
        unit.toe += points;
        if listed {
            match unit.weapons.iter_mut().find(|w| w.weapon == weapon) {
                Some(w) => w.points += points,
                None => unit.weapons.push(ToeEntry {
                    weapon: weapon.to_string(),
                    points,
                }),
            }
        }
        for id in std::iter::once(piece.clone()).chain(parent) {
            self.charge(&id, cp, false, &mut events);
        }
        let mine = self.repaired.get_mut(&player).expect("checked");
        let here = mine.get_mut(&hex).expect("checked");
        let left = here.get_mut(weapon).expect("checked");
        *left -= points;
        if *left == 0 {
            here.remove(weapon);
        }
        if here.is_empty() {
            mine.remove(&hex);
        }
        if mine.is_empty() {
            self.repaired.remove(&player);
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::pid;
    use crate::order::Order;
    use crate::state::Modules;
    use crate::state::test_support::auto_place;
    use cna_data::map::HexId;

    const TANKS: &str = "unit:it:iim-tank-bn";
    const MALETTI: &str = "unit:it:maletti-div-hq";

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// Phase C of GT1, the Axis to act, with the tank battalion three
    /// points down and those three repaired in its hex.
    fn organising(data: &Data, modules: Modules) -> (GameState, HexId, String) {
        let mut state = GameState::with_modules(data, 3, modules).unwrap();
        auto_place(data, &mut state);
        let tanks = pid(TANKS);
        let p = state.pieces.get_mut(&tanks).unwrap();
        let w = p.weapons.iter_mut().find(|w| w.points >= 3).unwrap();
        w.points -= 3;
        let weapon = w.weapon.clone();
        p.toe -= 3;
        let hex = p.hex().unwrap().clone();
        state.repaired.insert(
            Side::Axis,
            [(hex.clone(), [(weapon.clone(), 3)].into())].into(),
        );
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::Organisation);
        (state, hex, weapon)
    }

    fn absorb(piece: &str, weapon: &str, points: u32) -> Order {
        Order::Absorb {
            piece: pid(piece),
            weapon: weapon.to_string(),
            points,
        }
    }

    #[test]
    fn a_unit_absorbs_repaired_points_in_its_hex_at_a_cp_per_two_19_61() {
        let data = graziani();
        let (mut state, _, weapon) = organising(&data, Modules::ALL);
        let (tanks, hq) = (pid(TANKS), pid(MALETTI));
        let toe = state.pieces[&tanks].toe;
        let parent_with = state.pieces[&hq].hex() == state.pieces[&tanks].hex();
        let before = (state.pieces[&tanks].cp_spent, state.pieces[&hq].cp_spent);
        state
            .apply(&data, Side::Axis, absorb(TANKS, &weapon, 3))
            .unwrap();
        let p = &state.pieces[&tanks];
        assert_eq!(p.toe, toe + 3);
        assert!(
            p.weapons
                .iter()
                .any(|w| w.weapon == weapon && w.points >= 3)
        );
        assert_eq!(p.cp_spent, before.0 + Cp::whole(2));
        if parent_with {
            assert_eq!(state.pieces[&hq].cp_spent, before.1 + Cp::whole(2));
        }
        assert!(state.repaired.is_empty());
    }

    #[test]
    fn only_points_waiting_in_the_unit_s_hex_and_only_in_phase_c_20_41() {
        let data = graziani();
        let (mut state, _, weapon) = organising(&data, Modules::ALL);
        let err = state
            .apply(&data, Side::Axis, absorb(TANKS, &weapon, 4))
            .unwrap_err();
        assert_eq!(err.case, "20.41");
        let far = state
            .pieces
            .values()
            .find(|q| {
                q.side == Side::Axis
                    && q.kind == PieceKind::Combat
                    && q.hex().is_some()
                    && q.hex() != state.pieces[&pid(TANKS)].hex()
            })
            .unwrap()
            .id
            .clone();
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Absorb {
                    piece: far,
                    weapon: weapon.clone(),
                    points: 1,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "20.41");
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        let err = state
            .apply(&data, Side::Axis, absorb(TANKS, &weapon, 1))
            .unwrap_err();
        assert_eq!(err.case, "20.41");
    }

    #[test]
    fn never_above_printed_strength_19_61() {
        let data = graziani();
        let (mut state, hex, weapon) = organising(&data, Modules::ALL);
        let tanks = pid(TANKS);
        let ceiling = state.toe_ceiling(&data, &state.pieces[&tanks]);
        state.pieces.get_mut(&tanks).unwrap().toe = ceiling - 1;
        let err = state
            .apply(&data, Side::Axis, absorb(TANKS, &weapon, 2))
            .unwrap_err();
        assert_eq!(err.case, "19.61");
        assert_eq!(state.repaired[&Side::Axis][&hex][&weapon], 3);
    }

    #[test]
    fn tank_points_do_not_rebuild_infantry_19_6x() {
        let data = graziani();
        let (mut state, hex, weapon) = organising(&data, Modules::ALL);
        let foot = state.pieces.values().find(|q| {
            q.side == Side::Axis
                && q.kind == PieceKind::Combat
                && q.hex() == Some(&hex)
                && !state.parts(&data, q).iter().any(|p| p.tank)
        });
        let Some(foot) = foot.map(|q| q.id.clone()) else {
            // Nobody else in the hex: stand a Libyan battalion there.
            let bn = pid("unit:it:viii-libyan-bn");
            state.pieces.get_mut(&bn).unwrap().location = crate::piece::Location::Hex(hex);
            let err = state
                .apply(
                    &data,
                    Side::Axis,
                    absorb("unit:it:viii-libyan-bn", &weapon, 1),
                )
                .unwrap_err();
            assert_eq!(err.case, "19.6x");
            return;
        };
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Absorb {
                    piece: foot,
                    weapon,
                    points: 1,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "19.6x");
    }

    #[test]
    fn phase_c_opens_for_repaired_points_without_abstract_supply_20_41() {
        let data = graziani();
        let modules = Modules {
            breakdown: true,
            ..Modules::default()
        };
        let (mut state, _, weapon) = organising(&data, modules);
        state
            .apply(&data, Side::Axis, absorb(TANKS, &weapon, 2))
            .unwrap();
    }
}
