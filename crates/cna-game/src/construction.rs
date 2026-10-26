//! Construction (§24): projects begun in phase C and counted stage by
//! stage. A fortification level (24.4x) needs an engineer-capable unit and
//! an infantry battalion of 3+ TOE points standing in the hex, three
//! stages in which neither spends CP nor sits in a storm (24.11, 24.22),
//! and goes down when the third is counted. Stores are ignored under §32
//! (32.11). Minefields and the other items wait (`NOT_SIMULATED.md`).

use crate::clock::Phase;
use crate::combat::strength::Class;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::{HexId, Terrain};
use cna_data::scenario::Side;
use cna_rules::movement::Weather as StepWeather;
use serde::{Deserialize, Serialize};

/// What a project builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Item {
    /// One fortification level (24.4x).
    Fortification,
    /// A real minefield (24.3x).
    RealMinefield,
    /// A dummy minefield, which looks like a real one (24.31).
    DummyMinefield,
}

impl Item {
    /// Stages of work the Construction Chart asks (24.17).
    pub fn stages(self) -> u8 {
        match self {
            Item::Fortification => 3,
            Item::RealMinefield | Item::DummyMinefield => 1,
        }
    }
}

/// A project under way in a hex (24.1x): one to a hex for fortifications
/// (24.47).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub side: Side,
    pub item: Item,
    pub builders: Vec<PieceId>,
    /// Stages counted so far.
    #[serde(default)]
    pub stages: u8,
}

/// A minefield (26.1x): its owner, whether it is real (the owner's
/// secret until an enemy enters it), and whether an enemy has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Minefield {
    pub side: Side,
    pub real: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub revealed: bool,
}

/// What a real minefield costs in ammunition under §32 (24.33).
pub const MINEFIELD_AMMO: u32 = 15;

/// A fortification never goes past 2 except in Alexandria and Cairo,
/// and a city only back to its own level (24.43, 25.12).
const BUILT_MAX: u8 = 2;

impl GameState {
    /// Whether `piece` can do engineering (23.1x): an engineer unit by its
    /// characteristics row, or an HQ marked E.
    pub(crate) fn engineer_capable(&self, data: &Data, piece: &Piece) -> bool {
        let row = piece
            .nation
            .zip(piece.id_code.as_deref())
            .and_then(|(n, code)| data.chars.row(n, code));
        if row.is_some_and(|r| r.unit_type.contains("Engineer")) {
            return true;
        }
        piece.kind == PieceKind::Hq
            && piece
                .id
                .unit()
                .and_then(|u| data.oa.unit(&u))
                .is_some_and(|u| u.name.contains("^E"))
    }

    /// A combat unit with at least three infantry TOE points (24.41).
    fn infantry_battalion(&self, data: &Data, piece: &Piece) -> bool {
        piece.kind == PieceKind::Combat
            && self
                .parts(data, piece)
                .iter()
                .filter(|p| p.class == Class::Infantry)
                .map(|p| p.points)
                .sum::<u32>()
                >= 3
    }

    /// `Order::Build` (24.1x, 24.4x): in phase C, `builders` of the
    /// player's standing in `hex` begin a project there.
    pub(crate) fn build(
        &mut self,
        data: &Data,
        player: Side,
        item: Item,
        hex: &HexId,
        builders: &[PieceId],
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::Organisation || !self.modules.construction {
            return Err(Illegal::new(
                "24.11",
                format!(
                    "construction begins in phase C's construction segment, not at {}",
                    self.clock
                ),
            ));
        }
        for id in builders {
            let Some(p) = self.pieces.get(id).filter(|p| p.side == player) else {
                return Err(Illegal::new(
                    "24.11",
                    format!("{id} is not one of {player}'s"),
                ));
            };
            if p.hex() != Some(hex) {
                return Err(Illegal::new("24.11", format!("{id} is not in {hex}")));
            }
            if self.projects.values().any(|j| j.builders.contains(id)) {
                return Err(Illegal::new(
                    "24.13",
                    format!("{id} is already building: one project at a time"),
                ));
            }
        }
        if self.projects.contains_key(hex) {
            return Err(Illegal::new(
                "24.47",
                format!("a project is already under way in {hex}"),
            ));
        }
        let payment = match item {
            Item::Fortification => {
                self.fortification_site(data, player, hex, builders)?;
                None
            }
            Item::RealMinefield | Item::DummyMinefield => {
                self.minefield_site(data, player, item, hex, builders)?
            }
        };
        let mut events = Vec::new();
        if let Some(pay) = payment {
            self.spend_ammo(vec![pay], &mut events);
        }
        self.projects.insert(
            hex.clone(),
            Project {
                side: player,
                item,
                builders: builders.to_vec(),
                stages: 0,
            },
        );
        events.insert(
            0,
            Event::ConstructionBegun {
                side: player,
                item,
                hex: hex.clone(),
                builders: builders.to_vec(),
            },
        );
        Ok(events)
    }

