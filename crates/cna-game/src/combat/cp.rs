//! The CP of combat (SPI 6.3, 11.2x): charged by a hex's role in the
//! segment, to every unit in the hex, taking part or not, under a
//! ceiling per combat segment of 5 for a phasing unit and 3 for a
//! non-phasing one. A defender of a bad attack gets 2 back of what it
//! was charged (6.3's dagger, `cp-costs.json`) unless it barraged or was
//! barraged, together with any DP that CP earned over its CPA (6.21).

use crate::event::Event;
use crate::piece::PieceId;
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;

/// What a phasing unit pays at most in one combat segment (11.2x).
pub const PHASING_CEILING: Cp = Cp::whole(5);
/// What a non-phasing unit pays at most in one combat segment.
pub const NON_PHASING_CEILING: Cp = Cp::whole(3);
/// The refund on a final differential of −4 or worse.
pub const BAD_ATTACK_REFUND: Cp = Cp::whole(2);

impl GameState {
    fn pieces_in(&self, hex: &HexId) -> Vec<PieceId> {
        self.pieces
            .values()
            .filter(|p| p.hex() == Some(hex))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Charges every piece in `hex` `cost`, up to its ceiling.
    pub fn charge_combat_hex(&mut self, hex: &HexId, cost: Cp, events: &mut Vec<Event>) {
        let phasing = self.phasing();
        for id in self.pieces_in(hex) {
            let own = Some(self.pieces[&id].side) == phasing;
            let ceiling = if own {
                PHASING_CEILING
            } else {
                NON_PHASING_CEILING
            };
            let already = self.combat.charged.get(&id).copied().unwrap_or(Cp::ZERO);
            let add = cost.min(ceiling.saturating_sub(already));
            if add == Cp::ZERO {
                continue;
            }
            self.combat.charged.insert(id.clone(), already + add);
            self.charge(&id, add, !own, events);
            events.push(Event::CombatCp {
                piece: id,
                cost: add,
            });
        }
    }

    /// Gives every piece in `hex` back up to `refund` of the combat CP it
    /// was charged this segment, unless it barraged or was barraged
    /// (6.3), and with it any DP that CP had earned over the CPA
    /// (6.21): the unit ends as if it had paid the net cost outright.
    pub fn refund_combat_hex(&mut self, hex: &HexId, refund: Cp, events: &mut Vec<Event>) {
        for id in self.pieces_in(hex) {
            if self.combat.barrage_involved.contains(&id) {
                continue;
            }
            let already = self.combat.charged.get(&id).copied().unwrap_or(Cp::ZERO);
            let back = refund.min(already);
            if back == Cp::ZERO {
                continue;
            }
            self.combat.charged.insert(id.clone(), already - back);
            let cpa = Cp::whole(self.cpa(&self.pieces[&id]).unwrap_or(0));
            let p = self.pieces.get_mut(&id).expect("listed");
            let over = p.cp_spent.saturating_sub(cpa).floor();
            p.cp_spent = p.cp_spent.saturating_sub(back);
            p.cp_enemy_half = p.cp_enemy_half.saturating_sub(back);
            let dp = over - p.cp_spent.saturating_sub(cpa).floor();
            if dp > 0 {
                p.cohesion.refund_dp(dp);
            }
            events.push(Event::CombatCpRefunded {
                piece: id,
                cp: back,
                dp,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::movement::test_support::{hex, moving, pid};
    use cna_data::scenario::Side;

    #[test]
    fn every_unit_in_the_hex_pays_up_to_the_ceiling() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = moving(&data, 3, Side::Cw);
        let mut events = Vec::new();
        // C3922: four Commonwealth pieces, phasing. Barraged (3) then
        // assaulting (5): 5 in all, never 8.
        let c3922 = hex("C3922");
        state.charge_combat_hex(&c3922, Cp::whole(3), &mut events);
        for id in [
            "unit:cw:1-krrc",
            "unit:cw:3-cldgds",
            "unit:cw:4-rha",
            "unit:cw:7-med",
        ] {
            assert_eq!(state.pieces[&pid(id)].cp_spent, Cp::whole(3), "{id}");
        }
        state.charge_combat_hex(&c3922, Cp::whole(5), &mut events);
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(5));
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_enemy_half, Cp::ZERO);
        // The Axis at C4020, non-phasing: 3 at most, in the enemy half; a
        // bad attack gives 2 back unless the unit was in a barrage.
        let c4020 = hex("C4020");
        state.charge_combat_hex(&c4020, Cp::whole(3), &mut events);
        let bn = pid("unit:it:viii-libyan-bn");
        let hq = pid("unit:it:1st-libyan-infantry-hq");
        assert_eq!(state.pieces[&bn].hex(), Some(&c4020));
        assert_eq!(state.pieces[&hq].hex(), Some(&c4020));
        assert_eq!(state.pieces[&hq].cp_spent, Cp::whole(3));
        state.combat.barrage_involved.insert(hq.clone());
        state.refund_combat_hex(&c4020, BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&hq].cp_spent, Cp::whole(3));
        assert_eq!(state.pieces[&bn].cp_spent, Cp::whole(1));
        assert_eq!(state.pieces[&bn].cp_enemy_half, Cp::whole(1));
        assert_eq!(state.combat.charged[&bn], Cp::whole(1));
        assert!(events.contains(&Event::CombatCpRefunded {
            piece: bn.clone(),
            cp: BAD_ATTACK_REFUND,
            dp: 0,
        }));
        // Nothing more to give back once it is gone.
        state.refund_combat_hex(&c4020, BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&bn].cp_spent, Cp::ZERO);
        state.refund_combat_hex(&c4020, BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&bn].cp_spent, Cp::ZERO);
    }

    #[test]
    fn a_refund_takes_back_the_dp_the_refunded_cp_earned() {
        // 6.21 and 6.3 together: a defender already at its CPA, probed
        // at −4 or worse, nets no CP and so no DP; the 2 it spends
        // later in the stage earn 2, not 4. A full assault nets 1 CP
        // and 1 DP, and the next 2 make 3.
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = moving(&data, 3, Side::Cw);
        let mut events = Vec::new();
        let c4020 = hex("C4020");
        let bn = pid("unit:it:viii-libyan-bn");
        let cpa = Cp::whole(state.cpa(&state.pieces[&bn]).unwrap());
        state.pieces.get_mut(&bn).unwrap().cp_spent = cpa;
        state.charge_combat_hex(&c4020, Cp::whole(2), &mut events);
        state.refund_combat_hex(&c4020, BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&bn].cp_spent, cpa);
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 0);
        assert_eq!(state.pieces[&bn].cohesion.level, 0);
        assert!(events.contains(&Event::CombatCpRefunded {
            piece: bn.clone(),
            cp: BAD_ATTACK_REFUND,
            dp: 2,
        }));
        state.charge(&bn, Cp::whole(2), true, &mut events);
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 2);
        assert_eq!(state.pieces[&bn].cohesion.level, -2);

        let mut state = moving(&data, 3, Side::Cw);
        state.pieces.get_mut(&bn).unwrap().cp_spent = cpa;
        state.charge_combat_hex(&c4020, NON_PHASING_CEILING, &mut events);
        state.refund_combat_hex(&c4020, BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&bn].cp_spent, cpa + Cp::whole(1));
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 1);
        assert_eq!(state.pieces[&bn].cohesion.level, -1);
        state.charge(&bn, Cp::whole(2), true, &mut events);
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 3);
        assert_eq!(state.pieces[&bn].cohesion.level, -3);
    }
}
