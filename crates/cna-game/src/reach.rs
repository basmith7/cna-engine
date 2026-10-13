//! The legal-move query (M3 spec, *Legal moves*): every hex a stack can
//! end a voluntary move in, with the cheapest path there and what it
//! costs, from the same per-step pricing `apply` uses (`step_into`), so
//! the overlay never shows a move the engine would refuse nor hides one
//! it would take. Numbers the engine charges on the way (DP) are
//! reported; nothing here changes the state.

use crate::data::Data;
use crate::event::Illegal;
use crate::movement::Start;
use crate::piece::{Location, PieceId};
use crate::state::{GameState, Pending};
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// What the move is for: the order it will be sent as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    /// `Order::Move` by the phasing player (8.1x).
    Move,
    /// `Order::React` by the non-phasing player (8.5x).
    React,
}

/// A hex the stack can end in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reach {
    pub hex: HexId,
    /// What the stack pays in all, breaking off included.
    pub cp: Cp,
    /// The hexes entered, cheapest first by CP then by length.
    pub path: Vec<HexId>,
    /// The most DP any piece in the stack earns on the way (6.21).
    pub dp: u32,
    /// The hex is in a known enemy ZOC: the move ends there (10.2x).
    pub ends_in_zoc: bool,
    /// `apply` may stop the stack short of this hex: the path passes
    /// next to enemy units that have not declared their ZOC (10.16; the
    /// cost from there on assumes they exert none) or next to units that
    /// may react (8.5x); after the answer the rest is a new order.
    pub may_be_interrupted: bool,
}

/// A settled hex of the search.
#[derive(Debug, Clone)]
struct Node {
    cost: Cp,
    len: usize,
    prev: Option<HexId>,
    flagged: bool,
    /// Arriving here offers the enemy a reaction, so anything beyond
    /// may be cut off (8.5x).
    reaction: bool,
    zoc: bool,
}

