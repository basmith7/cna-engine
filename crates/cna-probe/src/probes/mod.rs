//! Every probe. The caller fills in the commits.

use crate::output::Probe;
use cna_data::close_assault::CloseAssault;

pub mod r012;

pub struct Ctx {
    pub table: CloseAssault,
}

impl Ctx {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Ctx {
            table: CloseAssault::load()?,
        })
    }
}

pub type ProbeFn = fn(&Ctx) -> Probe;

pub fn all() -> Vec<(&'static str, ProbeFn)> {
    vec![("R-012", r012::probe)]
}
