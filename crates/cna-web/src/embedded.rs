//! The data compiled in: a `Source` over the files `build.rs` embedded.

use anyhow::{Context, Result};
use cna_data::source::Source;
use cna_game::data::Data;
use std::sync::OnceLock;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub use generated::RULES_COMMIT;

/// The scenario this build plays. Only Graziani's Offensive is embedded
/// and tested (spec, *Non-goals*).
pub const SCENARIO: &str = "grazianis-offensive";

/// `vendor/cna/data` as compiled into this crate.
pub struct Embedded;

impl Source for Embedded {
    fn read(&self, rel: &str) -> Result<String> {
        let i = generated::FILES
            .binary_search_by_key(&rel, |(path, _)| path)
            .ok()
            .with_context(|| format!("no such embedded file: {rel}"))?;
        Ok(generated::FILES[i].1.to_owned())
    }

    fn list(&self, dir: &str) -> Result<Vec<String>> {
        let prefix = format!("{dir}/");
        Ok(generated::FILES
            .iter()
            .filter_map(|(path, _)| path.strip_prefix(&prefix))
            .filter(|rest| !rest.contains('/'))
            .map(str::to_owned)
            .collect())
    }
}

/// The scenario's `Data`, built from the embedded files on first use.
/// The files are compiled in and the native tests load them, so a
/// failure here is a build bug, not a runtime condition.
pub fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        Data::load_from(SCENARIO, &Embedded, RULES_COMMIT.to_owned())
            .expect("the embedded data loads")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::source::Fs;

    #[test]
    fn embedded_files_are_the_checkout_files() {
        for dir in ["map", "tables", "oa", "scenarios", "errata"] {
            assert_eq!(Embedded.list(dir).unwrap(), Fs.list(dir).unwrap(), "{dir}");
            for name in Embedded.list(dir).unwrap() {
                let rel = format!("{dir}/{name}");
                assert_eq!(
                    Embedded.read(&rel).unwrap(),
                    Fs.read(&rel).unwrap(),
                    "{rel}"
                );
            }
        }
        assert!(Embedded.read("map/raw/anything").is_err());
    }

    #[test]
    fn embedded_data_equals_the_checkout_data() {
        let native = Data::load(SCENARIO).unwrap();
        assert_eq!(native.rules_commit, RULES_COMMIT);
        assert_eq!(format!("{:?}", data()), format!("{native:?}"));
    }
}
