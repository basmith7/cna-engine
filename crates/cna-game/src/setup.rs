//! Set-up: a scenario's deployments expanded into pieces (59.2).
//!
//! A unit entry stands for the named counter plus what its OA sheet
//! assigns to it (its `parent` subtree), adjusted by the entry's
//! indicators: `less` and `detached` remove a unit with its subtree,
//! `attached` and `assigned` add one, `consists_of` replaces the subtree,
//! `alone` is the counter only. A formation entry is every counter on that
//! sheet. The named unit always starts (R-096), but a subtree member whose
//! sheet gives an arrival after the scenario's start is left to the
//! reinforcement schedule, as a scheduled parent brings its sheet (4.42).
//!
//! The Land-Game-only abstraction is applied first (60.92), so the supply
//! blocks seen here are supply units, which become inert pieces.

use crate::piece::{Load, Location, Piece, PieceId, PieceKind};
use crate::state::Modules;
use anyhow::{Context, Result, bail};
use cna_data::characteristics::{Characteristics, Equivalent, Rating, Row};
use cna_data::cp::Cp;
use cna_data::map::Map;
use cna_data::oa::{Arrival, Nation, Oa, ToeMark, Unit, UnitId};
use cna_data::scenario::{
    Locator, Placement, Scenario, Side, Subject, SupplyKind, TruckClass, UnitEntry,
};
use cna_data::time::GameTime;
use cna_data::weapons::{WeaponClass, WeaponSystems};
use std::collections::BTreeSet;

/// A time no arrival is after: subtree with no arrival filter.
const NEVER: GameTime = GameTime::new(u8::MAX, 3);

/// The pieces of a set-up, with what the expansion could not check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub pieces: Vec<Piece>,
    /// Placement `constraint` sentences the grammar cannot check (59.2);
    /// the owner is trusted to honour them.
    pub unchecked: Vec<Unchecked>,
    /// Motorisation points where each deployment's medium trucks start
    /// (32.5x, 60.92); empty with the module off.
    pub motorisation: Vec<Motorisation>,
}

/// A deployment's motorisation points. Where the deployment is still
/// pending, they follow `anchor`, its first piece, to the hex it lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motorisation {
    pub side: Side,
    pub location: Location,
    pub points: u32,
    pub anchor: Option<PieceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unchecked {
    pub side: Side,
    pub constraint: String,
    pub pieces: Vec<PieceId>,
}

/// Expands every deployment and supply block of the scenario, as loaded,
/// into pieces. Fails on a hex or place not on the map, a unit not in the
/// OA, an indicator naming a unit outside the entry, or a unit placed
/// twice (R-094).
pub fn expand(
    scenario: &Scenario,
    oa: &Oa,
    chars: &Characteristics,
    map: &Map,
    modules: Modules,
) -> Result<Setup> {
    let scenario = scenario.land_only();
    // 32.5, 59.63: which truck classes give motorisation points, as the
    // scenario's abstraction types them; none named, none given.
    let motorising: Vec<TruckClass> = scenario
        .abstractions
        .as_ref()
        .and_then(|a| a.air_and_logistics.as_ref())
        .and_then(|a| a.motorisation.as_ref())
        .map(|m| m.from.clone())
        .unwrap_or_default();
    let mut setup = Setup {
        pieces: Vec::new(),
        unchecked: Vec::new(),
        motorisation: Vec::new(),
    };
    let mut placed = BTreeSet::new();
    for side in [Side::Axis, Side::Cw] {
        let set = scenario.side(side);
        // A unit an entry names as `attached` or `assigned` is that
        // entry's, with its subtree, even where another entry sets its
        // sheet formation up whole (61.38's reassignments).
        let mut named = BTreeSet::new();
        for e in set.deployments.iter().flat_map(|d| &d.units) {
            for a in e.attached.iter().chain(&e.assigned) {
                let mut sub = Vec::new();
                subtree(a, oa, NEVER, &mut sub)?;
                named.extend(sub.into_iter().map(|m| m.id));
            }
        }
        for (di, d) in set.deployments.iter().enumerate() {
            let location = resolve(&d.placement, map)
                .with_context(|| format!("{side} deployment {}", di + 1))?;
            let mut ids = Vec::new();
            for e in &d.units {
                let members = members(e, oa, scenario.start, &named)
                    .with_context(|| format!("{side} deployment {}", di + 1))?;
                for m in members {
                    let unit = oa.unit(&m.id).expect("resolved by members");
                    if !placed.insert(m.id.clone()) {
                        bail!("{}: placed twice (R-094)", m.id);
                    }
                    let piece = piece(side, unit, m.parent, e, chars, oa, location.clone());
                    ids.push(piece.id.clone());
                    setup.pieces.push(piece);
                }
            }
            // Each truck point of a motorising class is one point.
            let points: u32 = d.trucks.map_or(0, |t| {
                motorising
                    .iter()
                    .map(|c| match c {
                        TruckClass::Light => t.light,
                        TruckClass::Medium => t.medium,
                        TruckClass::Heavy => t.heavy,
                    })
                    .sum()
            });
            if modules.abstract_supply && points > 0 {
                setup.motorisation.push(Motorisation {
                    side,
                    location: location.clone(),
                    points,
                    anchor: ids.first().cloned(),
                });
            }
            log_constraint(&mut setup, side, &d.placement, ids);
        }
        let mut n = 0;
        let mut dummies = 0;
        for (bi, b) in set.supply.iter().enumerate() {
            let kind = match b.kind {
                Some(SupplyKind::SupplyUnit) => PieceKind::SupplyUnit,
                Some(SupplyKind::DummySupplyUnit) => PieceKind::DummySupplyUnit,
                _ => continue,
            };
            let location = resolve(&b.placement, map)
                .with_context(|| format!("{side} supply block {}", bi + 1))?;
            // The figures are each supply unit's own unless the block is
            // pooled (scenario schema, `supplyBlock.count` and `pooled`,
            // cna f33ac72). A pool is the owner's to share out, which
            // set-up has no order for yet.
            if b.pooled && kind == PieceKind::SupplyUnit && modules.abstract_supply {
                bail!(
                    "{side} supply block {}: pooled supply-unit loads are not supported",
                    bi + 1
                );
            }
            let mut ids = Vec::new();
            for _ in 0..b.count {
                let id = if kind == PieceKind::SupplyUnit {
                    n += 1;
                    PieceId::supply(side, n)
                } else {
                    dummies += 1;
                    PieceId::dummy_supply(side, dummies)
                };
                ids.push(id.clone());
                let load =
                    (modules.abstract_supply && kind == PieceKind::SupplyUnit).then(|| Load {
                        ammo: b.ammo.unwrap_or(0),
                        fuel: b.fuel.unwrap_or(0),
                    });
                setup
                    .pieces
                    .push(supply_piece(side, kind, id, location.clone(), load));
            }
            log_constraint(&mut setup, side, &b.placement, ids);
        }
    }
    Ok(setup)
}