impl GameState {
    /// The hexes `pieces` (one stack of `side`'s) can end a move of
    /// `purpose` in now, each with its cheapest path; `Err` with the
    /// case `apply` would give when the stack cannot move at all.
    pub fn reachable(
        &self,
        data: &Data,
        side: Side,
        pieces: &[PieceId],
        purpose: Purpose,
    ) -> Result<Vec<Reach>, Illegal> {
        match self.to_act() {
            None => return Err(Illegal::new("60.22", "the game is over")),
            Some(s) if s != side => {
                return Err(Illegal::new(
                    "5.2",
                    format!("{side} may not act: waiting on {s} at {}", self.clock),
                ));
            }
            Some(_) => {}
        }
        let (from, shift) = match purpose {
            Purpose::Move => {
                if let Some(e) = self.pending_refusal() {
                    return Err(e);
                }
                self.stack_to_move(side, pieces)?
            }
            Purpose::React => {
                let Some(Pending::Reaction { eligible, .. }) = &self.pending else {
                    return Err(Illegal::new("8.5x", "no reaction is pending"));
                };
                for id in pieces {
                    if !eligible.contains(id) {
                        return Err(Illegal::new(
                            "8.5x",
                            format!("{id} was not offered a reaction (eligible: {eligible:?})"),
                        ));
                    }
                }
                (self.moving_stack(side, pieces, false)?, false)
            }
        };
        let mover = self.mover(data, pieces);
        let (start_in_zoc, start_flagged) = match self.start(data, &mover, &from, false)? {
            Start::In(in_zoc) => (in_zoc, false),
            Start::Undeclared(hexes) => match purpose {
                // The move is parked on the declaration (10.16); the
                // hexes are priced as if it then went on unhindered.
                Purpose::Move => (false, true),
                Purpose::React => {
                    return Err(Illegal::new(
                        "8.5x",
                        format!(
                            "{} stands next to {hexes:?}, whose ZOC is undeclared: a reacting unit never enters an enemy ZOC",
                            pieces[0]
                        ),
                    ));
                }
            },
        };
        if shift && let Some(id) = pieces.iter().find(|id| self.shifted.contains(id)) {
            return Err(Illegal::new(
                "18.2x",
                format!("{id} has made its reserve shift this segment"),
            ));
        }
        // A move pays break-off once (8.6x) and the first hex of a
        // reinforcement's move is free (20.1x); a reaction pays neither.
        let break_off = match purpose {
            Purpose::Move if mover.break_off => pieces
                .iter()
                .map(|id| Self::break_off_cost(&self.pieces[id]))
                .max()
                .unwrap_or(Cp::ZERO),
            _ => Cp::ZERO,
        };
        let first_free =
            shift || purpose == Purpose::Move && pieces.iter().all(|id| self.pieces[id].landed);
        // The most the stack may spend on the path (8.1x, 18.2x).
        let margin: Option<Cp> = match purpose {
            Purpose::Move => pieces
                .iter()
                .filter_map(|id| {
                    self.voluntary_cap(id)
                        .map(|(cap, _, _)| cap.saturating_sub(self.pieces[id].cp_own_half()))
                })
                .min(),
            Purpose::React => None,
        };
        if margin.is_some_and(|m| break_off > m) {
            return Ok(Vec::new());
        }

        let mut nodes: BTreeMap<HexId, Node> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        nodes.insert(
            from.clone(),
            Node {
                cost: Cp::ZERO,
                len: 0,
                prev: None,
                flagged: start_flagged,
                reaction: false,
                zoc: false,
            },
        );
        heap.push(Reverse((Cp::ZERO, 0usize, from.clone())));
        while let Some(Reverse((cost, len, at))) = heap.pop() {
            let node = nodes[&at].clone();
            if (node.cost, node.len) != (cost, len) || node.zoc {
                continue;
            }
            if shift && len == 1 {
                continue;
            }
            let at_start = at == from;
            for to in data.map.neighbours(&at) {
                let Ok((step, zoc)) =
                    self.step_into(data, &mover, &at, &to, at_start && start_in_zoc)
                else {
                    continue;
                };
                let mut flagged = node.flagged || node.reaction;
                let mut in_zoc = false;
                let step = match zoc {
                    Zoc::Clear => step,
                    Zoc::Exerts => {
                        if purpose == Purpose::React || shift {
                            continue;
                        }
                        in_zoc = true;
                        Cp::ZERO
                    }
                    Zoc::Undeclared(_) => {
                        if purpose == Purpose::React {
                            continue;
                        }
                        flagged = true;
                        step
                    }
                };
                let step = if at_start && first_free {
                    Cp::ZERO
                } else {
                    step
                };
                let total = cost + step;
                if margin.is_some_and(|m| total + break_off > m)
                    || self.would_collapse(pieces, total + break_off)
                {
                    continue;
                }
                let better = match nodes.get(&to) {
                    None => true,
                    Some(n) => (total, len + 1) < (n.cost, n.len),
                };
                if better {
                    nodes.insert(
                        to.clone(),
                        Node {
                            cost: total,
                            len: len + 1,
                            prev: Some(at.clone()),
                            flagged,
                            reaction: purpose == Purpose::Move
                                && mover.combat
                                && !self.reactors_next_to(data, &to, pieces, false).is_empty(),
                            zoc: in_zoc,
                        },
                    );
                    heap.push(Reverse((total, len + 1, to)));
                }
            }
        }
        let path_to = |hex: &HexId| {
            let mut path = vec![hex.clone()];
            let mut cur = hex;
            while let Some(prev) = &nodes[cur].prev {
                if prev == &from {
                    break;
                }
                path.push(prev.clone());
                cur = prev;
            }
            path.reverse();
            path
        };
        let mut reaches: Vec<Reach> = nodes
            .iter()
            .filter(|(hex, _)| **hex != from)
            .map(|(hex, n)| {
                let cp = n.cost + break_off;
                Reach {
                    hex: hex.clone(),
                    cp,
                    path: path_to(hex),
                    dp: pieces
                        .iter()
                        .map(|id| self.dp_for(id, cp))
                        .max()
                        .unwrap_or(0),
                    ends_in_zoc: n.zoc,
                    may_be_interrupted: n.flagged,
                }
            })
            .collect();
        if purpose == Purpose::React {
            // The ceiling binds where a reaction ends (9.14), never on
            // the way.
            let mut probe = self.clone();
            reaches.retain(|r| {
                for id in pieces {
                    probe.pieces.get_mut(id).expect("checked").location =
                        Location::Hex(r.hex.clone());
                }
                probe.over_ceiling_at(data, &r.hex).is_none()
            });
        }
        Ok(reaches)
    }

