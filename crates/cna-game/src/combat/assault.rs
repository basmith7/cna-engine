//! Steps 5 and 6 (SPI §14, §15): anti-armour fire, then the close
//! assaults in the phasing player's order; the must-attack rule (10.3x)
//! as the step ends.

use super::cp::{BAD_ATTACK_REFUND, NON_PHASING_CEILING, PHASING_CEILING};
use super::losses::Share;
use super::strength::{Class, Kind, Part};
use super::terrain::{hex_shift, hexside_shift};
use super::{AssaultRecord, RetreatReason};
use crate::clock::Segment;
use crate::data::Data;
use crate::event::{AssaultRoll, Event, Illegal};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::{GameState, Pending};
use cna_data::characteristics::Equivalent;
use cna_data::close_assault::Side as CrtSide;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::anti_armour::damage_for;
use cna_rules::close_assault::{column_for, loss_base_factor, resolve_side, shift};
use cna_rules::morale::{MoraleLookup, lookup};
use cna_rules::strength::assault_actual;
use cna_rules::supply::Role;
use std::collections::{BTreeMap, BTreeSet};

/// A unit's points committed to one attack or defence, and where from.
#[derive(Debug, Clone)]
struct Commit {
    piece: PieceId,
    hex: HexId,
    points: u32,
}

/// One anti-armour attack as rolled: whose, from where, at which hex,
/// at what strength, and the points that absorb its damage.
struct AntiArmourAttack {
    side: Side,
    from: BTreeSet<HexId>,
    target: HexId,
    points: u32,
    shift: i32,
    shares: Vec<Share>,
}

/// Surrender when assaulted at this cohesion or worse (15.88, E-009).
const SURRENDER_LEVEL: i32 = -17;
/// Losing this share of committed TOE points costs 3 DP (15.86).
const DISORGANISING_PCT: u32 = 30;
const DISORGANISING_DP: u32 = 3;

/// The parts `points` of a piece's TOE draw on, in order.
fn allocated(parts: &[Part], points: u32) -> Vec<(Part, u32)> {
    let mut left = points;
    let mut out = Vec::new();
    for p in parts {
        if left == 0 {
            break;
        }
        let n = p.points.min(left);
        if n > 0 {
            out.push((p.clone(), n));
            left -= n;
        }
    }
    out
}

impl GameState {
    /// Hexes assigned a close assault by a unit still on the map and not
    /// yet assaulted: what keeps the step open. An assault whose
    /// attackers were all eliminated in step 5 or an earlier assault is
    /// nobody's to make.
    pub(super) fn unresolved_assaults(&self, phasing: Side) -> Vec<HexId> {
        self.phasing_commits(phasing, true)
            .into_iter()
            .map(|(_, h)| h)
            .filter(|h| !self.combat.assaults.contains_key(h))
            .collect()
    }

    /// The hexes the phasing player still has to assault in the close
    /// assault step (`unresolved_assaults`, each once); empty at any
    /// other moment or while something is pending. What a client
    /// highlights.
    pub fn assaults_to_resolve(&self) -> Vec<HexId> {
        if self.clock.segment != Some(Segment::CloseAssault) || self.pending.is_some() {
            return Vec::new();
        }
        let mut hexes = self
            .phasing()
            .map(|p| self.unresolved_assaults(p))
            .unwrap_or_default();
        hexes.sort();
        hexes.dedup();
        hexes
    }

    /// Whether any assignment, by a unit still on the map or not, names
    /// `target` for close assault.
    fn assault_assigned(&self, phasing: Side, target: &HexId) -> bool {
        self.combat
            .assignments
            .get(&phasing)
            .into_iter()
            .flatten()
            .flat_map(|a| a.close_assault.iter())
            .any(|al| al.target.as_ref() == Some(target))
    }

    /// The phasing player's assignments, flattened to commits of `kind`
    /// by the units still on the map.
    fn phasing_commits(&self, phasing: Side, close_assault: bool) -> Vec<(Commit, HexId)> {
        let mut out = Vec::new();
        for a in self.combat.assignments.get(&phasing).into_iter().flatten() {
            let Some(hex) = self.pieces[&a.piece].hex().cloned() else {
                continue;
            };
            let allocs = if close_assault {
                &a.close_assault
            } else {
                &a.anti_armour
            };
            for al in allocs {
                if let Some(t) = &al.target {
                    out.push((
                        Commit {
                            piece: a.piece.clone(),
                            hex: hex.clone(),
                            points: al.points,
                        },
                        t.clone(),
                    ));
                }
            }
        }
        out
    }

    /// The defender's loss base (15.83, R-011, R-122) as shares: each
    /// committed unit at its whole TOE, since the points it holds back
    /// count; pinned and retreated-in units; infantry, which may not be
    /// withheld; and in an overrun every wholly withheld unit. Also
    /// returns the held-back points, which are not committed (15.86).
    fn defender_loss_shares(
        &self,
        data: &Data,
        defenders: &[Commit],
        in_hex: &[PieceId],
        overrun: bool,
    ) -> (Vec<Share>, u32) {
        let held_back: u32 = defenders
            .iter()
            .map(|c| self.pieces[&c.piece].toe.saturating_sub(c.points))
            .sum();
        let mut shares: Vec<Share> = defenders
            .iter()
            .map(|c| Share {
                piece: c.piece.clone(),
                points: c.points.max(self.pieces[&c.piece].toe),
            })
            .collect();
        for id in in_hex {
            let p = &self.pieces[id];
            if shares.iter().any(|s| s.piece == *id) || !self.can_fight(p) {
                continue;
            }
            let counted = self.combat.pinned.contains(id)
                || self.combat.retreated.contains(id)
                || overrun
                || self.class_of(data, p) == Class::Infantry;
            if counted {
                shares.push(Share {
                    piece: id.clone(),
                    points: p.toe,
                });
            }
        }
        (shares, held_back)
    }

    /// The non-phasing player's commits of `kind` in `hex`: what it
    /// assigned, plus — for close assault — every unlisted infantry
    /// unit in full, since only armour and guns may be withheld (14.14,
    /// 15.1x; the engine's reading).
    fn defending_commits(
        &self,
        data: &Data,
        side: Side,
        hex: &HexId,
        close_assault: bool,
    ) -> Vec<Commit> {
        let mut out = Vec::new();
        let assigned: BTreeSet<&PieceId> = self
            .combat
            .assignments
            .get(&side)
            .into_iter()
            .flatten()
            .map(|a| &a.piece)
            .collect();
        for a in self.combat.assignments.get(&side).into_iter().flatten() {
            if self.pieces[&a.piece].hex() != Some(hex) {
                continue;
            }
            let allocs = if close_assault {
                &a.close_assault
            } else {
                &a.anti_armour
            };
            let points: u32 = allocs.iter().map(|al| al.points).sum();
            if points > 0 {
                out.push(Commit {
                    piece: a.piece.clone(),
                    hex: hex.clone(),
                    points,
                });
            }
        }
        if close_assault {
            for p in self.pieces.values() {
                if p.side != side
                    || p.hex() != Some(hex)
                    || assigned.contains(&p.id)
                    || !self.can_fight(p)
                    || self.combat.pinned.contains(&p.id)
                    || self.combat.retreated.contains(&p.id)
                    || self.class_of(data, p) != Class::Infantry
                {
                    continue;
                }
                out.push(Commit {
                    piece: p.id.clone(),
                    hex: hex.clone(),
                    points: p.toe,
                });
            }
        }
        out
    }

    /// A unit that may add strength: a combat unit or HQ with points and
    /// not collapsed.
    fn can_fight(&self, p: &Piece) -> bool {
        matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
            && p.toe > 0
            && !p.cohesion.collapsed(&self.ruleset)
    }

    fn raw_sum(&self, data: &Data, commits: &[Commit], kind: Kind) -> u32 {
        commits
            .iter()
            .map(|c| self.raw_points(data, &self.pieces[&c.piece], kind, c.points))
            .sum()
    }