/// A supply unit or dummy, fresh: no CP spent, nothing attached.
pub(crate) fn supply_piece(
    side: Side,
    kind: PieceKind,
    id: PieceId,
    location: Location,
    load: Option<Load>,
) -> Piece {
    Piece {
        id,
        side,
        kind,
        nation: None,
        id_code: None,
        parent: None,
        attached_to: None,
        toe: 0,
        weapons: Vec::new(),
        cpa: None,
        morale: None,
        status: None,
        equivalent: None,
        garrison_of: None,
        group: None,
        location,
        load,
        cp_spent: Cp::ZERO,
        cp_enemy_half: Cp::ZERO,
        cohesion: Default::default(),
        contact: false,
        engaged: false,
        reserve: None,
        released: None,
        released_assaulted: false,
        landed: false,
        fuelled: false,
        fuelled_over: false,
        motorised: 0,
        escorted: false,
        breakdown_halves: 0,
        breakdown_checked: None,
        flown: false,
    }
}

/// A unit the set-up brings, and what it is under.
struct Member {
    id: UnitId,
    parent: Option<UnitId>,
}

/// The sheet subtree under `head`, less members whose own arrival is
/// after `start`. `head` itself is always included (R-096).
fn subtree(head: &UnitId, oa: &Oa, start: GameTime, out: &mut Vec<Member>) -> Result<()> {
    let unit = oa
        .unit(head)
        .with_context(|| format!("{head}: not on any OA sheet"))?;
    out.push(Member {
        id: head.clone(),
        parent: unit.parent.clone(),
    });
    let mut stack: Vec<&UnitId> = oa.children(head).iter().rev().collect();
    while let Some(id) = stack.pop() {
        let u = oa.unit(id).expect("children are units");
        if let Some(Arrival::At(t)) = u.arrives
            && t > start
        {
            continue;
        }
        out.push(Member {
            id: id.clone(),
            parent: u.parent.clone(),
        });
        stack.extend(oa.children(id).iter().rev());
    }
    Ok(())
}

/// Every unit one set-up entry stands for. Units in `named` (some
/// entry's `attached` or `assigned`) come only through those lists.
fn members(
    e: &UnitEntry,
    oa: &Oa,
    start: GameTime,
    named: &BTreeSet<UnitId>,
) -> Result<Vec<Member>> {
    let mut out = Vec::new();
    let head = match &e.subject {
        Subject::Formation(f) => {
            let formation = oa
                .formation(f)
                .with_context(|| format!("{f}: no such OA sheet"))?;
            for u in &formation.units {
                if let Some(Arrival::At(t)) = u.arrives
                    && t > start
                {
                    continue;
                }
                out.push(Member {
                    id: u.id.clone(),
                    parent: u.parent.clone(),
                });
            }
            None
        }
        Subject::Unit(u) => {
            if e.alone {
                let unit = oa
                    .unit(u)
                    .with_context(|| format!("{u}: not on any OA sheet"))?;
                out.push(Member {
                    id: u.clone(),
                    parent: unit.parent.clone(),
                });
            } else if let Some(list) = &e.consists_of {
                subtree(u, oa, start, &mut out)?;
                out.truncate(1);
                for x in list {
                    oa.unit(x)
                        .with_context(|| format!("{x}: not on any OA sheet"))?;
                    out.push(Member {
                        id: x.clone(),
                        parent: Some(u.clone()),
                    });
                }
            } else {
                subtree(u, oa, start, &mut out)?;
            }
            Some(u.clone())
        }
    };
    // What the entry could have held, arrivals aside: an indicator must
    // name something in it, or the data is wrong.
    let mut full = Vec::new();
    match &e.subject {
        Subject::Unit(u) => subtree(u, oa, NEVER, &mut full)?,
        Subject::Formation(f) => {
            for u in &oa.formation(f).expect("checked above").units {
                full.push(Member {
                    id: u.id.clone(),
                    parent: None,
                });
            }
        }
    }
    for r in e.less.iter().chain(&e.detached) {
        if !full.iter().any(|m| &m.id == r) {
            bail!(
                "{}: `{r}` is not in the entry",
                head.as_ref()
                    .map_or_else(|| "formation".into(), ToString::to_string)
            );
        }
        let mut gone = Vec::new();
        subtree(r, oa, NEVER, &mut gone)?;
        out.retain(|m| !gone.iter().any(|g| g.id == m.id));
    }
    out.retain(|m| Some(&m.id) == head.as_ref() || !named.contains(&m.id));
    for a in e.attached.iter().chain(&e.assigned) {
        let mut added = Vec::new();
        subtree(a, oa, start, &mut added)?;
        added[0].parent = head.clone();
        for m in added {
            if !out.iter().any(|x| x.id == m.id) {
                out.push(m);
            }
        }
    }
    Ok(out)
}

