//! Forming desert raiders (27.13, 27.16, R-147): in the organisation
//! phase a raider counter not in play is placed with a unit of its own
//! nation, at maximum morale, whose recce or armoured-car TOE point it
//! takes for good. One eliminated is formed again the same way from the
//! fourth Game-Turn after, a stage later in the turn than it was lost
//! (E-026: lost in turn 55, stage 2, formed again in turn 59, stage 3).

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::oa::UnitId;
use cna_data::scenario::{Side, Status, Subject, UnitEntry};
use cna_data::time::GameTime;

/// Game-Turns an eliminated raider waits before it is formed again
/// (27.16).
const REFORM_TURNS: u8 = 4;

/// The first stage a raider lost at `lost` may be formed again (27.16,
/// E-026): four Game-Turns on, one stage later.
pub(crate) fn reform_from(lost: GameTime) -> GameTime {
    let turn = lost.game_turn.saturating_add(REFORM_TURNS);
    if lost.opstage >= 3 {
        GameTime::new(turn.saturating_add(1), 1)
    } else {
        GameTime::new(turn, lost.opstage + 1)
    }
}

impl GameState {
    /// Marks raider `id` eliminated now, for 27.16's wait.
    pub(crate) fn lose_raider(&mut self, id: &PieceId) {
        if let Some(p) = self.pieces.get_mut(id) {
            p.location = Location::Eliminated;
        }
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        self.raiders_lost.insert(id.clone(), now);
    }

    /// Whether `player` may form `raider` from `from` now, and where.
    fn check_form(
        &self,
        data: &Data,
        player: Side,
        raider: &PieceId,
        from: &PieceId,
    ) -> Result<HexId, Illegal> {
        let bad = |why: String| Illegal::new("27.13", why);
        if self.clock.phase != Phase::Organisation {
            return Err(bad(format!(
                "a raider is formed in the organisation phase, not at {}",
                self.clock
            )));
        }
        let unit_id = raider
            .unit()
            .ok_or_else(|| bad(format!("{raider} is not a counter")))?;
        let unit = data
            .oa
            .unit(&unit_id)
            .filter(|u| u.raider.is_some())
            .ok_or_else(|| bad(format!("{raider} is not a raider counter")))?;
        let values = unit.raider.as_ref().expect("filtered");
        let side = side_of(&unit_id);
        if side != player {
            return Err(bad(format!("{raider} is {side}'s, not {player}'s")));
        }
        if let Some(p) = self.pieces.get(raider)
            && p.location != Location::Eliminated
        {
            return Err(bad(format!("{raider} is already in play")));
        }
        if let Some(lost) = self.raiders_lost.get(raider) {
            let ready = reform_from(*lost);
            let now = GameTime::new(self.clock.game_turn, self.clock.stage);
            if now < ready {
                return Err(Illegal::new(
                    "27.16",
                    format!(
                        "{raider} was eliminated in Game-Turn {}, stage {}: it is formed again from turn {}, stage {}",
                        lost.game_turn, lost.opstage, ready.game_turn, ready.opstage
                    ),
                ));
            }
        }
        let source = self
            .pieces
            .get(from)
            .filter(|p| p.side == player && p.kind == PieceKind::Combat)
            .ok_or_else(|| bad(format!("{from} is not one of {player}'s combat units")))?;
        let hex = source
            .hex()
            .cloned()
            .ok_or_else(|| bad(format!("{from} is not on the map")))?;
        let source_unit = from
            .unit()
            .and_then(|u| data.oa.unit(&u))
            .ok_or_else(|| bad(format!("{from} is not on any OA sheet")))?;
        if source_unit.id.nation() != unit_id.nation() {
            return Err(bad(format!(
                "{raider} is formed from a unit of its own nation (27.2b)"
            )));
        }
        if !source_unit
            .vehicle
            .is_some_and(|v| values.formed_from_vehicle.contains(&v))
        {
            return Err(bad(format!(
                "{from} is not a recce or armoured-car unit (R-147)"
            )));
        }
        // Morale training (17.3x) is not simulated: every unit is at its
        // basic morale but one still in training.
        if source.status == Some(Status::InTraining) {
            return Err(bad(format!(
                "{from} is in training, not at its maximum morale"
            )));
        }
        // The unit keeps a point: the rules do not say a raider may take
        // a unit's last, and a unit at no TOE is gone.
        if source.toe < 2 {
            return Err(bad(format!("{from} would have no TOE points left")));
        }
        Ok(hex)
    }

