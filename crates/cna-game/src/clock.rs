//! The game clock: where in the sequence of play (SPI 5.2, 7.1x; cna
//! `rules/20-sequence-of-play.md`) the game stands. A Game-Turn has three
//! OpStages; each OpStage runs the joint phases A–E once, then F–L for
//! Player A and again for Player B. Phase G is the movement-combat cycle,
//! split into segments. There is no phase I.

use cna_data::time::GameTime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A phase of an OpStage, named as cna's `phase` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// A.
    InitiativeDeclaration,
    /// B.
    Weather,
    /// C.
    Organisation,
    /// D.
    Arrival,
    /// E.
    CwFleet,
    /// F.
    ReserveDesignation,
    /// G.
    MovementCombat,
    /// H.
    TruckConvoy,
    /// J.
    Rail,
    /// K.
    Repair,
    /// L.
    Patrol,
}

impl Phase {
    /// The printed letter (5.2).
    pub fn letter(self) -> char {
        match self {
            Phase::InitiativeDeclaration => 'A',
            Phase::Weather => 'B',
            Phase::Organisation => 'C',
            Phase::Arrival => 'D',
            Phase::CwFleet => 'E',
            Phase::ReserveDesignation => 'F',
            Phase::MovementCombat => 'G',
            Phase::TruckConvoy => 'H',
            Phase::Rail => 'J',
            Phase::Repair => 'K',
            Phase::Patrol => 'L',
        }
    }

    /// A–E are done by both players at once; F–L by one player at a time.
    pub fn is_joint(self) -> bool {
        self < Phase::ReserveDesignation
    }

    fn next(self) -> Option<Phase> {
        Some(match self {
            Phase::InitiativeDeclaration => Phase::Weather,
            Phase::Weather => Phase::Organisation,
            Phase::Organisation => Phase::Arrival,
            Phase::Arrival => Phase::CwFleet,
            Phase::CwFleet => Phase::ReserveDesignation,
            Phase::ReserveDesignation => Phase::MovementCombat,
            Phase::MovementCombat => Phase::TruckConvoy,
            Phase::TruckConvoy => Phase::Rail,
            Phase::Rail => Phase::Repair,
            Phase::Repair => Phase::Patrol,
            Phase::Patrol => return None,
        })
    }
}

/// Which player's half of the OpStage: Player A acts first (7.1x).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Half {
    A,
    B,
}

/// A segment of phase G: movement, breakdown, the six combat steps and
/// reserve release, in the order 5.2 gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Segment {
    Movement,
    Breakdown,
    PositionDetermination,
    Barrage,
    RetreatBeforeAssault,
    ForceAssignment,
    AntiArmour,
    CloseAssault,
    ReserveRelease,
}

impl Segment {
    fn next(self) -> Option<Segment> {
        Some(match self {
            Segment::Movement => Segment::Breakdown,
            Segment::Breakdown => Segment::PositionDetermination,
            Segment::PositionDetermination => Segment::Barrage,
            Segment::Barrage => Segment::RetreatBeforeAssault,
            Segment::RetreatBeforeAssault => Segment::ForceAssignment,
            Segment::ForceAssignment => Segment::AntiArmour,
            Segment::AntiArmour => Segment::CloseAssault,
            Segment::CloseAssault => Segment::ReserveRelease,
            Segment::ReserveRelease => return None,
        })
    }
}

/// A moment in the sequence of play. `half` is set in phases F–L only and
/// `segment` in phase G only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Clock {
    pub game_turn: u8,
    pub stage: u8,
    pub phase: Phase,
    pub half: Option<Half>,
    pub segment: Option<Segment>,
}

impl Clock {
    /// Phase A of the first OpStage of a Game-Turn.
    pub fn start(game_turn: u8) -> Clock {
        Clock::opening(GameTime::new(game_turn, 1))
    }

    /// Phase A of a given OpStage: where a scenario's play opens.
    pub fn opening(at: GameTime) -> Clock {
        Clock {
            game_turn: at.game_turn,
            stage: at.opstage,
            phase: Phase::InitiativeDeclaration,
            half: None,
            segment: None,
        }
    }

    /// The next moment of the fixed sequence: segments of G in order,
    /// phases in order, Player A's half then Player B's, three OpStages,
    /// then the next Game-Turn's phase A.
    pub fn next(&self) -> Clock {
        let mut c = *self;
        if self.phase == Phase::MovementCombat
            && let Some(seg) = self.segment
            && let Some(next) = seg.next()
        {
            c.segment = Some(next);
            return c;
        }
        c.segment = None;
        match self.phase.next() {
            Some(p) => {
                c.phase = p;
                if p == Phase::ReserveDesignation {
                    c.half = Some(Half::A);
                }
                if p == Phase::MovementCombat {
                    c.segment = Some(Segment::Movement);
                }
            }
            None => match self.half {
                Some(Half::A) => {
                    c.phase = Phase::ReserveDesignation;
                    c.half = Some(Half::B);
                }
                _ => {
                    c.half = None;
                    c.phase = Phase::InitiativeDeclaration;
                    if self.stage < 3 {
                        c.stage += 1;
                    } else {
                        c.stage = 1;
                        c.game_turn += 1;
                    }
                }
            },
        }
        c
    }