fn piece(
    side: Side,
    unit: &Unit,
    parent: Option<UnitId>,
    entry: &UnitEntry,
    chars: &Characteristics,
    oa: &Oa,
    location: Location,
) -> Piece {
    let row = unit
        .id_code
        .as_deref()
        .and_then(|code| chars.row(unit.id.nation(), code));
    let is_head = matches!(&entry.subject, Subject::Unit(u) if u == &unit.id);
    let (toe, weapons) = match (&entry.toe, is_head) {
        (Some(toe), true) => (toe.iter().map(|t| t.points).sum(), toe.clone()),
        _ => (toe_points(unit, row), unit.toe.clone()),
    };
    let equivalent = row.and_then(|r| match r.equivalent {
        Equivalent::ByUnit => unit.equivalent,
        e => Some(e),
    });
    Piece {
        id: PieceId::from(&unit.id),
        side,
        kind: if unit.commander.is_some() {
            PieceKind::Commander
        } else if equivalent == Some(Equivalent::Hq) {
            PieceKind::Hq
        } else {
            PieceKind::Combat
        },
        nation: Some(unit.id.nation()),
        id_code: unit.id_code.clone(),
        parent,
        attached_to: None,
        toe,
        weapons,
        cpa: match &unit.commander {
            Some(c) => Some(Rating {
                value: i32::try_from(c.cpa).ok(),
                mark: None,
                raw: c.cpa.to_string(),
            }),
            None => row.and_then(|r| r.cpa.clone()),
        },
        morale: oa.morale(&unit.id),
        status: entry.status,
        equivalent,
        garrison_of: oa.formation_of(&unit.id).garrison_hex(),
        group: None,
        location,
        load: None,
        cp_spent: Cp::ZERO,
        cp_enemy_half: Cp::ZERO,
        cohesion: Default::default(),
        contact: false,
        engaged: false,
        reserve: None,
        released: None,
        released_assaulted: false,
        landed: false,
        fuelled: false,
        fuelled_over: false,
        motorised: 0,
        escorted: false,
        breakdown_halves: 0,
        breakdown_checked: None,
        flown: false,
    }
}

/// The pieces a scheduled arrival brings (20.1x, 4.42): the named unit
/// and its sheet subtree, less members with a later arrival of their own.
pub fn reinforcement(
    side: Side,
    head: &UnitId,
    oa: &Oa,
    chars: &Characteristics,
    at: GameTime,
    location: Location,
) -> Result<Vec<Piece>> {
    let mut members = Vec::new();
    subtree(head, oa, at, &mut members)?;
    let entry = UnitEntry {
        subject: Subject::Unit(head.clone()),
        less: Vec::new(),
        assigned: Vec::new(),
        detached: Vec::new(),
        attached: Vec::new(),
        consists_of: None,
        alone: false,
        status: None,
        toe: None,
        notes: Vec::new(),
    };
    Ok(members
        .into_iter()
        .filter_map(|m| {
            let unit = oa.unit(&m.id)?;
            if unit.id_code.is_none() && unit.toe.is_empty() {
                return None;
            }
            Some(piece(
                side,
                unit,
                m.parent,
                &entry,
                chars,
                oa,
                location.clone(),
            ))
        })
        .collect())
}

/// TOE points as the sheet prints them: `N` is the row's `max_toe` plus
/// any points listed with it; `U@x` and `O@x` are `x`; else the sum of
/// the listed weapons.
pub(crate) fn toe_points(unit: &Unit, row: Option<&Row>) -> u32 {
    let listed: u32 = unit.toe.iter().map(|t| t.points).sum();
    match unit.toe_mark {
        Some(ToeMark::Full) => {
            let max = row
                .and_then(|r| r.max_toe.as_ref())
                .and_then(|m| m.value)
                .unwrap_or(0);
            u32::try_from(max).unwrap_or(0) + listed
        }
        Some(ToeMark::Under(x)) | Some(ToeMark::Over(x)) => x,
        None => listed,
    }
}

