//! Move orders (SPI 8.1x, 6.1x): a stack moves hex by hex in a movement
//! segment, each step priced by `cna_rules::movement::step_cost` for
//! every piece in it, the stack paying the dearest (R-004); CP over the
//! CPA earns DP at once (6.21); a foot unit never spends past 150 % of
//! its CPA in its own half (8.1x).

use crate::clock::{Phase, Segment};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::{GameState, Weather};
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::{Side, Status};
use cna_rules::movement::{Mobility, Step, Weather as StepWeather, step_cost};

/// The Commonwealth never goes west of Marble Arch, A2109 (8.1x).
const MARBLE_ARCH_COL: u8 = 9;

impl GameState {
    /// The weather over a hex, for movement (29.4x, 29.5x).
    pub(crate) fn weather_at(&self, hex: &HexId) -> StepWeather {
        match &self.weather {
            Weather::Sandstorm { sheets } if sheets.contains(&hex.sheet()) => {
                StepWeather::Sandstorm
            }
            Weather::Rainstorm { sheets } if sheets.contains(&hex.sheet()) => {
                StepWeather::Rainstorm
            }
            _ => StepWeather::Normal,
        }
    }

    /// The CPA a piece moves on (6.1x): its printed allowance, or for an
    /// HQ the slowest unit under it (its whole subtree, 6.15); none for a
    /// printed 0, which cannot move on its own.
    pub(crate) fn cpa(&self, piece: &Piece) -> Option<u32> {
        let own = piece.cpa.as_ref()?.value?;
        let mut cpa = own;
        if piece.kind == PieceKind::Hq
            && let Some(unit) = piece.id.unit()
        {
            let mut heads = vec![unit];
            while let Some(head) = heads.pop() {
                for q in self.pieces.values() {
                    if q.parent.as_ref() != Some(&head) {
                        continue;
                    }
                    if let Some(v) = q.cpa.as_ref().and_then(|r| r.value)
                        && v > 0
                    {
                        cpa = cpa.min(v);
                    }
                    if let Some(u) = q.id.unit() {
                        heads.push(u);
                    }
                }
            }
        }
        (cpa > 0).then_some(u32::try_from(cpa).unwrap_or(0))
    }

    /// How a piece moves on the chart: foot at CPA 10 or less (8.1x),
    /// recce by its characteristics row, any other vehicle.
    pub(crate) fn mobility(&self, data: &Data, piece: &Piece) -> Mobility {
        if self.cpa(piece).is_none_or(|c| c <= 10) {
            return Mobility::Foot;
        }
        let recce = piece
            .nation
            .zip(piece.id_code.as_deref())
            .and_then(|(n, code)| data.chars.row(n, code))
            .is_some_and(|r| {
                r.unit_type.contains("Recce") || r.unit_type.contains("Reconnaissance")
            });
        if recce {
            Mobility::Recce
        } else {
            Mobility::Vehicle
        }
    }

    /// The hex a stack to move stands on, after checking it is the
    /// player's, together, and able to move.
    fn moving_stack(&self, player: Side, pieces: &[PieceId]) -> Result<HexId, Illegal> {
        let Some(first) = pieces.first() else {
            return Err(Illegal::new("8.1x", "no pieces to move"));
        };
        let mut from: Option<HexId> = None;
        for (i, id) in pieces.iter().enumerate() {
            if pieces[..i].contains(id) {
                return Err(Illegal::new("8.1x", format!("{id} is listed twice")));
            }
            let p = self
                .pieces
                .get(id)
                .ok_or_else(|| Illegal::new("8.1x", format!("{id}: no such piece")))?;
            if p.side != player {
                return Err(Illegal::new("8.1x", format!("{id} is not {player}'s")));
            }
            let Location::Hex(h) = &p.location else {
                return Err(Illegal::new("8.1x", format!("{id} is not on the map")));
            };
            match &from {
                None => from = Some(h.clone()),
                Some(f) if f != h => {
                    return Err(Illegal::new(
                        "8.1x",
                        format!("{id} is at {h}, not with {first} at {f}"),
                    ));
                }
                Some(_) => {}
            }
            self.able_to_move(p)?;
        }
        Ok(from.expect("at least one piece"))
    }

