//! Prisoners (§28): the infantry a surrender or a close assault's
//! capture roll takes become the captor's prisoner points, standing with
//! the captor; the captor marches them in its truck convoy phase to a
//! departure hex, where they leave play. Guards and escape (28.2x) are
//! not simulated yet (`NOT_SIMULATED.md`).

use crate::clock::Phase;
use crate::combat::strength::Class;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::movement::Mobility;

/// What prisoner points walk per phase H (28.13).
pub const PRISONER_CPA: u32 = 8;
/// The most prisoner points in one hex (28.14).
pub const MAX_PER_HEX: u32 = 40;

/// Where `captor`'s prisoners leave play (28.15): Alexandria and Cairo
/// for the Commonwealth; for the Axis, A2802, the hex units leave by for
/// the Tripolitania box (8.8x), until cna places 28.15's Sirte box.
pub fn departure_hexes(data: &Data, captor: Side) -> Vec<HexId> {
    match captor {
        Side::Cw => ["cairo", "alexandria-east", "alexandria-west"]
            .iter()
            .flat_map(|id| {
                data.map
                    .place(id)
                    .expect("places.json has Cairo and Alexandria")
                    .hexes
                    .clone()
            })
            .collect(),
        Side::Axis => vec![HexId::parse("A2802").expect("a hex id")],
    }
}

impl GameState {
    /// The prisoner points `ids` yield on surrendering (28.11): their
    /// infantry TOE points; an HQ with none yields one (3.36). Read
    /// before their losses are taken.
    pub(crate) fn prisoner_points(&self, data: &Data, ids: &[PieceId]) -> u32 {
        ids.iter()
            .map(|id| {
                let p = &self.pieces[id];
                let infantry: u32 = self
                    .parts(data, p)
                    .iter()
                    .filter(|part| part.class == Class::Infantry)
                    .map(|part| part.points)
                    .sum();
                if infantry == 0 && p.kind == PieceKind::Hq {
                    1
                } else {
                    infantry
                }
            })
            .sum()
    }

    /// `captor` takes `points` prisoners in `hex` (28.11); in a
    /// departure hex they leave play at once (28.15).
    pub(crate) fn capture_prisoners(
        &mut self,
        data: &Data,
        captor: Side,
        hex: &HexId,
        points: u32,
        events: &mut Vec<Event>,
    ) {
        if points == 0 {
            return;
        }
        events.push(Event::PrisonersTaken {
            side: captor,
            hex: hex.clone(),
            points,
        });
        self.add_prisoners(data, captor, hex, points, events);
    }

    fn add_prisoners(
        &mut self,
        data: &Data,
        captor: Side,
        hex: &HexId,
        points: u32,
        events: &mut Vec<Event>,
    ) {
        if departure_hexes(data, captor).contains(hex) {
            *self.prisoners_out.entry(captor).or_default() += points;
            events.push(Event::PrisonersGone {
                side: captor,
                hex: hex.clone(),
                points,
            });
        } else {
            *self
                .prisoners
                .entry(captor)
                .or_default()
                .entry(hex.clone())
                .or_default() += points;
        }
    }

    /// Whether the phasing side holds prisoners that may march (28.13).
    pub(crate) fn prisoners_to_move(&self) -> bool {
        self.phasing().is_some_and(|side| {
            self.prisoners
                .get(&side)
                .is_some_and(|m| m.keys().any(|h| !self.marched.contains(h)))
        })
    }