/// A single hex or a place is a location; a box is a box; anything the
/// owner must choose among is pending.
fn resolve(p: &Placement, map: &Map) -> Result<Location> {
    for l in &p.locators {
        match l {
            Locator::Hexes(h) => {
                for hex in h {
                    if map.hex(hex).is_none() {
                        bail!("{hex}: not on the map");
                    }
                }
            }
            Locator::Place(ids) => {
                for id in ids {
                    map.place(id)
                        .with_context(|| format!("{id}: not in places.json"))?;
                }
            }
            Locator::Within { of, .. } | Locator::WestOf(of) => {
                if map.hex(of).is_none() {
                    bail!("{of}: not on the map");
                }
            }
            Locator::Sheets(s) => {
                for sheet in s {
                    if map.sheet(*sheet).is_none() {
                        bail!("sheet {sheet}: not on the map");
                    }
                }
            }
            Locator::Box(_)
            | Locator::Region(_)
            | Locator::AirFacility
            | Locator::StackedWith(_) => {}
        }
    }
    match p.sole() {
        Some(Locator::Hexes(h)) if h.len() == 1 => Ok(Location::Hex(h[0].clone())),
        // A choice of places (Cairo or Helwan) still starts at the first:
        // offering the others would add set-up orders and move every
        // golden game's indices. `log_constraint` logs the alternatives.
        Some(Locator::Place(ids)) => Ok(Location::Hex(
            map.place(&ids[0]).expect("checked").hex.clone(),
        )),
        Some(Locator::Box(b)) => Ok(Location::Box(*b)),
        _ => Ok(Location::Pending(p.clone())),
    }
}

/// 32.8x, the Land Game without the Air Game: AA is thinned at set-up.
/// The Commonwealth loses every AA unit (one whose TOE is all AA) and
/// every AA point; the Germans their *z* and *bb* units and light AA
/// points; the Italians their *ss*, *tt* and *uu* units, the AA points of
/// *pp* units and the heavy AA points of *kk* units. A unit stripped to
/// no TOE goes too. Returns the pieces removed.
pub fn thin_anti_air(setup: &mut Setup, weapons: &WeaponSystems) -> Result<Vec<PieceId>> {
    let mut removed = Vec::new();
    setup.pieces.retain_mut(|p| {
        let Some(nation) = p.nation else {
            return true;
        };
        let code = p.id_code.as_deref().unwrap_or_default();
        let aa = |name: &str| {
            weapons
                .resolve(nation, name)
                .filter(|r| r.class == WeaponClass::AntiAir)
                .map(|r| r.weapon.trim_start_matches("'I' ").starts_with("Heavy"))
        };
        let all_aa = !p.weapons.is_empty() && p.weapons.iter().all(|w| aa(&w.weapon).is_some());
        let unit_goes = match nation {
            Nation::Cw => all_aa,
            Nation::De => matches!(code, "z" | "bb"),
            Nation::It => matches!(code, "ss" | "tt" | "uu"),
        };
        let strips = |heavy: bool| match nation {
            Nation::Cw => true,
            Nation::De => !heavy,
            Nation::It => code == "pp" || (code == "kk" && heavy),
        };
        let before = p.toe;
        if !unit_goes {
            let toe = &mut p.toe;
            p.weapons.retain(|w| match aa(&w.weapon) {
                Some(heavy) if strips(heavy) => {
                    *toe -= w.points;
                    false
                }
                _ => true,
            });
        }
        let keep = !unit_goes && (p.toe > 0 || before == 0);
        if !keep {
            removed.push(p.id.clone());
        }
        keep
    });
    for u in &mut setup.unchecked {
        u.pieces.retain(|id| !removed.contains(id));
    }
    for m in &setup.motorisation {
        if let Some(anchor) = &m.anchor
            && removed.contains(anchor)
        {
            bail!("{anchor}: removed by 32.8x but anchors motorisation points");
        }
    }
    Ok(removed)
}