    /// Whether a piece may move voluntarily at all.
    fn able_to_move(&self, p: &Piece) -> Result<(), Illegal> {
        match p.kind {
            PieceKind::Combat | PieceKind::Hq => {}
            PieceKind::SupplyUnit | PieceKind::DummySupplyUnit => {
                return Err(Illegal::new(
                    "32.x",
                    format!("{}: supply units are inert in this mission", p.id),
                ));
            }
        }
        if self.cpa(p).is_none() {
            return Err(Illegal::new(
                "6.1x",
                format!("{}: a CPA of 0 cannot move on its own", p.id),
            ));
        }
        if p.cohesion.collapsed(&self.ruleset) {
            return Err(Illegal::new(
                "6.26",
                format!(
                    "{} is at cohesion {}: it cannot move",
                    p.id, p.cohesion.level
                ),
            ));
        }
        if p.status == Some(Status::InTraining) {
            return Err(Illegal::new(
                "5.2",
                format!("{} is in training and may not move voluntarily", p.id),
            ));
        }
        Ok(())
    }

    /// Prices a path for a stack: the hexes entered and what each step
    /// costs the stack (the dearest piece's price, R-004).
    fn plan(
        &self,
        data: &Data,
        pieces: &[PieceId],
        from: &HexId,
        path: &[HexId],
    ) -> Result<Vec<(HexId, Cp)>, Illegal> {
        if path.is_empty() {
            return Err(Illegal::new("8.1x", "an empty path"));
        }
        let side = self.pieces[&pieces[0]].side;
        let mobilities: Vec<Mobility> = pieces
            .iter()
            .map(|id| self.mobility(data, &self.pieces[id]))
            .collect();
        let mut steps = Vec::with_capacity(path.len());
        let mut at = from.clone();
        for to in path {
            let Some(hex) = data.map.hex(to) else {
                return Err(Illegal::new("8.1x", format!("{to}: not on the map")));
            };
            if !data.map.adjacent(&at, to) {
                return Err(Illegal::new(
                    "8.1x",
                    format!("{to} is not adjacent to {at}"),
                ));
            }
            if let Some(enemy) = self
                .pieces
                .values()
                .find(|q| q.side != side && q.hex() == Some(to))
            {
                return Err(Illegal::new(
                    "8.1x",
                    format!("{to} holds the enemy unit {}", enemy.id),
                ));
            }
            if side == Side::Cw && to.sheet() == 'A' && to.col() < MARBLE_ARCH_COL {
                return Err(Illegal::new(
                    "8.1x",
                    format!("{to} is west of Marble Arch: no Commonwealth unit goes there"),
                ));
            }
            let hexside = data.map.hexside(&at, to);
            let step = Step {
                from: data.map.hex(&at).expect("on the map"),
                to: hex,
                hexside,
                climbs: hexside.is_some_and(|hs| hs.climbs(&at, to)),
                weather: self.weather_at(to),
            };
            let mut cost = Cp::ZERO;
            for (id, m) in pieces.iter().zip(&mobilities) {
                match step_cost(&data.terrain, *m, &step) {
                    Ok(c) => cost = cost.max(c),
                    Err(p) => {
                        return Err(Illegal::new(
                            p.case,
                            format!("{id} may not enter {to} from {at}: {}", p.reason),
                        ));
                    }
                }
            }
            steps.push((to.clone(), cost));
            at = to.clone();
        }
        Ok(steps)
    }

