//! The static data a game reads: the scenario and everything from
//! `vendor/cna` it refers to, plus the commit it all came from. Not part
//! of the state; a save names the scenario and the cna commit instead,
//! and loading one checks both against the `Data` it is given.
//!
//! The engine never touches the file system or spawns a process itself
//! (the spec's condition for a WebAssembly build): `Data::load` is a
//! native convenience that reads `vendor/cna` and asks git for its
//! commit; another host fills the same struct by other means.

use anyhow::Result;
use cna_data::characteristics::Characteristics;
use cna_data::initiative::InitiativeRatings;
use cna_data::map::Map;
use cna_data::oa::Oa;
use cna_data::off_map::OffMapDistances;
use cna_data::scenario::Scenario;
use cna_data::schedule::Schedule;
use cna_data::stacking::StackingPoints;
use cna_data::terrain::Terrain;
use cna_data::weapons::WeaponSystems;
use cna_data::weather::Weather;

pub struct Data {
    /// The `vendor/cna` commit the tables were read from; what a new
    /// game records as `rules_commit` and a loaded one is checked against.
    pub rules_commit: String,
    pub scenario: Scenario,
    pub oa: Oa,
    pub chars: Characteristics,
    pub weapons: WeaponSystems,
    pub map: Map,
    pub terrain: Terrain,
    pub stacking: StackingPoints,
    pub weather: Weather,
    pub initiative: InitiativeRatings,
    pub schedule: Schedule,
    pub off_map: OffMapDistances,
}

impl Data {
    /// Loads `data/scenarios/<slug>.json` and the tables and map it needs
    /// from the `vendor/cna` checkout, recording its commit. Native only.
    pub fn load(slug: &str) -> Result<Data> {
        Ok(Data {
            rules_commit: cna_data::rules_commit()?,
            scenario: Scenario::load(slug)?,
            oa: Oa::load_all()?,
            chars: Characteristics::load()?,
            weapons: WeaponSystems::load()?,
            map: Map::load()?,
            terrain: Terrain::load()?,
            stacking: StackingPoints::load()?,
            weather: Weather::load()?,
            initiative: InitiativeRatings::load()?,
            schedule: Schedule::load()?,
            off_map: OffMapDistances::load()?,
        })
    }

    /// The scenario's slug: its id without the `scenario:` prefix.
    pub fn slug(&self) -> &str {
        self.scenario
            .id
            .strip_prefix("scenario:")
            .unwrap_or(&self.scenario.id)
    }
}