    /// Where and with whom a fortification level may be begun (24.4x).
    fn fortification_site(
        &self,
        data: &Data,
        player: Side,
        hex: &HexId,
        builders: &[PieceId],
    ) -> Result<(), Illegal> {
        let pieces: Vec<&Piece> = builders.iter().map(|id| &self.pieces[id]).collect();
        let engineer = pieces.iter().position(|p| self.engineer_capable(data, p));
        let infantry = pieces
            .iter()
            .enumerate()
            .any(|(i, p)| Some(i) != engineer && self.infantry_battalion(data, p));
        if engineer.is_none() || !infantry {
            return Err(Illegal::new(
                "24.41",
                "a fortification needs an engineer-capable unit and an infantry battalion of 3+ TOE points",
            ));
        }
        let terrain = data.map.hex(hex).map(|h| h.terrain);
        let city = terrain == Some(Terrain::MajorCity);
        let level = self.fortification_level(data, hex);
        let ceiling = if city {
            crate::combat::terrain::city_level(data, hex)
        } else {
            BUILT_MAX
        };
        if level >= ceiling {
            return Err(Illegal::new(
                "24.43",
                format!("{hex} is at level {level}: at most {ceiling}"),
            ));
        }
        if !city
            && matches!(
                terrain,
                Some(Terrain::SaltMarsh | Terrain::Delta | Terrain::Mountain | Terrain::Desert)
                    | None
            )
        {
            return Err(Illegal::new(
                "24.44",
                format!("no fortification is built in {hex}'s terrain"),
            ));
        }
        // A city's reduced levels are rebuilt even in enemy ZOC (24.45).
        if !city && self.zoc_at(data, hex, player) != Zoc::Clear {
            return Err(Illegal::new(
                "24.44",
                format!("{hex} is in an enemy ZOC: no fortification is begun there"),
            ));
        }
        Ok(())
    }

