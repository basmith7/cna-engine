//! The data compiled in: a `Source` over the files `build.rs` embedded.

use anyhow::{Context, Result};
use cna_data::source::Source;
use cna_game::data::Data;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub use generated::RULES_COMMIT;

/// The scenario the start screen offers first.
pub const DEFAULT_SCENARIO: &str = "grazianis-offensive";

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

/// A scenario's `Data`, built from the embedded files on first use and
/// kept for the page's life (one per scenario, so leaked). An unknown
/// slug is an `Err`; the files are compiled in and the native tests
/// load them, so any other failure is a build bug.
pub fn data(slug: &str) -> Result<&'static Data> {
    static DATA: OnceLock<Mutex<BTreeMap<String, &'static Data>>> = OnceLock::new();
    let mut cache = DATA
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(d) = cache.get(slug) {
        return Ok(d);
    }
    let d: &'static Data = Box::leak(Box::new(
        Data::load_from(slug, &Embedded, RULES_COMMIT.to_owned())
            .with_context(|| format!("scenario {slug}"))?,
    ));
    cache.insert(slug.to_owned(), d);
    Ok(d)
}

/// The scenarios a new game can start, `(slug, name)` in file order:
/// every embedded one (`cna_game::scenarios` smoke-tests each).
pub fn scenarios() -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for file in Embedded.list("scenarios")? {
        let Some(slug) = file.strip_suffix(".json") else {
            continue;
        };
        let v: serde_json::Value =
            serde_json::from_str(&Embedded.read(&format!("scenarios/{file}"))?)?;
        let name = v["name"]
            .as_str()
            .with_context(|| format!("{file} has no name"))?;
        out.push((slug.to_owned(), name.to_owned()));
    }
    Ok(out)
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
        for (slug, _) in scenarios().unwrap() {
            let native = Data::load(&slug).unwrap();
            assert_eq!(native.rules_commit, RULES_COMMIT);
            assert_eq!(
                format!("{:?}", data(&slug).unwrap()),
                format!("{native:?}"),
                "{slug}"
            );
        }
    }

    #[test]
    fn scenarios_offers_every_one_and_graziani_first_named() {
        let all = scenarios().unwrap();
        let slugs: Vec<&str> = all.iter().map(|(s, _)| s.as_str()).collect();
        assert!(slugs.contains(&DEFAULT_SCENARIO));
        assert!(all.contains(&("operation-crusader".into(), "Operation Crusader".into())));
        assert_eq!(all.len(), Embedded.list("scenarios").unwrap().len());
        assert!(slugs.contains(&"rommels-arrival"));
        assert!(data("no-such-scenario").is_err());
    }
}
