//! Abstract supply (SPI 32.1x, 32.2x, R-119): which friendly supply
//! units a piece may draw on, traced as a medium truck would move it.

use crate::data::Data;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::movement::{Mobility, Step, step_cost};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

impl GameState {
    /// The hexes a supply trace of `side` may not enter (32.17, R-110):
    /// any holding an enemy combat unit, and any enemy-ZOC hex without a
    /// friendly combat unit in it. Every enemy hex that could exert a ZOC
    /// does: 10.16's declarations belong to a movement segment, not to a
    /// trace. Supply units and HQs are not units here (R-119): they
    /// block, deny and lift nothing.
    pub(crate) fn trace_blocked(&self, data: &Data, side: Side) -> BTreeSet<HexId> {
        let enemy = side.other();
        let mut blocked = BTreeSet::new();
        let mut friendly = BTreeSet::new();
        for p in self.pieces.values() {
            if p.kind != PieceKind::Combat {
                continue;
            }
            if let Some(h) = p.hex() {
                if p.side == enemy {
                    blocked.insert(h.clone());
                } else {
                    friendly.insert(h.clone());
                }
            }
        }
        let enemy_hexes: Vec<HexId> = blocked.iter().cloned().collect();
        for src in enemy_hexes {
            if !self.could_exert(data, &src) {
                continue;
            }
            for n in data.map.neighbours(&src) {
                if !friendly.contains(&n) && self.reaches(data, &src, &n) {
                    blocked.insert(n);
                }
            }
        }
        blocked
    }