    /// The first moment after this phase: for G, skipping the remaining
    /// segments.
    pub fn end_phase(&self) -> Clock {
        let mut c = *self;
        c.segment = (self.phase == Phase::MovementCombat).then_some(Segment::ReserveRelease);
        c.next()
    }
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GT{} stage {} phase {}",
            self.game_turn,
            self.stage,
            self.phase.letter()
        )?;
        if let Some(h) = self.half {
            write!(f, " player {h:?}")?;
        }
        if let Some(s) = self.segment {
            write!(f, " {s:?}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(from: Clock, n: usize) -> Vec<Clock> {
        let mut out = vec![from];
        for _ in 0..n {
            let next = out.last().unwrap().next();
            out.push(next);
        }
        out
    }

    fn phases(clocks: &[Clock]) -> Vec<(Phase, Option<Half>)> {
        let mut out: Vec<(Phase, Option<Half>)> = Vec::new();
        for c in clocks {
            if out.last() != Some(&(c.phase, c.half)) {
                out.push((c.phase, c.half));
            }
        }
        out
    }

    #[test]
    fn one_stage_visits_a_to_e_then_f_to_l_twice() {
        let clocks = walk(Clock::start(1), 60);
        let stage1: Vec<_> = clocks.iter().copied().filter(|c| c.stage == 1).collect();
        let seen = phases(&stage1);
        use Phase::*;
        let joint: Vec<_> = [
            InitiativeDeclaration,
            Weather,
            Organisation,
            Arrival,
            CwFleet,
        ]
        .into_iter()
        .map(|p| (p, None))
        .collect();
        let half = |h| {
            [
                ReserveDesignation,
                MovementCombat,
                TruckConvoy,
                Rail,
                Repair,
                Patrol,
            ]
            .into_iter()
            .map(move |p| (p, Some(h)))
        };
        let expect: Vec<_> = joint
            .into_iter()
            .chain(half(Half::A))
            .chain(half(Half::B))
            .collect();
        assert_eq!(seen, expect);
        let after = clocks.iter().find(|c| c.stage == 2).unwrap();
        assert_eq!(
            *after,
            Clock {
                game_turn: 1,
                stage: 2,
                phase: Phase::InitiativeDeclaration,
                half: None,
                segment: None
            }
        );
    }

    #[test]
    fn phase_g_walks_its_segments_in_order() {
        let g = Clock {
            game_turn: 1,
            stage: 1,
            phase: Phase::MovementCombat,
            half: Some(Half::A),
            segment: Some(Segment::Movement),
        };
        let clocks = walk(g, 9);
        let segs: Vec<_> = clocks.iter().map(|c| c.segment).collect();
        assert_eq!(
            segs,
            [
                Some(Segment::Movement),
                Some(Segment::Breakdown),
                Some(Segment::PositionDetermination),
                Some(Segment::Barrage),
                Some(Segment::RetreatBeforeAssault),
                Some(Segment::ForceAssignment),
                Some(Segment::AntiArmour),
                Some(Segment::CloseAssault),
                Some(Segment::ReserveRelease),
                None,
            ]
        );
        assert_eq!(clocks[9].phase, Phase::TruckConvoy);
        assert_eq!(clocks[9].half, Some(Half::A));
        // Entering G sets the first segment.
        let f = Clock {
            phase: Phase::ReserveDesignation,
            segment: None,
            ..g
        };
        assert_eq!(f.next(), g);
    }

    #[test]
    fn stage_three_leads_to_the_next_game_turn() {
        let last = Clock {
            game_turn: 1,
            stage: 3,
            phase: Phase::Patrol,
            half: Some(Half::B),
            segment: None,
        };
        assert_eq!(last.next(), Clock::start(2));
    }

    #[test]
    fn graziani_has_36_half_stages_each_visited_once() {
        let mut halves = Vec::new();
        let mut c = Clock::start(1);
        while c.game_turn <= 6 {
            if c.phase == Phase::ReserveDesignation {
                halves.push((c.game_turn, c.stage, c.half.unwrap()));
            }
            c = c.next();
        }
        assert_eq!(halves.len(), 36);
        let mut unique = halves.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 36);
        assert_eq!(halves[0], (1, 1, Half::A));
        assert_eq!(halves[35], (6, 3, Half::B));
        assert_eq!(c, Clock::start(7));
    }

    #[test]
    fn serde_uses_cna_phase_names() {
        let c = Clock {
            game_turn: 2,
            stage: 3,
            phase: Phase::MovementCombat,
            half: Some(Half::B),
            segment: Some(Segment::CloseAssault),
        };
        let json = serde_json::to_value(c).unwrap();
        assert_eq!(json["phase"], "movement-combat");
        assert_eq!(json["half"], "B");
        assert_eq!(json["segment"], "close-assault");
        assert_eq!(serde_json::from_value::<Clock>(json).unwrap(), c);
        assert_eq!(c.to_string(), "GT2 stage 3 phase G player B CloseAssault");
    }
}
