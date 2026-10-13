//! Where the data comes from. Every loader reads through a `Source`, so
//! the same tables load from the `vendor/cna` checkout natively and from
//! strings compiled into a WebAssembly build.

use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// A read-only view of `vendor/cna/data`. Paths are relative to that
/// directory and `/`-separated (`tables/weather.json`, `errata`).
pub trait Source {
    /// The text of one file.
    fn read(&self, rel: &str) -> Result<String>;
    /// The names of the files directly in `dir`, sorted. Names only, not
    /// paths: `list("errata")` gives `E-008.json`, not `errata/E-008.json`.
    fn list(&self, dir: &str) -> Result<Vec<String>>;
}

/// The `vendor/cna/data` checkout. Native only.
pub struct Fs;

impl Source for Fs {
    fn read(&self, rel: &str) -> Result<String> {
        let path = crate::cna_root().join("data").join(rel);
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
    }

    fn list(&self, dir: &str) -> Result<Vec<String>> {
        let path = crate::cna_root().join("data").join(dir);
        let mut names = Vec::new();
        for e in std::fs::read_dir(&path).with_context(|| format!("listing {}", path.display()))? {
            let e = e?;
            if e.file_type()?.is_file() {
                names.push(e.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}

/// Files held in memory, keyed by their relative path.
#[derive(Debug, Default, Clone)]
pub struct MemSource {
    files: BTreeMap<String, String>,
}

impl MemSource {
    /// From `(path, text)` pairs; a path repeated takes the last text.
    pub fn new(files: impl IntoIterator<Item = (String, String)>) -> MemSource {
        MemSource {
            files: files.into_iter().collect(),
        }
    }

    /// Every file under `dirs` in `src`, read once. Only the files
    /// directly in each directory; the loaders read nothing deeper.
    pub fn copy_from(src: &dyn Source, dirs: &[&str]) -> Result<MemSource> {
        let mut files = BTreeMap::new();
        for dir in dirs {
            for name in src.list(dir)? {
                let rel = format!("{dir}/{name}");
                files.insert(rel.clone(), src.read(&rel)?);
            }
        }
        Ok(MemSource { files })
    }
}

impl Source for MemSource {
    fn read(&self, rel: &str) -> Result<String> {
        self.files
            .get(rel)
            .cloned()
            .with_context(|| format!("no such file: {rel}"))
    }

    fn list(&self, dir: &str) -> Result<Vec<String>> {
        let prefix = format!("{dir}/");
        Ok(self
            .files
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter(|rest| !rest.contains('/'))
            .map(str::to_owned)
            .collect())
    }
}

/// Reads and parses one JSON file.
pub fn read_json<T: serde::de::DeserializeOwned>(src: &dyn Source, rel: &str) -> Result<T> {
    serde_json::from_str(&src.read(rel)?).with_context(|| format!("parsing {rel}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_lists_sorted_names_and_reads_relative_paths() {
        let names = Fs.list("errata").unwrap();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert!(names.contains(&"E-008.json".to_owned()), "{names:?}");
        let map = Fs.list("map").unwrap();
        assert!(
            map.contains(&"hexes.json".to_owned()) && !map.contains(&"raw".to_owned()),
            "files only: {map:?}"
        );
        assert!(Fs.read("tables/weather.json").unwrap().contains("rows"));
        assert!(Fs.read("tables/no-such.json").is_err());
    }

    #[test]
    fn mem_source_lists_only_direct_children() {
        let m = MemSource::new([
            ("errata/E-1.json".to_owned(), "{}".to_owned()),
            ("errata/deep/x.json".to_owned(), "{}".to_owned()),
            ("tables/a.json".to_owned(), "{}".to_owned()),
        ]);
        assert_eq!(m.list("errata").unwrap(), vec!["E-1.json"]);
        assert_eq!(m.read("tables/a.json").unwrap(), "{}");
        assert!(m.read("tables/b.json").is_err());
    }

    #[test]
    fn mem_source_copied_from_fs_lists_the_same_files() {
        let m = MemSource::copy_from(&Fs, &["errata", "tables"]).unwrap();
        assert_eq!(m.list("errata").unwrap(), Fs.list("errata").unwrap());
        assert_eq!(m.list("tables").unwrap(), Fs.list("tables").unwrap());
    }

    #[test]
    fn copy_from_fails_on_a_file_it_cannot_read() {
        // Lists a file it then cannot read; the copy must not silently
        // drop it and surface later as a confusing "no such file".
        struct Listed;
        impl Source for Listed {
            fn read(&self, rel: &str) -> Result<String> {
                anyhow::bail!("cannot read {rel}")
            }
            fn list(&self, _: &str) -> Result<Vec<String>> {
                Ok(vec!["a.json".to_owned()])
            }
        }
        let err = MemSource::copy_from(&Listed, &["tables"]).unwrap_err();
        assert!(err.to_string().contains("tables/a.json"), "{err}");
    }
}