    /// What a medium truck (or foot infantry) pays to trace from `at`
    /// into the adjacent `to`, or none where the chart prohibits the step
    /// or `to` is blocked (32.17).
    pub(crate) fn trace_step(
        &self,
        data: &Data,
        mobility: Mobility,
        blocked: &BTreeSet<HexId>,
        at: &HexId,
        to: &HexId,
    ) -> Option<Cp> {
        if blocked.contains(to) {
            return None;
        }
        let (from_hex, to_hex) = (data.map.hex(at)?, data.map.hex(to)?);
        let hexside = data.map.hexside(at, to);
        let step = Step {
            from: from_hex,
            to: to_hex,
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(at, to)),
            weather: self.weather_at(to),
        };
        step_cost(&data.terrain, mobility, &step).ok()
    }

    /// The friendly real supply units with ammunition or fuel left that a
    /// piece may draw on (32.17): within half its CPA, traced as a medium
    /// truck would move (as infantry for a foot unit). Sorted by id.
    pub fn supply_in_range(&self, data: &Data, piece: &PieceId) -> Vec<PieceId> {
        let Some(p) = self.pieces.get(piece) else {
            return Vec::new();
        };
        let (Some(start), Some(cpa)) = (p.hex(), self.cpa(p)) else {
            return Vec::new();
        };
        // Half the CPA, to the half point.
        let budget = Cp::halves(cpa);
        let mobility = self.mobility(data, p);
        let blocked = self.trace_blocked(data, p.side);
        let mut best = BTreeMap::from([(start.clone(), Cp::ZERO)]);
        let mut queue = BinaryHeap::from([Reverse((Cp::ZERO, start.clone()))]);
        while let Some(Reverse((cost, at))) = queue.pop() {
            if best.get(&at).is_some_and(|b| *b < cost) {
                continue;
            }
            for to in data.map.neighbours(&at) {
                let Some(step) = self.trace_step(data, mobility, &blocked, &at, &to) else {
                    continue;
                };
                let total = cost + step;
                if total <= budget && best.get(&to).is_none_or(|b| total < *b) {
                    best.insert(to.clone(), total);
                    queue.push(Reverse((total, to)));
                }
            }
        }
        // `pieces` is a BTreeMap: the result comes out sorted by id.
        self.pieces
            .values()
            .filter(|q| q.side == p.side && q.kind == PieceKind::SupplyUnit)
            .filter(|q| q.load.is_some_and(|l| l.ammo + l.fuel > 0))
            .filter(|q| q.hex().is_some_and(|h| best.contains_key(h)))
            .map(|q| q.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::{Load, Location};
    use crate::state::Modules;
    use crate::state::test_support::auto_place;
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};
    use std::collections::BTreeMap;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn game(data: &Data, seed: u64) -> GameState {
        let mut state = GameState::with_modules(data, seed, Modules::ALL).unwrap();
        auto_place(data, &mut state);
        state
    }

    fn pick(rng: &mut ChaCha8Rng, n: usize) -> usize {
        (rng.next_u64() % n as u64) as usize
    }

    /// The cheapest cost to every hex within `budget`, by relaxing every
    /// edge until nothing improves: no queue, so it shares nothing with
    /// the implementation but the step price (32.17).
    fn brute_force(
        state: &GameState,
        data: &Data,
        side: Side,
        mobility: Mobility,
        start: &HexId,
        budget: Cp,
    ) -> BTreeMap<HexId, Cp> {
        let blocked = state.trace_blocked(data, side);
        let mut dist = BTreeMap::from([(start.clone(), Cp::ZERO)]);
        loop {
            let mut changed = false;
            let known: Vec<(HexId, Cp)> = dist.iter().map(|(h, c)| (h.clone(), *c)).collect();
            for (h, c) in known {
                for n in data.map.neighbours(&h) {
                    let Some(step) = state.trace_step(data, mobility, &blocked, &h, &n) else {
                        continue;
                    };
                    let total = c + step;
                    if total <= budget && dist.get(&n).is_none_or(|old| total < *old) {
                        dist.insert(n, total);
                        changed = true;
                    }
                }
            }
            if !changed {
                return dist;
            }
        }
    }

    #[test]
    fn a_unit_draws_only_on_what_the_range_reaches_32_17() {
        let data = graziani();
        let mut rng = ChaCha8Rng::seed_from_u64(17);
        let mut returned = 0;
        let mut refused = 0;
        for seed in 0..4u64 {
            let mut state = game(&data, seed);
            for side in [Side::Axis, Side::Cw] {
                let combat: Vec<PieceId> = state
                    .pieces
                    .values()
                    .filter(|p| p.side == side && p.kind == PieceKind::Combat && p.hex().is_some())
                    .map(|p| p.id.clone())
                    .collect();
                let supply: Vec<PieceId> = state
                    .pieces
                    .values()
                    .filter(|p| p.side == side && p.load.is_some())
                    .map(|p| p.id.clone())
                    .collect();
                for _ in 0..6 {
                    let who = combat[pick(&mut rng, combat.len())].clone();
                    let at = state.pieces[&who].hex().unwrap().clone();
                    // Scatter the supply units near the piece, some spent.
                    for id in &supply {
                        let mut h = at.clone();
                        for _ in 0..pick(&mut rng, 14) {
                            let n = data.map.neighbours(&h);
                            h = n[pick(&mut rng, n.len())].clone();
                        }
                        let p = state.pieces.get_mut(id).unwrap();
                        p.location = Location::Hex(h);
                        let (ammo, fuel) = match pick(&mut rng, 4) {
                            0 => (0, 0),
                            1 => (0, 5),
                            2 => (7, 0),
                            _ => (40, 60),
                        };
                        p.load = Some(Load { ammo, fuel });
                    }
                    let piece = &state.pieces[&who];
                    let mobility = state.mobility(&data, piece);
                    let budget = Cp::halves(state.cpa(piece).unwrap_or(0));
                    let dist = brute_force(&state, &data, side, mobility, &at, budget);
                    let got = state.supply_in_range(&data, &who);
                    for id in &supply {
                        let p = &state.pieces[id];
                        let spent = p.load.is_some_and(|l| l.ammo + l.fuel == 0);
                        let near = p.hex().is_some_and(|h| dist.contains_key(h));
                        assert_eq!(
                            got.contains(id),
                            near && !spent,
                            "{who} -> {id} at {:?}",
                            p.hex()
                        );
                        if near && !spent {
                            returned += 1;
                        } else {
                            refused += 1;
                        }
                    }
                    assert!(got.windows(2).all(|w| w[0] < w[1]), "sorted by id");
                }
            }
        }
        assert!(returned >= 20 && refused >= 20, "{returned} / {refused}");
    }

    #[test]
    fn combat_units_block_and_open_the_trace_but_supply_units_do_not_r_119() {
        let data = graziani();
        let mut state = game(&data, 3);
        // An Axis side's blocked set always holds the hexes of enemy
        // combat units and the ZOC hexes beside strong ones.
        let blocked = state.trace_blocked(&data, Side::Axis);
        let enemy_hex = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::Combat && p.hex().is_some())
            .and_then(|p| p.hex().cloned())
            .unwrap();
        assert!(blocked.contains(&enemy_hex));
        let zoc_hex = blocked
            .iter()
            .find(|h| state.holder(h).is_none())
            .cloned()
            .expect("some enemy ZOC hex stands empty");
        // A friendly supply unit or HQ standing in it lifts nothing ...
        let supply = PieceId::supply(Side::Axis, 1);
        state.pieces.get_mut(&supply).unwrap().location = Location::Hex(zoc_hex.clone());
        assert!(state.trace_blocked(&data, Side::Axis).contains(&zoc_hex));
        // ... a friendly combat unit does.
        let unit = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex().is_some())
            .map(|p| p.id.clone())
            .unwrap();
        state.pieces.get_mut(&unit).unwrap().location = Location::Hex(zoc_hex.clone());
        assert!(!state.trace_blocked(&data, Side::Axis).contains(&zoc_hex));
    }
}
