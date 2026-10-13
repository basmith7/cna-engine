//! Step 2 resolved (SPI 12.3x, 12.4x): every plotted fire pooled by
//! target, rolled, and applied together after the last roll.

use super::cp::{NON_PHASING_CEILING, PHASING_CEILING};
use super::strength::Kind;
use super::terrain::hex_shift;
use crate::data::Data;
use crate::event::Event;
use crate::piece::PieceId;
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::Nation;
use cna_rules::barrage::{BarrageResult, resolve, terrain_shift};
use cna_rules::strength::actual;
use cna_rules::supply::Role;
use std::collections::{BTreeMap, BTreeSet};

/// One barrage: the fires pooled against one target.
#[derive(Default)]
struct Pool {
    firers: Vec<PieceId>,
    hexes: BTreeSet<HexId>,
    raw: u32,
}

impl GameState {
    /// Rolls every barrage and applies the hits and pins (12.0): the
    /// phasing player's first. Forward guns pool across hexes (Italians
    /// within one hex, 12.1x); back guns fire by hex. Pins hold only
    /// non-gun units (12.44, E-002); losses come after the last roll
    /// (12.45).
    pub(crate) fn resolve_barrages(&mut self, data: &Data, events: &mut Vec<Event>) {
        let Some(phasing) = self.phasing() else {
            return;
        };
        let mut losses: Vec<(PieceId, u32)> = Vec::new();
        for side in [phasing, phasing.other()] {
            // Key: the target, and the firing hex where fire is not pooled.
            // Ammunition (32.21): paid at the roll by each firer, as
            // the plot was checked to be payable when it was sealed.
            let plots = self.combat.plots.get(&side).cloned().unwrap_or_default();
            let firers: Vec<PieceId> = plots
                .iter()
                .map(|f| f.piece.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let plan = self.plan_ammo(data, &firers, Role::Barrage, side == phasing);
            self.spend_ammo(plan.payments, events);
            let mut pools: BTreeMap<(PieceId, Option<HexId>), Pool> = BTreeMap::new();
            for f in plots {
                let firer = &self.pieces[&f.piece];
                let Some(hex) = firer.hex().cloned() else {
                    continue;
                };
                let pooled =
                    self.combat.forward.contains(&f.piece) && firer.nation != Some(Nation::It);
                let raw = self.raw_points(data, firer, Kind::Barrage, f.points);
                let pool = pools
                    .entry((f.target.clone(), (!pooled).then_some(hex.clone())))
                    .or_default();
                pool.firers.push(f.piece.clone());
                pool.hexes.insert(hex);
                pool.raw += raw;
            }
            let mut firing_hexes = BTreeSet::new();
            let mut target_hexes = BTreeSet::new();
            for ((target, _), pool) in pools {
                let Some(hex) = self.pieces[&target].hex().cloned() else {
                    continue;
                };
                let points = actual(pool.raw);
                let shift = terrain_shift(
                    &self.ruleset,
                    hex_shift(data, &hex, Kind::Barrage).unwrap_or(0),
                    side == phasing,
                );
                let class = self.class_of(data, &self.pieces[&target]);
                let reading = self.rng.two_dice_sequential();
                events.push(Event::DieRolled {
                    purpose: format!("barrage at {target}"),
                    value: reading,
                });
                let result = resolve(&data.barrage, class.key(), points, shift, reading);
                match result {
                    BarrageResult::Pinned if !self.is_gun_unit(data, &self.pieces[&target]) => {
                        self.combat.pinned.insert(target.clone());
                    }
                    BarrageResult::Lose(n) => losses.push((target.clone(), n)),
                    _ => {}
                }
                *self.combat.barraged.entry(hex.clone()).or_default() += points;
                self.combat
                    .barrage_involved
                    .extend(pool.firers.iter().cloned());
                self.combat.barrage_involved.extend(
                    self.pieces
                        .values()
                        .filter(|p| p.hex() == Some(&hex))
                        .map(|p| p.id.clone()),
                );
                events.push(Event::Barrage {
                    side,
                    firers: pool.firers,
                    target,
                    points,
                    shift,
                    reading,
                    result,
                });
                firing_hexes.extend(pool.hexes);
                target_hexes.insert(hex);
            }
            // CP (6.3): a firing hex pays the phasing or non-phasing
            // combat charge; a barraged hex pays 3 either way.
            let firing = if side == phasing {
                PHASING_CEILING
            } else {
                NON_PHASING_CEILING
            };
            for hex in firing_hexes {
                self.charge_combat_hex(&hex, firing, events);
            }
            for hex in target_hexes {
                self.charge_combat_hex(&hex, NON_PHASING_CEILING, events);
            }
        }
        for (id, n) in losses {
            self.take_loss(&id, n, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment::test_support::at_sollum;
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Fire, Order};
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;

    #[test]
    fn a_barrage_is_rolled_pooled_and_charged() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_sollum(&data);
        assert_eq!(state.phasing(), Some(Side::Cw));
        let rha = pid("unit:cw:4-rha");
        let target = pid("unit:it:viii-libyan-bn");
        let raw = state.raw_points(&data, &state.pieces[&rha], Kind::Barrage, 4);
        assert_eq!(raw, 32);
        for (side, forward) in [(Side::Cw, vec![rha.clone()]), (Side::Axis, vec![])] {
            state
                .apply(&data, side, Order::GunPositions { forward })
                .unwrap();
        }
        state
            .apply(
                &data,
                Side::Cw,
                Order::Barrage {
                    fires: vec![Fire {
                        piece: rha.clone(),
                        points: 4,
                        target: target.clone(),
                    }],
                },
            )
            .unwrap();
        let before = state.pieces[&target].toe;
        let events = state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        let rolled = events
            .iter()
            .find_map(|e| match e.clone() {
                Event::Barrage {
                    side,
                    firers,
                    target: t,
                    points,
                    shift,
                    reading,
                    result,
                } => Some((
                    side,
                    firers.clone(),
                    t.clone(),
                    points,
                    shift,
                    reading,
                    result,
                )),
                _ => None,
            })
            .expect("a barrage event");
        assert_eq!(rolled.0, Side::Cw);
        assert_eq!(rolled.1, vec![rha.clone()]);
        assert_eq!(rolled.2, target);
        assert_eq!(rolled.3, 3);
        // C4020 is clear: no shift.
        assert_eq!(rolled.4, 0);
        assert_eq!(rolled.6, resolve(&data.barrage, "infantry", 3, 0, rolled.5));
        match rolled.6 {
            BarrageResult::Pinned => assert!(state.combat.pinned.contains(&target)),
            BarrageResult::Lose(n) => assert_eq!(state.pieces[&target].toe, before - n),
            BarrageResult::NoEffect => assert_eq!(state.pieces[&target].toe, before),
            BarrageResult::Unresolved => panic!("unresolved reading"),
        }
        // Seed 3's first reading, pinned down so the fixture stays honest.
        assert_eq!(rolled.5, 35);
        assert_eq!(rolled.6, BarrageResult::NoEffect);
        assert_eq!(state.combat.barraged[&hex("C4020")], 3);
        // CP: the firing hex pays 5 (phasing barrage), the target hex 3.
        for id in ["unit:cw:1-krrc", "unit:cw:4-rha", "unit:cw:7-med"] {
            assert_eq!(state.pieces[&pid(id)].cp_spent, Cp::whole(5), "{id}");
        }
        for id in [
            "unit:it:viii-libyan-bn",
            "unit:it:1st-libyan-artillery-regt",
        ] {
            assert_eq!(state.pieces[&pid(id)].cp_spent, Cp::whole(3), "{id}");
            assert!(state.combat.barrage_involved.contains(&pid(id)), "{id}");
        }
        assert!(state.combat.barrage_involved.contains(&rha));
        assert!(
            !state
                .combat
                .barrage_involved
                .contains(&pid("unit:cw:1-krrc"))
        );
        assert_eq!(
            state.clock.segment,
            Some(crate::clock::Segment::RetreatBeforeAssault)
        );
    }

    #[test]
    fn losses_come_off_the_largest_entry_first_and_eliminate_at_zero() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_sollum(&data);
        let rha = pid("unit:cw:4-rha");
        let mut events = Vec::new();
        let toe = state.pieces[&rha].toe;
        let biggest = state.pieces[&rha]
            .weapons
            .iter()
            .map(|w| w.points)
            .max()
            .unwrap();
        state.take_loss(&rha, 1, &mut events);
        assert_eq!(state.pieces[&rha].toe, toe - 1);
        assert_eq!(
            state.pieces[&rha]
                .weapons
                .iter()
                .map(|w| w.points)
                .max()
                .unwrap(),
            biggest - 1
        );
        assert_eq!(
            events[0],
            Event::Losses {
                piece: rha.clone(),
                points: 1,
                toe: toe - 1
            }
        );
        state.take_loss(&rha, 99, &mut events);
        assert_eq!(state.pieces[&rha].toe, 0);
        assert!(state.pieces[&rha].weapons.iter().all(|w| w.points == 0));
        assert_eq!(
            state.pieces[&rha].location,
            crate::piece::Location::Eliminated
        );
        assert_eq!(events.last(), Some(&Event::Eliminated { piece: rha }));
    }
}
