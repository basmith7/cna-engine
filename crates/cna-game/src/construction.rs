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
}

impl Item {
    /// Stages of work the Construction Chart asks (24.17).
    pub fn stages(self) -> u8 {
        match self {
            Item::Fortification => 3,
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
        // The only item yet: a new one must say what it needs here.
        let Item::Fortification = item;
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
        self.projects.insert(
            hex.clone(),
            Project {
                side: player,
                item,
                builders: builders.to_vec(),
                stages: 0,
            },
        );
        Ok(vec![Event::ConstructionBegun {
            side: player,
            item,
            hex: hex.clone(),
            builders: builders.to_vec(),
        }])
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
            let level = self.fortification_level(data, &hex) + 1;
            self.fortifications.insert(hex.clone(), level);
            events.push(Event::FortificationBuilt {
                side: project.side,
                hex,
                level,
            });
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
        let engineer = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.hex().is_some() && state.engineer_capable(data, p))
            .expect("an Axis engineer on the map")
            .id
            .clone();
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