fn log_constraint(setup: &mut Setup, side: Side, p: &Placement, pieces: Vec<PieceId>) {
    if p.together && pieces.len() > 1 {
        let g = setup
            .pieces
            .iter()
            .filter_map(|q| q.group)
            .max()
            .map_or(1, |g| g + 1);
        for q in setup.pieces.iter_mut().filter(|q| pieces.contains(&q.id)) {
            q.group = Some(g);
        }
    }
    if let Some(Locator::Place(ids)) = p.sole()
        && ids.len() > 1
    {
        setup.unchecked.push(Unchecked {
            side,
            constraint: format!("or {}", ids[1..].join(" or ")),
            pieces: pieces.clone(),
        });
    }
    if let Some(c) = &p.constraint {
        setup.unchecked.push(Unchecked {
            side,
            constraint: c.clone(),
            pieces,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::{Location, PieceId, PieceKind};
    use cna_data::characteristics::{Characteristics, Equivalent};
    use cna_data::map::{HexId, Map};
    use cna_data::oa::{Oa, UnitId};
    use cna_data::scenario::{Locator, OffMapBox, Scenario, Side, Status};
    use cna_data::schedule::Schedule;
    use std::collections::BTreeSet;

    struct Fixture {
        scenario: Scenario,
        oa: Oa,
        chars: Characteristics,
        map: Map,
        setup: Setup,
    }

    fn graziani() -> Fixture {
        let scenario = Scenario::load("grazianis-offensive").unwrap();
        let oa = Oa::load_all().unwrap();
        let chars = Characteristics::load().unwrap();
        let map = Map::load().unwrap();
        let setup = expand(&scenario, &oa, &chars, &map, Modules::default()).unwrap();
        Fixture {
            scenario,
            oa,
            chars,
            map,
            setup,
        }
    }

    fn uid(s: &str) -> UnitId {
        UnitId::parse(s).unwrap()
    }

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    impl Fixture {
        fn piece(&self, id: &str) -> &Piece {
            let id = PieceId::from(uid(id));
            self.setup
                .pieces
                .iter()
                .find(|p| p.id == id)
                .unwrap_or_else(|| panic!("no piece {id}"))
        }

        fn has(&self, id: &str) -> bool {
            let id = PieceId::from(uid(id));
            self.setup.pieces.iter().any(|p| p.id == id)
        }

        fn side_pieces(&self, side: Side, kind: PieceKind) -> usize {
            self.setup
                .pieces
                .iter()
                .filter(|p| p.side == side && p.kind == kind)
                .count()
        }
    }

    // Pinned by hand from the OA sheets and the set-up. Per entry, the
    // named unit plus its sheet subtree, less `less`/`detached` and units
    // arriving after 1/1, plus `attached`:
    //
    // Axis, 26 deployments: 1st CCNN 13+1 = 14; 63rd Cirene 14+1 = 15
    // (HQ, two regt HQs each with three bns, MG bn, AT coy, arty regt,
    // eng bn, bers coy = 14, plus LXIII(L) attached); 1st Libyan 11; 2nd
    // Libyan 10+1 = 11; Aresca 1st Regt 5-3 = 2; 62nd Marmarica 13+1 =
    // 14; Maletti 14+1 = 15; 64th Catanzaro 13; 4th CCNN 12; Trivioli
    // 2nd Regt 6-2 = 4; Bardia garrison sheet 11; Libyan Tank Command HQ
    // alone 1; Tobruk garrison sheet 23-1 = 22 (the San Giorgio is `less`); Benghazi sheet 4; Libyan Para Regt 3; Derna sheet
    // 4; Mechili bn 1; Giarabub sheet 6; Scheferzen, Maddalena, EGUG,
    // Benina, Soluch AT coy and arty regt, X Corps arty: 7; "anywhere in Libya" 6;
    // Saharan Detachment 7; sheets A/B 11; Tripoli box: 2nd CCNN 13 +
    // 3rd CCNN sheet 7 + two corps arty 2 = 22; Tripolitania 1.
    // Sum: 14+15+11+11+2+14+15+13+12+4+11+1+22+4+3+4+1+6+7+6+7+11+22+1
    // = 217.
    //
    // Commonwealth, 16 deployments: singles at C4131 (2), C3926 (1),
    // C3922 (4), C3721 (2), C3520, C3320, C3020 (3) = 12; Selby Force
    // 1+3 attached = 4; 7th Armd HQ: 1 + 4th Armd Bde 3 + 7th Armd Bde 3
    // + 7 Spt 6 - 6 detached - 7 Mtr HQ (arrives 68/2) = 7; 4 Ind HQ: 1 +
    // 5 Ind 4 + 11 Ind 4 + 1 CIH + 25 Fld = 11 (7 Ind HQ and 31 Fld
    // detached, 1 Fld arrives 3/3); 16th Inf Bde 5-1 (AT coy arrives
    // 50/1) = 4; Alexandria: 2 NZ HQ alone 1 + 4 NZ HQ 4 + 27 NZ + 2 NZ
    // Cav + 4 NZ Fld = 8; 6 Aus HQ: 1 + 16 Aus 4 + 17 Aus 4 + 6 Aus Cav
    // = 10 (19 Aus and both fld regts `less`); Cairo 4; sheets D/E 6; the 1st Buffs and 1st
    // Hampshires, assigned to the Matruh garrison (4.44b), 2. The 7th Motor
    // HQ and 1st Fld are `less`; the arrival filter already dropped them.
    // Sum: 12+4+7+11+4+8+10+4+6+2 = 68.
    fn load(slug: &str) -> Result<Setup> {
        let scenario = Scenario::load(slug).unwrap();
        let oa = Oa::load_all().unwrap();
        let chars = Characteristics::load().unwrap();
        let map = Map::load().unwrap();
        expand(&scenario, &oa, &chars, &map, Modules::default())
    }

    #[test]
    fn a_unit_assigned_elsewhere_leaves_its_sheet_formation() {
        // Rommel's Arrival sets the 7th Armoured Division up whole, while
        // an entry at A2022 assigns its 6 RTR to the 3rd Armoured Brigade
        // (61.38). The named assignment wins: one piece, under the 3rd.
        let setup = load("rommels-arrival").unwrap();
        let rtr: Vec<&Piece> = setup
            .pieces
            .iter()
            .filter(|p| p.id == PieceId::from(uid("unit:cw:6-rtr")))
            .collect();
        assert_eq!(rtr.len(), 1);
        assert_eq!(rtr[0].parent, Some(uid("unit:cw:3-armd-bde-hq")));
        assert_eq!(rtr[0].location, Location::Hex(hex("A2022")));
    }

    #[test]
    fn piece_counts_are_pinned() {
        let f = graziani();
        let oa_pieces =
            |side| f.side_pieces(side, PieceKind::Combat) + f.side_pieces(side, PieceKind::Hq);
        assert_eq!(oa_pieces(Side::Axis), 217);
        assert_eq!(oa_pieces(Side::Cw), 68);
        // 60.92: 3+2+1+1+8+3+3+1 supply units and 3+3 dummies for the Axis;
        // 5+1+3 and 3 for the Commonwealth, plus the 3+3 `adds`.
        assert_eq!(f.side_pieces(Side::Axis, PieceKind::SupplyUnit), 22);
        assert_eq!(f.side_pieces(Side::Axis, PieceKind::DummySupplyUnit), 6);
        assert_eq!(f.side_pieces(Side::Cw, PieceKind::SupplyUnit), 15);
        assert_eq!(f.side_pieces(Side::Cw, PieceKind::DummySupplyUnit), 3);
        assert_eq!(f.setup.pieces.len(), 217 + 68 + 22 + 6 + 15 + 3);
    }

    #[test]
    fn two_formations_by_hand() {
        let f = graziani();
        let under = |head: &str| -> BTreeSet<String> {
            let hex = f.piece(head).hex().unwrap().clone();
            f.setup
                .pieces
                .iter()
                .filter(|p| p.hex() == Some(&hex))
                .map(|p| p.id.to_string())
                .collect()
        };
        assert_eq!(under("unit:it:63rd-cirene-div-hq").len(), 15);
        assert_eq!(under("unit:cw:4-in-hq").len(), 11);
    }

    #[test]
    fn in_training_and_less() {
        let f = graziani();
        let aus = f.piece("unit:cw:6-aus-hq");
        assert_eq!(aus.status, Some(Status::InTraining));
        assert_eq!(f.piece("unit:cw:2-3-aus").status, Some(Status::InTraining));
        for gone in [
            "unit:cw:19-aus-hq",
            "unit:cw:2-4-aus",
            "unit:cw:2-1-afld",
            "unit:cw:2-2-afld",
        ] {
            assert!(!f.has(gone), "{gone} should be `less`");
        }
        assert_eq!(aus.location, Location::Hex(hex("E1730")));
    }

    #[test]
    fn detached_units_are_not_under_their_parent_entry() {
        let f = graziani();
        assert_eq!(
            f.piece("unit:cw:7th-armored-div-hq").location,
            Location::Hex(hex("D3612"))
        );
        assert_eq!(
            f.piece("unit:cw:1-krrc").location,
            Location::Hex(hex("C3922"))
        );
        assert_eq!(
            f.piece("unit:cw:1-krrc").parent,
            Some(uid("unit:cw:7-spt-hq"))
        );
        // Detached and not placed elsewhere: it arrives with the schedule.
        assert!(!f.has("unit:cw:7-in-hq"));
        assert!(!f.has("unit:cw:1-rssx"));
        // The Libyan tank battalions are detached from their regiments.
        assert_eq!(
            f.piece("unit:it:aresca-1st-regt-hq").location,
            Location::Hex(hex("C3919"))
        );
        assert_ne!(
            f.piece("unit:it:im-tank-bn").location,
            Location::Hex(hex("C3919"))
        );
    }

    #[test]
    fn attached_units_are_under_the_entry() {
        let f = graziani();
        let bn = f.piece("unit:it:im-tank-bn");
        assert_eq!(bn.location, Location::Hex(hex("C4218")));
        assert_eq!(bn.parent, Some(uid("unit:it:1st-ccnn-div-hq")));
        let essex = f.piece("unit:cw:1-essex");
        assert_eq!(essex.location, Location::Hex(hex("D3714")));
        assert_eq!(
            essex.parent,
            Some(uid("unit:cw:selby-force-the-matruh-garrison"))
        );
    }

    #[test]
    fn alone_places_only_the_hq() {
        let f = graziani();
        assert_eq!(
            f.piece("unit:it:libyan-tank-command-hq").location,
            Location::Hex(hex("C4807"))
        );
        assert_eq!(f.piece("unit:cw:2-nz-hq").parent, None);
        assert!(f.piece("unit:cw:2-nz-hq").is_pending());
    }

    #[test]
    fn single_hexes_and_places_resolve() {
        let f = graziani();
        for side in [Side::Axis, Side::Cw] {
            for d in &f.scenario.side(side).deployments {
                let expect = match d.placement.sole() {
                    Some(Locator::Hexes(h)) if h.len() == 1 => Location::Hex(h[0].clone()),
                    Some(Locator::Place(p)) if p.len() == 1 => {
                        Location::Hex(f.map.place(&p[0]).unwrap().hex.clone())
                    }
                    _ => continue,
                };
                for e in &d.units {
                    if let cna_data::scenario::Subject::Unit(u) = &e.subject {
                        assert_eq!(f.piece(u.as_str()).location, expect, "{u}");
                    }
                }
            }
        }
        assert_eq!(
            f.piece("unit:it:30th-gaf-s-di-c-hq").location,
            Location::Hex(hex("C4321"))
        );
        assert_eq!(
            f.piece("unit:it:4th-fanterie-mtr-regt").location,
            Location::Box(OffMapBox::Tripolitania)
        );
    }

    #[test]
    fn free_placements_are_pending() {
        let f = graziani();
        for id in [
            "unit:it:xxi-corps-artillery-regt",
            "unit:it:saharan-detachment-hq",
            "unit:it:xviii-libyan-bn",
            "unit:cw:8-med",
            "unit:cw:4-nz-hq",
        ] {
            assert!(f.piece(id).is_pending(), "{id}");
        }
        let Location::Pending(p) = &f.piece("unit:it:saharan-detachment-hq").location else {
            panic!()
        };
        assert_eq!(
            p.locators[0],
            Locator::Within {
                of: hex("C0716"),
                hexes: 3
            }
        );
        let pending = f.setup.pieces.iter().filter(|p| p.is_pending()).count();
        // 6 Libya + 7 Saharan + 11 sheets A/B + 6 D/E + 8 Alexandria, and
        // the 3+3+3+3 Axis and 3+3+3 Commonwealth supply units on a sheet
        // or at Alexandria.
        assert_eq!(pending, 6 + 7 + 11 + 6 + 8 + 12 + 9);
    }

    #[test]
    fn no_unit_is_placed_twice() {
        let f = graziani();
        let ids: BTreeSet<&PieceId> = f.setup.pieces.iter().map(|p| &p.id).collect();
        assert_eq!(ids.len(), f.setup.pieces.len());
    }

    #[test]
    fn set_up_and_schedule_do_not_overlap() {
        let f = graziani();
        let schedule = Schedule::load().unwrap();
        let mut subtree = Vec::new();
        for row in schedule
            .rows()
            .filter(|r| r.at >= f.scenario.start && r.at <= f.scenario.end)
        {
            for u in &row.units {
                let mut stack = vec![u.clone()];
                while let Some(x) = stack.pop() {
                    stack.extend(f.oa.children(&x).iter().cloned());
                    subtree.push(x);
                }
            }
        }
        assert!(subtree.len() >= 12);
        for u in subtree {
            assert!(
                !f.has(u.as_str()),
                "{u} is both set up and scheduled (R-096)"
            );
        }
        // R-096 the other way: a unit the set-up names starts on the map
        // whatever its sheet says.
        assert_eq!(
            f.piece("unit:cw:1-fr-mtr-marine").location,
            Location::Hex(hex("C3926"))
        );
    }

    #[test]
    fn toe_cpa_morale_and_kind() {
        let f = graziani();
        let mg = f.piece("unit:it:giariabub-oasis-mg-bn");
        assert_eq!(mg.toe, 4); // U@4
        assert_eq!(f.piece("unit:it:30-gaf-antitank-bn").toe, 6);
        assert_eq!(f.piece("unit:it:9th-gaf-s-di-c-arty-regt").toe, 6);
        let bn = f.piece("unit:it:i-30-gaf-s-di-c-bn");
        let max = f
            .chars
            .row(cna_data::oa::Nation::It, "u")
            .unwrap()
            .max_toe
            .as_ref()
            .unwrap();
        assert_eq!(bn.toe, max.value.unwrap() as u32); // N
        assert_eq!(f.piece("unit:cw:4-in-hq").toe, 0); // N with no max TOE
        assert_eq!(f.piece("unit:cw:4-in-hq").kind, PieceKind::Hq);
        assert_eq!(bn.kind, PieceKind::Combat);
        assert_eq!(
            f.piece("unit:cw:4-in-hq").cpa.as_ref().unwrap().value,
            Some(30)
        );
        assert_eq!(f.piece("unit:it:xviii-libyan-bn").morale, Some(-2));
        assert_eq!(bn.morale, Some(-3));
        assert_eq!(
            f.piece("unit:cw:selby-force-the-matruh-garrison").morale,
            None
        );
        assert_eq!(bn.nation, Some(cna_data::oa::Nation::It));
        assert_eq!(bn.id_code.as_deref(), Some("u"));
    }

    #[test]
    fn every_piece_has_a_stacking_equivalent() {
        let f = graziani();
        for p in &f.setup.pieces {
            if p.kind == PieceKind::Combat || p.kind == PieceKind::Hq {
                assert!(p.equivalent.is_some(), "{}: no equivalent", p.id);
                assert_ne!(p.equivalent, Some(Equivalent::ByUnit), "{}", p.id);
            } else {
                assert_eq!(p.equivalent, None);
            }
        }
        assert_eq!(f.piece("unit:cw:4-in-hq").equivalent, Some(Equivalent::Hq));
        assert_eq!(
            f.piece("unit:it:63-anti-tank-coy").equivalent,
            Some(Equivalent::Company)
        );
    }

    #[test]
    fn supply_units_are_placed_like_units() {
        let f = graziani();
        let at = |h: &str| {
            f.setup
                .pieces
                .iter()
                .filter(|p| p.kind == PieceKind::SupplyUnit && p.hex() == Some(&hex(h)))
                .count()
        };
        assert_eq!(at("C4807"), 3); // Tobruk
        assert_eq!(at("D3714"), 5); // Mersa Matruh
        assert_eq!(at("E1730"), 3); // Cairo, from `adds`
        let tripoli = f
            .setup
            .pieces
            .iter()
            .filter(|p| {
                p.kind == PieceKind::SupplyUnit && p.location == Location::Box(OffMapBox::Tripoli)
            })
            .count();
        assert_eq!(tripoli, 8);
        let dummy = f
            .setup
            .pieces
            .iter()
            .find(|p| p.id == PieceId::dummy_supply(Side::Axis, 1))
            .unwrap();
        assert_eq!(dummy.kind, PieceKind::DummySupplyUnit);
        assert!(dummy.is_pending());
        assert_eq!((dummy.nation, dummy.toe, dummy.morale), (None, 0, None));
    }

    // 32.1x, 60.92: each real supply unit carries its block's figures
    // (per unit, as the scenario schema now says unless `pooled`); Derna's printed
    // dash is no ammunition; dummies carry nothing.
    #[test]
    fn supply_units_carry_their_block_loads() {
        let f = graziani();
        assert!(f.setup.pieces.iter().all(|p| p.load.is_none()));
        let on = expand(&f.scenario, &f.oa, &f.chars, &f.map, Modules::ALL).unwrap();
        let loads_at = |h: &str| -> Vec<Option<Load>> {
            on.pieces
                .iter()
                .filter(|p| p.hex() == Some(&hex(h)))
                .filter(|p| p.kind == PieceKind::SupplyUnit)
                .map(|p| p.load)
                .collect()
        };
        let tobruk = Some(Load { ammo: 15, fuel: 50 });
        assert_eq!(loads_at("C4807"), vec![tobruk; 3]);
        let derna = on
            .pieces
            .iter()
            .find(|p| p.load == Some(Load { ammo: 0, fuel: 10 }));
        assert!(derna.is_some());
        assert_eq!(
            loads_at("D3714"),
            vec![Some(Load { ammo: 40, fuel: 60 }); 5]
        );
        for p in &on.pieces {
            match p.kind {
                PieceKind::SupplyUnit => {
                    let l = p.load.unwrap_or_else(|| panic!("{} has no load", p.id));
                    assert!(l.ammo <= crate::piece::MAX_AMMO && l.fuel <= crate::piece::MAX_FUEL);
                }
                _ => assert_eq!(p.load, None, "{}", p.id),
            }
        }
    }

    #[test]
    fn constraints_are_logged_unchecked() {
        let f = graziani();
        let texts: Vec<&str> = f
            .setup
            .unchecked
            .iter()
            .map(|u| u.constraint.as_str())
            .collect();
        // Graziani's sentences are all typed now (cna bd108b6); only the
        // Cairo-or-Helwan choice the engine does not yet offer is left.
        assert_eq!(texts, vec!["or helwan"]);
        let helwan = &f.setup.unchecked[0];
        assert_eq!(helwan.side, Side::Cw);
        assert!(
            helwan
                .pieces
                .contains(&PieceId::from(uid("unit:cw:6-aus-hq")))
        );
    }

    // 32.5, 59.63: the truck classes that motorise come from the
    // scenario's typed `motorisation.from` (cna 277cd31), not a fixed
    // "medium". The Axis's first deployment has 10/25/10 trucks.
    #[test]
    fn motorisation_counts_the_classes_the_scenario_names() {
        use cna_data::scenario::TruckClass;
        let mut f = graziani();
        let first = |s: &Setup| {
            s.motorisation
                .iter()
                .find(|m| m.side == Side::Axis)
                .map(|m| m.points)
        };
        let on = expand(&f.scenario, &f.oa, &f.chars, &f.map, Modules::ALL).unwrap();
        assert_eq!(first(&on), Some(25));
        f.scenario
            .abstractions
            .as_mut()
            .unwrap()
            .air_and_logistics
            .as_mut()
            .unwrap()
            .motorisation
            .as_mut()
            .unwrap()
            .from = vec![TruckClass::Medium, TruckClass::Heavy];
        let on = expand(&f.scenario, &f.oa, &f.chars, &f.map, Modules::ALL).unwrap();
        assert_eq!(first(&on), Some(35));
    }

    #[test]
    fn a_pooled_supply_unit_block_is_refused_with_the_module_on() {
        let mut f = graziani();
        let abs = f.scenario.abstractions.as_mut().unwrap();
        let axis = abs.air_and_logistics.as_mut().unwrap();
        let blocks = axis
            .replaces
            .get_mut(&Side::Axis)
            .unwrap()
            .supply
            .as_mut()
            .unwrap();
        let b = blocks
            .iter_mut()
            .find(|b| b.kind == Some(SupplyKind::SupplyUnit))
            .unwrap();
        b.pooled = true;
        assert!(expand(&f.scenario, &f.oa, &f.chars, &f.map, Modules::default()).is_ok());
        let err = expand(&f.scenario, &f.oa, &f.chars, &f.map, Modules::ALL).unwrap_err();
        assert!(format!("{err:#}").contains("pooled"), "{err:#}");
    }

    #[test]
    fn pieces_round_trip_through_json() {
        let f = graziani();
        let json = serde_json::to_string(&f.setup.pieces).unwrap();
        let back: Vec<Piece> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, f.setup.pieces);
    }

    #[test]
    fn a_placement_hex_off_the_map_is_an_error() {
        let mut scenario = Scenario::load("grazianis-offensive").unwrap();
        let oa = Oa::load_all().unwrap();
        let chars = Characteristics::load().unwrap();
        let map = Map::load().unwrap();
        let d = &mut scenario.sides.get_mut(&Side::Axis).unwrap().deployments[0];
        d.placement = cna_data::scenario::Placement::of(Locator::Hexes(vec![hex("C9999")]));
        let err = format!(
            "{:#}",
            expand(&scenario, &oa, &chars, &map, Modules::default()).unwrap_err()
        );
        assert!(err.contains("C9999"), "{err}");
    }
}
