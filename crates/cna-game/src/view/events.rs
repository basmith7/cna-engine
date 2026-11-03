//! A batch of events as one side may see it (3.61, limited
//! intelligence). Every `Event` variant is one of:
//!
//! - *public*: both sides see it as it is;
//! - *own*: only the side it concerns sees it; the enemy gets nothing in
//!   its place (the board change it made still shows in the state view);
//! - *reduced*: the enemy gets it without the fields listed.
//!
//! In every case an enemy supply id is shown only as its alias
//! (`Aliases::alias_json`), and an id list carrying one (`pieces`,
//! `attackers`, …) is sorted after aliasing, so its order (an arrival
//! lists real units before dummies) gives nothing away. A spectator
//! (`viewer` none) is everyone's enemy. With open information
//! (`open`) every event passes as it is.
//!
//! The grounds are cna's restatement of 3.61–3.62
//! (`rules/95-abstract-logistics-and-air.md`, *Limited intelligence*:
//! "nothing about an enemy counter's status, composition or attributes";
//! "in land combat only totals are revealed, never the units behind
//! them"; close assault reveals unit types, "not numbers or which weapons
//! were lost"; POW, guard, construction-item, truck and replacement
//! numbers secret), the combat restatement (`rules/60-combat.md`:
//! allocations written down secretly; a barrage names its target and
//! "state[s] the actual points against it"; each side totals its actual
//! close-assault points), and what the hot-seat view already hides
//! (loads, motorisation, repaired points, minefield reality, project
//! items).
//!
//! Who an event concerns comes from its own `side` field, or from the
//! side of the piece it names (`state.pieces`, where an eliminated piece
//! stays; else the side in a supply id). An event whose side cannot be
//! told is treated as the enemy's: an *own* one is dropped, a *reduced*
//! one reduced.
//!
//! | Variant | Class | Concerns | The enemy loses / why |
//! |---|---|---|---|
//! | `phase-started` | public | | |
//! | `phase-skipped` | public | | |
//! | `die-rolled` | public, but *own* for `mines at <hex> for <piece>` | that piece | the mine roll tells a 1-point TOE loss of a named piece (status, 3.61); other rolls are made in the open |
//! | `initiative-determined` | public | | |
//! | `rommel-recalled` | public | | |
//! | `initiative-declared` | public | | |
//! | `weather-set` | public | | |
//! | `placed` | public | | a counter on the map; any stack may be examined |
//! | `unplaced` | public | | the counter leaves the map |
//! | `set-up-complete` | public | | |
//! | `arrived` | public | | counters appear (ids aliased, list sorted) |
//! | `moved-off-map` | public | | a counter's location |
//! | `moved` | reduced | `pieces[0]` | `cost`: CP spent makes `cp_spent` derivable (status) |
//! | `disorganised` | own | `piece` | DP and cohesion level are status; even the bare fact is (stricter than the spec, which kept `piece`) |
//! | `move-interrupted` | public | | the non-phasing player must answer it |
//! | `zoc-declared` | public | | |
//! | `reaction-offered` | public | | the non-phasing player must answer it |
//! | `contact-broken` | reduced | `piece` | `cost` (CP, status) |
//! | `reserve-designated` | public | | a board marker |
//! | `reserve-released` | public | | a board marker |
//! | `reserve-flipped` | public | | a board marker |
//! | `sealed` | public | | that an answer is in, not what |
//! | `gun-positions-set` | public | | `forward` is a marker once revealed |
//! | `barrage-plotted` | reduced | `side` | `fires`: which unit fires how many points is an allocation, "never the units behind them"; the targets and totals follow in `barrage` |
//! | `barrage` | reduced | `side` | `firers` ("never the units behind them"); `points` kept: 12.0 states the actual points against the target (the spec listed `points`; the rule settles it the other way) |
//! | `retreated-before-assault` | public | | counters move |
//! | `anti-armour` | reduced | `side` | `points`, `shift`: 3.61 reveals the types that fired, not numbers; the damage the target takes is kept |
//! | `assault` | reduced | the side of `attackers[0]` (else the phasing side) attacks, the other defends | the enemy side's `attacker_raw` / `defender_raw` (raw points are finer than the totals 15.2 announces) and its roll's `raw_lost` and `toe_lost` ("not numbers or which weapons were lost"); actual totals (`basic`), column, readings, percentages and results kept |
//! | `prisoners` | reduced | the captor, `side.other()` | `points` (POW numbers secret) |
//! | `surrendered` | public | | a board fact |
//! | `engaged` | public | | a board marker |
//! | `retreat-ordered` | public | | the retreat is seen; its reason follows from the public assault result |
//! | `retreat-bought-out` | public | | how far short the stack stood is seen; what it paid is in `losses` (own) |
//! | `losses` | own | `piece` | per-piece TOE lost and left: the units behind the totals (stricter than the spec, which kept `points`) |
//! | `fuel-spent` | own | `from` | loads |
//! | `ammunition-spent` | own | `from` | loads |
//! | `ammunition-short` | own | `piece` | loads, status |
//! | `supply-unit-spent` | own | `piece` | tells the unit was real and empty; the counter's removal shows in the state |
//! | `supply-destroyed` | own | `piece` | as above; the roll (`die-rolled`) is made in the open |
//! | `broke-down` | own | `piece` | breakdown marker contents secret |
//! | `wrecks-captured` | public | | the captor learns the contents by taking them; the loser knew them |
//! | `field-repaired` | own | the side whose `field_rolled` holds `(side, hex, weapon)` | replacement points secret |
//! | `air-lifted` | reduced | `piece` | `fuel_from`: which unit paid tells it is real and loaded |
//! | `railed` | public | | counters move (ids aliased, list sorted) |
//! | `attached` | own | `piece` | `parent` is removed from the enemy's view of a piece |
//! | `detached` | own | `piece` | as above |
//! | `motorisation-attached` | own | `piece` | motorisation is the owner's |
//! | `convoy-bombed` | reduced | Axis | `units`, `sunk`: how many real supply units sailed (truck and supply numbers secret; it gives the dummies away) |
//! | `supply-planned` | own | `side` | the real units to come |
//! | `motorisation-lost` | own | `side` | motorisation is the owner's |
//! | `motorisation-detached` | own | `piece` | motorisation is the owner's |
//! | `load-transferred` | own | `from` | loads |
//! | `eliminated` | public | | a board fact |
//! | `forces-assigned` | reduced | `side` | `assignments`: per-piece role allocations, written down secretly; the types that took part show in `assault` |
//! | `combat-cp` | own | `piece` | CP, status |
//! | `combat-cp-refunded` | own | `piece` | CP and DP, status |
//! | `captured` | public | | a board fact |
//! | `prisoners-taken` | reduced | `side` | `points` (POW numbers secret) |
//! | `prisoners-marched` | reduced | `side` | `points`, `cp` |
//! | `prisoners-left-map` | reduced | `side` | `points` |
//! | `prisoners-gone` | reduced | `side` | `points` |
//! | `construction-begun` | reduced | `side` | `item` (construction item secret) |
//! | `construction-lost` | reduced | `side` | `item` |
//! | `fortification-built` | public | | the finished level is on the board |
//! | `minefield-laid` | public | | carries no reality |
//! | `minefield-revealed` | public | | turned face up |
//! | `minefield-removed` | public | | |
//! | `guards-detailed` | reduced | `piece` | `points` (guard numbers secret) |
//! | `guards-released` | reduced | `piece` | `points` |
//! | `prisoners-escaped` | public | | both sides know the numbers: the captor lost them, their side gets them back |
//! | `prisoners-freed` | public | | as above |
//! | `absorbed` | own | `piece` | replacement points secret |
//! | `game-over` | public | | |