    /// Step 5: every anti-armour attack rolled at the strength each side
    /// has as the step opens, the damage landed after the last roll
    /// (14.0: all fire is simultaneous). Phasing fire pools by target
    /// hex and is shifted by the hex and the hexside crossed; non-phasing
    /// fire from a defended hex goes at every armoured unit assigned
    /// against it in either role (14.11, R-015). Damage is absorbed by
    /// armour protection, the best-protected points first (14.4x).
    pub(crate) fn resolve_anti_armour(&mut self, data: &Data, events: &mut Vec<Event>) {
        let Some(phasing) = self.phasing() else {
            return;
        };
        let other = phasing.other();
        let mut attacks: Vec<AntiArmourAttack> = Vec::new();
        // Phasing fire, by target hex.
        let mut by_target: BTreeMap<HexId, Vec<Commit>> = BTreeMap::new();
        for (c, t) in self.phasing_commits(phasing, false) {
            by_target.entry(t).or_default().push(c);
        }
        for (target, firers) in by_target {
            let raw = self.raw_sum(data, &firers, Kind::AntiArmour);
            // A hexside 14.34 bans was refused at assignment.
            let hexside = firers
                .iter()
                .map(|c| hexside_shift(data, &c.hex, &target, Kind::AntiArmour).unwrap_or(0))
                .min()
                .unwrap_or(0);
            let shift = cna_rules::anti_armour::terrain_shift(
                &self.ruleset,
                true,
                hex_shift(data, &target, Kind::AntiArmour).unwrap_or(0),
                hexside,
            );
            // Committed defenders, and units that retreated in (14.13).
            let mut shares: Vec<Share> = self
                .defending_commits(data, other, &target, false)
                .into_iter()
                .chain(self.defending_commits(data, other, &target, true))
                .map(|c| Share {
                    piece: c.piece,
                    points: c.points,
                })
                .collect();
            for p in self.pieces.values() {
                if p.side == other
                    && p.hex() == Some(&target)
                    && self.combat.retreated.contains(&p.id)
                    && !shares.iter().any(|s| s.piece == p.id)
                {
                    shares.push(Share {
                        piece: p.id.clone(),
                        points: p.toe,
                    });
                }
            }
            attacks.push(AntiArmourAttack {
                side: phasing,
                from: firers.iter().map(|c| c.hex.clone()).collect(),
                target,
                points: cna_rules::strength::actual(raw),
                shift,
                shares,
            });
        }
        // Non-phasing fire from each defended hex at the armour assigned
        // against it, whichever role.
        let defended: BTreeSet<HexId> = self
            .combat
            .assignments
            .get(&other)
            .into_iter()
            .flatten()
            .filter(|a| !a.anti_armour.is_empty())
            .filter_map(|a| self.pieces[&a.piece].hex().cloned())
            .collect();
        let against: Vec<(Commit, HexId)> = self
            .phasing_commits(phasing, false)
            .into_iter()
            .chain(self.phasing_commits(phasing, true))
            .collect();
        for hex in defended {
            let firers = self.defending_commits(data, other, &hex, false);
            let raw = self.raw_sum(data, &firers, Kind::AntiArmour);
            // Armour assaulting across a hexside 14.34 bans is out of
            // reach of return fire too.
            let armour: Vec<&Commit> = against
                .iter()
                .filter(|(c, t)| {
                    *t == hex
                        && self
                            .parts(data, &self.pieces[&c.piece])
                            .iter()
                            .any(Part::armoured)
                        && hexside_shift(data, &c.hex, &hex, Kind::AntiArmour).is_some()
                })
                .map(|(c, _)| c)
                .collect();
            if armour.is_empty() {
                continue;
            }
            let hex_of_armour = armour
                .iter()
                .map(|c| hex_shift(data, &c.hex, Kind::AntiArmour).unwrap_or(0))
                .min()
                .unwrap_or(0);
            let hexside = armour
                .iter()
                .map(|c| hexside_shift(data, &c.hex, &hex, Kind::AntiArmour).unwrap_or(0))
                .min()
                .unwrap_or(0);
            let shift =
                cna_rules::anti_armour::terrain_shift(&self.ruleset, false, hex_of_armour, hexside);
            attacks.push(AntiArmourAttack {
                side: other,
                from: BTreeSet::from([hex.clone()]),
                target: hex,
                points: cna_rules::strength::actual(raw),
                shift,
                shares: armour
                    .iter()
                    .map(|c| Share {
                        piece: c.piece.clone(),
                        points: c.points,
                    })
                    .collect(),
            });
        }
        // Every roll, then every loss.
        let mut rolled: Vec<(AntiArmourAttack, u32)> = Vec::new();
        for a in attacks {
            let reading = self.rng.two_dice_sequential();
            let purpose = if a.side == phasing {
                format!("anti-armour fire at {}", a.target)
            } else {
                format!("anti-armour fire from {}", a.target)
            };
            events.push(Event::DieRolled {
                purpose,
                value: reading,
            });
            let damage = damage_for(
                &data.anti_armour,
                a.points,
                a.shift,
                a.side == phasing,
                reading,
            );
            events.push(Event::AntiArmour {
                side: a.side,
                target: a.target.clone(),
                points: a.points,
                shift: a.shift,
                reading,
                damage,
            });
            rolled.push((a, damage));
        }
        for (mut a, damage) in rolled {
            self.absorb(data, &mut a.shares, |p| p.protection, damage, events);
            if a.side == phasing {
                for h in &a.from {
                    self.charge_combat_hex(h, PHASING_CEILING, events);
                }
            }
            self.charge_combat_hex(&a.target, NON_PHASING_CEILING, events);
        }
    }