    /// Whether charging the stack `cost` would collapse a piece (6.26).
    /// `apply` takes such a move; the query does not offer it, which
    /// also bounds the search for a motorised stack under no cap. A
    /// custom path can still be sent.
    fn would_collapse(&self, pieces: &[PieceId], cost: Cp) -> bool {
        pieces.iter().any(|id| {
            let mut cohesion = self.pieces[id].cohesion;
            cohesion.earn_dp(self.dp_for(id, cost));
            cohesion.collapsed(&self.ruleset)
        })
    }

    /// The DP charging a piece `cost` more CP would earn it (6.21), as
    /// `charge` counts them.
    fn dp_for(&self, id: &PieceId, cost: Cp) -> u32 {
        let p = &self.pieces[id];
        let cpa = Cp::whole(self.cpa(p).unwrap_or(0));
        let before = p.cp_spent.saturating_sub(cpa).floor();
        (p.cp_spent + cost).saturating_sub(cpa).floor() - before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Phase, Segment};
    use crate::event::Event;
    use crate::movement::test_support::{hex, moving, mv, path, pid};
    use crate::order::Order;
    use crate::piece::{Location, Reserve};
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn ids(v: &[&str]) -> Vec<PieceId> {
        v.iter().map(|s| pid(s)).collect()
    }

    fn by_hex(reaches: &[Reach]) -> BTreeMap<String, Reach> {
        reaches
            .iter()
            .map(|r| (r.hex.as_str().to_owned(), r.clone()))
            .collect()
    }

    /// What `apply` charges the stack for a path: the step costs plus
    /// any break-off, and whether it was stopped for a declaration.
    fn charged(events: &[Event]) -> (Cp, bool) {
        let mut cp = Cp::ZERO;
        let mut interrupted = false;
        for e in events {
            match e {
                Event::Moved { cost, .. } | Event::ContactBroken { cost, .. } => cp += *cost,
                Event::MoveInterrupted { .. } => interrupted = true,
                _ => {}
            }
        }
        (cp, interrupted)
    }

    #[test]
    fn the_query_is_refused_as_the_order_would_be() {
        let data = graziani();
        let mut state = crate::state::test_support::set_up(&data, 3);
        // Phase A: no movement segment.
        let err = state
            .reachable(&data, Side::Axis, &ids(&["unit:cw:1-krrc"]), Purpose::Move)
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        // The wrong player.
        let err = state
            .reachable(&data, Side::Cw, &ids(&["unit:cw:1-krrc"]), Purpose::Move)
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        // A supply unit, the enemy's piece, no reaction pending.
        let err = state
            .reachable(
                &data,
                Side::Cw,
                &[PieceId::supply(Side::Cw, 1)],
                Purpose::Move,
            )
            .unwrap_err();
        assert_eq!(err.case, "32.x");
        let err = state
            .reachable(
                &data,
                Side::Cw,
                &ids(&["unit:it:maletti-div-hq"]),
                Purpose::Move,
            )
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        let err = state
            .reachable(&data, Side::Cw, &ids(&["unit:cw:1-krrc"]), Purpose::React)
            .unwrap_err();
        assert_eq!(err.case, "8.5x");
    }