    /// Every (raider counter, unit) pair `player` may form now (27.13),
    /// for the browser's phase C control.
    pub fn raider_formings(&self, data: &Data, player: Side) -> Vec<(PieceId, PieceId)> {
        if self.clock.phase != Phase::Organisation || self.to_act() != Some(player) {
            return Vec::new();
        }
        let raiders: Vec<PieceId> = data
            .oa
            .units()
            .filter(|u| u.raider.is_some() && side_of(&u.id) == player)
            .map(|u| PieceId::from(&u.id))
            .collect();
        let mut out = Vec::new();
        for raider in &raiders {
            for p in self.pieces.values() {
                if p.side == player
                    && p.kind == PieceKind::Combat
                    && self.check_form(data, player, raider, &p.id).is_ok()
                {
                    out.push((raider.clone(), p.id.clone()));
                }
            }
        }
        out
    }

    /// `Order::FormRaider` (27.13, 27.16, R-147): `raider`, a counter not
    /// in play, is formed in `from`'s hex from one of its TOE points.
    pub(crate) fn form_raider(
        &mut self,
        data: &Data,
        player: Side,
        raider: &PieceId,
        from: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let hex = self.check_form(data, player, raider, from)?;
        let unit = raider
            .unit()
            .and_then(|u| data.oa.unit(&u))
            .expect("checked");
        let weapon = self.pieces[from]
            .weapons
            .iter()
            .find(|w| w.points > 0)
            .map(|w| w.weapon.clone());
        let took =
            self.take_loss_entry(from, None) || self.take_loss_entry(from, weapon.as_deref());
        debug_assert!(took, "a unit with two TOE points gives one");
        let mut piece = match self.pieces.get(raider) {
            Some(p) => p.clone(),
            None => crate::setup::piece_for(player, unit, &data.chars, &data.oa),
        };
        piece.location = Location::Hex(hex.clone());
        // Formed, it is on the map until it moves (27.31).
        piece.hidden = false;
        piece.cp_spent = Cp::ZERO;
        piece.cp_enemy_half = Cp::ZERO;
        piece.contact = false;
        piece.engaged = false;
        self.pieces.insert(raider.clone(), piece);
        Ok(vec![Event::RaiderFormed {
            piece: raider.clone(),
            from: from.clone(),
            hex,
        }])
    }
}

fn side_of(unit: &UnitId) -> Side {
    match unit.nation() {
        cna_data::oa::Nation::Cw => Side::Cw,
        _ => Side::Axis,
    }
}