use super::Aliases;
use crate::event::Event;
use crate::piece::PieceId;
use crate::state::GameState;
use cna_data::scenario::Side;
use serde_json::Value;

/// What a viewer who is not the side concerned gets.
enum Seen {
    Public,
    /// Dropped for anyone but this side.
    Own(Option<Side>),
    /// These body fields removed for anyone but this side.
    Reduced(Option<Side>, &'static [&'static str]),
    /// A close assault, with the attacking side.
    Assault(Option<Side>),
}

/// Fields of an id list that may carry an enemy supply id.
const ID_LISTS: [&str; 5] = ["pieces", "attackers", "firers", "builders", "forward"];

/// Each of `events` as `viewer` may see it (3.61), in order; an *own*
/// event of the other side is left out, and so is any event naming one of
/// its hidden pieces (27.31), even with open information.
pub fn events_for(
    state: &GameState,
    aliases: &Aliases,
    events: &[Event],
    viewer: Option<Side>,
    open: bool,
) -> Vec<Value> {
    let hidden = super::state::hidden_from(state, viewer);
    events
        .iter()
        .filter_map(|event| {
            let mut v = serde_json::to_value(event).expect("an event serialises");
            if super::state::names_any(&v, &hidden) {
                return None;
            }
            if open {
                return Some(v);
            }
            let theirs = |owner: Option<Side>| owner.is_none() || owner != viewer;
            let body = v
                .as_object_mut()
                .and_then(|m| m.values_mut().next())
                .and_then(Value::as_object_mut);
            match (seen(state, event), body) {
                (Seen::Public, _) => {}
                (Seen::Own(owner), _) => {
                    if theirs(owner) {
                        return None;
                    }
                }
                (Seen::Reduced(owner, fields), Some(body)) => {
                    if theirs(owner) {
                        for f in fields {
                            body.remove(*f);
                        }
                    }
                }
                (Seen::Assault(attacker), Some(body)) => {
                    let defender = attacker.map(Side::other);
                    for (side, raw, roll) in [
                        (attacker, "attacker_raw", "attacker"),
                        (defender, "defender_raw", "defender"),
                    ] {
                        if theirs(side) {
                            body.remove(raw);
                            if let Some(r) = body.get_mut(roll).and_then(Value::as_object_mut) {
                                r.remove("raw_lost");
                                r.remove("toe_lost");
                            }
                        }
                    }
                }
                (Seen::Reduced(..) | Seen::Assault(_), None) => {
                    unreachable!("a reduced event has fields")
                }
            }
            if let Some(body) = v
                .as_object_mut()
                .and_then(|m| m.values_mut().next())
                .and_then(Value::as_object_mut)
            {
                for key in ID_LISTS {
                    if let Some(Value::Array(ids)) = body.get_mut(key) {
                        let aliased = |id: &Value| match id {
                            Value::String(s) => aliases.alias_text(viewer, s) != *s,
                            _ => false,
                        };
                        if ids.iter().any(aliased) {
                            ids.iter_mut().for_each(|id| aliases.alias_json(viewer, id));
                            ids.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                        }
                    }
                }
            }
            aliases.alias_json(viewer, &mut v);
            Some(v)
        })
        .collect()
}