    /// `Order::MarchPrisoners` (28.13–28.15): in the captor's truck
    /// convoy phase, `points` of its prisoners in `from` walk `path` for
    /// at most 8 CP as foot infantry, never into an enemy hex nor one an
    /// enemy ZOC reaches without a friendly combat unit, at most 40 to a
    /// hex; at a departure hex they leave play.
    pub(crate) fn march_prisoners(
        &mut self,
        data: &Data,
        player: Side,
        from: &HexId,
        path: &[HexId],
        points: u32,
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::TruckConvoy || self.phasing() != Some(player) {
            return Err(Illegal::new(
                "28.13",
                format!(
                    "prisoners march in their captor's truck convoy phase, not at {}",
                    self.clock
                ),
            ));
        }
        let held = self
            .prisoners
            .get(&player)
            .and_then(|m| m.get(from))
            .copied()
            .unwrap_or(0);
        if points == 0 || points > held {
            return Err(Illegal::new(
                "28.13",
                format!("{player} holds {held} prisoner points in {from}, not {points}"),
            ));
        }
        if self.marched.contains(from) {
            return Err(Illegal::new(
                "28.13",
                format!("the prisoners in {from} marched there this phase: no further"),
            ));
        }
        let Some(to) = path.last() else {
            return Err(Illegal::new("28.13", "a march needs a path"));
        };
        let departures = departure_hexes(data, player);
        let none = Default::default();
        let mut cp = Cp::ZERO;
        let mut at = from;
        for (i, step) in path.iter().enumerate() {
            if step == from {
                return Err(Illegal::new(
                    "28.13",
                    format!("the march comes back to {from}, where it started"),
                ));
            }
            if !data.map.adjacent(at, step) {
                return Err(Illegal::new(
                    "8.1x",
                    format!("{step} is not adjacent to {at}"),
                ));
            }
            if let Some(enemy) = self
                .pieces
                .values()
                .find(|q| q.side != player && q.hex() == Some(step))
            {
                return Err(Illegal::new(
                    "28.12",
                    format!("{step} holds the enemy unit {}", enemy.id),
                ));
            }
            if !self.friendly_combat_in(step, player, &[])
                && self.zoc_at(data, step, player) != Zoc::Clear
            {
                return Err(Illegal::new(
                    "28.12",
                    format!("{step} is enemy-controlled: prisoners do not enter it"),
                ));
            }
            let Some(c) = self.trace_step(data, Mobility::Foot, &none, at, step) else {
                return Err(Illegal::new(
                    "8.37",
                    format!("prisoners may not walk from {at} into {step}"),
                ));
            };
            cp += c;
            if cp > Cp::whole(PRISONER_CPA) {
                return Err(Illegal::new(
                    "28.13",
                    format!("the march costs {cp} CP by {step}: prisoners have {PRISONER_CPA}"),
                ));
            }
            if departures.contains(step) && i + 1 < path.len() {
                return Err(Illegal::new(
                    "28.15",
                    format!("prisoners leave play on entering {step}: the path ends there"),
                ));
            }
            at = step;
        }
        let there = self
            .prisoners
            .get(&player)
            .and_then(|m| m.get(to))
            .copied()
            .unwrap_or(0);
        if !departures.contains(to) && there + points > MAX_PER_HEX {
            return Err(Illegal::new(
                "28.14",
                format!(
                    "{to} would hold {} prisoner points: at most {MAX_PER_HEX}",
                    there + points
                ),
            ));
        }
        let mine = self.prisoners.get_mut(&player).expect("checked");
        let left = mine.get_mut(from).expect("checked");
        *left -= points;
        if *left == 0 {
            mine.remove(from);
        }
        if mine.is_empty() {
            self.prisoners.remove(&player);
        }
        let mut events = vec![Event::PrisonersMarched {
            side: player,
            path: std::iter::once(from.clone())
                .chain(path.iter().cloned())
                .collect(),
            points,
            cp,
        }];
        self.add_prisoners(data, player, to, points, &mut events);
        self.marched.insert(to.clone());
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use crate::piece::Location;
    use crate::state::test_support::{set_up, through_combat};

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A game at the Commonwealth's phase H holding `points` prisoners
    /// in `at`.
    fn marching(data: &Data, at: &str, points: u32) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        state.clock.phase = Phase::TruckConvoy;
        state.clock.segment = None;
        state
            .prisoners
            .entry(Side::Cw)
            .or_default()
            .insert(hex(at), points);
        state
    }