/// A set-up entry that is the counter alone.
pub(crate) fn alone(unit: &UnitId) -> UnitEntry {
    UnitEntry {
        subject: Subject::Unit(unit.clone()),
        less: Vec::new(),
        assigned: Vec::new(),
        detached: Vec::new(),
        attached: Vec::new(),
        consists_of: None,
        under: None,
        alone: true,
        status: None,
        toe: None,
        notes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{moving, pid};
    use crate::order::Order;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// Phase C of Graziani's set-up state.
    fn organising(data: &Data) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        state.clock.phase = Phase::Organisation;
        state.clock.half = None;
        state.clock.segment = None;
        state
    }

    fn form(raider: &str, from: &str) -> Order {
        Order::FormRaider {
            raider: pid(raider),
            from: pid(from),
        }
    }

    #[test]
    fn a_raider_is_formed_from_an_armoured_car_point_on_the_map_27_13() {
        let data = graziani();
        let mut state = organising(&data);
        let hus = pid("unit:cw:11-hus");
        let toe = state.pieces[&hus].toe;
        let hex = state.pieces[&hus]
            .hex()
            .cloned()
            .expect("11 Hussars on the map");
        let events = state
            .apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"))
            .unwrap();
        assert!(
            matches!(&events[0], Event::RaiderFormed { piece, .. } if piece == &pid("unit:cw:lrdg-1"))
        );
        let lrdg = &state.pieces[&pid("unit:cw:lrdg-1")];
        assert_eq!(lrdg.kind, PieceKind::Raider);
        assert_eq!(lrdg.side, Side::Cw);
        assert_eq!(lrdg.hex(), Some(&hex));
        assert!(!lrdg.hidden, "27.31: formed, it is on the map");
        assert_eq!(lrdg.cpa.as_ref().and_then(|r| r.value), Some(50));
        assert_eq!(state.pieces[&hus].toe, toe - 1, "the point never reverts");
        // The same counter twice, and the other side's counter, are not
        // to be had.
        let twice = state.apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"));
        assert!(twice.unwrap_err().reason.contains("already in play"));
        let dsa = state.apply(&data, Side::Cw, form("unit:de:dsa", "unit:cw:11-hus"));
        assert!(dsa.is_err());
    }

    #[test]
    fn only_a_recce_unit_of_the_raider_s_nation_forms_one_in_phase_c() {
        let data = graziani();
        let mut state = organising(&data);
        // An infantry battalion has no recce point.
        let krrc = state.apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:1-krrc"));
        assert!(krrc.unwrap_err().reason.contains("recce"));
        // Not outside the organisation phase.
        let mut moving = moving(&data, 3, Side::Cw);
        let late = moving.apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"));
        assert!(late.unwrap_err().reason.contains("organisation phase"));
    }

    #[test]
    fn an_eliminated_raider_is_formed_again_four_turns_on_27_16() {
        // E-026's example: lost in turn 55, stage 2; formed again in
        // turn 59, stage 3.
        assert_eq!(reform_from(GameTime::new(55, 2)), GameTime::new(59, 3));
        assert_eq!(reform_from(GameTime::new(55, 3)), GameTime::new(60, 1));
        let data = graziani();
        let mut state = organising(&data);
        state
            .apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"))
            .unwrap();
        state.clock.game_turn = 55;
        state.clock.stage = 2;
        state.lose_raider(&pid("unit:cw:lrdg-1"));
        state.clock.game_turn = 59;
        let early = state.apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"));
        assert_eq!(early.unwrap_err().case, "27.16");
        state.clock.stage = 3;
        state
            .apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"))
            .unwrap();
        assert!(state.pieces[&pid("unit:cw:lrdg-1")].hex().is_some());
    }

    #[test]
    fn a_formed_raider_that_moves_is_off_the_map_again_27_31() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let movement = state.clock;
        state.clock.phase = Phase::Organisation;
        state.clock.half = None;
        state.clock.segment = None;
        state
            .apply(&data, Side::Cw, form("unit:cw:lrdg-1", "unit:cw:11-hus"))
            .unwrap();
        state.clock = movement;
        let id = pid("unit:cw:lrdg-1");
        let at = state.pieces[&id].hex().cloned().unwrap();
        let to = data
            .map
            .neighbours(&at)
            .into_iter()
            .find(|h| {
                state.pieces_at(h).is_empty()
                    && state
                        .clone()
                        .apply(
                            &data,
                            Side::Cw,
                            crate::movement::test_support::mv(&["unit:cw:lrdg-1"], &[h.as_str()]),
                        )
                        .is_ok()
            })
            .expect("a hex next to the Hussars the raider may enter");
        state
            .apply(
                &data,
                Side::Cw,
                crate::movement::test_support::mv(&["unit:cw:lrdg-1"], &[to.as_str()]),
            )
            .unwrap();
        assert_eq!(state.pieces[&id].hex(), Some(&to));
        assert!(state.pieces[&id].hidden);
    }

    #[test]
    fn the_view_offers_its_viewer_the_raiders_it_may_form() {
        let data = graziani();
        let state = organising(&data);
        let offered = state.raider_formings(&data, Side::Cw);
        assert!(offered.contains(&(pid("unit:cw:lrdg-1"), pid("unit:cw:11-hus"))));
        assert!(
            offered
                .iter()
                .all(|(r, _)| r.as_str().starts_with("unit:cw:lrdg-"))
        );
        let v = crate::view::view_for(
            &state,
            &data,
            &crate::view::Aliases::new([0; 32]),
            Some(Side::Cw),
            false,
        )
        .unwrap();
        let list = v["raider_formings"].as_array().unwrap();
        assert!(list.iter().any(|f| f["raider"] == "unit:cw:lrdg-1"
            && f["from"] == "unit:cw:11-hus"
            && f["name"] == "LRDG 1"));
        let axis = crate::view::view_for(
            &state,
            &data,
            &crate::view::Aliases::new([0; 32]),
            Some(Side::Axis),
            false,
        )
        .unwrap();
        let theirs = axis["raider_formings"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(theirs.iter().all(|f| f["raider"] == "unit:de:dsa"));
        // Outside phase C nothing is offered.
        let later = moving(&data, 3, Side::Cw);
        assert!(later.raider_formings(&data, Side::Cw).is_empty());
    }
}