    #[test]
    fn a_foot_battalion_reaches_up_to_150_percent_of_its_cpa_and_dp_is_counted() {
        let data = graziani();
        let state = moving(&data, 3, Side::Cw);
        // 1 KRRC (CPA 10, foot) at C3922, on the Halfaya escarpment.
        let krrc = ids(&["unit:cw:1-krrc"]);
        let reaches = state
            .reachable(&data, Side::Cw, &krrc, Purpose::Move)
            .unwrap();
        let r = by_hex(&reaches);
        assert!(
            reaches.iter().all(|x| x.cp <= Cp::whole(15)),
            "the 150 % cap (8.1x)"
        );
        assert!(reaches.iter().any(|x| x.cp == Cp::whole(15)));
        assert!(
            reaches.iter().all(|x| x.path.len() <= 15),
            "15 track hexes at 1 CP at most"
        );
        // Along row 39: 2 CP a clear hex, 1 on the unfinished road
        // (a track), as the move test pins; the DP is the CP past 10.
        assert_eq!(r["C3923"].cp, Cp::whole(2));
        assert_eq!(r["C3923"].path, path(&["C3923"]));
        assert_eq!(r["C3923"].dp, 0);
        // Straight along the row the move test pays 13; the cheapest
        // way (by the road) is 10, within the CPA, so no DP.
        let c3929 = &r["C3929"];
        assert_eq!(c3929.cp, Cp::whole(10));
        assert_eq!(c3929.dp, 0);
        let mut probe = state.clone();
        let events = probe
            .apply(
                &data,
                Side::Cw,
                Order::Move {
                    pieces: krrc.clone(),
                    path: c3929.path.clone(),
                    assault: false,
                },
            )
            .unwrap();
        assert_eq!(charged(&events), (Cp::whole(10), false));
        // C3722: not straight up the escarpment (2 + 6) but round by
        // the slope track through C3821 for 3.
        assert_eq!(r["C3722"].cp, Cp::whole(3));
        assert_eq!(r["C3722"].path, path(&["C3821", "C3722"]));
        // The sea is not reachable; nor is where the stack stands.
        assert!(!r.contains_key("C4022"));
        assert!(!r.contains_key("C3922"));
    }

    #[test]
    fn a_mixed_stack_pays_the_dearer_price_and_goes_where_both_may() {
        let data = graziani();
        let state = moving(&data, 3, Side::Cw);
        // 1 KRRC (foot) with 7 Med (artillery, a vehicle) at C3922: by
        // the track to C3822 both pay 1; the escarpment to C3722 is
        // closed to the vehicle, so to the stack (8.4x, R-004).
        let stack = ids(&["unit:cw:1-krrc", "unit:cw:7-med"]);
        let r = by_hex(
            &state
                .reachable(&data, Side::Cw, &stack, Purpose::Move)
                .unwrap(),
        );
        assert_eq!(r["C3822"].cp, Cp::whole(1));
        assert_eq!(
            r["C3722"].path,
            path(&["C3821", "C3722"]),
            "round by the slope track"
        );
        assert_eq!(r["C3722"].cp, Cp::whole(4));
        // Halfaya (C3922) is reached from below only by the track: 7 Med
        // put at C3823 on the plain gets up by C3822, never straight.
        let mut state = state;
        state
            .pieces
            .get_mut(&pid("unit:cw:7-med"))
            .unwrap()
            .location = Location::Hex(hex("C3823"));
        state
            .pieces
            .get_mut(&pid("unit:cw:1-krrc"))
            .unwrap()
            .location = Location::Hex(hex("C3823"));
        let r = by_hex(
            &state
                .reachable(&data, Side::Cw, &ids(&["unit:cw:7-med"]), Purpose::Move)
                .unwrap(),
        );
        assert_eq!(r["C3922"].path, path(&["C3822", "C3922"]));
        assert_eq!(r["C3922"].cp, Cp::whole(2));
    }