    /// Where and by whom a minefield may be laid (24.3x): one engineer
    /// unit or a Commonwealth E-HQ, on the chart's terrain, never in a
    /// major city or an enemy ZOC, one field to a hex. A real one under
    /// §32 takes its ammunition from a supply unit in the hex.
    fn minefield_site(
        &self,
        data: &Data,
        player: Side,
        item: Item,
        hex: &HexId,
        builders: &[PieceId],
    ) -> Result<Option<crate::ammo::AmmoPayment>, Illegal> {
        let layer = match builders {
            [one] => Some(&self.pieces[one]),
            _ => None,
        };
        let able = layer.is_some_and(|p| {
            (p.kind == PieceKind::Combat && self.engineer_capable(data, p))
                || (p.kind == PieceKind::Hq && p.side == Side::Cw && self.engineer_capable(data, p))
        });
        if !able {
            return Err(Illegal::new(
                "24.31",
                "a minefield is laid by one engineer unit or a Commonwealth E-HQ",
            ));
        }
        if self.minefields.contains_key(hex) {
            return Err(Illegal::new(
                "24.35",
                format!("{hex} already has a minefield"),
            ));
        }
        let terrain = data.map.hex(hex).map(|h| h.terrain);
        if !matches!(
            terrain,
            Some(Terrain::Clear | Terrain::Gravel | Terrain::SaltMarsh)
        ) {
            return Err(Illegal::new(
                "24.35",
                format!("no minefield is laid in {hex}'s terrain"),
            ));
        }
        if self.zoc_at(data, hex, player) != Zoc::Clear {
            return Err(Illegal::new(
                "24.35",
                format!("{hex} is in an enemy ZOC: no minefield is laid there"),
            ));
        }
        if item == Item::DummyMinefield || !self.modules.abstract_supply {
            return Ok(None);
        }
        let from = self
            .pieces
            .values()
            .find(|p| {
                p.side == player
                    && p.hex() == Some(hex)
                    && p.load.as_ref().is_some_and(|l| l.ammo >= MINEFIELD_AMMO)
            })
            .map(|p| p.id.clone())
            .ok_or_else(|| {
                Illegal::new(
                    "24.33",
                    format!("a real minefield needs {MINEFIELD_AMMO} ammunition from a supply unit in {hex}"),
                )
            })?;
        Ok(Some(crate::ammo::AmmoPayment {
            piece: builders[0].clone(),
            from,
            ammo: MINEFIELD_AMMO,
        }))
    }

    /// What `piece`, moving with `stack`, pays on top of the terrain to
    /// enter `to`'s minefield (TEC, 26.21–26.23): a friendly field 1 on
    /// foot and 4 motorised, an enemy one 4 on foot and the unit's CPA
    /// motorised; with an engineer battalion or a Commonwealth E-HQ in the
    /// stack (or being one), +4 into an enemy field and nothing into a
    /// friendly one.
    pub(crate) fn minefield_cost(
        &self,
        data: &Data,
        side: Side,
        stack: &[PieceId],
        piece: &PieceId,
        motorised: bool,
        to: &HexId,
    ) -> Cp {
        let Some(field) = self.minefields.get(to) else {
            return Cp::ZERO;
        };
        let friendly = field.side == side;
        let escorted = stack.iter().any(|id| {
            let p = &self.pieces[id];
            self.engineer_capable(data, p) && (p.kind == PieceKind::Combat || p.side == Side::Cw)
        });
        match (friendly, escorted, motorised) {
            (true, true, _) => Cp::ZERO,
            (true, false, false) => Cp::whole(1),
            (true, false, true) | (false, true, _) | (false, false, false) => Cp::whole(4),
            (false, false, true) => Cp::whole(self.cpa(&self.pieces[piece]).unwrap_or(0)),
        }
    }

    /// An enemy entering a minefield turns it face up (26.11).
    pub(crate) fn reveal_minefield(&mut self, side: Side, to: &HexId, events: &mut Vec<Event>) {
        if let Some(field) = self.minefields.get_mut(to)
            && field.side != side
            && !field.revealed
        {
            field.revealed = true;
            events.push(Event::MinefieldRevealed {
                hex: to.clone(),
                real: field.real,
            });
        }
    }

    /// The end of a movement phase (26.11): a dummy field an enemy paid
    /// to enter is taken off.
    pub(crate) fn lift_revealed_dummies(&mut self, events: &mut Vec<Event>) {
        let gone: Vec<HexId> = self
            .minefields
            .iter()
            .filter(|(_, f)| f.revealed && !f.real)
            .map(|(h, _)| h.clone())
            .collect();
        for hex in gone {
            self.minefields.remove(&hex);
            events.push(Event::MinefieldRemoved { hex });
        }
    }

