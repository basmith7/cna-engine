//! Move orders (SPI 8.1x, 6.1x): a stack moves hex by hex in a movement
//! segment, each step priced by `cna_rules::movement::step_cost` for
//! every piece in it, the stack paying the dearest (R-004); CP over the
//! CPA earns DP at once (6.21); a foot unit never spends past 150 % of
//! its CPA in its own half (8.1x).

use crate::clock::{Phase, Segment};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind, Reserve};
use crate::state::{GameState, Pending, Weather};
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::{Side, Status};
use cna_rules::movement::{Mobility, Step, Weather as StepWeather, step_cost};

/// The Commonwealth never goes west of Marble Arch, A2109 (8.1x).
const MARBLE_ARCH_COL: u8 = 9;

/// A priced path: the hexes the stack will enter with each step's cost,
/// whether it first breaks off from contact, and where it has to pause
/// for a ZOC declaration (the hex it was about to enter, the hexes to
/// ask about).
#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub(crate) steps: Vec<(HexId, Cp)>,
    pub(crate) break_off: bool,
    pub(crate) interrupt: Option<(HexId, Vec<HexId>)>,
    /// The last step enters an enemy ZOC (10.2x).
    pub(crate) ends_in_zoc: bool,
}

/// A stack as a plan prices it: who it is, how each piece moves, and
/// whether it breaks off first.
#[derive(Debug)]
pub(crate) struct Mover {
    pub(crate) side: Side,
    /// A combat unit is in it, so it may enter an enemy ZOC (10.29).
    pub(crate) combat: bool,
    pub(crate) break_off: bool,
    mobilities: Vec<Mobility>,
    pub(crate) pieces: Vec<PieceId>,
}