    /// Charges a piece CP, earning a DP for every whole point over its
    /// CPA (6.21).
    pub(crate) fn charge(
        &mut self,
        id: &PieceId,
        cost: Cp,
        enemy_half: bool,
        events: &mut Vec<Event>,
    ) {
        let cpa = Cp::whole(self.cpa(&self.pieces[id]).unwrap_or(0));
        let p = self.pieces.get_mut(id).expect("checked");
        let before = p.cp_spent.saturating_sub(cpa).floor();
        p.cp_spent += cost;
        if enemy_half {
            p.cp_enemy_half += cost;
        }
        let dp = p.cp_spent.saturating_sub(cpa).floor() - before;
        if dp > 0 {
            p.cohesion.earn_dp(dp);
            events.push(Event::Disorganised {
                piece: id.clone(),
                dp,
                level: p.cohesion.level,
            });
        }
    }

    /// `Order::Move`: the stack walks `path` in the phasing player's
    /// movement segment.
    pub(crate) fn move_pieces(
        &mut self,
        data: &Data,
        player: Side,
        pieces: &[PieceId],
        path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::Movement)
        {
            return Err(Illegal::new(
                "8.1x",
                format!(
                    "voluntary movement happens in a movement segment, not at {}",
                    self.clock
                ),
            ));
        }
        let from = self.moving_stack(player, pieces)?;
        let steps = self.plan(data, pieces, &from, path)?;
        let total: Cp = steps.iter().map(|(_, c)| *c).sum();
        for id in pieces {
            let p = &self.pieces[id];
            let cpa = self.cpa(p).expect("able to move");
            if cpa <= 10 {
                // 150 % of the CPA, in half points.
                let cap = Cp::halves(cpa * 3);
                if p.cp_own_half() + total > cap {
                    return Err(Illegal::new(
                        "8.1x",
                        format!(
                            "{id} (CPA {cpa}) has spent {} and may not go past {cap} voluntarily; the path costs {total}",
                            p.cp_own_half()
                        ),
                    ));
                }
            }
        }
        let mut events = Vec::new();
        let mut at = from;
        for (to, cost) in steps {
            for id in pieces {
                self.pieces.get_mut(id).expect("checked").location = Location::Hex(to.clone());
            }
            events.push(Event::Moved {
                pieces: pieces.to_vec(),
                from: at,
                to: to.clone(),
                cost,
            });
            for id in pieces {
                self.charge(id, cost, false, &mut events);
            }
            at = to;
        }
        Ok(events)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::order::Order;
    use crate::state::test_support::set_up;

    pub fn pid(s: &str) -> PieceId {
        PieceId::from(cna_data::oa::UnitId::parse(s).unwrap())
    }

    pub fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    pub fn path(v: &[&str]) -> Vec<HexId> {
        v.iter().map(|s| hex(s)).collect()
    }

