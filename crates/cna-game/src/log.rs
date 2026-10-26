//! The log: what a game is made of, and nothing else. Replaying it on the
//! same `vendor/cna` commit rebuilds the state byte for byte. The commit
//! a state or log names is checked against `Data::rules_commit`, never
//! against git: the engine stays free of the file system.

use crate::clock::Phase;
use crate::data::Data;
use crate::order::LoggedOrder;
use crate::piece::{Location, MAX_AMMO, MAX_FUEL, PieceId, PieceKind};
use crate::state::{GameState, Modules, Pending};
use anyhow::{Context, Result, bail};
use cna_data::map::HexId;
use cna_data::scenario::{Locator, OffMapBox, Side};
use cna_rules::ruleset::Ruleset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Log {
    pub scenario: String,
    pub rules_commit: String,
    pub ruleset: Ruleset,
    #[serde(default, skip_serializing_if = "Modules::is_default")]
    pub modules: Modules,
    pub seed: u64,
    pub orders: Vec<LoggedOrder>,
}

impl GameState {
    /// The game so far as a log.
    pub fn to_log(&self) -> Log {
        Log {
            scenario: self.scenario.clone(),
            rules_commit: self.rules_commit.clone(),
            ruleset: self.ruleset,
            modules: self.modules,
            seed: self.rng.seed(),
            orders: self.log.clone(),
        }
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// Reads a saved state; fails if it is of another scenario, was
    /// made on another `vendor/cna` commit than `data`, or breaks an
    /// invariant `apply` relies on (`check_invariants`): a hostile or
    /// hand-edited save is an error, never a panic.
    pub fn from_json(json: &str, data: &Data) -> Result<GameState> {
        let state: GameState = serde_json::from_str(json).context("parsing the state")?;
        state.check(data)?;
        state.check_invariants(data)?;
        Ok(state)
    }

    /// What serde cannot check: the clock's shape (an OpStage of 1–3,
    /// `half` exactly in phases F–L, `segment` exactly in phase G) and
    /// a pending that names someone (a sealed step with nobody waiting
    /// panicked in `Pending::side`), and every piece and hex the state
    /// names (locations, the per-piece and per-hex sets, the combat
    /// record, the pending) existing, with reaction and retreat pieces
    /// of the side asked.
    pub fn check_invariants(&self, data: &Data) -> Result<()> {
        let c = &self.clock;
        if !(1..=3).contains(&c.stage) {
            bail!("the clock's OpStage is {}, not 1–3", c.stage);
        }
        if c.half.is_some() == c.phase.is_joint() {
            bail!(
                "the clock's half is {} in phase {}",
                if c.half.is_some() { "set" } else { "unset" },
                c.phase.letter()
            );
        }
        if c.segment.is_some() != (c.phase == Phase::MovementCombat) {
            bail!(
                "the clock's segment is {} in phase {}",
                if c.segment.is_some() { "set" } else { "unset" },
                c.phase.letter()
            );
        }
        let ids = Ids { state: self, data };
        for (key, p) in &self.pieces {
            if *key != p.id {
                bail!("the piece filed under {key} says it is {}", p.id);
            }
            match &p.location {
                Location::Hex(h) => ids.hex("a piece's location", h)?,
                Location::Transit { from, to, done } => {
                    if from == to || *done == 0 || *done == u8::MAX {
                        bail!(
                            "the transit of {key} is from {from} to {to}, {done} done: not a journey"
                        );
                    }
                }
                Location::Pending(p) => {
                    for l in &p.locators {
                        match l {
                            Locator::Hexes(hs) => hs
                                .iter()
                                .try_for_each(|h| ids.hex("a pending placement", h))?,
                            Locator::WestOf(h) => ids.hex("a pending placement", h)?,
                            _ => {}
                        }
                    }
                }
                Location::Box(_) | Location::Eliminated => {}
            }
            if let Some(h) = &p.garrison_of {
                ids.hex("a garrison", h)?;
            }
            // 32.1x: only real supply units carry loads, and only with
            // the module on.
            match (
                &p.load,
                self.modules.abstract_supply && p.kind == PieceKind::SupplyUnit,
            ) {
                (Some(l), true) if l.ammo > MAX_AMMO || l.fuel > MAX_FUEL => bail!(
                    "{key}: a load of {} ammunition and {} fuel is over {MAX_AMMO}/{MAX_FUEL}",
                    l.ammo,
                    l.fuel
                ),
                (Some(_), true) | (None, false) => {}
                (Some(_), false) => bail!("{key} has a load but is not a supply unit with one"),
                (None, true) => bail!("the supply unit {key} has no load"),
            }
        }
        // 32.5x: points stand in a hex or a box, with the module on.
        if !self.modules.abstract_supply
            && !(self.motorisation.is_empty() && self.pending_motorisation.is_empty())
        {
            bail!("motorisation points exist without the abstract supply module");
        }
        for key in self.motorisation.values().flat_map(|m| m.keys()) {
            let box_name = serde_json::from_value::<OffMapBox>(key.as_str().into()).is_ok();
            if !box_name {
                ids.hex("the motorisation", &HexId::parse(key)?)?;
            }
        }
        for id in self.pending_motorisation.keys() {
            ids.piece("the pending motorisation", id)?;
            if !self.pieces[id].is_pending() {
                bail!("the pending motorisation names {id}, which is not waiting to be placed");
            }
        }
        ids.pieces("moved", &self.moved)?;
        ids.pieces("continuing", &self.continuing)?;
        ids.pieces("shifted", &self.shifted)?;
        self.zoc.keys().try_for_each(|h| ids.hex("the zoc", h))?;
        let c = &self.combat;
        ids.pieces("forward", &c.forward)?;
        ids.pieces("probing", &c.probing)?;
        ids.pieces("pinned", &c.pinned)?;
        ids.pieces("retreated", &c.retreated)?;
        ids.pieces("barrage_involved", &c.barrage_involved)?;
        c.charged
            .keys()
            .try_for_each(|id| ids.piece("charged", id))?;
        c.barraged.keys().try_for_each(|h| ids.hex("barraged", h))?;
        c.assaults.keys().try_for_each(|h| ids.hex("assaults", h))?;
        for fire in c.plots.values().flatten() {
            ids.piece("a barrage plot", &fire.piece)?;
            ids.piece("a barrage plot", &fire.target)?;
        }
        for a in c.assignments.values().flatten() {
            ids.piece("an assignment", &a.piece)?;
            for al in a.anti_armour.iter().chain(&a.close_assault) {
                if let Some(h) = &al.target {
                    ids.hex("an assignment", h)?;
                }
            }
        }
        match &self.pending {
            None | Some(Pending::SetUp { .. }) | Some(Pending::RetreatBeforeAssault { .. }) => {}
            Some(Pending::Sealed { waiting, .. }) => {
                if waiting.is_empty() {
                    bail!("a sealed step is pending with nobody waiting to answer it");
                }
                if let Some(s) = waiting
                    .iter()
                    .enumerate()
                    .find_map(|(i, s)| waiting[..i].contains(s).then_some(s))
                {
                    bail!("a sealed step names {s} twice");
                }
            }
            Some(Pending::DeclareZoc { hexes, .. }) => {
                hexes.iter().try_for_each(|h| ids.hex("the pending", h))?
            }
            Some(Pending::Reaction { side, eligible }) => {
                for id in eligible {
                    ids.piece("the pending", id)?;
                    ids.of_side("the reaction", id, *side)?;
                }
            }
            Some(Pending::ForcedRetreat {
                side, pieces, from, ..
            }) => {
                for id in pieces {
                    ids.piece("the pending", id)?;
                    ids.of_side("the forced retreat", id, *side)?;
                }
                from.iter().try_for_each(|h| ids.hex("the pending", h))?;
            }
        }
        Ok(())
    }

    /// Whether `data` is this game's: the same scenario and the same
    /// `vendor/cna` commit.
    pub fn check(&self, data: &Data) -> Result<()> {
        check(&self.scenario, &self.rules_commit, data)
    }
}

/// What a state's ids may refer to: its pieces and the map's hexes.
struct Ids<'a> {
    state: &'a GameState,
    data: &'a Data,
}