/// Where a stack starts from, as far as ZOC goes.
#[derive(Debug)]
pub(crate) enum Start {
    /// Standing in an enemy ZOC (true) or clear of one.
    In(bool),
    /// Next to these enemy hexes that have not declared (10.16).
    Undeclared(Vec<HexId>),
}

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
    /// printed 0, which cannot move on its own. A supply unit moves on 15
    /// with its motorisation points, else not at all (32.5x).
    pub(crate) fn cpa(&self, piece: &Piece) -> Option<u32> {
        // The points carrying a supply unit move it at CPA 15 (32.5x).
        if matches!(
            piece.kind,
            PieceKind::SupplyUnit | PieceKind::DummySupplyUnit
        ) {
            return (self.modules.abstract_supply && piece.motorised > 0).then_some(15);
        }
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
    pub(crate) fn moving_stack(
        &self,
        player: Side,
        pieces: &[PieceId],
        phasing: bool,
    ) -> Result<HexId, Illegal> {
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
            if phasing && !self.may_continue(id) {
                return Err(Illegal::new(
                    "8.2x",
                    format!(
                        "{id} ended the last movement segment more than two hexes from any enemy combat unit and may not move again this phase"
                    ),
                ));
            }
        }
        // In the movement segment a supply unit goes only with a friendly
        // combat unit, which it starts and ends stacked with (32.31): both
        // unmoved this segment, or one it has moved with.
        let kind = |id: &PieceId| self.pieces[id].kind;
        let escorted = |s: &PieceId| {
            pieces.iter().any(|c| {
                kind(c) == PieceKind::Combat
                    && if self.moved.contains(s) {
                        self.escorts.get(s).is_some_and(|e| e.contains(c))
                    } else {
                        !self.moved.contains(c)
                    }
            })
        };
        if phasing
            && pieces.iter().any(|id| {
                matches!(kind(id), PieceKind::SupplyUnit | PieceKind::DummySupplyUnit)
                    && !escorted(id)
            })
        {
            return Err(Illegal::new(
                "32.31",
                "a supply unit moves in the movement segment only with a friendly combat unit it started the segment with",
            ));
        }
        Ok(from.expect("at least one piece"))
    }

    /// Whether a piece may move voluntarily at all.
    pub(crate) fn able_to_move(&self, p: &Piece) -> Result<(), Illegal> {
        match p.kind {
            PieceKind::Combat | PieceKind::Hq => {}
            PieceKind::SupplyUnit | PieceKind::DummySupplyUnit => {
                if !self.modules.abstract_supply {
                    return Err(Illegal::new(
                        "32.x",
                        format!("{}: supply units are inert without abstract supply", p.id),
                    ));
                }
                if p.motorised == 0 {
                    return Err(Illegal::new(
                        "32.31",
                        format!(
                            "{}: a supply unit moves only with its motorisation points",
                            p.id
                        ),
                    ));
                }
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
        if p.reserve == Some(Reserve::II) {
            return Err(Illegal::new(
                "18.2x",
                format!("{} is in Reserve II and may not move until released", p.id),
            ));
        }
        Ok(())
    }

    /// Whether a stack is making a Reserve I shift (18.2x): every piece
    /// in Reserve I, or none of them.
    fn reserve_shift(&self, pieces: &[PieceId]) -> Result<bool, Illegal> {
        let in_reserve = |id: &PieceId| self.pieces[id].reserve == Some(Reserve::I);
        let n = pieces.iter().filter(|id| in_reserve(id)).count();
        if n != 0 && n != pieces.len() {
            return Err(Illegal::new(
                "18.2x",
                "a unit in reserve moves only with units in reserve",
            ));
        }
        Ok(n != 0)
    }

    /// Prices a path for a stack: the hexes entered and what each step
    /// costs the stack (the dearest piece's price, R-004), with the ZOC
    /// rules of 10.2x: entering an enemy ZOC is free and ends the move,
    /// never straight from one ZOC into another, and a non-combat stack
    /// never into one unoccupied (10.29). The plan stops short of a hex
    /// next to enemy units that have not said whether they exert a ZOC
    /// (10.16).
    pub(crate) fn plan(
        &self,
        data: &Data,
        pieces: &[PieceId],
        from: &HexId,
        path: &[HexId],
    ) -> Result<Plan, Illegal> {
        self.plan_with(data, pieces, from, path, false)
    }

    /// `plan`, where `forced` lets a stack leave an enemy ZOC it entered
    /// this segment: a forced retreat (15.82) is no voluntary move.
    pub(crate) fn plan_with(
        &self,
        data: &Data,
        pieces: &[PieceId],
        from: &HexId,
        path: &[HexId],
        forced: bool,
    ) -> Result<Plan, Illegal> {
        if path.is_empty() {
            return Err(Illegal::new("8.1x", "an empty path"));
        }
        let mover = self.mover(data, pieces);
        let mut plan = Plan {
            break_off: mover.break_off,
            ..Plan::default()
        };
        let mut at_in_zoc = match self.start(data, &mover, from, forced)? {
            Start::In(in_zoc) => in_zoc,
            Start::Undeclared(hexes) => {
                // The first step is checked before the move is parked on
                // the declaration: an order that could never be taken is
                // refused, not logged (8.1x).
                self.entry(data, mover.side, from, &path[0])?;
                plan.interrupt = Some((path[0].clone(), hexes));
                return Ok(plan);
            }
        };
        let mut at = from.clone();
        for to in path {
            let (cost, zoc) = self.step_into(data, &mover, &at, to, at_in_zoc)?;
            match zoc {
                Zoc::Undeclared(hexes) => {
                    plan.interrupt = Some((to.clone(), hexes));
                    return Ok(plan);
                }
                Zoc::Exerts => {
                    plan.steps.push((to.clone(), Cp::ZERO));
                    plan.ends_in_zoc = true;
                    if path.last() != Some(to) {
                        return Err(Illegal::new(
                            "10.2x",
                            format!("{to} is in an enemy ZOC: the move ends there"),
                        ));
                    }
                    return Ok(plan);
                }
                Zoc::Clear => {}
            }
            at_in_zoc = false;
            plan.steps.push((to.clone(), cost));
            at = to.clone();
        }
        Ok(plan)
    }

    /// The fixed facts about a stack a plan prices.
    pub(crate) fn mover(&self, data: &Data, pieces: &[PieceId]) -> Mover {
        Mover {
            side: self.pieces[&pieces[0]].side,
            combat: pieces
                .iter()
                .any(|id| self.pieces[id].kind == PieceKind::Combat),
            // A stack in contact or engaged breaks off before it moves
            // (8.6x). Contact only exists in an enemy ZOC; an engaged
            // unit need not be in one, so this is the pieces' state, not
            // the hex's.
            break_off: pieces
                .iter()
                .any(|id| self.pieces[id].contact || self.pieces[id].engaged),
            mobilities: pieces
                .iter()
                .map(|id| self.mobility(data, &self.pieces[id]))
                .collect(),
            pieces: pieces.to_vec(),
        }
    }

    /// Where the stack stands: in an enemy ZOC it either breaks off or
    /// is stopped (10.22), unless another friendly combat unit there
    /// cancels the ZOC (10.26), as it did when the stack entered; next
    /// to enemy units that have not said whether they exert one, the
    /// move waits (10.16).
    pub(crate) fn start(
        &self,
        data: &Data,
        mover: &Mover,
        from: &HexId,
        forced: bool,
    ) -> Result<Start, Illegal> {
        Ok(match self.zoc_at(data, from, mover.side) {
            Zoc::Exerts => {
                let cancelled = self.friendly_combat_in(from, mover.side, &mover.pieces);
                if !mover.break_off && !cancelled && !forced {
                    return Err(Illegal::new(
                        "10.2x",
                        format!(
                            "{} entered an enemy ZOC at {from} this segment and stays until a later one",
                            mover.pieces[0]
                        ),
                    ));
                }
                Start::In(!cancelled)
            }
            Zoc::Clear => Start::In(false),
            Zoc::Undeclared(hexes) => Start::Undeclared(hexes),
        })
    }

    /// One step of the stack from `at` into `to`: what it costs (the
    /// dearest piece's price, R-004) and the enemy ZOC it finds there,
    /// with the refusals of 8.1x, the chart, and 10.2x: never straight
    /// from one ZOC into another, and a non-combat stack never into one
    /// unoccupied (10.29). Entering a ZOC is free (10.2x): the cost is
    /// returned for the caller to drop.
    pub(crate) fn step_into(
        &self,
        data: &Data,
        mover: &Mover,
        at: &HexId,
        to: &HexId,
        at_in_zoc: bool,
    ) -> Result<(Cp, Zoc), Illegal> {
        let hex = self.entry(data, mover.side, at, to)?;
        let hexside = data.map.hexside(at, to);
        let step = Step {
            from: data.map.hex(at).expect("on the map"),
            to: hex,
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(at, to)),
            weather: self.weather_at(to),
        };
        let mut cost = Cp::ZERO;
        for (id, m) in mover.pieces.iter().zip(&mover.mobilities) {
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
        let zoc = if self.friendly_combat_in(to, mover.side, &mover.pieces) {
            Zoc::Clear
        } else {
            self.zoc_at(data, to, mover.side)
        };
        if zoc == Zoc::Exerts {
            if at_in_zoc {
                return Err(Illegal::new(
                    "10.2x",
                    format!("{at} and {to} are both enemy-controlled: no ZOC-to-ZOC move"),
                ));
            }
            if !mover.combat {
                return Err(Illegal::new(
                    "10.29",
                    format!(
                        "{to} is an unoccupied enemy-controlled hex: no non-combat unit enters one"
                    ),
                ));
            }
        }
        Ok((cost, zoc))
    }

    /// Whether a unit of `side` may step from `at` into `to` at all
    /// (8.1x): on the map, adjacent, not an enemy-held hex, and east of
    /// Marble Arch for the Commonwealth. The terrain is the chart's.
    fn entry<'a>(
        &self,
        data: &'a Data,
        side: Side,
        at: &HexId,
        to: &HexId,
    ) -> Result<&'a cna_data::map::Hex, Illegal> {
        let Some(hex) = data.map.hex(to) else {
            return Err(Illegal::new("8.1x", format!("{to}: not on the map")));
        };
        if !data.map.adjacent(at, to) {
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
        Ok(hex)
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
        // Supply units have no cohesion to lose (32.1x: no combat value).
        let supply = matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit);
        if dp > 0 && !supply {
            p.cohesion.earn_dp(dp);
            events.push(Event::Disorganised {
                piece: id.clone(),
                dp,
                level: p.cohesion.level,
            });
        }
    }

    /// What a voluntary move needs before any path is looked at: a
    /// movement segment, and a stack of the player's that may move now;
    /// where it stands and whether it is a Reserve I shift (18.2x).
    pub(crate) fn stack_to_move(
        &self,
        player: Side,
        pieces: &[PieceId],
    ) -> Result<(HexId, bool), Illegal> {
        if self.clock.phase == Phase::TruckConvoy && self.modules.abstract_supply {
            // The truck convoy phase moves supply units alone, and only
            // those that did not move in G (32.31).
            for id in pieces {
                let p = self
                    .pieces
                    .get(id)
                    .ok_or_else(|| Illegal::new("32.31", format!("no piece {id}")))?;
                if !matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit) {
                    return Err(Illegal::new(
                        "32.31",
                        format!("{id}: only supply units move in the truck convoy phase"),
                    ));
                }
                if p.escorted {
                    return Err(Illegal::new(
                        "32.31",
                        format!("{id} moved in the movement and combat phase: not again in H"),
                    ));
                }
            }
            let from = self.moving_stack(player, pieces, false)?;
            return Ok((from, false));
        }
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
        let shift = pieces
            .iter()
            .all(|id| self.pieces.get(id).is_some_and(|p| p.reserve.is_some()));
        let from = self.moving_stack(player, pieces, !shift)?;
        let shift = self.reserve_shift(pieces)?;
        Ok((from, shift))
    }

    /// What breaking off costs a piece (8.6x).
    pub(crate) fn break_off_cost(p: &Piece) -> Cp {
        if p.engaged {
            Cp::whole(4)
        } else if p.contact {
            Cp::whole(2)
        } else {
            Cp::ZERO
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
        assault: bool,
        fuel_from: Option<&PieceId>,
    ) -> Result<Vec<Event>, Illegal> {
        let (from, shift) = self.stack_to_move(player, pieces)?;
        let mut plan = self.plan(data, pieces, &from, path)?;
        // Supply stacking binds where the move ends (32.1x).
        let supply = pieces
            .iter()
            .filter(|id| {
                matches!(
                    self.pieces[*id].kind,
                    PieceKind::SupplyUnit | PieceKind::DummySupplyUnit
                )
            })
            .count();
        if supply > 0
            && let Some(end) = path.last()
            && let Some(e) = self.supply_stack_refusal(data, PieceKind::SupplyUnit, end, supply)
        {
            return Err(e);
        }
        if shift {
            // A Reserve I unit shifts one hex a segment, free, never into
            // an enemy ZOC (18.2x).
            if path.len() != 1 {
                return Err(Illegal::new(
                    "18.2x",
                    "a unit in reserve shifts one hex a movement segment",
                ));
            }
            if let Some(id) = pieces.iter().find(|id| self.shifted.contains(id)) {
                return Err(Illegal::new(
                    "18.2x",
                    format!("{id} has made its reserve shift this segment"),
                ));
            }
            if plan.ends_in_zoc {
                return Err(Illegal::new(
                    "18.2x",
                    format!(
                        "{}: a unit in reserve never shifts into an enemy ZOC",
                        path[0]
                    ),
                ));
            }
            for step in &mut plan.steps {
                step.1 = Cp::ZERO;
            }
        }
        // A reinforcement's first hex is free (20.1x).
        let landed = pieces.iter().all(|id| self.pieces[id].landed);
        if landed && let Some(step) = plan.steps.first_mut() {
            step.1 = Cp::ZERO;
        }
        let mut total: Cp = plan.steps.iter().map(|(_, c)| *c).sum();
        let break_off = Self::break_off_cost;
        if plan.break_off && !plan.steps.is_empty() {
            total += pieces
                .iter()
                .map(|id| break_off(&self.pieces[id]))
                .max()
                .unwrap_or(Cp::ZERO);
        }
        for id in pieces {
            let p = &self.pieces[id];
            if let Some((cap, case, why)) = self.voluntary_cap(id)
                && p.cp_own_half() + total > cap
            {
                return Err(Illegal::new(
                    case,
                    format!(
                        "{id} has spent {} and the path costs {total}, past {cap}: {why}",
                        p.cp_own_half()
                    ),
                ));
            }
        }
        let payments = if plan.steps.is_empty() {
            Vec::new()
        } else {
            self.plan_fuel(data, pieces, total, fuel_from)?
        };
        let mut events = Vec::new();
        self.spend_fuel(data, total, payments, &mut events);
        if plan.break_off && !plan.steps.is_empty() {
            for id in pieces {
                let cost = break_off(&self.pieces[id]);
                if cost > Cp::ZERO {
                    events.push(Event::ContactBroken {
                        piece: id.clone(),
                        cost,
                    });
                    self.charge(id, cost, false, &mut events);
                }
                let p = self.pieces.get_mut(id).expect("checked");
                p.contact = false;
                p.engaged = false;
            }
        }
        let combat = pieces
            .iter()
            .any(|id| self.pieces[id].kind == PieceKind::Combat);
        let mut at = from.clone();
        let mut reaction = None;
        for (to, cost) in plan.steps {
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
            // Next to enemy units that may react: the move waits here
            // (8.5x). Only a combat unit's arrival asks the question.
            if combat {
                let eligible = self.reactors_next_to(data, &at, pieces, assault);
                if !eligible.is_empty() {
                    reaction = Some(eligible);
                    break;
                }
            }
        }
        if at != from {
            self.moved.extend(pieces.iter().cloned());
            self.release_contact_around(data, &from, player);
            if shift {
                self.shifted.extend(pieces.iter().cloned());
            }
            let in_g = self.clock.phase == Phase::MovementCombat;
            let combat: Vec<PieceId> = pieces
                .iter()
                .filter(|id| self.pieces[*id].kind == PieceKind::Combat)
                .cloned()
                .collect();
            for id in pieces {
                if in_g
                    && matches!(
                        self.pieces[id].kind,
                        PieceKind::SupplyUnit | PieceKind::DummySupplyUnit
                    )
                {
                    self.escorts
                        .entry(id.clone())
                        .or_default()
                        .extend(combat.iter().cloned());
                }
                let p = self.pieces.get_mut(id).expect("checked");
                p.landed = false;
                // A supply unit moves in G or in H, not both (32.31).
                p.escorted |=
                    in_g && matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit);
            }
        }
        if let Some(eligible) = reaction {
            events.push(Event::ReactionOffered {
                pieces: eligible.clone(),
                at,
            });
            self.pending = Some(Pending::Reaction {
                side: player.other(),
                eligible,
            });
            return Ok(events);
        }
        if let Some((before, declare)) = plan.interrupt {
            events.push(Event::MoveInterrupted {
                pieces: pieces.to_vec(),
                at,
                before,
                declare: declare.clone(),
            });
            self.pending = Some(Pending::DeclareZoc {
                side: player.other(),
                hexes: declare,
                at_start: false,
            });
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
            assault: false,
            fuel_from: None,
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
    fn a_collapsed_unit_cannot_move() {
        // 6.26: at the collapse level a unit stays where it is.
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let krrc = pid("unit:cw:1-krrc");
        state.pieces.get_mut(&krrc).unwrap().cohesion.level = -cna_rules::cohesion::COLLAPSE;
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap_err();
        assert_eq!(err.case, "6.26");
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
                    assault: false,
                    fuel_from: None,
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
        // C3922. By the unfinished road (a track) to C3822 both pay 1.
        let events = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc", "unit:cw:7-med"], &["C3822"]),
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::Moved {
                pieces: vec![pid("unit:cw:1-krrc"), pid("unit:cw:7-med")],
                from: hex("C3922"),
                to: hex("C3822"),
                cost: Cp::whole(1)
            }]
        );
        assert_eq!(state.pieces[&pid("unit:cw:7-med")].cp_spent, Cp::whole(1));
        // Up the escarpment to C3722 the battalion alone pays 2 + 6; the
        // stack may not go (R-004, 8.4x).
        let before = snapshot(&state);
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc", "unit:cw:7-med"], &["C3722"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "8.4x");
        assert_eq!(snapshot(&state), before);
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3722"]))
            .unwrap();
        assert_eq!(state.pieces[&pid("unit:cw:1-krrc")].cp_spent, Cp::whole(9));
        assert_eq!(
            state.pieces[&pid("unit:cw:7-med")].hex(),
            Some(&hex("C3822"))
        );
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
    fn after_the_first_segment_only_units_near_the_enemy_move_again() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 1 KRRC steps east, far from any Axis unit; 11 Hussars at C3020
        // sit next to the Fort Maddalena garrison battalion.
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap();
        crate::state::test_support::through_combat(&data, &mut state);
        assert!(state.continuing.contains(&pid("unit:cw:11-hus")));
        assert!(!state.continuing.contains(&pid("unit:cw:1-krrc")));
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3924"]))
            .unwrap_err();
        assert_eq!(err.case, "8.2x");
        assert_eq!(snapshot(&state), before);
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:11-hus"], &["C3021"]))
            .unwrap();
        // A new phase starts afresh: in the Axis half every Axis unit may
        // make its first move.
        crate::state::test_support::through_combat(&data, &mut state);
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(state.cycle, 0);
        assert!(state.continuing.is_empty());
        state
            .apply(
                &data,
                Side::Axis,
                mv(&["unit:it:xxi-corps-artillery-regt"], &["A0102"]),
            )
            .unwrap_or_else(|e| panic!("{e}"));
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

    #[test]
    fn a_seam_pair_is_crossed_and_refused_like_any_hexside() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 1 KRRC (foot) at C0633 (rough) crosses the seam slope to D0701
        // (clear), the higher hex: 2 + 2 up; back is 3 + 1 down.
        let krrc = pid("unit:cw:1-krrc");
        state.pieces.get_mut(&krrc).unwrap().location = Location::Hex(hex("C0633"));
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["D0701"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::Moved {
                pieces: vec![krrc.clone()],
                from: hex("C0633"),
                to: hex("D0701"),
                cost: Cp::whole(4)
            }
        );
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C0633"]))
            .unwrap();
        assert_eq!(state.pieces[&krrc].cp_spent, Cp::whole(8));
        // 7 Med (vehicles) at D2233 may not climb the seam escarpment
        // into E2301, the higher hex (8.4x); by the coast road across
        // C4033|D4001 it pays the road rate, ½.
        let med = pid("unit:cw:7-med");
        state.pieces.get_mut(&med).unwrap().location = Location::Hex(hex("D2233"));
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:7-med"], &["E2301"]))
            .unwrap_err();
        assert_eq!(err.case, "8.4x");
        assert_eq!(snapshot(&state), before);
        state.pieces.get_mut(&med).unwrap().location = Location::Hex(hex("C4033"));
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:7-med"], &["D4001"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::Moved {
                pieces: vec![med],
                from: hex("C4033"),
                to: hex("D4001"),
                cost: Cp::halves(1)
            }
        );
    }

    #[test]
    fn an_engaged_unit_disengages_for_4_cp_whatever_the_zoc() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 1 KRRC at C3922 is engaged from an earlier close assault, but
        // no enemy ZOC reaches C3922: it still pays 4 CP to disengage
        // (8.6x) before the 2 for C3923.
        let krrc = pid("unit:cw:1-krrc");
        state.pieces.get_mut(&krrc).unwrap().engaged = true;
        assert_eq!(state.zoc_at(&data, &hex("C3922"), Side::Cw), Zoc::Clear);
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::ContactBroken {
                piece: krrc.clone(),
                cost: Cp::whole(4)
            }
        );
        let p = &state.pieces[&krrc];
        assert_eq!(p.cp_spent, Cp::whole(6));
        assert!(!p.engaged);
    }

    #[test]
    fn a_move_from_an_undeclared_hex_is_validated_before_the_interrupt() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        // 1 KRRC stands at C4021 next to the 1st Libyan Division at
        // C4020, whose ZOC has not been declared this segment. A path
        // that is not even adjacent is refused outright, not parked on
        // the declaration.
        let krrc = pid("unit:cw:1-krrc");
        state.pieces.get_mut(&krrc).unwrap().location = Location::Hex(hex("C4021"));
        assert!(matches!(
            state.zoc_at(&data, &hex("C4021"), Side::Cw),
            Zoc::Undeclared(_)
        ));
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C0101"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        assert!(err.reason.contains("not adjacent"), "{}", err.reason);
        assert_eq!(snapshot(&state), before);
        assert_eq!(state.pending, None);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4020"]))
            .unwrap_err();
        assert_eq!(err.case, "8.1x");
        assert!(err.reason.contains("enemy"), "{}", err.reason);
        assert_eq!(snapshot(&state), before);
        // A sound first step still waits for the declaration.
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4121"]))
            .unwrap();
        assert!(matches!(events[0], Event::MoveInterrupted { .. }));
        assert!(matches!(state.pending, Some(Pending::DeclareZoc { .. })));
    }
}