    /// A set-up game in GT1 OpStage 1 with `first` as Player A, in its
    /// phase G movement segment.
    pub fn moving(data: &Data, seed: u64, first: Side) -> GameState {
        let mut state = set_up(data, seed);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative { player_a: first },
            )
            .unwrap();
        state.apply(data, first, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::MovementCombat);
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        state
    }

    pub fn mv(pieces: &[&str], p: &[&str]) -> Order {
        Order::Move {
            pieces: pieces.iter().map(|s| pid(s)).collect(),
            path: path(p),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::order::Order;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    #[test]
    fn a_move_outside_a_movement_segment_is_illegal() {
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
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        assert_eq!(snapshot(&state), before);
    }

    #[test]
    fn a_foot_battalion_pays_the_chart_and_overspend_is_dp() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let krrc = pid("unit:cw:1-krrc");
        assert_eq!(state.pieces[&krrc].hex(), Some(&hex("C3922")));
        assert_eq!(state.cpa(&state.pieces[&krrc]), Some(10));
        // Seven clear hexes east along row 39, the C3926-C3927 unfinished
        // road a track: 2+2+2+2+1+2+2 = 13 CP, 3 over the CPA.
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(
                    &["unit:cw:1-krrc"],
                    &[
                        "C3923", "C3924", "C3925", "C3926", "C3927", "C3928", "C3929",
                    ],
                ),
            )
            .unwrap();
        let p = &state.pieces[&krrc];
        assert_eq!(p.hex(), Some(&hex("C3929")));
        assert_eq!(p.cp_spent, Cp::whole(13));
        assert_eq!(p.cohesion.level, -3);
        assert_eq!(p.cohesion.dp_tally, 3);
        let costs: Vec<Cp> = events
            .iter()
            .filter_map(|e| match e {
                Event::Moved { cost, .. } => Some(*cost),
                _ => None,
            })
            .collect();
        assert_eq!(costs, [2, 2, 2, 2, 1, 2, 2].map(Cp::whole));
        assert!(events.contains(&Event::Disorganised {
            piece: krrc.clone(),
            dp: 1,
            level: -1
        }));
        // Up to 150 % (15) is allowed; past it is not (8.1x).
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3930"]))
            .unwrap();
        assert_eq!(state.pieces[&krrc].cp_spent, Cp::whole(15));
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3931"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        assert!(
            err.reason.contains("150") || err.reason.contains("15"),
            "{}",
            err.reason
        );
        assert_eq!(snapshot(&state), before);
        let moves = state
            .log
            .iter()
            .filter(|o| matches!(o.order, Order::Move { .. }))
            .count();
        assert_eq!(moves, 2);
    }

    #[test]
    fn bad_paths_pieces_and_enemy_hexes_are_rejected() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let before = snapshot(&state);
        let cases: Vec<(Order, &str)> = vec![
            // Not adjacent.
            (mv(&["unit:cw:1-krrc"], &["C3924"]), "8.1x"),
            // Not on the map.
            (mv(&["unit:cw:1-krrc"], &["C9999"]), "8.1x"),
            // Not the player's.
            (mv(&["unit:it:maletti-div-hq"], &["C3618"]), "8.1x"),
            // No such piece, no pieces, no path, a piece twice.
            (mv(&["unit:cw:nobody"], &["C3923"]), "8.1x"),
            (mv(&[], &["C3923"]), "8.1x"),
            (mv(&["unit:cw:1-krrc"], &[]), "8.1x"),
            (
                mv(&["unit:cw:1-krrc", "unit:cw:1-krrc"], &["C3923"]),
                "8.1x",
            ),
            // Two pieces on different hexes.
            (mv(&["unit:cw:1-krrc", "unit:cw:1-rnf"], &["C3923"]), "8.1x"),
            // Into an enemy-occupied hex: 11 Hussars at C3020 next to the
            // Fort Maddalena garrison at C3019.
            (mv(&["unit:cw:11-hus"], &["C3019"]), "8.1x"),
            // Sea.
            (mv(&["unit:cw:1-krrc"], &["C4022"]), "8.37"),
            // A supply unit is inert.
            (
                Order::Move {
                    pieces: vec![PieceId::supply(Side::Cw, 1)],
                    path: path(&["C3923"]),
                },
                "32.x",
            ),
        ];
        assert!(data.map.adjacent(&hex("C3020"), &hex("C3019")));
        assert_eq!(
            state.pieces[&pid("unit:it:fort-maddalenna-garrison-bn")].hex(),
            Some(&hex("C3019"))
        );
        for (order, case) in cases {
            let err = state.apply(&data, Side::Cw, order.clone()).unwrap_err();
            assert_eq!(err.case, case, "{order:?}: {}", err.reason);
            assert_eq!(snapshot(&state), before, "{order:?}");
        }
    }

    #[test]
    fn a_mixed_stack_pays_the_dearer_cost_and_obeys_both_prohibitions() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 1 KRRC (foot) and 7 Med (artillery, CPA 20, a vehicle) share
        // C3922. Up the escarpment to C3921 the battalion alone pays 8; the
        // stack may not go (R-004, 8.4x).
        let before = snapshot(&state);
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc", "unit:cw:7-med"], &["C3921"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "8.4x");
        assert_eq!(snapshot(&state), before);
        // Along the coast east both can go; the stack pays the foot rate
        // (2), the dearer of 2 and 2; by the unfinished road C3926-C3927
        // foot pays 1 and the vehicle 1: both are charged 1.
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc", "unit:cw:7-med"], &["C3923"]),
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::Moved {
                pieces: vec![pid("unit:cw:1-krrc"), pid("unit:cw:7-med")],
                from: hex("C3922"),
                to: hex("C3923"),
                cost: Cp::whole(2)
            }]
        );
        assert_eq!(state.pieces[&pid("unit:cw:7-med")].cp_spent, Cp::whole(2));
        assert_eq!(
            state.pieces[&pid("unit:cw:7-med")].hex(),
            Some(&hex("C3923"))
        );
        // The battalion alone climbs for 8 (foot, 2 + 6).
        state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc"], &["C3922", "C3921"]),
            )
            .unwrap();
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(12));
    }

    #[test]
    fn a_vehicle_on_the_road_pays_half_points() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        // The 1st Libyan Division HQ (CPA 20 printed; its slowest attached
        // unit is CPA 10, so it moves at 10, on foot) at C4020; the LXII(L)
        // tank battalion (CPA 30) at C3918 with the 62nd Marmarica.
        let hq = pid("unit:it:1st-libyan-infantry-hq");
        assert_eq!(state.cpa(&state.pieces[&hq]), Some(10));
        assert_eq!(state.mobility(&data, &state.pieces[&hq]), Mobility::Foot);
        let tanks = pid("unit:it:lxiil-tank-bn");
        assert_eq!(state.pieces[&tanks].hex(), Some(&hex("C3918")));
        assert_eq!(
            state.mobility(&data, &state.pieces[&tanks]),
            Mobility::Vehicle
        );
        // C3918 to C3919 (clear, plain): 2; C3919 to C3920 by track: 1;
        // C3920 to C4020 by track: 1; C4020 to C4120 by road: ½.
        state
            .apply(
                &data,
                Side::Axis,
                mv(
                    &["unit:it:lxiil-tank-bn"],
                    &["C3919", "C3920", "C4020", "C4120"],
                ),
            )
            .unwrap();
        assert_eq!(state.pieces[&tanks].cp_spent, Cp::halves(9));
        assert_eq!(state.pieces[&tanks].cohesion.level, 0);
    }

    #[test]
    fn the_commonwealth_never_goes_west_of_marble_arch() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let hus = pid("unit:cw:11-hus");
        state.pieces.get_mut(&hus).unwrap().location = Location::Hex(hex("A2110"));
        let before = snapshot(&state);
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:11-hus"], &["A2109", "A2108"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        assert!(err.reason.contains("Marble Arch"), "{}", err.reason);
        assert_eq!(snapshot(&state), before);
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:11-hus"], &["A2109"]))
            .unwrap();
    }

    #[test]
    fn cp_is_reset_at_the_start_of_an_opstage_and_rest_restores_cohesion() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let krrc = pid("unit:cw:1-krrc");
        state
            .apply(
                &data,
                Side::Cw,
                mv(
                    &["unit:cw:1-krrc"],
                    &[
                        "C3923", "C3924", "C3925", "C3926", "C3927", "C3928", "C3929",
                    ],
                ),
            )
            .unwrap();
        assert_eq!(state.pieces[&krrc].cohesion.level, -3);
        crate::state::test_support::pass_stage(&data, &mut state);
        assert_eq!(state.clock.stage, 2);
        let p = &state.pieces[&krrc];
        assert_eq!(p.cp_spent, Cp::ZERO);
        // It spent CP, so no rest: still -3. A unit that spent nothing
        // rests, but never above 0.
        assert_eq!(p.cohesion.level, -3);
        crate::state::test_support::pass_stage(&data, &mut state);
        assert_eq!(state.pieces[&krrc].cohesion.level, 0);
        assert_eq!(state.pieces[&krrc].cohesion.dp_tally, 3);
    }
}