    /// The end of an OpStage (24.11, 24.2x): a project whose builders all
    /// stand in its hex and spent no CP, out of any storm, counts the
    /// stage; with every stage counted the item is built. A project whose
    /// builders left or are gone is lost.
    pub(crate) fn count_construction(&mut self, data: &Data, events: &mut Vec<Event>) {
        let hexes: Vec<HexId> = self.projects.keys().cloned().collect();
        for hex in hexes {
            let project = &self.projects[&hex];
            let present = project
                .builders
                .iter()
                .all(|id| self.pieces.get(id).is_some_and(|p| p.hex() == Some(&hex)));
            if !present {
                let project = self.projects.remove(&hex).expect("listed");
                events.push(Event::ConstructionLost {
                    side: project.side,
                    item: project.item,
                    hex,
                });
                continue;
            }
            let idle = project
                .builders
                .iter()
                .all(|id| self.pieces[id].cp_spent == Cp::ZERO);
            if !idle || self.weather_at(&hex) != StepWeather::Normal {
                continue;
            }
            let project = self.projects.get_mut(&hex).expect("listed");
            project.stages += 1;
            if project.stages < project.item.stages() {
                continue;
            }
            let project = self.projects.remove(&hex).expect("listed");
            match project.item {
                Item::Fortification => {
                    let level = self.fortification_level(data, &hex) + 1;
                    self.fortifications.insert(hex.clone(), level);
                    events.push(Event::FortificationBuilt {
                        side: project.side,
                        hex,
                        level,
                    });
                }
                Item::RealMinefield | Item::DummyMinefield => {
                    self.minefields.insert(
                        hex.clone(),
                        Minefield {
                            side: project.side,
                            real: project.item == Item::RealMinefield,
                            revealed: false,
                        },
                    );
                    events.push(Event::MinefieldLaid {
                        side: project.side,
                        hex,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::Order;
    use crate::state::Modules;
    use crate::state::test_support::{auto_place, pass_stage};

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// Phase C of GT1 OpStage 1 with every module on, and an Axis
    /// engineer and infantry battalion stood together on clear ground
    /// out of the Commonwealth's reach.
    fn organising(data: &Data) -> (GameState, HexId, Vec<PieceId>) {
        let mut state = GameState::with_modules(data, 3, Modules::ALL).unwrap();
        auto_place(data, &mut state);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::Organisation);
        let engineer = pid("unit:it:201st-engineer-bn");
        assert!(state.engineer_capable(data, &state.pieces[&engineer]));
        let infantry = pid("unit:it:ix-libyan-bn");
        let site = hex("B1010");
        assert_eq!(data.map.hex(&site).unwrap().terrain, Terrain::Clear);
        for id in [&engineer, &infantry] {
            state.pieces.get_mut(id).unwrap().location = crate::piece::Location::Hex(site.clone());
        }
        (state, site, vec![engineer, infantry])
    }

    fn build(site: &HexId, builders: &[PieceId]) -> Order {
        Order::Build {
            item: Item::Fortification,
            hex: site.clone(),
            builders: builders.to_vec(),
        }
    }

    #[test]
    fn a_fortification_goes_down_after_three_idle_stages_24_41() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap();
        let mut built = Vec::new();
        for _ in 0..3 {
            assert_eq!(state.fortification_level(&data, &site), 0);
            built = pass_stage(&data, &mut state);
        }
        assert!(
            built.contains(&Event::FortificationBuilt {
                side: Side::Axis,
                hex: site.clone(),
                level: 1,
            }),
            "{built:?}"
        );
        assert_eq!(state.fortification_level(&data, &site), 1);
        assert!(state.projects.is_empty());
    }

    #[test]
    fn a_builder_spending_cp_loses_the_stage_and_one_leaving_the_project_24_11() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap();
        state.pieces.get_mut(&builders[0]).unwrap().cp_spent = Cp::whole(1);
        let mut events = Vec::new();
        state.count_construction(&data, &mut events);
        assert_eq!(state.projects[&site].stages, 0);
        state.count_construction(&data, &mut events);
        // Still charged: the field resets only at the next stage.
        assert_eq!(state.projects[&site].stages, 0);
        state.pieces.get_mut(&builders[1]).unwrap().location =
            crate::piece::Location::Hex(hex("B1011"));
        state.count_construction(&data, &mut events);
        assert!(state.projects.is_empty());
        assert!(events.contains(&Event::ConstructionLost {
            side: Side::Axis,
            item: Item::Fortification,
            hex: site,
        }));
    }

    #[test]
    fn a_fortification_needs_an_engineer_and_infantry_and_a_free_hex_24_41_24_47() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        for bad in [vec![builders[0].clone()], vec![builders[1].clone()]] {
            let err = state
                .apply(&data, Side::Axis, build(&site, &bad))
                .unwrap_err();
            assert_eq!(err.case, "24.41");
        }
        state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap();
        let err = state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap_err();
        assert_eq!(err.case, "24.13");
    }

    #[test]
    fn no_fortification_beyond_level_two_or_in_forbidden_terrain_24_43_24_44() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        state.fortifications.insert(site.clone(), 2);
        let err = state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap_err();
        assert_eq!(err.case, "24.43");
        let desert = data
            .map
            .hexes()
            .find(|h| h.terrain == Terrain::Desert)
            .unwrap()
            .id
            .clone();
        for id in &builders {
            state.pieces.get_mut(id).unwrap().location =
                crate::piece::Location::Hex(desert.clone());
        }
        let err = state
            .apply(&data, Side::Axis, build(&desert, &builders))
            .unwrap_err();
        assert_eq!(err.case, "24.44");
    }

    fn lay(item: Item, site: &HexId, layer: &PieceId) -> Order {
        Order::Build {
            item,
            hex: site.clone(),
            builders: vec![layer.clone()],
        }
    }

    #[test]
    fn an_engineer_lays_a_dummy_minefield_in_one_stage_24_31() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        let err = state
            .apply(
                &data,
                Side::Axis,
                lay(Item::DummyMinefield, &site, &builders[1]),
            )
            .unwrap_err();
        assert_eq!(err.case, "24.31");
        state
            .apply(
                &data,
                Side::Axis,
                lay(Item::DummyMinefield, &site, &builders[0]),
            )
            .unwrap();
        let events = pass_stage(&data, &mut state);
        assert!(events.contains(&Event::MinefieldLaid {
            side: Side::Axis,
            hex: site.clone(),
        }));
        assert_eq!(
            state.minefields[&site],
            Minefield {
                side: Side::Axis,
                real: false,
                revealed: false,
            }
        );
    }

