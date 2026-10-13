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

use crate::piece::{Location, Piece, PieceId, PieceKind};
use anyhow::{Context, Result, bail};
use cna_data::characteristics::{Characteristics, Equivalent, Row};
use cna_data::cp::Cp;
use cna_data::map::Map;
use cna_data::oa::{Arrival, Oa, ToeMark, Unit, UnitId};
use cna_data::scenario::{Locator, Placement, Scenario, Side, Subject, SupplyKind, UnitEntry};
use cna_data::time::GameTime;
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
pub fn expand(scenario: &Scenario, oa: &Oa, chars: &Characteristics, map: &Map) -> Result<Setup> {
    let scenario = scenario.land_only();
    let mut setup = Setup {
        pieces: Vec::new(),
        unchecked: Vec::new(),
    };
    let mut placed = BTreeSet::new();
    for side in [Side::Axis, Side::Cw] {
        let set = scenario.side(side);
        for (di, d) in set.deployments.iter().enumerate() {
            let location = resolve(&d.placement, map)
                .with_context(|| format!("{side} deployment {}", di + 1))?;
            let mut ids = Vec::new();
            for e in &d.units {
                let members = members(e, oa, scenario.start)
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
                setup.pieces.push(Piece {
                    id,
                    side,
                    kind,
                    nation: None,
                    id_code: None,
                    parent: None,
                    toe: 0,
                    weapons: Vec::new(),
                    cpa: None,
                    morale: None,
                    status: None,
                    equivalent: None,
                    garrison_of: None,
                    location: location.clone(),
                    cp_spent: Cp::ZERO,
                    cp_enemy_half: Cp::ZERO,
                    cohesion: Default::default(),
                    contact: false,
                    engaged: false,
                    reserve: None,
                    released: None,
                    landed: false,
                });
            }
            log_constraint(&mut setup, side, &b.placement, ids);
        }
    }
    Ok(setup)
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

/// Every unit one set-up entry stands for.
fn members(e: &UnitEntry, oa: &Oa, start: GameTime) -> Result<Vec<Member>> {
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
        kind: if equivalent == Some(Equivalent::Hq) {
            PieceKind::Hq
        } else {
            PieceKind::Combat
        },
        nation: Some(unit.id.nation()),
        id_code: unit.id_code.clone(),
        parent,
        toe,
        weapons,
        cpa: row.and_then(|r| r.cpa.clone()),
        morale: oa.morale(&unit.id),
        status: entry.status,
        equivalent,
        garrison_of: oa.formation_of(&unit.id).garrison_hex(),
        location,
        cp_spent: Cp::ZERO,
        cp_enemy_half: Cp::ZERO,
        cohesion: Default::default(),
        contact: false,
        engaged: false,
        reserve: None,
        released: None,
        landed: false,
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
fn toe_points(unit: &Unit, row: Option<&Row>) -> u32 {
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
    match &p.locator {
        Locator::Hexes(h) => {
            for hex in h {
                if map.hex(hex).is_none() {
                    bail!("{hex}: not on the map");
                }
            }
            if h.len() == 1 {
                return Ok(Location::Hex(h[0].clone()));
            }
        }
        Locator::Place(id) => {
            let place = map
                .place(id)
                .with_context(|| format!("{id}: not in places.json"))?;
            return Ok(Location::Hex(place.hex.clone()));
        }
        Locator::Box(b) => return Ok(Location::Box(*b)),
        Locator::Within { of, .. } => {
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
        Locator::Region(_) | Locator::AirFacility => {}
    }
    Ok(Location::Pending(p.clone()))
}

fn log_constraint(setup: &mut Setup, side: Side, p: &Placement, pieces: Vec<PieceId>) {
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
        let setup = expand(&scenario, &oa, &chars, &map).unwrap();
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
                let expect = match &d.placement.locator {
                    Locator::Hexes(h) if h.len() == 1 => Location::Hex(h[0].clone()),
                    Locator::Place(p) => Location::Hex(f.map.place(p).unwrap().hex.clone()),
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
            p.locator,
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

    #[test]
    fn constraints_are_logged_unchecked() {
        let f = graziani();
        let texts: Vec<&str> = f
            .setup
            .unchecked
            .iter()
            .map(|u| u.constraint.as_str())
            .collect();
        assert!(texts.contains(&"or Helwan (E1430)"));
        assert!(texts.contains(&"any one hex of map D that lies west of Mersa Matruh (D3714); all three units in that one hex (60.92)"));
        let helwan = f
            .setup
            .unchecked
            .iter()
            .find(|u| u.constraint == "or Helwan (E1430)")
            .unwrap();
        assert_eq!(helwan.side, Side::Cw);
        assert!(
            helwan
                .pieces
                .contains(&PieceId::from(uid("unit:cw:6-aus-hq")))
        );
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
        d.placement.locator = Locator::Hexes(vec![hex("C9999")]);
        let err = format!("{:#}", expand(&scenario, &oa, &chars, &map).unwrap_err());
        assert!(err.contains("C9999"), "{err}");
    }
}