/// The side `id` belongs to: from the state, else from a supply id.
fn side_of(state: &GameState, id: &PieceId) -> Option<Side> {
    if let Some(p) = state.pieces.get(id) {
        return Some(p.side);
    }
    [Side::Axis, Side::Cw]
        .into_iter()
        .find(|s| id.as_str().starts_with(&format!("supply:{s}:")))
}

fn seen(state: &GameState, event: &Event) -> Seen {
    use Event as E;
    use Seen::{Assault, Own, Public, Reduced};
    let of = |id: &PieceId| side_of(state, id);
    match event {
        E::PhaseStarted { .. }
        | E::PhaseSkipped { .. }
        | E::InitiativeDetermined { .. }
        | E::RommelRecalled
        | E::RaiderSpotted { .. }
        | E::RommelRaided { .. }
        | E::InitiativeDeclared { .. }
        | E::WeatherSet { .. }
        | E::Placed { .. }
        | E::Unplaced { .. }
        | E::SetUpComplete
        | E::Arrived { .. }
        | E::MovedOffMap { .. }
        | E::MoveInterrupted { .. }
        | E::ZocDeclared { .. }
        | E::ReactionOffered { .. }
        | E::ReserveDesignated { .. }
        | E::ReserveReleased { .. }
        | E::ReserveFlipped { .. }
        | E::Sealed { .. }
        | E::GunPositionsSet { .. }
        | E::RetreatedBeforeAssault { .. }
        | E::Surrendered { .. }
        | E::Engaged { .. }
        | E::RetreatOrdered { .. }
        | E::RetreatBoughtOut { .. }
        | E::WrecksCaptured { .. }
        | E::Railed { .. }
        | E::Eliminated { .. }
        | E::Captured { .. }
        | E::FortificationBuilt { .. }
        | E::MinefieldLaid { .. }
        | E::MinefieldRevealed { .. }
        | E::MinefieldRemoved { .. }
        | E::PrisonersEscaped { .. }
        | E::PrisonersFreed { .. }
        | E::BizertaOpened
        | E::GameOver { .. } => Public,
        E::DieRolled { purpose, .. } => match purpose
            .strip_prefix("mines at ")
            .and_then(|rest| rest.split_once(" for "))
        {
            Some((_, piece)) => Own(of(&PieceId::from(piece.to_owned()))),
            None => Public,
        },
        E::Disorganised { piece, .. }
        | E::Losses { piece, .. }
        | E::AmmunitionShort { piece }
        | E::SupplyUnitSpent { piece }
        | E::SupplyDestroyed { piece }
        | E::BrokeDown { piece, .. }
        | E::Attached { piece, .. }
        | E::Detached { piece, .. }
        | E::MotorisationAttached { piece, .. }
        | E::MotorisationDetached { piece, .. }
        | E::CombatCp { piece, .. }
        | E::CombatCpRefunded { piece, .. }
        | E::Absorbed { piece, .. } => Own(of(piece)),
        E::FuelSpent { from, .. }
        | E::AmmunitionSpent { from, .. }
        | E::LoadTransferred { from, .. } => Own(of(from)),
        E::SupplyPlanned { side, .. } | E::MotorisationLost { side, .. } => Own(Some(*side)),
        // R-144: the Commonwealth learns only the lanes it bombs.
        E::ConvoyLanesSet { .. } | E::ConvoyCancelled { .. } => Own(Some(Side::Axis)),
        E::FieldRepaired { hex, weapon, .. } => {
            let mut rolled = [Side::Axis, Side::Cw].into_iter().filter(|s| {
                state
                    .field_rolled
                    .contains(&(*s, hex.clone(), weapon.clone()))
            });
            match (rolled.next(), rolled.next()) {
                (Some(s), None) => Own(Some(s)),
                _ => Own(None),
            }
        }
        E::Moved { pieces, .. } => Reduced(pieces.first().and_then(of), &["cost"]),
        E::ContactBroken { piece, .. } => Reduced(of(piece), &["cost"]),
        E::BarragePlotted { side, .. } => Reduced(Some(*side), &["fires"]),
        E::Barrage { side, .. } => Reduced(Some(*side), &["firers"]),
        // 3.61: the target side learns the types that fired, not numbers.
        E::AntiArmour { side, .. } => Reduced(Some(*side), &["points", "shift"]),
        E::ForcesAssigned { side, .. } => Reduced(Some(*side), &["assignments"]),
        E::Assault { attackers, .. } => {
            Assault(attackers.first().and_then(of).or_else(|| state.phasing()))
        }
        E::Prisoners { side, .. } => Reduced(Some(side.other()), &["points"]),
        E::PrisonersTaken { side, .. }
        | E::PrisonersLeftMap { side, .. }
        | E::PrisonersGone { side, .. } => Reduced(Some(*side), &["points"]),
        E::PrisonersMarched { side, .. } => Reduced(Some(*side), &["points", "cp"]),
        E::ConstructionBegun { side, .. } | E::ConstructionLost { side, .. } => {
            Reduced(Some(*side), &["item"])
        }
        E::GuardsDetailed { piece, .. } | E::GuardsReleased { piece, .. } => {
            Reduced(of(piece), &["points"])
        }
        E::AirLifted { piece, .. } => Reduced(of(piece), &["fuel_from"]),
        E::ConvoyBombed { .. } => Reduced(Some(Side::Axis), &["units", "sunk"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::RetreatReason;
    use crate::construction::Item;
    use crate::data::Data;
    use crate::event::AssaultRoll;
    use crate::log::Log;
    use crate::order::{Allocation, Assignment, Fire};
    use crate::piece::{Location, PieceKind};
    use crate::state::Modules;
    use cna_data::cp::Cp;
    use cna_data::map::HexId;
    use cna_rules::barrage::BarrageResult;
    use serde_json::json;
    use std::sync::OnceLock;

    const KEY: [u8; 32] = [9; 32];

    fn data() -> &'static Data {
        static DATA: OnceLock<Data> = OnceLock::new();
        DATA.get_or_init(|| Data::load("grazianis-offensive").unwrap())
    }

    fn state() -> GameState {
        GameState::with_modules(data(), 7, Modules::ALL).unwrap()
    }

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    /// The first piece of `side` whose kind is `kind`.
    fn piece(s: &GameState, side: Side, kind: PieceKind) -> PieceId {
        s.pieces
            .values()
            .find(|p| p.side == side && p.kind == kind)
            .unwrap()
            .id
            .clone()
    }

    struct Ids {
        unit: PieceId,
        supply: PieceId,
        dummy: PieceId,
        enemy_unit: PieceId,
    }

    fn ids(s: &GameState) -> Ids {
        Ids {
            unit: piece(s, Side::Axis, PieceKind::Combat),
            supply: piece(s, Side::Axis, PieceKind::SupplyUnit),
            dummy: piece(s, Side::Axis, PieceKind::DummySupplyUnit),
            enemy_unit: piece(s, Side::Cw, PieceKind::Combat),
        }
    }

    fn whole(e: &Event) -> Value {
        serde_json::to_value(e).unwrap()
    }

    fn seen_by(s: &GameState, e: &Event, viewer: Option<Side>) -> Vec<Value> {
        events_for(
            s,
            &Aliases::new(KEY),
            std::slice::from_ref(e),
            viewer,
            false,
        )
    }

    /// `v`'s body without `fields`, each of which must be there.
    fn without(mut v: Value, fields: &[&str]) -> Value {
        let body = v.as_object_mut().unwrap().values_mut().next().unwrap();
        for f in fields {
            assert!(body.as_object_mut().unwrap().remove(*f).is_some(), "{f}");
        }
        v
    }

    /// `v` without its assault roll `side`'s loss numbers.
    fn without_losses(mut v: Value, side: &str) -> Value {
        let r = v["assault"][side].as_object_mut().unwrap();
        assert!(r.remove("raw_lost").is_some() && r.remove("toe_lost").is_some());
        v
    }

    /// Every string and key inside `v`.
    fn strings(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(m) => m.iter().for_each(|(k, x)| {
                out.push(k.clone());
                strings(x, out)
            }),
            _ => {}
        }
    }

    /// Whether `v` names a real or dummy supply id of `side` unaliased.
    fn names_supply_of(v: &Value, side: Side) -> bool {
        let mut all = Vec::new();
        strings(v, &mut all);
        let prefix = format!("supply:{side}:");
        all.iter().any(|s| {
            s.match_indices(prefix.as_str()).any(|(i, _)| {
                let tail = &s[i + prefix.len()..];
                let tail = tail.strip_prefix("dummy-").unwrap_or(tail);
                tail.starts_with(|c: char| c.is_ascii_digit())
            })
        })
    }

    /// The golden game with every module on, order by order: each
    /// batch of events with the state just after it.
    fn golden() -> &'static Vec<(GameState, Vec<Event>)> {
        static GOLDEN: OnceLock<Vec<(GameState, Vec<Event>)>> = OnceLock::new();
        GOLDEN.get_or_init(|| {
            let path = cna_data::repo_root().join("tests/games/graziani-m5.json");
            let log: Log = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            let mut s = GameState::with_modules(data(), log.seed, log.modules).unwrap();
            s.ruleset = log.ruleset;
            log.orders
                .iter()
                .map(|o| {
                    let events = s.apply(data(), o.player, o.order.clone()).unwrap();
                    (s.clone(), events)
                })
                .collect()
        })
    }

    #[test]
    fn own_events_reach_their_side_alone() {
        let s = state();
        let Ids {
            unit,
            supply,
            enemy_unit,
            ..
        } = ids(&s);
        let own = [
            Event::DieRolled {
                purpose: format!("mines at C0101 for {unit}"),
                value: 5,
            },
            Event::Disorganised {
                piece: unit.clone(),
                dp: 3,
                level: -2,
            },
            Event::Losses {
                piece: unit.clone(),
                points: 2,
                toe: 4,
            },
            Event::FuelSpent {
                piece: unit.clone(),
                from: supply.clone(),
                fuel: 3,
            },
            Event::AmmunitionSpent {
                piece: unit.clone(),
                from: supply.clone(),
                ammo: 2,
            },
            Event::AmmunitionShort {
                piece: unit.clone(),
            },
            Event::SupplyUnitSpent {
                piece: supply.clone(),
            },
            Event::SupplyDestroyed {
                piece: supply.clone(),
            },
            Event::BrokeDown {
                piece: unit.clone(),
                weapon: "m13-40".into(),
                reading: 31,
                percent: 10,
                points: 1,
            },
            Event::Attached {
                piece: unit.clone(),
                to: unit.clone(),
            },
            Event::Detached {
                piece: unit.clone(),
                from: unit.clone(),
            },
            Event::MotorisationAttached {
                piece: supply.clone(),
                points: 4,
            },
            Event::SupplyPlanned {
                side: Side::Axis,
                die: 4,
                units: 3,
                arrives: 5,
            },
            Event::MotorisationLost {
                side: Side::Axis,
                percent: 3,
                of: 100,
                points: 3,
            },
            Event::MotorisationDetached {
                piece: supply.clone(),
                points: 4,
            },
            Event::LoadTransferred {
                from: supply.clone(),
                to: supply.clone(),
                ammo: 1,
                fuel: 1,
            },
            Event::CombatCp {
                piece: unit.clone(),
                cost: Cp::whole(3),
            },
            Event::CombatCpRefunded {
                piece: unit.clone(),
                cp: Cp::whole(3),
                dp: 1,
            },
            Event::Absorbed {
                piece: unit.clone(),
                weapon: "infantry".into(),
                points: 2,
                cp: Cp::whole(1),
            },
        ];
        for e in &own {
            assert_eq!(seen_by(&s, e, Some(Side::Axis)), vec![whole(e)], "{e:?}");
            assert!(seen_by(&s, e, Some(Side::Cw)).is_empty(), "{e:?}");
            assert!(seen_by(&s, e, None).is_empty(), "{e:?}");
        }
        // The mine roll is the side's whose piece it names.
        let theirs = Event::DieRolled {
            purpose: format!("mines at C0101 for {enemy_unit}"),
            value: 5,
        };
        assert!(seen_by(&s, &theirs, Some(Side::Axis)).is_empty());
        assert_eq!(seen_by(&s, &theirs, Some(Side::Cw)), vec![whole(&theirs)]);
        // Any other roll is made in the open.
        let open = Event::DieRolled {
            purpose: "weather".into(),
            value: 2,
        };
        assert_eq!(seen_by(&s, &open, None), vec![whole(&open)]);
    }

    #[test]
    fn a_field_repair_is_its_roller_s_and_an_untold_one_nobody_s() {
        let mut s = state();
        let e = Event::FieldRepaired {
            hex: hex("C0101"),
            weapon: "m13-40".into(),
            attempted: 2,
            die: 3,
            repaired: 1,
            fuel_from: None,
        };
        assert!(seen_by(&s, &e, Some(Side::Axis)).is_empty(), "untold");
        assert!(seen_by(&s, &e, Some(Side::Cw)).is_empty(), "untold");
        s.field_rolled
            .insert((Side::Axis, hex("C0101"), "m13-40".into()));
        assert_eq!(seen_by(&s, &e, Some(Side::Axis)), vec![whole(&e)]);
        assert!(seen_by(&s, &e, Some(Side::Cw)).is_empty());
        assert!(seen_by(&s, &e, None).is_empty());
    }

    #[test]
    fn reduced_events_lose_their_listed_fields_for_the_enemy_only() {
        let s = state();
        let Ids {
            unit,
            supply,
            enemy_unit,
            ..
        } = ids(&s);
        let a = Aliases::new(KEY);
        let lift = Event::AirLifted {
            piece: supply.clone(),
            from: Location::Hex(hex("C0101")),
            to: Location::Hex(hex("C0102")),
            fuel_from: supply.clone(),
        };
        let reduced: Vec<(Event, &[&str])> = vec![
            (
                Event::AntiArmour {
                    side: Side::Axis,
                    target: hex("C0101"),
                    points: 7,
                    shift: 0,
                    reading: 52,
                    damage: 4,
                },
                &["points", "shift"],
            ),
            (
                Event::Moved {
                    pieces: vec![unit.clone()],
                    from: hex("C0101"),
                    to: hex("C0102"),
                    cost: Cp::whole(2),
                },
                &["cost"],
            ),
            (
                Event::ContactBroken {
                    piece: unit.clone(),
                    cost: Cp::whole(2),
                },
                &["cost"],
            ),
            (
                Event::BarragePlotted {
                    side: Side::Axis,
                    fires: vec![Fire {
                        piece: unit.clone(),
                        points: 3,
                        target: enemy_unit.clone(),
                    }],
                },
                &["fires"],
            ),
            (
                Event::Barrage {
                    side: Side::Axis,
                    firers: vec![unit.clone()],
                    target: enemy_unit.clone(),
                    points: 6,
                    shift: -1,
                    reading: 42,
                    result: BarrageResult::Lose(1),
                },
                &["firers"],
            ),
            (
                Event::ForcesAssigned {
                    side: Side::Axis,
                    assignments: vec![Assignment {
                        piece: unit.clone(),
                        anti_armour: vec![],
                        close_assault: vec![Allocation {
                            target: Some(hex("C0102")),
                            points: 2,
                        }],
                    }],
                },
                &["assignments"],
            ),
            (
                // The Commonwealth's men, so the Axis holds them.
                Event::Prisoners {
                    side: Side::Cw,
                    points: 2,
                },
                &["points"],
            ),
            (
                Event::PrisonersTaken {
                    side: Side::Axis,
                    hex: hex("C0101"),
                    points: 2,
                },
                &["points"],
            ),
            (
                Event::PrisonersMarched {
                    side: Side::Axis,
                    path: vec![hex("C0101"), hex("C0102")],
                    points: 2,
                    cp: Cp::whole(1),
                },
                &["points", "cp"],
            ),
            (
                Event::PrisonersLeftMap {
                    side: Side::Axis,
                    points: 2,
                    stages: 3,
                },
                &["points"],
            ),
            (
                Event::PrisonersGone {
                    side: Side::Axis,
                    hex: hex("C0101"),
                    points: 2,
                },
                &["points"],
            ),
            (
                Event::ConstructionBegun {
                    side: Side::Axis,
                    item: Item::DummyMinefield,
                    hex: hex("C0101"),
                    builders: vec![unit.clone()],
                },
                &["item"],
            ),
            (
                Event::ConstructionLost {
                    side: Side::Axis,
                    item: Item::RealMinefield,
                    hex: hex("C0101"),
                },
                &["item"],
            ),
            (
                Event::GuardsDetailed {
                    piece: unit.clone(),
                    hex: hex("C0101"),
                    points: 1,
                },
                &["points"],
            ),
            (
                Event::GuardsReleased {
                    piece: unit.clone(),
                    hex: hex("C0101"),
                    points: 1,
                },
                &["points"],
            ),
            (lift.clone(), &["fuel_from"]),
            (
                Event::ConvoyBombed {
                    lane: 1,
                    bomb_points: 40,
                    reading: 33,
                    percent: 20,
                    units: 3,
                    sunk: 1,
                },
                &["units", "sunk"],
            ),
        ];
        for (e, fields) in &reduced {
            assert_eq!(seen_by(&s, e, Some(Side::Axis)), vec![whole(e)], "{e:?}");
            for viewer in [Some(Side::Cw), None] {
                let mut cut = without(whole(e), fields);
                a.alias_json(viewer, &mut cut);
                assert_eq!(seen_by(&s, e, viewer), vec![cut], "{e:?} for {viewer:?}");
            }
        }
        // The lifted supply unit reaches the enemy as its alias.
        assert_eq!(
            seen_by(&s, &lift, Some(Side::Cw))[0]["air-lifted"]["piece"],
            json!(a.alias(Some(Side::Cw), &supply).as_str())
        );
    }

    #[test]
    fn an_assault_hides_each_side_s_raw_points_and_losses_from_the_other() {
        let s = state();
        let roll = |reading| AssaultRoll {
            reading,
            pct: 20,
            raw_lost: 30,
            toe_lost: 2,
            engaged: true,
            retreat: 0,
            captured: false,
        };
        let e = Event::Assault {
            target: hex("C0102"),
            attackers: vec![ids(&s).unit],
            probe: false,
            attacker_raw: 61,
            defender_raw: 40,
            basic: 2,
            shifts: -1,
            column: "+1".into(),
            attacker: roll(33),
            defender: roll(44),
        };
        let axis = without_losses(without(whole(&e), &["defender_raw"]), "defender");
        let cw = without_losses(without(whole(&e), &["attacker_raw"]), "attacker");
        let neither = without_losses(without(cw.clone(), &["defender_raw"]), "defender");
        assert_eq!(seen_by(&s, &e, Some(Side::Axis)), vec![axis]);
        assert_eq!(seen_by(&s, &e, Some(Side::Cw)), vec![cw]);
        assert_eq!(seen_by(&s, &e, None), vec![neither]);
    }

    #[test]
    fn a_hidden_piece_s_moves_reach_its_own_side_alone_27_31() {
        let mut s = state();
        let raider = ids(&s).enemy_unit;
        s.pieces.get_mut(&raider).unwrap().hidden = true;
        let e = Event::Moved {
            pieces: vec![raider.clone()],
            from: hex("C4807"),
            to: hex("C4808"),
            cost: Cp::whole(3),
        };
        for open in [false, true] {
            let seen = |viewer| {
                events_for(
                    &s,
                    &Aliases::new(KEY),
                    std::slice::from_ref(&e),
                    viewer,
                    open,
                )
            };
            assert!(seen(Some(Side::Axis)).is_empty(), "open {open}");
            assert!(seen(None).is_empty(), "open {open}");
            assert_eq!(seen(Some(Side::Cw)), vec![whole(&e)], "open {open}");
        }
    }

    #[test]
    fn public_events_pass_unchanged_but_for_aliases() {
        let s = state();
        let Ids {
            unit,
            supply,
            dummy,
            enemy_unit,
        } = ids(&s);
        let a = Aliases::new(KEY);
        let placed = Event::Placed {
            piece: dummy.clone(),
            hex: hex("C0103"),
        };
        let sample = [
            Event::RommelRecalled,
            Event::SetUpComplete,
            placed.clone(),
            Event::Eliminated {
                piece: unit.clone(),
            },
            Event::Captured {
                piece: supply.clone(),
                hex: hex("C0101"),
            },
            Event::Surrendered {
                pieces: vec![enemy_unit.clone()],
            },
            Event::RetreatOrdered {
                pieces: vec![unit.clone()],
                hexes: 2,
                reason: RetreatReason::AssaultResult,
            },
            Event::MinefieldLaid {
                side: Side::Axis,
                hex: hex("C0101"),
            },
        ];
        for e in &sample {
            for viewer in [Some(Side::Axis), Some(Side::Cw), None] {
                let mut v = whole(e);
                a.alias_json(viewer, &mut v);
                assert_eq!(seen_by(&s, e, viewer), vec![v], "{e:?} for {viewer:?}");
            }
        }
        assert_eq!(
            seen_by(&s, &placed, Some(Side::Cw))[0]["placed"]["piece"],
            json!(a.alias(Some(Side::Cw), &dummy).as_str())
        );
        // From the golden game: every public event naming no supply unit
        // passes as it is to either side and a spectator.
        let mut checked = 0;
        for (s, events) in golden() {
            for e in events.iter().filter(|e| matches!(seen(s, e), Seen::Public)) {
                let v = whole(e);
                if names_supply_of(&v, Side::Axis) || names_supply_of(&v, Side::Cw) {
                    continue;
                }
                for viewer in [Some(Side::Axis), Some(Side::Cw), None] {
                    assert_eq!(seen_by(s, e, viewer), vec![v.clone()], "{e:?}");
                }
                checked += 1;
            }
        }
        assert!(checked > 100, "{checked}");
    }

    #[test]
    fn an_id_list_with_enemy_supply_is_sorted_after_aliasing() {
        let s = state();
        let a = Aliases::new(KEY);
        // As a convoy lands them: the real units first, then a dummy.
        let pieces: Vec<PieceId> = s
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit)
            .take(3)
            .map(|p| p.id.clone())
            .chain([ids(&s).dummy])
            .collect();
        let e = Event::Arrived {
            pieces: pieces.clone(),
            at: Location::Hex(hex("C0101")),
        };
        assert_eq!(seen_by(&s, &e, Some(Side::Axis)), vec![whole(&e)]);
        let mut aliased: Vec<String> = pieces
            .iter()
            .map(|p| a.alias(Some(Side::Cw), p).as_str().to_owned())
            .collect();
        aliased.sort();
        assert_eq!(
            seen_by(&s, &e, Some(Side::Cw))[0]["arrived"]["pieces"],
            json!(aliased)
        );
    }

    #[test]
    fn no_enemy_supply_id_reaches_a_viewer_in_the_golden_game() {
        let a = Aliases::new(KEY);
        let mut own_seen = [false; 2];
        let mut enemy_named = [false; 2];
        for (s, events) in golden() {
            let whole = Value::Array(events.iter().map(whole).collect());
            for (i, viewer) in [Side::Axis, Side::Cw].into_iter().enumerate() {
                let seen = Value::Array(events_for(s, &a, events, Some(viewer), false));
                assert!(!names_supply_of(&seen, viewer.other()), "{seen}");
                own_seen[i] |= names_supply_of(&seen, viewer);
                enemy_named[i] |= names_supply_of(&whole, viewer.other());
            }
            let spectator = Value::Array(events_for(s, &a, events, None, false));
            assert!(!names_supply_of(&spectator, Side::Axis), "{spectator}");
            assert!(!names_supply_of(&spectator, Side::Cw), "{spectator}");
        }
        assert_eq!(own_seen, [true; 2], "own supply ids pass as they are");
        assert_eq!(enemy_named, [true; 2], "the game names both sides' supply");
    }

    #[test]
    fn open_information_passes_every_event_as_it_is() {
        let a = Aliases::new(KEY);
        for (s, events) in golden() {
            let whole: Vec<Value> = events.iter().map(whole).collect();
            for viewer in [Some(Side::Axis), Some(Side::Cw), None] {
                assert_eq!(events_for(s, &a, events, viewer, true), whole);
            }
        }
    }
}