    #[test]
    fn a_real_minefield_takes_fifteen_ammunition_from_a_supply_unit_in_the_hex_24_33() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        let err = state
            .apply(
                &data,
                Side::Axis,
                lay(Item::RealMinefield, &site, &builders[0]),
            )
            .unwrap_err();
        assert_eq!(err.case, "24.33");
        let supply = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.load.as_ref().is_some_and(|l| l.ammo >= 20))
            .unwrap()
            .id
            .clone();
        let before = state.pieces[&supply].load.as_ref().unwrap().ammo;
        state.pieces.get_mut(&supply).unwrap().location = crate::piece::Location::Hex(site.clone());
        let events = state
            .apply(
                &data,
                Side::Axis,
                lay(Item::RealMinefield, &site, &builders[0]),
            )
            .unwrap();
        assert!(events.contains(&Event::AmmunitionSpent {
            piece: builders[0].clone(),
            from: supply.clone(),
            ammo: MINEFIELD_AMMO,
        }));
        assert_eq!(
            state.pieces[&supply].load.as_ref().unwrap().ammo,
            before - MINEFIELD_AMMO
        );
        pass_stage(&data, &mut state);
        assert!(state.minefields[&site].real);
    }

    #[test]
    fn one_minefield_to_a_hex_and_none_in_rough_24_35() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        state.minefields.insert(
            site.clone(),
            Minefield {
                side: Side::Axis,
                real: false,
                revealed: false,
            },
        );
        let err = state
            .apply(
                &data,
                Side::Axis,
                lay(Item::DummyMinefield, &site, &builders[0]),
            )
            .unwrap_err();
        assert_eq!(err.case, "24.35");
        let rough = data
            .map
            .hexes()
            .find(|h| h.terrain == Terrain::Rough)
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&builders[0]).unwrap().location =
            crate::piece::Location::Hex(rough.clone());
        let err = state
            .apply(
                &data,
                Side::Axis,
                lay(Item::DummyMinefield, &rough, &builders[0]),
            )
            .unwrap_err();
        assert_eq!(err.case, "24.35");
    }

    /// The Axis movement segment of GT1 OpStage 1 with a foot battalion
    /// beside an empty clear hex no enemy controls; its CP to walk in.
    fn walking(data: &Data) -> (GameState, PieceId, HexId, Cp) {
        use cna_rules::movement::Mobility;
        let state = crate::movement::test_support::moving(data, 3, Side::Axis);
        let candidates: Vec<(PieceId, HexId)> = state
            .pieces
            .values()
            .filter(|p| {
                p.side == Side::Axis
                    && p.kind == PieceKind::Combat
                    && state.mobility(data, p) == Mobility::Foot
                    && !state.engineer_capable(data, p)
            })
            .filter_map(|p| p.hex().map(|h| (p.id.clone(), h.clone())))
            .flat_map(|(id, h)| {
                data.map
                    .neighbours(&h)
                    .into_iter()
                    .filter(|n| {
                        state.holder(n).is_none()
                            && data.map.hex(n).is_some_and(|x| x.terrain == Terrain::Clear)
                            && state.zoc_at(data, n, Side::Axis) == Zoc::Clear
                    })
                    .map(move |n| (id.clone(), n))
            })
            .collect();
        let (piece, to, base) = candidates
            .into_iter()
            .find_map(|(piece, to)| {
                let mut probe = state.clone();
                probe.apply(data, Side::Axis, walk(&piece, &to)).ok()?;
                let base = probe.pieces[&piece].cp_spent;
                Some((piece, to, base))
            })
            .expect("a foot battalion with room to walk");
        (state, piece, to, base)
    }

    fn walk(piece: &PieceId, to: &HexId) -> Order {
        Order::Move {
            pieces: vec![piece.clone()],
            path: vec![to.clone()],
            assault: false,
            fuel_from: None,
        }
    }

    #[test]
    fn foot_units_pay_four_into_an_enemy_minefield_and_one_into_their_own_26_21() {
        let data = graziani();
        for (owner, extra) in [(Side::Cw, 4), (Side::Axis, 1)] {
            let (mut state, piece, to, base) = walking(&data);
            state.minefields.insert(
                to.clone(),
                Minefield {
                    side: owner,
                    real: false,
                    revealed: false,
                },
            );
            state.apply(&data, Side::Axis, walk(&piece, &to)).unwrap();
            assert_eq!(
                state.pieces[&piece].cp_spent,
                base + Cp::whole(extra),
                "{owner}"
            );
            // An enemy entering a field turns it face up (26.11).
            assert_eq!(state.minefields[&to].revealed, owner == Side::Cw);
        }
    }

    #[test]
    fn a_dummy_an_enemy_paid_to_enter_is_gone_at_the_end_of_the_phase_26_11() {
        let data = graziani();
        for real in [false, true] {
            let (mut state, piece, to, _) = walking(&data);
            state.minefields.insert(
                to.clone(),
                Minefield {
                    side: Side::Cw,
                    real,
                    revealed: false,
                },
            );
            let events = state.apply(&data, Side::Axis, walk(&piece, &to)).unwrap();
            assert!(events.contains(&Event::MinefieldRevealed {
                hex: to.clone(),
                real,
            }));
            assert!(state.minefields.contains_key(&to));
            pass_stage(&data, &mut state);
            assert_eq!(state.minefields.contains_key(&to), real, "real: {real}");
        }
    }

    #[test]
    fn construction_waits_for_its_module_and_phase_c() {
        let data = graziani();
        let (mut state, site, builders) = organising(&data);
        state.modules.construction = false;
        let err = state
            .apply(&data, Side::Axis, build(&site, &builders))
            .unwrap_err();
        assert_eq!(err.case, "24.11");
    }
}