    /// The side's largest unit equivalent, as the size table names it.
    fn largest(&self, ids: impl Iterator<Item = PieceId>) -> Option<&'static str> {
        let mut best: Option<u8> = None;
        for id in ids {
            let rank = match self.pieces[&id].equivalent {
                Some(Equivalent::Brigade) => 3,
                Some(Equivalent::Battalion) => 2,
                Some(Equivalent::Company) => 1,
                _ => continue,
            };
            best = Some(best.map_or(rank, |b: u8| b.max(rank)));
        }
        best.map(|b| match b {
            3 => "brigade",
            2 => "battalion",
            _ => "company",
        })
    }

    /// Adjusted morale for a side (15.6x): the largest unit's basic
    /// morale plus the Morale Modifier Table roll at its cohesion.
    fn adjusted_morale(&mut self, data: &Data, ids: &[PieceId], events: &mut Vec<Event>) -> i32 {
        let Some(lead) = ids
            .iter()
            .max_by_key(|id| match self.pieces[*id].equivalent {
                Some(Equivalent::Brigade) => 3,
                Some(Equivalent::Battalion) => 2,
                Some(Equivalent::Company) => 1,
                _ => 0,
            })
            .cloned()
        else {
            return 0;
        };
        let p = &self.pieces[&lead];
        let basic = p.morale.unwrap_or(0);
        let level = p.cohesion.level;
        let reading = self.rng.two_dice_sequential();
        events.push(Event::DieRolled {
            purpose: format!("morale of {lead}"),
            value: reading,
        });
        // A `surrender` cell reads as the worst modifier: the automatic
        // surrender of 15.88 is the −17 threshold above.
        let modifier = match lookup(&data.morale, level, reading) {
            MoraleLookup::Modifier(m) => m.parse::<i32>().unwrap_or(-4),
            // The printed gap at −4, reading 56, is −2 (R-024).
            MoraleLookup::Gap if level.clamp(-17, 8) == -4 && reading == 56 => -2,
            MoraleLookup::Gap | MoraleLookup::Overlap(_) => 0,
        };
        basic + modifier
    }

    /// The 50 % rule (15.9x), applied as the assignments are revealed and
    /// before anti-armour fire: a phasing unit none of whose assaults
    /// takes half the TOE points it has now probes with all of them.
    pub(super) fn record_probes(&mut self, phasing: Side) {
        let mut probing = BTreeSet::new();
        for a in self.combat.assignments.get(&phasing).into_iter().flatten() {
            if a.close_assault.is_empty() {
                continue;
            }
            let most = a
                .close_assault
                .iter()
                .map(|al| al.points)
                .max()
                .unwrap_or(0);
            if most * 2 < self.pieces[&a.piece].toe {
                probing.insert(a.piece.clone());
            }
        }
        self.combat.probing = probing;
    }

    /// `Order::Assault`: the assault assigned against `target` (15.2x),
    /// resolved on the spot; a retreat result leaves a forced retreat
    /// pending for the defender.
    pub(crate) fn assault(
        &mut self,
        data: &Data,
        player: Side,
        target: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        let phasing = self.phasing();
        if self.clock.segment != Some(Segment::CloseAssault) || phasing != Some(player) {
            return Err(Illegal::new(
                "15.0",
                format!(
                    "{player} assaults in its close assault step, not at {}",
                    self.clock
                ),
            ));
        }
        let phasing = player;
        let other = phasing.other();
        if self.combat.assaults.contains_key(target) {
            return Err(Illegal::new(
                "15.21",
                format!("{target} has been assaulted this segment"),
            ));
        }
        let attackers: Vec<Commit> = self
            .phasing_commits(phasing, true)
            .into_iter()
            .filter(|(_, t)| t == target)
            .map(|(c, _)| c)
            .collect();
        if attackers.is_empty() {
            let reason = if self.assault_assigned(phasing, target) {
                format!("every unit assigned to assault {target} has been eliminated")
            } else {
                format!("no close assault was assigned against {target}")
            };
            return Err(Illegal::new("15.2x", reason));
        }
        let mut events = Vec::new();
        let attacker_ids: Vec<PieceId> = attackers.iter().map(|c| c.piece.clone()).collect();
        // Ammunition (32.21): each attacker pays double as phasing; an
        // assault the attacker cannot pay for is refused unchanged
        // (R-120: a unit out of ammunition cannot assault).
        let payers: Vec<PieceId> = attacker_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let plan = self.plan_ammo(data, &payers, Role::Assault, true);
        plan.refuse()?;
        self.spend_ammo(plan.payments, &mut events);
        let probe = attacker_ids
            .iter()
            .all(|id| self.combat.probing.contains(id));

        // Surrender when assaulted (15.88, E-009).
        let surrendering: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| {
                p.side == other
                    && p.hex() == Some(target)
                    && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                    && p.cohesion.level <= SURRENDER_LEVEL
            })
            .map(|p| p.id.clone())
            .collect();
        if !surrendering.is_empty() {
            for id in &surrendering {
                let toe = self.pieces[id].toe;
                self.take_loss(id, toe, &mut events);
            }
            events.push(Event::Surrendered {
                pieces: surrendering,
            });
        }

        let defenders = self.defending_commits(data, other, target, true);
        // Committed defenders pay one each if they can (32.21). One that
        // cannot still defends at full strength: R-120 says it cannot
        // defend and an all-out hex surrenders, which is not built yet
        // (NOT_SIMULATED.md).
        let payers: Vec<PieceId> = defenders.iter().map(|c| c.piece.clone()).collect();
        let plan = self.plan_ammo(data, &payers, Role::Assault, false);
        self.spend_ammo(plan.payments, &mut events);
        for (piece, _) in plan.short {
            events.push(Event::AmmunitionShort { piece });
        }
        let in_hex: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.side == other && p.hex() == Some(target))
            .map(|p| p.id.clone())
            .collect();
        let fighters: Vec<&Piece> = in_hex
            .iter()
            .map(|id| &self.pieces[id])
            .filter(|p| self.can_fight(p))
            .collect();
        if fighters.is_empty() {
            // Nothing left to fight: the hex is taken without a roll.
            self.combat.assaults.insert(
                target.clone(),
                AssaultRecord {
                    probe,
                    basic: 0,
                    column: column_for(&data.close_assault, 0).to_owned(),
                },
            );
            for id in &attacker_ids {
                self.pieces.get_mut(id).expect("listed").cohesion.victory();
            }
            self.charge_assault_cp(&attackers, target, probe, false, &mut events);
            return Ok(events);
        }
        let all_pinned =
            !fighters.is_empty() && fighters.iter().all(|p| self.combat.pinned.contains(&p.id));
        let attacker_raw = self.raw_sum(data, &attackers, Kind::Offence);
        let base_defender_raw = self.raw_sum(data, &defenders, Kind::Defence);
        let basic_column_raw = if all_pinned { 0 } else { base_defender_raw };

        // Nothing committed, nothing pinned: the hex retreats three and
        // takes 3 DP (15.29, R-012).
        if defenders.is_empty() && !all_pinned && !fighters.is_empty() {
            let raw: u32 = in_hex
                .iter()
                .map(|id| self.raw_of(data, &self.pieces[id], Kind::Defence))
                .sum();
            let basic = assault_actual(attacker_raw, 0).0 as i32;
            self.combat.assaults.insert(
                target.clone(),
                AssaultRecord {
                    probe,
                    basic,
                    column: column_for(&data.close_assault, basic).to_owned(),
                },
            );
            for id in &in_hex {
                let p = self.pieces.get_mut(id).expect("in hex");
                p.cohesion.earn_dp(DISORGANISING_DP);
                events.push(Event::Disorganised {
                    piece: id.clone(),
                    dp: DISORGANISING_DP,
                    level: p.cohesion.level,
                });
            }
            self.charge_assault_cp(&attackers, target, probe, false, &mut events);
            self.order_retreat(
                other,
                in_hex,
                3,
                attackers.iter().map(|c| c.hex.clone()).collect(),
                RetreatReason::Withheld,
                raw,
                false,
                &mut events,
            );
            return Ok(events);
        }

        // Strengths (15.2x, 15.5x) and combined arms (15.4).
        let (a_act, d_act) = assault_actual(attacker_raw, basic_column_raw);
        let a_act = a_act.saturating_sub(self.unsupported_tanks(data, &attackers));
        let d_act = d_act.saturating_sub(self.unsupported_tanks(data, &defenders));
        let basic = a_act as i32 - d_act as i32;
        let mut shifts = 0;
        // Terrain: the hex, and the hexsides crossed (15.3x).
        shifts += hex_shift(data, target, Kind::Defence).unwrap_or(0);
        let hexsides: Vec<i32> = attackers
            .iter()
            .map(|c| c.hex.clone())
            .collect::<BTreeSet<_>>()
            .iter()
            .map(|h| hexside_shift(data, h, target, Kind::Offence).unwrap_or(0))
            .collect();
        shifts += hexsides.iter().copied().min().unwrap_or(0).min(0);
        shifts += hexsides.iter().copied().max().unwrap_or(0).max(0);
        // Size (15.5x).
        let no_defence = !fighters.is_empty()
            && fighters.iter().all(|p| {
                self.parts(data, p)
                    .iter()
                    .all(|part| part.rating(Kind::Defence).is_none())
            });
        if all_pinned {
            shifts += 2;
        } else if no_defence {
            shifts += 3;
        } else if base_defender_raw > 0 && attacker_raw >= 2 * base_defender_raw {
            shifts += 2;
        } else if attacker_raw > 0 && base_defender_raw >= 2 * attacker_raw {
            shifts -= 2;
        }
        let a_size = self.largest(attacker_ids.iter().cloned());
        let d_size = self.largest(in_hex.iter().cloned());
        if let (Some(a), Some(d)) = (a_size, d_size)
            && a != d
        {
            if let Some(s) = data.size_shifts.shift(a, d) {
                shifts += s;
            } else if let Some(s) = data.size_shifts.shift(d, a) {
                shifts -= s;
            }
        }
        // Morale (15.6x).
        let a_morale = self.adjusted_morale(data, &attacker_ids, &mut events);
        let d_morale = self.adjusted_morale(data, &in_hex, &mut events);
        shifts += a_morale - d_morale;
        let column = shift(
            &data.close_assault,
            column_for(&data.close_assault, basic),
            shifts,
        );
        let adjusted = basic + shifts;
        let overrun = data
            .close_assault
            .column_index(&column)
            .is_some_and(|i| i >= data.close_assault.column_index("+11").unwrap_or(usize::MAX));

        // The rolls (15.7x).
        let a_reading = self.rng.two_dice_sequential();
        events.push(Event::DieRolled {
            purpose: format!("close assault at {target}: attacker"),
            value: a_reading,
        });
        let d_reading = self.rng.two_dice_sequential();
        events.push(Event::DieRolled {
            purpose: format!("close assault at {target}: defender"),
            value: d_reading,
        });
        let a_roll = resolve_side(&data.close_assault, CrtSide::Attacker, &column, a_reading);
        let d_roll = resolve_side(&data.close_assault, CrtSide::Defender, &column, d_reading);

        // Loss bases (15.83, R-011, R-122).
        let (mut defender_shares, held_back) =
            self.defender_loss_shares(data, &defenders, &in_hex, overrun);
        let defender_base: u32 = defender_shares
            .iter()
            .map(|s| self.raw_points(data, &self.pieces[&s.piece], Kind::Defence, s.points))
            .sum();
        let factor = |side| {
            loss_base_factor(
                &self.ruleset,
                side,
                attacker_raw as f64,
                defender_base as f64,
            )
        };
        let a_lost = if no_defence {
            0
        } else {
            (a_roll.pct as f64 * attacker_raw as f64 * factor(CrtSide::Attacker) / 100.0).ceil()
                as u32
        };
        let d_exact = d_roll.pct as f64 * defender_base as f64 * factor(CrtSide::Defender) / 100.0;
        let d_lost = if overrun {
            d_exact.ceil()
        } else {
            d_exact.floor()
        } as u32;

        let mut attacker_shares: Vec<Share> = attackers
            .iter()
            .map(|c| Share {
                piece: c.piece.clone(),
                points: c.points,
            })
            .collect();
        let a_committed: u32 = attacker_shares.iter().map(|s| s.points).sum();
        let d_committed: u32 = defender_shares.iter().map(|s| s.points).sum::<u32>() - held_back;
        let paren_a = !attackers
            .iter()
            .any(|c| self.plain_in_hex(data, &c.hex, Kind::Offence));
        let paren_d = !self.plain_in_hex(data, target, Kind::Defence);
        let (_, a_toe) = self.absorb(
            data,
            &mut attacker_shares,
            |p| p.per_point(Kind::Offence, paren_a),
            a_lost,
            &mut events,
        );
        let (_, d_toe) = self.absorb(
            data,
            &mut defender_shares,
            |p| p.per_point(Kind::Defence, paren_d),
            d_lost,
            &mut events,
        );
        // Gun losses (15.84): forward guns pay vulnerability for half the
        // raw points lost; in an overrun the defender's back guns too.
        let a_gun = self.gun_losses(data, &attacker_ids, a_lost, true, false, &mut events);
        let d_gun = self.gun_losses(data, &in_hex, d_lost, false, overrun, &mut events);
        let a_toe = a_toe + a_gun;
        let d_toe = d_toe + d_gun;

        // Prisoners (15.89).
        for (side, hit, toe) in [
            (phasing, a_roll.captured, a_toe),
            (other, d_roll.captured, d_toe),
        ] {
            if hit && toe > 0 {
                let die = self.rng.d6();
                events.push(Event::DieRolled {
                    purpose: format!("prisoners from {side}"),
                    value: die,
                });
                let pct = data.prisoners.percent(die).unwrap_or(0);
                let points = (pct * toe).div_ceil(100);
                events.push(Event::Prisoners { side, points });
            }
        }
        // Disorganisation (15.86).
        for (ids, lost, committed) in [
            (&attacker_ids, a_toe, a_committed),
            (&in_hex, d_toe, d_committed),
        ] {
            if committed > 0 && lost * 100 >= DISORGANISING_PCT * committed {
                for id in ids {
                    let p = self.pieces.get_mut(id).expect("listed");
                    p.cohesion.earn_dp(DISORGANISING_DP);
                    events.push(Event::Disorganised {
                        piece: id.clone(),
                        dp: DISORGANISING_DP,
                        level: p.cohesion.level,
                    });
                }
            }
        }
        events.push(Event::Assault {
            target: target.clone(),
            attackers: attacker_ids.clone(),
            probe,
            attacker_raw,
            defender_raw: base_defender_raw,
            basic,
            shifts,
            column: column.clone(),
            attacker: AssaultRoll {
                reading: a_reading,
                pct: a_roll.pct,
                raw_lost: a_lost,
                toe_lost: a_toe,
                engaged: a_roll.engaged,
                retreat: 0,
                captured: a_roll.captured,
            },
            defender: AssaultRoll {
                reading: d_reading,
                pct: d_roll.pct,
                raw_lost: d_lost,
                toe_lost: d_toe,
                engaged: false,
                retreat: d_roll.retreat,
                captured: d_roll.captured,
            },
        });
        self.combat.assaults.insert(
            target.clone(),
            AssaultRecord {
                probe,
                basic,
                column: column.clone(),
            },
        );
        self.charge_assault_cp(&attackers, target, probe, adjusted <= -4, &mut events);

        // What is left in the hex decides the rest.
        let survivors: Vec<PieceId> = in_hex
            .iter()
            .filter(|id| self.pieces[*id].hex() == Some(target))
            .cloned()
            .collect();
        let cleared = !survivors
            .iter()
            .any(|id| self.pieces[id].kind == PieceKind::Combat);
        if cleared {
            // A hex cleared is a victory for every attacker (6.24).
            for id in &attacker_ids {
                if let Some(p) = self.pieces.get_mut(id) {
                    p.cohesion.victory();
                }
            }
            return Ok(events);
        }
        if d_roll.retreat > 0 {
            self.order_retreat(
                other,
                survivors,
                d_roll.retreat,
                attackers.iter().map(|c| c.hex.clone()).collect(),
                RetreatReason::AssaultResult,
                defender_base,
                overrun,
                &mut events,
            );
        } else if !probe {
            let engaged = a_roll.engaged;
            let mut locked = Vec::new();
            for id in attacker_ids.iter().chain(survivors.iter()) {
                let Some(p) = self.pieces.get_mut(id) else {
                    continue;
                };
                if p.hex().is_none() {
                    continue;
                }
                if engaged {
                    p.engaged = true;
                    locked.push(id.clone());
                } else {
                    p.contact = true;
                }
            }
            if engaged {
                events.push(Event::Engaged { pieces: locked });
            }
        }
        Ok(events)
    }

    /// Actual points lost to unsupported tanks (15.4): per hex, tank
    /// points beyond the infantry points beside them, one per 1–3; at
    /// most four for the assault as a whole. The 1–3 bands are taken
    /// hex by hex, as the support must be: one unsupported point in each
    /// of two hexes costs 2, not the 1 a pooled count would give.
    fn unsupported_tanks(&self, data: &Data, commits: &[Commit]) -> u32 {
        let mut by_hex: BTreeMap<&HexId, (u32, u32)> = BTreeMap::new();
        for c in commits {
            let parts = self.parts(data, &self.pieces[&c.piece]);
            for (part, n) in allocated(&parts, c.points) {
                let e = by_hex.entry(&c.hex).or_default();
                if part.tank {
                    e.0 += n;
                } else if part.class == Class::Infantry {
                    e.1 += n;
                }
            }
        }
        by_hex
            .values()
            .map(|(tanks, infantry)| tanks.saturating_sub(*infantry).div_ceil(3))
            .sum::<u32>()
            .min(4)
    }

    /// Vulnerability losses on forward guns (15.84): enough gun points
    /// to absorb half the raw points lost, at least one when any was.
    /// An attacker's artillery is at half vulnerability, rounded up.
    fn gun_losses(
        &mut self,
        data: &Data,
        ids: &[PieceId],
        raw_lost: u32,
        attacking: bool,
        overrun: bool,
        events: &mut Vec<Event>,
    ) -> u32 {
        if raw_lost == 0 {
            return 0;
        }
        let needed = raw_lost.div_ceil(2).max(1);
        let mut shares: Vec<Share> = ids
            .iter()
            .filter(|id| {
                let p = &self.pieces[*id];
                p.hex().is_some()
                    && self.is_gun_unit(data, p)
                    && (overrun || self.combat.forward.contains(*id))
            })
            .map(|id| Share {
                piece: id.clone(),
                points: self.pieces[id].toe,
            })
            .collect();
        if shares.is_empty() {
            return 0;
        }
        let (_, removed) = self.absorb(
            data,
            &mut shares,
            |p| {
                if p.flak || !p.gun() {
                    0
                } else if attacking {
                    p.vulnerability.div_ceil(2)
                } else {
                    p.vulnerability
                }
            },
            needed,
            events,
        );
        removed
    }

    /// The CP of one assault (6.3): the attacking hexes pay 5 (2 for a
    /// probe); the defended hex 3 (2 when probed), and for a final
    /// differential of −4 or worse every defender gets 2 back of all it
    /// was charged this segment, step 5 included. Read as the CP table's
    /// "2 back", so a probed hex that took anti-armour fire keeps 1,
    /// though 15.9x's "no CP at all" could be read as 0.
    fn charge_assault_cp(
        &mut self,
        attackers: &[Commit],
        target: &HexId,
        probe: bool,
        bad: bool,
        events: &mut Vec<Event>,
    ) {
        let hexes: BTreeSet<HexId> = attackers.iter().map(|c| c.hex.clone()).collect();
        let attack = if probe { Cp::whole(2) } else { PHASING_CEILING };
        for h in hexes {
            self.charge_combat_hex(&h, attack, events);
        }
        let defend = if probe {
            Cp::whole(2)
        } else {
            NON_PHASING_CEILING
        };
        self.charge_combat_hex(target, defend, events);
        if bad {
            self.refund_combat_hex(target, BAD_ATTACK_REFUND, events);
        }
    }

    /// Leaves a forced retreat pending for `side`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn order_retreat(
        &mut self,
        side: Side,
        pieces: Vec<PieceId>,
        hexes: u32,
        from: Vec<HexId>,
        reason: RetreatReason,
        raw: u32,
        overrun: bool,
        events: &mut Vec<Event>,
    ) {
        events.push(Event::RetreatOrdered {
            pieces: pieces.clone(),
            hexes,
            reason,
        });
        self.pending = Some(Pending::ForcedRetreat {
            side,
            pieces,
            hexes,
            from,
            reason,
            raw,
            overrun,
        });
    }

    /// Whether the enemy hex `e` was attacked this segment as 10.3x
    /// requires: close-assaulted, probed at a basic differential of −4
    /// or better, or barraged with actual points at least the non-gun
    /// battalion equivalents in it.
    fn held_off(&self, data: &Data, e: &HexId) -> bool {
        if let Some(a) = self.combat.assaults.get(e)
            && (!a.probe || a.basic >= -4)
        {
            return true;
        }
        // Battalion equivalents by the stacking chart: a brigade is two,
        // a company nothing (10.33).
        let equivalents: u32 = self
            .pieces
            .values()
            .filter(|p| p.hex() == Some(e) && p.kind == PieceKind::Combat)
            .filter(|p| !self.is_gun_unit(data, p))
            .map(|p| cna_rules::stacking::points(&data.stacking, p.equivalent))
            .sum();
        self.combat.barraged.get(e).copied().unwrap_or(0) >= equivalents
    }

    /// Whether a phasing hex owes an attack (10.32): it holds a combat
    /// unit that is not a gun and not pinned.
    fn owes_attack(&self, data: &Data, h: &HexId, phasing: Side) -> bool {
        self.pieces.values().any(|p| {
            p.side == phasing
                && p.hex() == Some(h)
                && p.kind == PieceKind::Combat
                && p.toe > 0
                && !self.is_gun_unit(data, p)
                && !self.combat.pinned.contains(&p.id)
        })
    }

    /// The close assault step is over: the must-attack rule (10.3x).
    /// The first phasing stack under a declared enemy ZOC whose source
    /// was neither assaulted nor held off retreats three hexes; the
    /// retreat itself takes its CP and the 3 DP (R-118, `forced_retreat`).
    /// True when that retreat is now pending and the clock waits. Called
    /// again as each retreat resolves.
    pub(crate) fn end_close_assault(&mut self, data: &Data, events: &mut Vec<Event>) -> bool {
        let Some(phasing) = self.phasing() else {
            return false;
        };
        let hexes: BTreeSet<HexId> = self
            .pieces
            .values()
            .filter(|p| p.side == phasing)
            .filter_map(|p| p.hex().cloned())
            .collect();
        for h in hexes {
            if !self.owes_attack(data, &h, phasing) {
                continue;
            }
            let sources: Vec<HexId> = self
                .zoc_sources(data, &h, phasing)
                .into_iter()
                .filter(|e| !self.held_off(data, e))
                .collect();
            if sources.is_empty() {
                continue;
            }
            let pieces: Vec<PieceId> = self
                .pieces
                .values()
                .filter(|p| p.side == phasing && p.hex() == Some(&h))
                .map(|p| p.id.clone())
                .collect();
            self.order_retreat(
                phasing,
                pieces,
                3,
                sources,
                RetreatReason::MustAttack,
                0,
                false,
                events,
            );
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment::test_support::at_sollum;
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Allocation, Assignment, Order};
    use crate::piece::Location;
    use crate::reach::Purpose;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn alloc(target: Option<&str>, points: u32) -> Allocation {
        Allocation {
            target: target.map(hex),
            points,
        }
    }

    fn assign(piece: &str, aa: Vec<Allocation>, ca: Vec<Allocation>) -> Assignment {
        Assignment {
            piece: pid(piece),
            anti_armour: aa,
            close_assault: ca,
        }
    }

    /// Steps 1–3 passed with nothing done; the Commonwealth to assign.
    fn assigning(data: &Data) -> GameState {
        let mut state = at_sollum(data);
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (Side::Axis, Order::GunPositions { forward: vec![] }),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(data, side, order).unwrap();
        }
        assert_eq!(state.clock.segment, Some(Segment::ForceAssignment));
        state
    }

    /// 1 RTR (seven A9 cruisers) brought to C4021 beside 1 KRRC.
    fn with_tanks(data: &Data) -> GameState {
        let mut state = at_sollum(data);
        state
            .pieces
            .get_mut(&pid("unit:cw:1-rtr"))
            .unwrap()
            .location = Location::Hex(hex("C4021"));
        state
    }

    #[test]
    fn return_fire_is_banned_across_the_hexside_the_assault_crosses() {
        // 14.34 bans anti-armour fire by either side across the hexside
        // the phasing points assault over. Both stacks move up to
        // C4121|C4220, a slope with no road or track, C4220 above: the
        // cruisers assault down it, so their own anti-armour fire is
        // refused and the anti-tank company at C4121 cannot fire back.
        let data = graziani();
        let mut state = with_tanks(&data);
        for p in state.pieces.values_mut() {
            match p.hex().cloned() {
                Some(h) if h == hex("C4020") => p.location = Location::Hex(hex("C4121")),
                Some(h) if h == hex("C4021") => p.location = Location::Hex(hex("C4220")),
                _ => {}
            }
        }
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (
                Side::Axis,
                Order::GunPositions {
                    forward: vec![pid("unit:it:1st-libyan-anti-tank-coy")],
                },
            ),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(&data, side, order).unwrap();
        }
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![alloc(Some("C4121"), 7)],
                        vec![],
                    )],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "14.34");
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![],
                        vec![alloc(Some("C4121"), 7)],
                    )],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 2)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        assert!(
            !events.iter().any(|e| matches!(e, Event::AntiArmour { .. })),
            "{events:?}"
        );
        assert_eq!(state.pieces[&pid("unit:cw:1-rtr")].toe, 7);
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
    }

    fn c4020_ids(state: &GameState) -> Vec<PieceId> {
        state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex("C4020")))
            .map(|p| p.id.clone())
            .collect()
    }

    // R-122: a committed defender's held-back points count in the loss
    // base; a wholly withheld gun counts only in an overrun.
    #[test]
    fn held_back_points_of_a_committed_defender_count_in_the_loss_base_r122() {
        let data = graziani();
        let state = at_sollum(&data);
        let coy = pid("unit:it:1st-libyan-anti-tank-coy");
        let toe = state.pieces[&coy].toe;
        assert!(toe > 1, "{toe}");
        let in_hex = vec![coy.clone()];
        let commit = Commit {
            piece: coy.clone(),
            hex: state.pieces[&coy].hex().unwrap().clone(),
            points: 1,
        };
        let (shares, held) =
            state.defender_loss_shares(&data, std::slice::from_ref(&commit), &in_hex, false);
        assert_eq!(shares.len(), 1);
        assert_eq!((shares[0].points, held), (toe, toe - 1));
        let (shares, _) = state.defender_loss_shares(&data, &[], &in_hex, false);
        assert!(shares.is_empty(), "withheld whole: not in the base");
        let (shares, _) = state.defender_loss_shares(&data, &[], &in_hex, true);
        assert_eq!(shares[0].points, toe, "an overrun counts it");
    }

    fn find_assault(events: &[Event]) -> Option<Event> {
        events
            .iter()
            .find(|e| matches!(e, Event::Assault { .. }))
            .cloned()
    }

    #[test]
    fn anti_armour_then_a_full_assault_on_the_crt() {
        let data = graziani();
        let mut state = with_tanks(&data);
        let c4020 = Some("C4020");
        // Steps 1–3: nothing.
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (
                Side::Axis,
                Order::GunPositions {
                    forward: vec![pid("unit:it:1st-libyan-anti-tank-coy")],
                },
            ),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(&data, side, order).unwrap();
        }
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![
                        assign("unit:cw:1-krrc", vec![], vec![alloc(c4020, 6)]),
                        assign("unit:cw:1-rtr", vec![], vec![alloc(c4020, 7)]),
                    ],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 2)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        // Step 5: the anti-tank company fires at the assaulting cruisers
        // (8 raw → 1 actual), unshifted (R-015).
        let aa = events
            .iter()
            .find(|e| matches!(e, Event::AntiArmour { .. }))
            .cloned()
            .expect("non-phasing anti-armour fire");
        let Event::AntiArmour {
            side,
            target,
            points,
            shift: aa_shift,
            reading,
            damage,
        } = aa
        else {
            unreachable!()
        };
        assert_eq!(
            (side, target, points, aa_shift),
            (Side::Axis, hex("C4020"), 1, 0)
        );
        assert_eq!(damage, damage_for(&data.anti_armour, 1, 0, false, reading));
        let rtr = pid("unit:cw:1-rtr");
        assert_eq!(state.pieces[&rtr].toe, 7 - damage.min(7));
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
        assert_eq!(state.to_act(), Some(Side::Cw));
        // Step 6: the one assault assigned.
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4121"),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "15.2x");
        let rtr_toe = state.pieces[&rtr].toe;
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let Some(Event::Assault {
            target,
            attackers,
            probe,
            attacker_raw,
            defender_raw,
            basic,
            shifts,
            column,
            attacker,
            defender,
        }) = find_assault(&events)
        else {
            panic!("no assault event in {events:?}");
        };
        assert_eq!(target, hex("C4020"));
        assert_eq!(attackers, vec![pid("unit:cw:1-krrc"), rtr.clone()]);
        assert!(!probe);
        // KRRC 6 × 1 plus the surviving cruisers × 3; every Libyan
        // battalion defends in full at 1 a point; the guns and HQs,
        // unassigned, are withheld.
        assert_eq!(attacker_raw, 6 + 3 * rtr_toe);
        assert_eq!(defender_raw, 30);
        let a_act = cna_rules::strength::actual(attacker_raw) as i32 - i32::from(rtr_toe > 6);
        assert_eq!(basic, a_act - 3);
        assert_eq!(
            column,
            shift(
                &data.close_assault,
                column_for(&data.close_assault, basic),
                shifts
            )
        );
        assert_eq!(
            attacker.pct,
            resolve_side(
                &data.close_assault,
                CrtSide::Attacker,
                &column,
                attacker.reading
            )
            .pct
        );
        assert_eq!(
            defender.pct,
            resolve_side(
                &data.close_assault,
                CrtSide::Defender,
                &column,
                defender.reading
            )
            .pct
        );
        assert_eq!(
            attacker.raw_lost,
            (attacker.pct as u32 * attacker_raw).div_ceil(100)
        );
        assert_eq!(
            state.combat.assaults[&hex("C4020")],
            AssaultRecord {
                probe: false,
                basic,
                column: column.clone(),
            }
        );
        // Pinned to seed 3 so the fixture stays honest.
        assert_eq!((attacker.reading, defender.reading), (43, 36));
        // Adjusted +1 under this seed: no refund, no retreat. CP: the
        // attacking hex 5; the defended hex 3 (the bad-attack refund has
        // its own test below).
        assert_eq!(basic + shifts, 1);
        assert_eq!(defender.retreat, 0);
        assert_eq!(state.pending, None);
        for id in ["unit:cw:1-krrc", "unit:cw:1-rtr", "unit:cw:7-med"] {
            assert_eq!(state.pieces[&pid(id)].cp_spent, Cp::whole(5), "{id}");
        }
        assert_eq!(
            state.pieces[&pid("unit:it:viii-libyan-bn")].cp_spent,
            Cp::whole(3)
        );
        // Once per segment.
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "15.21");
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
    }

    #[test]
    fn a_defender_who_commits_nothing_retreats_three_or_pays() {
        let data = graziani();
        let mut state = assigning(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-krrc",
                        vec![],
                        vec![alloc(Some("C4020"), 6)],
                    )],
                },
            )
            .unwrap();
        // The Axis withholds everything: the infantry would defend
        // regardless, so pin it all to make the hex commit nothing.
        for id in c4020_ids(&state) {
            state.combat.pinned.insert(id);
        }
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
        // Unpin the guns: the hex then has fighters who commit nothing
        // (an all-pinned hex would instead defend at zero, +2).
        let guns: Vec<PieceId> = c4020_ids(&state)
            .into_iter()
            .filter(|id| state.class_of(&data, &state.pieces[id]) == Class::Gun)
            .collect();
        for id in &guns {
            state.combat.pinned.remove(id);
        }
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        assert!(find_assault(&events).is_none());
        let all = c4020_ids(&state);
        assert_eq!(
            state.pending,
            Some(Pending::ForcedRetreat {
                side: Side::Axis,
                pieces: all.clone(),
                hexes: 3,
                from: vec![hex("C4021")],
                reason: RetreatReason::Withheld,
                raw: 40,
                overrun: false,
            })
        );
        for id in &all {
            assert_eq!(state.pieces[id].cohesion.level, -3, "{id}");
        }
        assert_eq!(state.pieces[&all[0]].cp_spent, Cp::whole(3));
        // The attacker still pays for an assault.
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(5));
        // One hex away from C4020 is two from C4021: one short, 10 % of
        // 40 raw = 4 raw of defence (four TOE points at 1 a point), the
        // R-012 buy-out.
        let away = data
            .map
            .neighbours(&hex("C4020"))
            .into_iter()
            .find(|n| {
                !data.map.adjacent(n, &hex("C4021"))
                    && !state.pieces.values().any(|p| p.hex() == Some(n))
            })
            .unwrap();
        let before: u32 = all.iter().map(|id| state.pieces[id].toe).sum();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::Retreat {
                    path: vec![away.clone()],
                },
            )
            .unwrap();
        assert!(events.contains(&Event::RetreatBoughtOut {
            pieces: all.clone(),
            short: 1,
        }));
        let after: u32 = all.iter().map(|id| state.pieces[id].toe).sum();
        assert_eq!(before - after, 4);
        assert!(
            all.iter()
                .filter(|id| state.pieces[*id].toe > 0)
                .all(|id| state.pieces[id].hex() == Some(&away))
        );
        assert_eq!(state.pending, None);
        // The step is still the Commonwealth's to end.
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
    }

    #[test]
    fn a_hex_at_minus_17_surrenders_when_assaulted() {
        let data = graziani();
        let mut state = assigning(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-krrc",
                        vec![],
                        vec![alloc(Some("C4020"), 6)],
                    )],
                },
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        for id in c4020_ids(&state) {
            state.pieces.get_mut(&id).unwrap().cohesion.level = -17;
        }
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let surrendered = events
            .iter()
            .find_map(|e| match e {
                Event::Surrendered { pieces } => Some(pieces.len()),
                _ => None,
            })
            .expect("a surrender");
        // Eight combat units and three HQs.
        assert_eq!(surrendered, 11);
        assert!(find_assault(&events).is_none());
        assert!(
            state
                .pieces
                .values()
                .filter(|p| p.kind == PieceKind::Combat)
                .all(|p| p.hex() != Some(&hex("C4020")))
        );
        // A hex cleared is a victory: 3 RP.
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cohesion.level, 3);
        assert_eq!(state.pending, None);
    }

    #[test]
    fn a_probe_is_half_or_less_and_costs_two() {
        let data = graziani();
        let mut state = assigning(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-krrc",
                        vec![],
                        vec![alloc(Some("C4020"), 2)],
                    )],
                },
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let Some(Event::Assault {
            probe,
            attacker_raw,
            basic,
            ..
        }) = find_assault(&events)
        else {
            panic!("no assault");
        };
        assert!(probe);
        assert_eq!(attacker_raw, 2);
        // 2 raw is under the floor: zero against 3 actual.
        assert_eq!(basic, -3);
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(2));
        assert!(!state.pieces[&pid("unit:cw:1-krrc")].engaged);
    }

    /// The assign step with C4020 declaring over C4021 and no other Axis
    /// hex nearby exerting; returns the state and the stack at C4021.
    fn under_a_declared_zoc(data: &Data) -> (GameState, Vec<PieceId>) {
        let mut state = assigning(data);
        // C4020 exerts over C4021 and says so; every other Axis hex
        // nearby was asked earlier and declined.
        let nearby: Vec<HexId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis)
            .filter_map(|p| p.hex().cloned())
            .filter(|h| data.map.within(&hex("C4021"), 5).contains(h))
            .collect();
        for h in nearby {
            state.zoc.insert(h, false);
        }
        state.zoc.insert(hex("C4020"), true);
        let stack: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex("C4021")))
            .map(|p| p.id.clone())
            .collect();
        (state, stack)
    }

    /// Both sides assign nothing: the step is skipped and the must-attack
    /// rule bites at once.
    fn assign_nothing(data: &Data, state: &mut GameState) -> Vec<Event> {
        state
            .apply(
                data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        state
            .apply(
                data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap()
    }

    /// A legal three-step path for `stack` from C4021 ending three hexes
    /// from C4020, over empty land: a breadth-first search.
    fn way_out(data: &Data, state: &GameState, stack: &[PieceId]) -> Vec<HexId> {
        let legal = |from: &HexId, to: &HexId| {
            data.map
                .hex(to)
                .is_some_and(|h| h.terrain != cna_data::map::Terrain::Sea)
                && !state.pieces.values().any(|p| p.hex() == Some(to))
                && state
                    .plan_with(data, stack, from, std::slice::from_ref(to), true)
                    .is_ok_and(|p| p.interrupt.is_none())
        };
        let mut frontier: Vec<Vec<HexId>> = vec![vec![hex("C4021")]];
        loop {
            let mut next = Vec::new();
            for p in &frontier {
                let at = p.last().unwrap();
                for n in data.map.neighbours(at) {
                    if p.contains(&n) || !legal(at, &n) {
                        continue;
                    }
                    let mut q = p.clone();
                    q.push(n.clone());
                    if q.len() == 4 && !data.map.within(&hex("C4020"), 2).contains(&n) {
                        return q[1..].to_vec();
                    }
                    next.push(q);
                }
            }
            assert!(!next.is_empty(), "a way out");
            frontier = next;
        }
    }

    #[test]
    fn a_unit_under_a_declared_zoc_that_attacks_nothing_must_retreat() {
        let data = graziani();
        let (mut state, before) = under_a_declared_zoc(&data);
        let krrc = pid("unit:cw:1-krrc");
        let spent = state.pieces[&krrc].cp_spent;
        let level = state.pieces[&krrc].cohesion.level;
        let events = assign_nothing(&data, &mut state);
        // Three hexes, no stopping short; the CP and the 3 DP come with
        // the retreat itself (R-118), not with the order.
        assert!(events.contains(&Event::RetreatOrdered {
            pieces: before.clone(),
            hexes: 3,
            reason: RetreatReason::MustAttack,
        }));
        assert_eq!(
            state.pending,
            Some(Pending::ForcedRetreat {
                side: Side::Cw,
                pieces: before.clone(),
                hexes: 3,
                from: vec![hex("C4020")],
                reason: RetreatReason::MustAttack,
                raw: 0,
                overrun: false,
            })
        );
        assert_eq!(state.pieces[&krrc].cp_spent, spent);
        assert_eq!(state.pieces[&krrc].cohesion.level, level);
        // No stopping short.
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::Retreat {
                    path: vec![hex("C3922")],
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "10.36");
        let path = way_out(&data, &state, &before);
        let at = path.last().unwrap().clone();
        let events = state
            .apply(&data, Side::Cw, Order::Retreat { path: path.clone() })
            .unwrap();
        assert!(before.iter().all(|id| state.pieces[id].hex() == Some(&at)));
        // The retreat was paid from the CP left and cost less than that,
        // so the unit ends at its CPA with the rest forfeited: 3 DP, no
        // more (R-118).
        let cpa = Cp::whole(state.cpa(&state.pieces[&krrc]).unwrap());
        let cost: Cp = events
            .iter()
            .filter_map(|e| match e {
                Event::Moved { cost, .. } => Some(*cost),
                _ => None,
            })
            .sum();
        assert!(
            cost > Cp::ZERO && spent + cost < cpa,
            "{spent} + {cost} < {cpa}"
        );
        assert_eq!(state.pieces[&krrc].cp_spent, cpa);
        assert_eq!(state.pieces[&krrc].cohesion.level, level - 3);
        assert!(events.contains(&Event::Disorganised {
            piece: krrc.clone(),
            dp: 3,
            level: level - 3,
        }));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::PhaseStarted { clock } if clock.segment == Some(Segment::ReserveRelease)
        )));
        assert_eq!(state.pending, None);
    }

    #[test]
    fn a_must_attack_stack_with_a_way_out_may_not_pass_and_surrender() {
        // 10.36: the whole unit surrenders only when the route is forced
        // into enemy ZOC; while a full retreat exists, Pass is refused
        // and nothing changes.
        let data = graziani();
        let (mut state, before) = under_a_declared_zoc(&data);
        assign_nothing(&data, &mut state);
        assert!(
            !state
                .reachable(&data, Side::Cw, &before, Purpose::ForcedRetreat)
                .unwrap()
                .is_empty()
        );
        let saved = state.clone();
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "10.36");
        assert_eq!(state, saved);
    }

    #[test]
    fn a_must_attack_retreat_overspends_only_what_it_costs_past_the_cpa() {
        // R-118: a unit already at its CPA has nothing left, so every CP
        // of the retreat is overspend and earns a DP per whole point, on
        // top of the 3.
        let data = graziani();
        let (mut state, before) = under_a_declared_zoc(&data);
        let krrc = pid("unit:cw:1-krrc");
        let cpa = Cp::whole(state.cpa(&state.pieces[&krrc]).unwrap());
        for id in &before {
            let cpa = Cp::whole(state.cpa(&state.pieces[id]).unwrap());
            state.pieces.get_mut(id).unwrap().cp_spent = cpa;
        }
        let level = state.pieces[&krrc].cohesion.level;
        assign_nothing(&data, &mut state);
        assert_eq!(state.pieces[&krrc].cp_spent, cpa);
        let path = way_out(&data, &state, &before);
        let events = state
            .apply(&data, Side::Cw, Order::Retreat { path })
            .unwrap();
        let cost: Cp = events
            .iter()
            .filter_map(|e| match e {
                Event::Moved { cost, .. } => Some(*cost),
                _ => None,
            })
            .sum();
        assert!(cost >= Cp::whole(1));
        assert_eq!(state.pieces[&krrc].cp_spent, cpa + cost);
        let overspend = i32::try_from(cost.floor()).unwrap();
        assert_eq!(state.pieces[&krrc].cohesion.level, level - 3 - overspend);
    }

    #[test]
    fn a_probe_at_minus_four_or_better_holds_a_hex_off() {
        let data = graziani();
        let mut state = assigning(&data);
        state.zoc.insert(hex("C4020"), true);
        // Six points would be 1 actual against 3, basic −2, and a full
        // assault. Two points (under the 5-raw floor) probe at −3,
        // enough to hold the hex off.
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-krrc",
                        vec![],
                        vec![alloc(Some("C4020"), 2)],
                    )],
                },
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        assert_eq!(state.combat.assaults[&hex("C4020")].basic, -3);
        if matches!(state.pending, Some(Pending::ForcedRetreat { .. })) {
            state.apply(&data, Side::Axis, Order::Pass).unwrap();
        }
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.pending, None);
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
    }

    /// Steps 1–3 passed with the anti-tank company forward; the
    /// Commonwealth to assign.
    fn assigning_against_the_anti_tank_coy(data: &Data, state: &mut GameState) {
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (
                Side::Axis,
                Order::GunPositions {
                    forward: vec![pid("unit:it:1st-libyan-anti-tank-coy")],
                },
            ),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(data, side, order).unwrap();
        }
        assert_eq!(state.clock.segment, Some(Segment::ForceAssignment));
    }

    fn anti_armour_events(events: &[Event]) -> Vec<(Side, u32, u32)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::AntiArmour {
                    side,
                    points,
                    damage,
                    ..
                } => Some((*side, *points, *damage)),
                _ => None,
            })
            .collect()
    }

    /// Sets a one-weapon piece to `points` TOE points.
    fn set_points(state: &mut GameState, id: &str, points: u32) {
        let p = state.pieces.get_mut(&pid(id)).unwrap();
        p.weapons[0].points = points;
        p.toe = points;
    }

    #[test]
    fn return_fire_hits_armour_in_the_anti_armour_role_too() {
        // 14.11: every armoured unit committed to either role is a
        // target; 1 RTR fires anti-armour only and is still shot at.
        let data = graziani();
        let mut state = with_tanks(&data);
        assigning_against_the_anti_tank_coy(&data, &mut state);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![alloc(Some("C4020"), 7)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 2)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        let fired = anti_armour_events(&events);
        // 21 raw cruiser points are 2 actual; the two 47/32s, 8 raw, 1.
        assert_eq!(fired.len(), 2, "{fired:?}");
        assert_eq!((fired[0].0, fired[0].1), (Side::Cw, 2));
        assert_eq!((fired[1].0, fired[1].1), (Side::Axis, 1));
        let damage = fired[1].2;
        assert_eq!(state.pieces[&pid("unit:cw:1-rtr")].toe, 7 - damage.min(7));
    }

    #[test]
    fn anti_armour_fire_is_simultaneous() {
        // 14.0: both sides fire at the strength they had before any
        // damage landed. The I(M) tank battalion (seven M 11/39, AA 2,
        // protection 2) joins the anti-tank company at C4020; 1 RTR,
        // raised to 30 A9 points, fires 25 of them (75 raw, 8 actual:
        // at least 8 damage, four M 11/39 points) and assaults with 5.
        let data = graziani();
        let mut state = with_tanks(&data);
        state
            .pieces
            .get_mut(&pid("unit:it:im-tank-bn"))
            .unwrap()
            .location = Location::Hex(hex("C4020"));
        set_points(&mut state, "unit:cw:1-rtr", 30);
        assigning_against_the_anti_tank_coy(&data, &mut state);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![alloc(Some("C4020"), 25)],
                        vec![alloc(Some("C4020"), 5)],
                    )],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![
                        assign(
                            "unit:it:1st-libyan-anti-tank-coy",
                            vec![alloc(None, 2)],
                            vec![],
                        ),
                        assign("unit:it:im-tank-bn", vec![alloc(None, 7)], vec![]),
                    ],
                },
            )
            .unwrap();
        let fired = anti_armour_events(&events);
        assert_eq!(fired.len(), 2, "{fired:?}");
        assert_eq!((fired[0].0, fired[0].1), (Side::Cw, 8));
        assert!(fired[0].2 >= 8, "{fired:?}");
        assert!(state.pieces[&pid("unit:it:im-tank-bn")].toe <= 3);
        // The Axis fired 8 + 14 = 22 raw, 2 actual: its strength before
        // the cruisers' fire landed, not after.
        assert_eq!((fired[1].0, fired[1].1), (Side::Axis, 2));
    }

    #[test]
    fn an_assault_whose_attackers_were_all_killed_no_longer_blocks_the_step() {
        // 1 RTR, down to one A9 point, is the only unit assigned to
        // assault C4020; seven 47/32 points (28 raw, 3 actual: at least
        // 1 damage on any roll) kill it in step 5.
        let data = graziani();
        let mut state = with_tanks(&data);
        set_points(&mut state, "unit:cw:1-rtr", 1);
        set_points(&mut state, "unit:it:1st-libyan-anti-tank-coy", 7);
        assigning_against_the_anti_tank_coy(&data, &mut state);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![],
                        vec![alloc(Some("C4020"), 1)],
                    )],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 7)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        assert!(events.contains(&Event::Eliminated {
            piece: pid("unit:cw:1-rtr")
        }));
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
        assert_eq!(state.to_act(), Some(Side::Cw));
        // Nobody is left to make the assault, and the step can end.
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "15.2x");
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
        assert!(!state.combat.assaults.contains_key(&hex("C4020")));
    }

    /// 1 KRRC and 1 RTR assigned against C4020 with `rtr_points` cruiser
    /// points to close assault; the anti-tank company returns fire, so
    /// the defended hex has paid 3 CP before the assault is made.
    fn after_return_fire(data: &Data, rtr_points: u32) -> GameState {
        after_return_fire_with(data, rtr_points, |_| {})
    }

    /// `after_return_fire`, with `prepare` run on the state before the
    /// combat segment opens.
    fn after_return_fire_with(
        data: &Data,
        rtr_points: u32,
        prepare: impl FnOnce(&mut GameState),
    ) -> GameState {
        let mut state = with_tanks(data);
        prepare(&mut state);
        assigning_against_the_anti_tank_coy(data, &mut state);
        state
            .apply(
                data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![
                        assign("unit:cw:1-krrc", vec![], vec![alloc(Some("C4020"), 6)]),
                        assign(
                            "unit:cw:1-rtr",
                            vec![],
                            vec![alloc(Some("C4020"), rtr_points)],
                        ),
                    ],
                },
            )
            .unwrap();
        state
            .apply(
                data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 2)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        assert_eq!(state.clock.segment, Some(Segment::CloseAssault));
        for id in c4020_ids(&state) {
            assert_eq!(state.combat.charged[&id], Cp::whole(3), "{id}");
        }
        state
    }

    #[test]
    fn unsupported_tanks_cost_at_most_four_for_the_whole_assault() {
        // 15.4: 30 unsupported A9 points from C4021 (ten bands) and seven
        // M 11/39 points from C4020 (three bands) cost four, not seven.
        let data = graziani();
        let mut state = with_tanks(&data);
        set_points(&mut state, "unit:cw:1-rtr", 30);
        let commits = vec![
            Commit {
                piece: pid("unit:cw:1-rtr"),
                hex: hex("C4021"),
                points: 30,
            },
            Commit {
                piece: pid("unit:it:im-tank-bn"),
                hex: hex("C4020"),
                points: 7,
            },
        ];
        assert_eq!(state.unsupported_tanks(&data, &commits[..1]), 4);
        assert_eq!(state.unsupported_tanks(&data, &commits[1..]), 3);
        assert_eq!(state.unsupported_tanks(&data, &commits), 4);
    }

    #[test]
    fn a_bad_attack_refunds_two_of_the_cp_the_hex_already_paid() {
        // 6.3: a final differential of −4 or worse gives every defender
        // back 2 of the CP it spent, including the 3 charged for the
        // anti-armour exchange in step 5. The attackers' morale is set
        // so low that no roll can lift the column.
        let data = graziani();
        let mut state = after_return_fire(&data, 7);
        for id in ["unit:cw:1-krrc", "unit:cw:1-rtr"] {
            state.pieces.get_mut(&pid(id)).unwrap().morale = Some(-20);
        }
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let Some(Event::Assault {
            probe,
            basic,
            shifts,
            ..
        }) = find_assault(&events)
        else {
            panic!("no assault");
        };
        assert!(!probe);
        assert!(basic + shifts <= -4, "{basic} + {shifts}");
        let bn = pid("unit:it:viii-libyan-bn");
        let coy = pid("unit:it:1st-libyan-anti-tank-coy");
        assert!(events.contains(&Event::CombatCpRefunded {
            piece: bn.clone(),
            cp: BAD_ATTACK_REFUND,
            dp: 0,
        }));
        for id in [&bn, &coy] {
            assert_eq!(state.pieces[id].cp_spent, Cp::whole(1), "{id}");
            assert_eq!(state.pieces[id].cp_enemy_half, Cp::whole(1), "{id}");
            assert_eq!(state.combat.charged[id], Cp::whole(1), "{id}");
            assert_eq!(state.pieces[id].cohesion.dp_tally, 0, "{id}");
        }
    }

    #[test]
    fn a_bad_attack_on_a_defender_at_its_cpa_costs_the_dp_of_the_net_cp() {
        // 6.21 with 6.3: the Libyan battalion starts the segment at its
        // CPA. Step 5 charges it 3 (3 DP), the −4 refund gives 2 back,
        // and the 2 DP that CP earned go with it: 1 CP over, 1 DP, as if
        // it had paid the table's net 1 outright. Two more CP in the
        // stage make 3 DP, not 5.
        let data = graziani();
        let bn = pid("unit:it:viii-libyan-bn");
        let mut state = after_return_fire_with(&data, 7, |s| {
            let cpa = Cp::whole(s.cpa(&s.pieces[&bn]).unwrap());
            s.pieces.get_mut(&bn).unwrap().cp_spent = cpa;
        });
        let cpa = Cp::whole(state.cpa(&state.pieces[&bn]).unwrap());
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 3);
        for id in ["unit:cw:1-krrc", "unit:cw:1-rtr"] {
            state.pieces.get_mut(&pid(id)).unwrap().morale = Some(-20);
        }
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        assert!(events.contains(&Event::CombatCpRefunded {
            piece: bn.clone(),
            cp: BAD_ATTACK_REFUND,
            dp: 2,
        }));
        assert_eq!(state.pieces[&bn].cp_spent, cpa + Cp::whole(1));
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 1);
        assert_eq!(state.pieces[&bn].cohesion.level, -1);
        let mut later = Vec::new();
        state.charge(&bn, Cp::whole(2), true, &mut later);
        assert_eq!(state.pieces[&bn].cohesion.dp_tally, 3);
        assert_eq!(state.pieces[&bn].cohesion.level, -3);
    }

    #[test]
    fn a_probe_is_judged_on_the_toe_available_at_assignment() {
        // 15.9x: 1 RTR commits 3 of 7 cruiser points, a probe. Seven
        // 47/32 points (28 raw, 3 actual) return fire and kill at least
        // one point in step 5; the attack is still a probe, judged on
        // the 7 points it had when assigned.
        let data = graziani();
        let mut state = with_tanks(&data);
        set_points(&mut state, "unit:it:1st-libyan-anti-tank-coy", 7);
        assigning_against_the_anti_tank_coy(&data, &mut state);
        state
            .apply(
                &data,
                Side::Cw,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:cw:1-rtr",
                        vec![],
                        vec![alloc(Some("C4020"), 3)],
                    )],
                },
            )
            .unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![assign(
                        "unit:it:1st-libyan-anti-tank-coy",
                        vec![alloc(None, 7)],
                        vec![],
                    )],
                },
            )
            .unwrap();
        let rtr = pid("unit:cw:1-rtr");
        let toe = state.pieces[&rtr].toe;
        assert!((3..7).contains(&toe), "{toe}");
        assert!(state.combat.probing.contains(&rtr));
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let Some(Event::Assault { probe, .. }) = find_assault(&events) else {
            panic!("no assault");
        };
        assert!(probe);
        assert!(state.combat.assaults[&hex("C4020")].probe);
        assert_eq!(state.pieces[&rtr].cp_spent, Cp::whole(2));
        assert!(!state.pieces[&rtr].engaged && !state.pieces[&rtr].contact);
    }
}
