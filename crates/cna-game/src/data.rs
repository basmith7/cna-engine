//! The static data a game reads: the scenario and everything from
//! `vendor/cna` it refers to. Not part of the state; a save names the
//! scenario and the cna commit instead.

use anyhow::Result;
use cna_data::characteristics::Characteristics;
use cna_data::map::Map;
use cna_data::oa::Oa;
use cna_data::scenario::Scenario;

pub struct Data {
    pub scenario: Scenario,
    pub oa: Oa,
    pub chars: Characteristics,
    pub map: Map,
}

impl Data {
    /// Loads `data/scenarios/<slug>.json` and the tables and map it needs.
    pub fn load(slug: &str) -> Result<Data> {
        Ok(Data {
            scenario: Scenario::load(slug)?,
            oa: Oa::load_all()?,
            chars: Characteristics::load()?,
            map: Map::load()?,
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