impl Ids<'_> {
    fn piece(&self, what: &str, id: &PieceId) -> Result<()> {
        if self.state.pieces.contains_key(id) {
            Ok(())
        } else {
            bail!("{what} names a piece that does not exist: {id}")
        }
    }

    fn pieces<'p>(&self, what: &str, ids: impl IntoIterator<Item = &'p PieceId>) -> Result<()> {
        ids.into_iter().try_for_each(|id| self.piece(what, id))
    }

    fn hex(&self, what: &str, h: &HexId) -> Result<()> {
        if self.data.map.hex(h).is_some() {
            Ok(())
        } else {
            bail!("{what} names a hex that is not on the map: {h}")
        }
    }

    /// `id` is a piece of `side` (it exists: checked first).
    fn of_side(&self, what: &str, id: &PieceId, side: Side) -> Result<()> {
        if self.state.pieces[id].side == side {
            Ok(())
        } else {
            bail!("{what} of {side} names a piece of the other side: {id}")
        }
    }
}

fn check(scenario: &str, commit: &str, data: &Data) -> Result<()> {
    if scenario != data.slug() {
        bail!("the game is of {scenario}, the data of {}", data.slug());
    }
    if commit != data.rules_commit {
        bail!(
            "made on vendor/cna {commit}, but the data is from {}",
            data.rules_commit
        );
    }
    Ok(())
}

