//! Every probe. The caller fills in the commits.

use crate::output::Probe;
use cna_data::anti_armour::AntiArmour;
use cna_data::barrage::Barrage;
use cna_data::close_assault::CloseAssault;
use cna_data::construction::Construction;
use cna_data::morale::MoraleModifier;
use cna_data::terrain::Terrain;

pub mod oddities;
pub mod r001;
pub mod r009;
pub mod r011;
pub mod r012;
pub mod r015;
pub mod r018;

pub struct Ctx {
    pub table: CloseAssault,
    pub barrage: Barrage,
    pub anti_armour: AntiArmour,
    pub terrain: Terrain,
    pub morale: MoraleModifier,
    pub construction: Construction,
}

impl Ctx {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Ctx {
            table: CloseAssault::load()?,
            barrage: Barrage::load()?,
            anti_armour: AntiArmour::load()?,
            terrain: Terrain::load()?,
            morale: MoraleModifier::load()?,
            construction: Construction::load()?,
        })
    }
}

pub type ProbeFn = fn(&Ctx) -> Probe;

pub fn all() -> Vec<(&'static str, ProbeFn)> {
    vec![
        ("chart-oddities", oddities::probe),
        ("R-001", r001::probe),
        ("R-009", r009::probe),
        ("R-011", r011::probe),
        ("R-012", r012::probe),
        ("R-015", r015::probe),
        ("R-018", r018::probe),
    ]
}