    fn march(from: &str, path: &[&str], points: u32) -> Order {
        Order::MarchPrisoners {
            from: hex(from),
            path: path.iter().map(|h| hex(h)).collect(),
            points,
        }
    }

    /// A hex next to Alexandria's west hex that prisoners may walk from.
    fn by_alexandria(data: &Data, state: &GameState) -> HexId {
        let alex = hex("E3613");
        let departures = departure_hexes(data, Side::Cw);
        data.map
            .neighbours(&alex)
            .into_iter()
            .find(|n| {
                !departures.contains(n)
                    && state
                        .trace_step(data, Mobility::Foot, &Default::default(), n, &alex)
                        .is_some()
            })
            .unwrap()
    }

    #[test]
    fn a_collapsed_unit_surrenders_its_infantry_as_prisoners_28_11() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let near = data.map.neighbours(&hex("C3924"));
        let spot = near
            .iter()
            .find(|h| {
                !data.map.neighbours(&hex("C3922")).contains(h)
                    && **h != hex("C3922")
                    && **h != hex("C3923")
                    && state.holder(h).is_none()
            })
            .unwrap()
            .clone();
        let bn = pid("unit:it:viii-libyan-bn");
        let p = state.pieces.get_mut(&bn).unwrap();
        p.location = Location::Hex(spot);
        p.cohesion.level = -26;
        let expected = state.prisoner_points(&data, &[bn]);
        // The battalion is beside C3923 too: it gives up at the first step.
        assert!(expected > 0);
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc"], &["C3923", "C3924"]),
            )
            .unwrap();
        assert!(
            events.contains(&Event::PrisonersTaken {
                side: Side::Cw,
                hex: hex("C3923"),
                points: expected,
            }),
            "{events:?}"
        );
        assert_eq!(state.prisoners[&Side::Cw][&hex("C3923")], expected);
    }

    #[test]
    fn an_hq_with_no_combat_values_yields_one_point_3_36() {
        let data = graziani();
        let state = set_up(&data, 3);
        let hq = state
            .pieces
            .values()
            .find(|p| p.kind == PieceKind::Hq && state.parts(&data, p).is_empty())
            .expect("an HQ with no rated points")
            .id
            .clone();
        assert_eq!(state.prisoner_points(&data, &[hq]), 1);
    }

    #[test]
    fn prisoners_taken_in_a_departure_hex_leave_play_28_15() {
        let data = graziani();
        let mut state = set_up(&data, 3);
        let mut events = Vec::new();
        state.capture_prisoners(&data, Side::Cw, &hex("E1730"), 5, &mut events);
        assert!(state.prisoners.is_empty());
        assert_eq!(state.prisoners_out[&Side::Cw], 5);
        assert!(events.contains(&Event::PrisonersGone {
            side: Side::Cw,
            hex: hex("E1730"),
            points: 5,
        }));
    }

    #[test]
    fn prisoners_march_to_alexandria_and_leave_play_28_15() {
        let data = graziani();
        let start = by_alexandria(&data, &set_up(&data, 3));
        let mut state = marching(&data, start.as_str(), 12);
        state
            .apply(&data, Side::Cw, march(start.as_str(), &["E3613"], 5))
            .unwrap();
        assert_eq!(state.prisoners_out[&Side::Cw], 5);
        assert_eq!(state.prisoners[&Side::Cw][&start], 7);
    }

    #[test]
    fn prisoners_walk_eight_cp_and_no_more_28_13() {
        let data = graziani();
        let start = hex("E2020");
        let mut state = marching(&data, start.as_str(), 5);
        let departures = departure_hexes(&data, Side::Cw);
        let mut path: Vec<HexId> = Vec::new();
        let mut at = start.clone();
        let mut cp = Cp::ZERO;
        while cp <= Cp::whole(PRISONER_CPA) {
            let (next, c) = data
                .map
                .neighbours(&at)
                .into_iter()
                .filter(|n| *n != start && !path.contains(n) && !departures.contains(n))
                .find_map(|n| {
                    state
                        .trace_step(&data, Mobility::Foot, &Default::default(), &at, &n)
                        .map(|c| (n, c))
                })
                .unwrap();
            cp += c;
            path.push(next.clone());
            at = next;
        }
        let names: Vec<&str> = path.iter().map(HexId::as_str).collect();
        let err = state
            .apply(&data, Side::Cw, march(start.as_str(), &names, 5))
            .unwrap_err();
        assert_eq!(err.case, "28.13");
        let short = &names[..names.len() - 1];
        state
            .apply(&data, Side::Cw, march(start.as_str(), short, 5))
            .unwrap();
        assert_eq!(state.prisoners[&Side::Cw][&path[path.len() - 2]], 5);
    }

    #[test]
    fn prisoners_march_only_in_their_captors_phase_h_28_13() {
        let data = graziani();
        let start = by_alexandria(&data, &set_up(&data, 3));
        let mut state = moving(&data, 3, Side::Cw);
        state
            .prisoners
            .entry(Side::Cw)
            .or_default()
            .insert(start.clone(), 5);
        let err = state
            .apply(&data, Side::Cw, march(start.as_str(), &["E3613"], 5))
            .unwrap_err();
        assert_eq!(err.case, "28.13");
    }

    #[test]
    fn at_most_forty_to_a_hex_and_no_second_march_28_14() {
        let data = graziani();
        let start = by_alexandria(&data, &set_up(&data, 3));
        let next = data
            .map
            .neighbours(&start)
            .into_iter()
            .find(|n| !departure_hexes(&data, Side::Cw).contains(n))
            .unwrap();
        let mut state = marching(&data, start.as_str(), 30);
        state
            .prisoners
            .get_mut(&Side::Cw)
            .unwrap()
            .insert(next.clone(), 15);
        let err = state
            .apply(&data, Side::Cw, march(next.as_str(), &[start.as_str()], 15))
            .unwrap_err();
        assert_eq!(err.case, "28.14");
        state
            .apply(&data, Side::Cw, march(next.as_str(), &[start.as_str()], 10))
            .unwrap();
        let err = state
            .apply(&data, Side::Cw, march(start.as_str(), &["E3613"], 5))
            .unwrap_err();
        assert_eq!(err.case, "28.13");
    }

    #[test]
    fn a_march_does_not_come_back_to_its_start_28_13() {
        let data = graziani();
        let start = by_alexandria(&data, &set_up(&data, 3));
        let next = data
            .map
            .neighbours(&start)
            .into_iter()
            .find(|n| !departure_hexes(&data, Side::Cw).contains(n))
            .unwrap();
        let mut state = marching(&data, start.as_str(), 25);
        let err = state
            .apply(
                &data,
                Side::Cw,
                march(start.as_str(), &[next.as_str(), start.as_str()], 25),
            )
            .unwrap_err();
        assert_eq!(err.case, "28.13");
    }

    #[test]
    fn prisoners_do_not_enter_an_enemy_hex_28_12() {
        let data = graziani();
        let start = by_alexandria(&data, &set_up(&data, 3));
        let mut state = marching(&data, start.as_str(), 5);
        let axis = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&axis).unwrap().location = Location::Hex(hex("E3613"));
        let err = state
            .apply(&data, Side::Cw, march(start.as_str(), &["E3613"], 5))
            .unwrap_err();
        assert_eq!(err.case, "28.12");
    }

    #[test]
    fn phase_h_opens_for_prisoners_alone_28_13() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state
            .prisoners
            .entry(Side::Cw)
            .or_default()
            .insert(hex("E2020"), 5);
        through_combat(&data, &mut state);
        while state.clock.phase == Phase::MovementCombat {
            let player = state.to_act().unwrap();
            state.apply(&data, player, Order::Pass).unwrap();
        }
        assert_eq!(state.clock.phase, Phase::TruckConvoy);
        assert_eq!(state.to_act(), Some(Side::Cw));
    }
}