/// Rebuilds the state a log describes by applying its orders afresh.
pub fn replay(data: &Data, log: &Log) -> Result<GameState> {
    check(&log.scenario, &log.rules_commit, data)?;
    let mut state = GameState::with_modules(data, log.seed, log.modules)?;
    state.ruleset = log.ruleset;
    for (i, o) in log.orders.iter().enumerate() {
        state
            .apply(data, o.player, o.order.clone())
            .with_context(|| format!("order {}: {:?}", i + 1, o))?;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Clock, Half, Phase, Segment};
    use crate::combat::SealedStep;
    use crate::order::Order;
    use crate::piece::PieceId;
    use crate::state::{Pending, test_support};
    use cna_data::map::HexId;
    use cna_data::oa::UnitId;
    use cna_data::scenario::Side;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        cna_data::repo_root().join("tests/games/setup.json")
    }

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The scripted game `tests/games/setup.json` is: every set-up order,
    /// then GT1 OpStage 1 passed through by both players, the combat
    /// steps answered with nothing (six orders a half). Regenerate it
    /// with `cargo test -p cna-game -- --ignored regenerate_setup_fixture`
    /// after a change to the set-up or the orders.
    fn scripted_setup(data: &Data) -> GameState {
        let mut state = GameState::new(data, 7).unwrap();
        test_support::auto_place(data, &mut state);
        test_support::pass_stage(data, &mut state);
        state
    }

    #[test]
    #[ignore]
    fn regenerate_setup_fixture() {
        let data = graziani();
        let state = scripted_setup(&data);
        let json = serde_json::to_string_pretty(&state.to_log()).unwrap() + "\n";
        std::fs::write(fixture(), json).unwrap();
    }

    #[test]
    fn the_setup_game_replays_to_the_same_state() {
        let data = graziani();
        let text = std::fs::read_to_string(fixture()).unwrap();
        let log: Log = serde_json::from_str(&text).unwrap();
        assert_eq!(log.orders.len(), 59 + 1 + 2 * 8);
        let live = scripted_setup(&data);
        assert_eq!(live.to_log(), log, "the fixture is stale: regenerate it");
        let replayed = replay(&data, &log).unwrap();
        assert_eq!(replayed.to_json().unwrap(), live.to_json().unwrap());
        assert_eq!(
            replayed.clock,
            Clock {
                stage: 2,
                ..Clock::start(1)
            }
        );
        assert_eq!(replayed.clock.phase, Phase::InitiativeDeclaration);
        assert!(replayed.pieces.values().all(|p| !p.is_pending()));
        // Every replayed order is in the log; a rejected one never is.
        let mut copy = replayed.clone();
        assert!(copy.apply(&data, Side::Cw, Order::Pass).is_err());
        assert_eq!(copy.log, replayed.log);
    }

    #[test]
    fn a_saved_state_from_another_rules_commit_fails_to_load() {
        let data = graziani();
        let state = GameState::new(&data, 1).unwrap();
        let json = state.to_json().unwrap();
        assert_eq!(GameState::from_json(&json, &data).unwrap(), state);
        let mut other = graziani();
        other.rules_commit = "0".repeat(40);
        let err = GameState::from_json(&json, &other).unwrap_err().to_string();
        assert!(err.contains("vendor/cna"), "{err}");
        let log = state.to_log();
        assert!(replay(&other, &log).is_err());
        let mut log = state.to_log();
        log.scenario = "italian-campaign".into();
        assert!(replay(&data, &log).is_err());
    }

    /// A save whose pending is `sealed` with nobody waiting: the one that
    /// used to panic in `Pending::side` (so in `to_act`).
    fn malformed(data: &Data, edit: impl FnOnce(&mut GameState)) -> String {
        let mut state = GameState::new(data, 1).unwrap();
        test_support::auto_place(data, &mut state);
        edit(&mut state);
        state.to_json().unwrap()
    }

    // 32.1x: a real supply unit's load is within 40 ammunition and 60
    // fuel, a dummy (or any other piece) has none, and a game without the
    // module has no loads at all.
    #[test]
    fn a_supply_unit_with_a_hostile_load_is_refused() {
        use crate::piece::Load;
        let data = graziani();
        let on = |edit: &dyn Fn(&mut GameState)| {
            let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
            test_support::auto_place(&data, &mut s);
            edit(&mut s);
            s.to_json().unwrap()
        };
        let real = PieceId::supply(Side::Axis, 1);
        let dummy = PieceId::dummy_supply(Side::Axis, 1);
        assert!(GameState::from_json(&on(&|_| {}), &data).is_ok());
        let cases = [
            ("ammo", &real, Some(Load { ammo: 41, fuel: 0 })),
            ("fuel", &real, Some(Load { ammo: 0, fuel: 61 })),
            ("no load", &real, None),
            ("dummy", &dummy, Some(Load { ammo: 1, fuel: 1 })),
        ];
        for (what, id, load) in cases {
            let json = on(&|s| s.pieces.get_mut(id).unwrap().load = load);
            let err = GameState::from_json(&json, &data).unwrap_err().to_string();
            assert!(err.contains("load"), "{what}: {err}");
        }
        // Module off: a load is out of place.
        let mut s = GameState::new(&data, 1).unwrap();
        s.pieces.get_mut(&real).unwrap().load = Some(Load { ammo: 1, fuel: 1 });
        let err = GameState::from_json(&s.to_json().unwrap(), &data).unwrap_err();
        assert!(err.to_string().contains("load"), "{err}");
    }

    // 32.5x: motorisation points stand somewhere real, with the module on.
    #[test]
    fn hostile_motorisation_is_refused() {
        let data = graziani();
        let edit = |f: &dyn Fn(&mut GameState), modules| {
            let mut s = GameState::with_modules(&data, 1, modules).unwrap();
            f(&mut s);
            s.to_json().unwrap()
        };
        let axis = |s: &mut GameState, key: &str| {
            s.motorisation
                .entry(Side::Axis)
                .or_default()
                .insert(key.into(), 1);
        };
        assert!(GameState::from_json(&edit(&|s| axis(s, "tripoli"), Modules::ALL), &data).is_ok());
        for key in ["Z9999", "mars"] {
            let json = edit(&|s| axis(s, key), Modules::ALL);
            assert!(GameState::from_json(&json, &data).is_err(), "{key}");
        }
        let json = edit(&|s| axis(s, "tripoli"), Modules::default());
        assert!(GameState::from_json(&json, &data).is_err());
        let json = edit(
            &|s| {
                s.pending_motorisation
                    .insert(PieceId::supply(Side::Cw, 1), 1);
            },
            Modules::ALL,
        );
        assert!(GameState::from_json(&json, &data).is_err());
    }

    #[test]
    fn a_sealed_pending_with_nobody_waiting_is_refused_not_a_panic() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting: vec![],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("waiting"), "{err}");
    }

    #[test]
    fn a_sealed_pending_naming_a_side_twice_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting: vec![Side::Axis, Side::Axis],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("twice"), "{err}");
    }

    #[test]
    fn a_clock_with_an_opstage_outside_one_to_three_is_refused() {
        let data = graziani();
        for stage in [0, 4] {
            let json = malformed(&data, |s| s.clock.stage = stage);
            let err = GameState::from_json(&json, &data).unwrap_err().to_string();
            assert!(err.contains("OpStage"), "{err}");
        }
    }

    #[test]
    fn a_half_outside_phases_f_to_l_or_missing_inside_them_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| s.clock.half = Some(Half::A));
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("half"), "{err}");
        let json = malformed(&data, |s| {
            s.clock.phase = Phase::ReserveDesignation;
            s.clock.half = None;
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("half"), "{err}");
    }

    #[test]
    fn a_segment_outside_phase_g_or_missing_in_it_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| s.clock.segment = Some(Segment::Barrage));
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("segment"), "{err}");
        let json = malformed(&data, |s| {
            s.clock.phase = Phase::MovementCombat;
            s.clock.half = Some(Half::A);
            s.clock.segment = None;
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("segment"), "{err}");
    }

    #[test]
    fn a_pending_naming_a_piece_or_hex_that_does_not_exist_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Reaction {
                side: Side::Cw,
                eligible: vec![PieceId::from(UnitId::parse("unit:cw:nobody").unwrap())],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("unit:cw:nobody"), "{err}");
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::DeclareZoc {
                side: Side::Cw,
                hexes: vec![HexId::parse("C9999").unwrap()],
                at_start: false,
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("C9999"), "{err}");
    }

    /// A real save (the placed set-up) with one field corrupted by `edit`
    /// on its JSON; what `from_json` says, which must be an error.
    fn corrupted(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let data = graziani();
        let json = malformed(&data, |_| {});
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        edit(&mut v);
        GameState::from_json(&v.to_string(), &data)
            .expect_err("a corrupted save must be refused")
            .to_string()
    }

    fn first_piece(v: &serde_json::Value) -> String {
        v["pieces"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone()
    }

    const GHOST: &str = "unit:cw:nobody";
    const NOWHERE: &str = "C9999";

    #[test]
    fn a_piece_on_a_hex_off_the_map_is_refused() {
        let err = corrupted(|v| {
            let id = first_piece(v);
            v["pieces"][&id]["location"] = serde_json::json!({ "hex": NOWHERE });
        });
        assert!(err.contains(NOWHERE) && err.contains("location"), "{err}");
    }

    #[test]
    fn a_transit_that_goes_nowhere_or_has_not_started_is_refused() {
        for (from, to, done) in [("tunis", "tunis", 1), ("tunis", "gabes", 0)] {
            let err = corrupted(|v| {
                let id = first_piece(v);
                v["pieces"][&id]["location"] =
                    serde_json::json!({ "transit": { "from": from, "to": to, "done": done } });
            });
            assert!(err.contains("transit"), "{err}");
        }
    }

    #[test]
    fn a_piece_filed_under_another_pieces_id_is_refused() {
        let err = corrupted(|v| {
            let id = first_piece(v);
            let piece = v["pieces"][&id].clone();
            v["pieces"]
                .as_object_mut()
                .unwrap()
                .insert(GHOST.into(), piece);
        });
        assert!(err.contains(GHOST), "{err}");
    }

    #[test]
    fn a_garrison_of_a_hex_off_the_map_is_refused() {
        let err = corrupted(|v| {
            let id = first_piece(v);
            v["pieces"][&id]["garrison_of"] = NOWHERE.into();
        });
        assert!(err.contains(NOWHERE), "{err}");
    }

    #[test]
    fn a_piece_set_naming_a_ghost_is_refused() {
        for field in ["moved", "continuing", "shifted"] {
            let err = corrupted(|v| v[field] = serde_json::json!([GHOST]));
            assert!(err.contains(GHOST) && err.contains(field), "{field}: {err}");
        }
        for field in [
            "forward",
            "probing",
            "pinned",
            "retreated",
            "barrage_involved",
        ] {
            let err = corrupted(|v| v["combat"][field] = serde_json::json!([GHOST]));
            assert!(err.contains(GHOST) && err.contains(field), "{field}: {err}");
        }
    }

    #[test]
    fn a_charged_map_naming_a_ghost_is_refused() {
        let err = corrupted(|v| v["combat"]["charged"] = serde_json::json!({ GHOST: 4 }));
        assert!(err.contains(GHOST) && err.contains("charged"), "{err}");
    }

    #[test]
    fn a_hex_map_naming_a_hex_off_the_map_is_refused() {
        let err = corrupted(|v| v["zoc"] = serde_json::json!({ NOWHERE: true }));
        assert!(err.contains(NOWHERE) && err.contains("zoc"), "{err}");
        let err = corrupted(|v| v["combat"]["barraged"] = serde_json::json!({ NOWHERE: 3 }));
        assert!(err.contains(NOWHERE) && err.contains("barraged"), "{err}");
        let record = serde_json::json!({ "probe": false, "basic": 0, "column": "1:1" });
        let err = corrupted(|v| v["combat"]["assaults"] = serde_json::json!({ NOWHERE: record }));
        assert!(err.contains(NOWHERE) && err.contains("assaults"), "{err}");
    }

    #[test]
    fn a_barrage_plot_naming_a_ghost_firer_or_target_is_refused() {
        for (firer, target) in [(GHOST.to_string(), None), (String::new(), Some(GHOST))] {
            let err = corrupted(|v| {
                let id = first_piece(v);
                let fire = serde_json::json!({
                    "piece": if firer.is_empty() { id.clone() } else { firer.clone() },
                    "points": 1,
                    "target": target.unwrap_or(&id),
                });
                v["combat"]["plots"] = serde_json::json!({ "axis": [fire] });
            });
            assert!(err.contains(GHOST) && err.contains("plot"), "{err}");
        }
    }

    #[test]
    fn an_assignment_naming_a_ghost_piece_or_a_missing_hex_is_refused() {
        let err = corrupted(|v| {
            v["combat"]["assignments"] = serde_json::json!({ "axis": [{ "piece": GHOST }] });
        });
        assert!(err.contains(GHOST) && err.contains("assignment"), "{err}");
        for key in ["anti_armour", "close_assault"] {
            let err = corrupted(|v| {
                let id = first_piece(v);
                v["combat"]["assignments"] = serde_json::json!({
                    "axis": [{ "piece": id, key: [{ "target": NOWHERE, "points": 1 }] }]
                });
            });
            assert!(
                err.contains(NOWHERE) && err.contains("assignment"),
                "{key}: {err}"
            );
        }
    }

    #[test]
    fn a_sealed_pending_naming_a_side_twice_is_refused_in_any_position() {
        // (Two sides only exist, so three waiting must repeat one.)
        let err = corrupted(|v| {
            v["pending"] = serde_json::json!(
                { "sealed": { "step": "barrage", "waiting": ["axis", "cw", "axis"] } }
            );
        });
        assert!(err.contains("twice"), "{err}");
    }

    #[test]
    fn a_reaction_with_a_piece_of_the_other_side_is_refused() {
        let err = corrupted(|v| {
            let (id, side) = foreign_piece(v);
            v["pending"] = serde_json::json!({ "reaction": { "side": side, "eligible": [id] } });
        });
        assert!(err.contains("reaction") && err.contains("side"), "{err}");
    }

    #[test]
    fn a_forced_retreat_with_a_piece_of_the_other_side_is_refused() {
        let err = corrupted(|v| {
            let (id, side) = foreign_piece(v);
            v["pending"] = serde_json::json!({ "forced-retreat": {
                "side": side, "pieces": [id], "hexes": 1, "from": [],
                "reason": "assault-result"
            } });
        });
        assert!(err.contains("retreat") && err.contains("side"), "{err}");
    }

    /// A piece id, and the side that is not its owner's.
    fn foreign_piece(v: &serde_json::Value) -> (String, &'static str) {
        let id = first_piece(v);
        let other = if v["pieces"][&id]["side"] == "axis" {
            "cw"
        } else {
            "axis"
        };
        (id, other)
    }

    #[test]
    fn every_state_along_the_golden_game_loads() {
        let data = graziani();
        let text =
            std::fs::read_to_string(cna_data::repo_root().join("tests/games/graziani-short.json"))
                .unwrap();
        let log: Log = serde_json::from_str(&text).unwrap();
        let mut state = GameState::new(&data, log.seed).unwrap();
        state.ruleset = log.ruleset;
        for (i, o) in log.orders.iter().enumerate() {
            state.apply(&data, o.player, o.order.clone()).unwrap();
            if i % 10 == 0 || i + 1 == log.orders.len() {
                let json = state.to_json().unwrap();
                let back = GameState::from_json(&json, &data)
                    .unwrap_or_else(|e| panic!("after order {}: {e}", i + 1));
                assert_eq!(back, state);
            }
        }
    }

    #[test]
    fn a_well_formed_save_in_every_phase_loads() {
        let data = graziani();
        let mut state = GameState::new(&data, 7).unwrap();
        test_support::auto_place(&data, &mut state);
        test_support::pass_stage(&data, &mut state);
        let json = state.to_json().unwrap();
        assert_eq!(GameState::from_json(&json, &data).unwrap(), state);
    }

    #[test]
    fn the_log_round_trips_through_json() {
        let data = graziani();
        let mut state = GameState::new(&data, 9).unwrap();
        test_support::auto_place(&data, &mut state);
        let log = state.to_log();
        let json = serde_json::to_string(&log).unwrap();
        let back: Log = serde_json::from_str(&json).unwrap();
        assert_eq!(back, log);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["seed"], 9);
        assert_eq!(v["orders"][0]["player"], "axis");
        assert!(v["orders"][0]["order"]["place"].is_object());
    }
}
