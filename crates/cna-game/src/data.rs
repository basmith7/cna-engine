//! The static data a game reads: the scenario and everything from
//! `vendor/cna` it refers to, plus the commit it all came from. Not part
//! of the state; a save names the scenario and the cna commit instead,
//! and loading one checks both against the `Data` it is given.
//!
//! The engine never touches the file system or spawns a process itself
//! (the spec's condition for a WebAssembly build): `Data::load_from`
//! reads every table through a `cna_data::source::Source`, and
//! `Data::load` is the native convenience that uses the `vendor/cna`
//! checkout and asks git for its commit.

use anyhow::Result;
use cna_data::anti_armour::AntiArmour;
use cna_data::assault_size::AssaultSizeShifts;
use cna_data::barrage::Barrage;
use cna_data::characteristics::Characteristics;
use cna_data::close_assault::CloseAssault;
use cna_data::initiative::InitiativeRatings;
use cna_data::map::Map;
use cna_data::morale::MoraleModifier;
use cna_data::oa::Oa;
use cna_data::off_map::OffMapDistances;
use cna_data::prisoners::Prisoners;
use cna_data::scenario::Scenario;
use cna_data::schedule::Schedule;
use cna_data::source::{Fs, Source};
use cna_data::stacking::StackingPoints;
use cna_data::terrain::Terrain;
use cna_data::weapons::WeaponSystems;
use cna_data::weather::Weather;

#[derive(Debug)]
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
    pub barrage: Barrage,
    pub anti_armour: AntiArmour,
    pub close_assault: CloseAssault,
    pub morale: MoraleModifier,
    pub size_shifts: AssaultSizeShifts,
    pub max_attachments: cna_data::max_attachments::MaxAttachments,
    pub prisoners: Prisoners,
    pub breakdown: cna_data::breakdown::Breakdown,
    pub vehicle_repair: cna_data::repair::VehicleRepair,
    pub calendar: cna_data::calendar::Calendar,
    pub motorisation_losses: cna_data::calendar::MotorisationLosses,
    pub simplified_supply: cna_data::calendar::SimplifiedSupply,
}

impl Data {
    /// Loads `data/scenarios/<slug>.json` and the tables and map it needs
    /// from the `vendor/cna` checkout, recording its commit. Native only.
    pub fn load(slug: &str) -> Result<Data> {
        Data::load_from(slug, &Fs, cna_data::rules_commit()?)
    }

    /// The same from any `Source`, with the `vendor/cna` commit the
    /// source's files came from.
    pub fn load_from(slug: &str, src: &dyn Source, rules_commit: String) -> Result<Data> {
        Ok(Data {
            rules_commit,
            scenario: Scenario::load_from(src, slug)?,
            oa: Oa::load_from(src)?,
            chars: Characteristics::load_from(src)?,
            weapons: WeaponSystems::load_from(src)?,
            map: Map::load_from(src)?,
            terrain: Terrain::load_from(src)?,
            stacking: StackingPoints::load_from(src)?,
            weather: Weather::load_from(src)?,
            initiative: InitiativeRatings::load_from(src)?,
            schedule: Schedule::load_from(src)?,
            off_map: OffMapDistances::load_from(src)?,
            barrage: Barrage::load_from(src)?,
            anti_armour: AntiArmour::load_from(src)?,
            close_assault: CloseAssault::load_from(src)?,
            morale: MoraleModifier::load_from(src)?,
            size_shifts: AssaultSizeShifts::load_from(src)?,
            max_attachments: cna_data::max_attachments::MaxAttachments::load_from(src)?,
            prisoners: Prisoners::load_from(src)?,
            breakdown: cna_data::breakdown::Breakdown::load_from(src)?,
            vehicle_repair: cna_data::repair::VehicleRepair::load_from(src)?,
            calendar: cna_data::calendar::Calendar::load_from(src)?,
            motorisation_losses: cna_data::calendar::MotorisationLosses::load_from(src)?,
            simplified_supply: cna_data::calendar::SimplifiedSupply::load_from(src)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::source::MemSource;

    /// The directories `Data` reads: what a WebAssembly build embeds.
    pub const DIRS: [&str; 5] = ["map", "tables", "oa", "scenarios", "errata"];

    /// Each field's `Debug` form: every field derives `Debug`, and the
    /// maps inside are `BTreeMap`s, so equal data prints equally.
    fn fields(d: &Data) -> Vec<(&'static str, String)> {
        vec![
            ("rules_commit", format!("{:?}", d.rules_commit)),
            ("scenario", format!("{:?}", d.scenario)),
            ("oa", format!("{:?}", d.oa)),
            ("chars", format!("{:?}", d.chars)),
            ("weapons", format!("{:?}", d.weapons)),
            ("map", format!("{:?}", d.map)),
            ("terrain", format!("{:?}", d.terrain)),
            ("stacking", format!("{:?}", d.stacking)),
            ("weather", format!("{:?}", d.weather)),
            ("initiative", format!("{:?}", d.initiative)),
            ("schedule", format!("{:?}", d.schedule)),
            ("off_map", format!("{:?}", d.off_map)),
            ("barrage", format!("{:?}", d.barrage)),
            ("anti_armour", format!("{:?}", d.anti_armour)),
            ("close_assault", format!("{:?}", d.close_assault)),
            ("morale", format!("{:?}", d.morale)),
            ("size_shifts", format!("{:?}", d.size_shifts)),
            ("max_attachments", format!("{:?}", d.max_attachments)),
            ("prisoners", format!("{:?}", d.prisoners)),
        ]
    }

    #[test]
    fn a_memory_source_loads_the_same_data_as_the_checkout() {
        let from_fs = Data::load("grazianis-offensive").unwrap();
        let mem = MemSource::copy_from(&Fs, &DIRS).unwrap();
        let from_mem =
            Data::load_from("grazianis-offensive", &mem, from_fs.rules_commit.clone()).unwrap();
        for ((name, a), (_, b)) in fields(&from_fs).into_iter().zip(fields(&from_mem)) {
            assert!(a == b, "{name} differs between Fs and MemSource");
        }
    }

    #[test]
    fn a_source_missing_a_table_fails_naming_it() {
        let mem = MemSource::copy_from(&Fs, &["scenarios", "oa"]).unwrap();
        let err = Data::load_from("grazianis-offensive", &mem, String::new())
            .unwrap_err()
            .to_string();
        assert!(err.contains("unit-characteristics"), "{err}");
    }
}
