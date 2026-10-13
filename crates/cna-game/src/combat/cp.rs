//! The CP of combat (SPI 6.3, 11.2x): charged by a hex's role in the
//! segment, to every unit in the hex, taking part or not, under a
//! ceiling per combat segment of 5 for a phasing unit and 3 for a
//! non-phasing one. A defender of a bad attack is charged 2 less (6.3's
//! asterisk, `cp-costs.json`) unless it barraged or was barraged.

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
    /// Charges every piece in `hex` `cost`, less `refund` for a piece that
    /// neither barraged nor was barraged this segment, up to its ceiling.
    pub fn charge_combat_hex(
        &mut self,
        hex: &HexId,
        cost: Cp,
        refund: Cp,
        events: &mut Vec<Event>,
    ) {
        let phasing = self.phasing();
        let ids: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.hex() == Some(hex))
            .map(|p| p.id.clone())
            .collect();
        for id in ids {
            let own = Some(self.pieces[&id].side) == phasing;
            let ceiling = if own {
                PHASING_CEILING
            } else {
                NON_PHASING_CEILING
            };
            let refund = if self.combat.barrage_involved.contains(&id) {
                Cp::ZERO
            } else {
                refund
            };
            let already = self.combat.charged.get(&id).copied().unwrap_or(Cp::ZERO);
            let add = cost
                .saturating_sub(refund)
                .min(ceiling.saturating_sub(already));
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
        state.charge_combat_hex(&c3922, Cp::whole(3), Cp::ZERO, &mut events);
        for id in [
            "unit:cw:1-krrc",
            "unit:cw:3-cldgds",
            "unit:cw:4-rha",
            "unit:cw:7-med",
        ] {
            assert_eq!(state.pieces[&pid(id)].cp_spent, Cp::whole(3), "{id}");
        }
        state.charge_combat_hex(&c3922, Cp::whole(5), Cp::ZERO, &mut events);
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(5));
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_enemy_half, Cp::ZERO);
        // The Axis at C4020, non-phasing: 3 at most, in the enemy half; a
        // bad attack refunds 2 unless the unit was in a barrage.
        let c4020 = hex("C4020");
        state.charge_combat_hex(&c4020, Cp::whole(3), BAD_ATTACK_REFUND, &mut events);
        let bn = pid("unit:it:xvii-libyan-bn");
        let hq = pid("unit:it:1st-libyan-infantry-hq");
        assert_eq!(state.pieces[&hq].hex(), Some(&c4020));
        assert_eq!(state.pieces[&hq].cp_spent, Cp::whole(1));
        assert_eq!(state.pieces[&hq].cp_enemy_half, Cp::whole(1));
        state.combat.barrage_involved.insert(hq.clone());
        state.charge_combat_hex(&c4020, Cp::whole(3), BAD_ATTACK_REFUND, &mut events);
        assert_eq!(state.pieces[&hq].cp_spent, Cp::whole(3));
        assert_eq!(state.pieces[&hq].cp_spent, Cp::whole(3));
        let _ = bn;
    }
}
