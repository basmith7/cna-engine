//! Roads and railroads built at set-up (59.12; 62.5, 61.6, 63.72): the
//! scenario's `construction.road_hexsides_built` and
//! `railroad_hexsides_built` upgrade those unfinished hexsides to road
//! and railroad, and `railroad_destroyed` cuts the railway, with the
//! `roads_built` module on. Games before the module see the map as drawn.

use crate::data::Data;
use crate::state::GameState;
use anyhow::{Result, bail};
use cna_data::map::{HexId, Hexside, HexsideFeature, Map};
use cna_data::scenario::Scenario;
use std::borrow::Cow;
use std::collections::BTreeSet;

/// The scenario's built hexsides, as hex pairs `(a, b)` with `a < b`.
#[derive(Debug, Clone, Default)]
pub struct Built {
    pub roads: BTreeSet<(HexId, HexId)>,
    pub railroads: BTreeSet<(HexId, HexId)>,
    /// Railroad hexes destroyed at the start (24.6x): the line is cut there.
    pub railroad_destroyed: BTreeSet<HexId>,
}

impl Built {
    pub fn load(scenario: &Scenario, map: &Map) -> Result<Built> {
        let Some(c) = scenario.construction.as_ref() else {
            return Ok(Built::default());
        };
        let strings = |key: &str| -> Vec<String> {
            c.get(key)
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        // An edge record with no hex across it (a road running off the
        // map) has no step to upgrade, so it is passed over.
        let off_map_edge = |k: &str| {
            k.split_once('|').is_some_and(|(a, side)| {
                HexId::parse(a).is_ok() && ["E", "SE", "SW", "W", "NW", "NE"].contains(&side)
            })
        };
        let pairs = |key: &str| -> Result<BTreeSet<(HexId, HexId)>> {
            let mut out = BTreeSet::new();
            for k in strings(key) {
                match map.hexside_pair(&k) {
                    Some(p) => {
                        out.insert(p);
                    }
                    None if off_map_edge(&k) => {}
                    None => bail!("{key}: {k} names no hexside"),
                }
            }
            Ok(out)
        };
        let railroad_destroyed = strings("railroad_destroyed")
            .iter()
            .map(|h| HexId::parse(h))
            .collect::<Result<_>>()?;
        Ok(Built {
            roads: pairs("road_hexsides_built")?,
            railroads: pairs("railroad_hexsides_built")?,
            railroad_destroyed,
        })
    }
}

fn key(a: &HexId, b: &HexId) -> (HexId, HexId) {
    if a <= b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}

impl GameState {
    /// The hexside between `a` and `b` as this game sees it: the map's,
    /// with the scenario's built roads and railroads finished when the
    /// `roads_built` module is on.
    pub(crate) fn hexside<'a>(
        &self,
        data: &'a Data,
        a: &HexId,
        b: &HexId,
    ) -> Option<Cow<'a, Hexside>> {
        let hs = data.map.hexside(a, b)?;
        if !self.modules.roads_built {
            return Some(Cow::Borrowed(hs));
        }
        let unfinished_road = hs.has(HexsideFeature::UnfinishedRoad);
        let unfinished_rail = hs.has(HexsideFeature::UnfinishedRailroad);
        if !unfinished_road && !unfinished_rail {
            return Some(Cow::Borrowed(hs));
        }
        let k = key(a, b);
        let road = unfinished_road && data.built.roads.contains(&k);
        let rail = unfinished_rail && data.built.railroads.contains(&k);
        if !road && !rail {
            return Some(Cow::Borrowed(hs));
        }
        let mut hs = hs.clone();
        for f in &mut hs.features {
            match f {
                HexsideFeature::UnfinishedRoad if road => *f = HexsideFeature::Road,
                HexsideFeature::UnfinishedRailroad if rail => *f = HexsideFeature::Railroad,
                _ => {}
            }
        }
        Some(Cow::Owned(hs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::PieceKind;
    use crate::reach::Purpose;
    use crate::state::Modules;
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;
    use cna_rules::movement::Mobility;

    fn crusader(roads_built: bool) -> (Data, GameState) {
        let data = Data::load("operation-crusader").unwrap();
        let state = GameState::with_modules(
            &data,
            1,
            Modules {
                roads_built,
                ..Modules::ALL
            },
        )
        .unwrap();
        (data, state)
    }

    #[test]
    fn a_built_road_is_a_road_at_set_up_and_moves_at_the_road_rate() {
        let (data, on) = crusader(true);
        let (_, off) = crusader(false);
        // A built hexside drawn as nothing but unfinished road, between
        // two empty hexes.
        let (a, b) = data
            .built
            .roads
            .iter()
            .find(|(a, b)| {
                data.map
                    .hexside(a, b)
                    .is_some_and(|hs| hs.features == [HexsideFeature::UnfinishedRoad])
                    && on.pieces_at(a).is_empty()
                    && on.pieces_at(b).is_empty()
            })
            .expect("a plain built road hexside")
            .clone();
        let road = |s: &GameState| s.hexside(&data, &a, &b).unwrap().road();
        assert!(road(&on), "{a}|{b} is a road with the module on");
        assert!(!road(&off), "{a}|{b} stays unfinished with it off");
        assert!(off.hexside(&data, &a, &b).unwrap().track());

        // A Commonwealth vehicle stepping along it pays the road rate
        // with the module on (half a point, 8.33) and the track rate off.
        let truck = on
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Cw
                    && p.kind == PieceKind::Combat
                    && on.mobility(&data, p) == Mobility::Vehicle
            })
            .expect("a Commonwealth vehicle")
            .id
            .clone();
        let cost = |s: &GameState| {
            let mover = s.mover(&data, std::slice::from_ref(&truck), Purpose::Move);
            s.step_into(&data, &mover, &a, &b, false).unwrap().0
        };
        let road_rate = data
            .terrain
            .cp("hex", "road")
            .map(|c| match c {
                cna_data::terrain::CpCell::Pair {
                    mot: cna_data::terrain::CpValue::Cost(c),
                    ..
                } => *c,
                other => panic!("{other:?}"),
            })
            .unwrap();
        assert_eq!(road_rate, Cp::halves(1));
        assert_eq!(cost(&on), road_rate);
        assert!(cost(&off) > road_rate, "{:?}", cost(&off));
    }

    #[test]
    fn games_before_the_module_see_the_map_as_drawn() {
        let (data, off) = crusader(false);
        for (a, b) in data.built.roads.iter().chain(&data.built.railroads) {
            assert_eq!(off.hexside(&data, a, b).as_deref(), data.map.hexside(a, b));
        }
    }

    #[test]
    fn every_built_hexside_is_drawn_unfinished() {
        for slug in ["operation-crusader", "the-last-chance", "rommels-arrival"] {
            let data = Data::load(slug).unwrap();
            assert!(!data.built.roads.is_empty(), "{slug}");
            for (a, b) in &data.built.roads {
                let hs = data.map.hexside(a, b).expect("recorded");
                assert!(hs.has(HexsideFeature::UnfinishedRoad), "{slug} {a}|{b}");
            }
            for (a, b) in &data.built.railroads {
                let hs = data.map.hexside(a, b).expect("recorded");
                assert!(hs.has(HexsideFeature::UnfinishedRailroad), "{slug} {a}|{b}");
            }
        }
    }
}