    #[test]
    fn the_coast_road_from_sidi_barrani_is_cheap_for_vehicles() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 7 Med (artillery, CPA 20, a vehicle) put at Sidi Barrani C4131.
        let med = pid("unit:cw:7-med");
        state.pieces.get_mut(&med).unwrap().location = Location::Hex(hex("C4131"));
        let r = by_hex(
            &state
                .reachable(&data, Side::Cw, &[med], Purpose::Move)
                .unwrap(),
        );
        // West along the coast road at ½ CP a hex, then off it.
        assert_eq!(r["C4130"].cp, Cp::HALF);
        assert_eq!(r["C4029"].cp, Cp::halves(3));
        assert_eq!(r["C4029"].path, path(&["C4130", "C4029"]));
        // A motorised unit has no cap (8.1x): it may go into DP, up to
        // where it would collapse (6.26), which the query does not offer.
        assert!(r.values().any(|x| x.dp > 0));
        assert!(r.values().all(|x| x.dp == 0 || x.cp > Cp::whole(20)));
        assert!(
            r.values().all(|x| x.dp < 26),
            "{:?}",
            r.values().map(|x| x.dp).max()
        );
        assert!(r.values().any(|x| x.dp == 25));
        assert!(r.len() > 200 && r.len() < 2000, "{}", r.len());
    }

    #[test]
    fn a_vehicle_spending_past_its_cpa_is_told_the_dp() {
        let data = graziani();
        let state = moving(&data, 3, Side::Axis);
        // LXII(L) tanks (CPA 30) at C3918: the move test pins 4½ CP to
        // C4120; the DP starts past 30 CP.
        let r = by_hex(
            &state
                .reachable(
                    &data,
                    Side::Axis,
                    &ids(&["unit:it:lxiil-tank-bn"]),
                    Purpose::Move,
                )
                .unwrap(),
        );
        assert_eq!(r["C4120"].cp, Cp::halves(9));
        assert_eq!(r["C4120"].dp, 0);
        let far = r
            .values()
            .find(|x| x.cp > Cp::whole(31))
            .expect("a far hex");
        assert_eq!(far.dp, far.cp.saturating_sub(Cp::whole(30)).floor());
    }

    #[test]
    fn the_commonwealth_stops_at_marble_arch() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let hus = pid("unit:cw:11-hus");
        state.pieces.get_mut(&hus).unwrap().location = Location::Hex(hex("A2110"));
        let r = by_hex(
            &state
                .reachable(&data, Side::Cw, &[hus], Purpose::Move)
                .unwrap(),
        );
        assert!(r.contains_key("A2109"));
        assert!(
            r.keys().all(|h| !h.starts_with('A') || hex(h).col() >= 9),
            "{:?}",
            r.keys()
        );
    }

    #[test]
    fn a_known_enemy_zoc_is_a_stop_and_an_undeclared_one_a_flag() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        // The Axis phasing: the Commonwealth's hexes have not declared.
        // LXII(L) tanks put at C3924, two hexes from the four
        // Commonwealth units at C3922 (as the reaction tests do).
        let tanks = pid("unit:it:lxiil-tank-bn");
        state.pieces.get_mut(&tanks).unwrap().location = Location::Hex(hex("C3924"));
        let r = by_hex(
            &state
                .reachable(
                    &data,
                    Side::Axis,
                    std::slice::from_ref(&tanks),
                    Purpose::Move,
                )
                .unwrap(),
        );
        let next_to_halfaya: Vec<&String> = r
            .keys()
            .filter(|h| data.map.adjacent(&hex(h), &hex("C3922")))
            .collect();
        assert!(!next_to_halfaya.is_empty());
        let flagged: Vec<&HexId> = r
            .values()
            .filter(|x| x.may_be_interrupted)
            .map(|x| &x.hex)
            .collect();
        assert!(!flagged.is_empty());
        assert!(r.values().all(|x| !x.ends_in_zoc));
        // Once C3922 has declared it exerts, those hexes are stops: free
        // to enter, the move ends there, nothing beyond them through them.
        state.zoc.insert(hex("C3922"), true);
        let r = by_hex(
            &state
                .reachable(&data, Side::Axis, &[tanks], Purpose::Move)
                .unwrap(),
        );
        // Halfaya's neighbours its ZOC reaches are stops now, no longer
        // flagged (other Commonwealth hexes further off still are).
        let stops: Vec<&Reach> = r
            .values()
            .filter(|x| !state.zoc_sources(&data, &x.hex, Side::Axis).is_empty())
            .collect();
        assert!(!stops.is_empty());
        for s in stops {
            assert!(s.ends_in_zoc, "{s:?}");
            let before = s.path.iter().rev().nth(1).cloned().unwrap_or(hex("C3924"));
            assert_eq!(
                s.cp,
                r.get(before.as_str()).map_or(Cp::ZERO, |b| b.cp),
                "{s:?}"
            );
            assert!(
                r.values()
                    .all(|x| !x.path[..x.path.len() - 1].contains(&s.hex)),
                "{s:?}"
            );
        }
    }

    #[test]
    fn golden_order_61_is_among_the_results_at_its_cost() {
        let data = graziani();
        let log: crate::log::Log = serde_json::from_str(
            &std::fs::read_to_string(cna_data::repo_root().join("tests/games/graziani-short.json"))
                .unwrap(),
        )
        .unwrap();
        let mut prefix = log.clone();
        prefix.orders.truncate(61);
        let state = crate::log::replay(&data, &prefix).unwrap();
        let Order::Move { pieces, path, .. } = &log.orders[61].order else {
            panic!("order 61 is a move");
        };
        let r = by_hex(
            &state
                .reachable(&data, Side::Axis, pieces, Purpose::Move)
                .unwrap(),
        );
        let c4021 = &r["C4021"];
        assert_eq!(&c4021.path, path);
        assert_eq!(c4021.cp, Cp::whole(1));
        assert_eq!(c4021.dp, 0);
        assert!(!c4021.ends_in_zoc && !c4021.may_be_interrupted);
        let mut probe = state.clone();
        let events = probe
            .apply(&data, Side::Axis, log.orders[61].order.clone())
            .unwrap();
        assert_eq!(charged(&events), (Cp::whole(1), false));
    }

    #[test]
    fn a_reaction_never_enters_a_zoc_and_minds_the_ceiling() {
        let data = graziani();
        let log: crate::log::Log = serde_json::from_str(
            &std::fs::read_to_string(cna_data::repo_root().join("tests/games/graziani-short.json"))
                .unwrap(),
        )
        .unwrap();
        let mut prefix = log.clone();
        prefix.orders.truncate(62);
        let state = crate::log::replay(&data, &prefix).unwrap();
        let Some(Pending::Reaction { side, eligible }) = &state.pending else {
            panic!("order 61 offers a reaction");
        };
        assert_eq!(*side, Side::Cw);
        let rha = ids(&["unit:cw:4-rha"]);
        assert!(eligible.contains(&rha[0]));
        let reaches = state
            .reachable(&data, Side::Cw, &rha, Purpose::React)
            .unwrap();
        let r = by_hex(&reaches);
        // Golden order 62 takes 4 RHA to C3923.
        assert_eq!(r["C3923"].path, path(&["C3923"]));
        assert!(
            reaches
                .iter()
                .all(|x| !x.ends_in_zoc && !x.may_be_interrupted)
        );
        // No hex next to the Libyans at C4021 whose ZOC reaches it.
        for x in &reaches {
            let mut probe = state.clone();
            let events = probe
                .apply(
                    &data,
                    Side::Cw,
                    Order::React {
                        pieces: rha.clone(),
                        path: x.path.clone(),
                    },
                )
                .unwrap_or_else(|e| panic!("{}: {e}", x.hex));
            assert_eq!(charged(&events), (x.cp, false), "{}", x.hex);
        }
        // A piece not offered: refused like the order.
        let err = state
            .reachable(&data, Side::Cw, &ids(&["unit:cw:11-hus"]), Purpose::React)
            .unwrap_err();
        assert_eq!(err.case, "8.5x");
    }

    #[test]
    fn a_reserve_shift_is_one_free_hex_and_only_once() {
        let data = graziani();
        let mut state = crate::state::test_support::set_up(&data, 3);
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
        let krrc = ids(&["unit:cw:1-krrc"]);
        state
            .apply(
                &data,
                Side::Cw,
                Order::DesignateReserve {
                    pieces: krrc.clone(),
                },
            )
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.pieces[&krrc[0]].reserve, Some(Reserve::I));
        let reaches = state
            .reachable(&data, Side::Cw, &krrc, Purpose::Move)
            .unwrap();
        assert!(!reaches.is_empty());
        assert!(
            reaches
                .iter()
                .all(|x| x.path.len() == 1 && x.cp == Cp::ZERO && x.dp == 0)
        );
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap();
        let err = state
            .reachable(&data, Side::Cw, &krrc, Purpose::Move)
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
    }

    /// Random stacks of the phasing player, both purposes: every path
    /// the query returns is accepted by `apply` on a clone at the cost
    /// it names (unless it is flagged and the engine does stop it), and
    /// every random walk `apply` accepts ends in a hex the query named
    /// at no lower cost.
    #[test]
    fn every_returned_path_is_accepted_and_every_accepted_walk_is_returned() {
        let data = graziani();
        let mut rng = ChaCha8Rng::seed_from_u64(61);
        let mut checked = 0;
        for seed in 0..6u64 {
            let first = if seed % 2 == 0 { Side::Axis } else { Side::Cw };
            let state = moving(&data, seed, first);
            let on_map: Vec<PieceId> = state
                .pieces
                .values()
                .filter(|p| p.side == first && p.hex().is_some())
                .map(|p| p.id.clone())
                .collect();
            for _ in 0..4 {
                let anchor = &on_map[(rng.next_u64() % on_map.len() as u64) as usize];
                let hex = state.pieces[anchor].hex().unwrap().clone();
                let mut stack: Vec<PieceId> = state
                    .pieces
                    .values()
                    .filter(|p| p.hex() == Some(&hex) && p.side == first)
                    .map(|p| p.id.clone())
                    .collect();
                // Sometimes only part of the stack.
                stack.retain(|_| rng.next_u64() % 3 != 0);
                if stack.is_empty() {
                    stack.push(anchor.clone());
                }
                let reaches = match state.reachable(&data, first, &stack, Purpose::Move) {
                    Ok(r) => r,
                    Err(e) => {
                        // Then any move at all is refused with that case.
                        let mut probe = state.clone();
                        let n = data.map.neighbours(&hex);
                        let err = probe
                            .apply(
                                &data,
                                first,
                                Order::Move {
                                    pieces: stack.clone(),
                                    path: vec![n[0].clone()],
                                    assault: false,
                                },
                            )
                            .unwrap_err();
                        assert_eq!(err.case, e.case, "{stack:?}: {} / {}", err.reason, e.reason);
                        continue;
                    }
                };
                check_both_ways(&data, &state, first, &stack, &reaches, &mut rng);
                checked += 1;
            }
        }
        assert!(checked >= 12, "{checked}");
    }

    fn check_both_ways(
        data: &Data,
        state: &GameState,
        side: Side,
        stack: &[PieceId],
        reaches: &[Reach],
        rng: &mut ChaCha8Rng,
    ) {
        let r = by_hex(reaches);
        for x in reaches {
            let mut probe = state.clone();
            let events = probe
                .apply(
                    data,
                    side,
                    Order::Move {
                        pieces: stack.to_vec(),
                        path: x.path.clone(),
                        assault: false,
                    },
                )
                .unwrap_or_else(|e| panic!("{}: {e}", x.hex));
            let (cp, _) = charged(&events);
            // One `Disorganised` per charge: the stack's DP is the most
            // any one piece took in all.
            let mut per_piece: BTreeMap<&PieceId, u32> = BTreeMap::new();
            for e in &events {
                if let Event::Disorganised { piece, dp, .. } = e {
                    *per_piece.entry(piece).or_default() += dp;
                }
            }
            let dp = per_piece.values().copied().max().unwrap_or(0);
            if probe.pieces[&stack[0]].hex() == Some(&x.hex) {
                assert_eq!((cp, dp), (x.cp, x.dp), "{x:?}");
            } else {
                // Stopped short, for a declaration or a reaction.
                assert!(
                    x.may_be_interrupted,
                    "{x:?}: stopped at {:?}",
                    probe.pieces[&stack[0]].hex()
                );
            }
        }
        let from = state.pieces[&stack[0]].hex().unwrap().clone();
        for _ in 0..40 {
            let mut path = Vec::new();
            let mut at = from.clone();
            let len = 1 + rng.next_u64() % 6;
            for _ in 0..len {
                let n = data.map.neighbours(&at);
                let next = n[(rng.next_u64() % n.len() as u64) as usize].clone();
                path.push(next.clone());
                at = next;
            }
            let mut probe = state.clone();
            let Ok(events) = probe.apply(
                data,
                side,
                Order::Move {
                    pieces: stack.to_vec(),
                    path: path.clone(),
                    assault: false,
                },
            ) else {
                continue;
            };
            let (cp, _) = charged(&events);
            let end = probe.pieces[&stack[0]].hex().unwrap();
            if end == &from || end != path.last().unwrap() {
                // Stopped short: the walk did not end where it meant to.
                continue;
            }
            let named = r
                .get(end.as_str())
                .unwrap_or_else(|| panic!("{path:?} accepted to {end} for {cp}, not returned"));
            assert!(named.cp <= cp, "{path:?}: {cp} < {named:?}");
        }
    }
}
