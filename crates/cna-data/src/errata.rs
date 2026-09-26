//! Errata from `cna/data/errata/E-*.json`: RFC 6902 patches applied to a
//! table at load.

use crate::cna_root;
use anyhow::{Context, Result};
use serde_json::Value;

/// Applies one errata file's patches. A patch that does not resolve fails
/// naming the errata id; it is never skipped.
pub fn apply_errata(table: &mut Value, errata: &Value) -> Result<()> {
    let id = errata["id"].as_str().unwrap_or("?");
    let patch: json_patch::Patch = serde_json::from_value(errata["patches"].clone())
        .with_context(|| format!("{id}: patches are not RFC 6902"))?;
    json_patch::patch(table, &patch).with_context(|| format!("{id}: patch does not apply"))?;
    Ok(())
}

/// Reads `data/tables/<name>.json` and applies every errata file whose
/// `table` is `name`, in id order.
pub fn load_table(name: &str) -> Result<Value> {
    let root = cna_root().join("data");
    let path = root.join("tables").join(format!("{name}.json"));
    let mut table: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )?;
    let mut files: Vec<_> = std::fs::read_dir(root.join("errata"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for f in files {
        let e: Value = serde_json::from_str(&std::fs::read_to_string(&f)?)
            .with_context(|| format!("parsing {}", f.display()))?;
        if e["table"] == name {
            apply_errata(&mut table, &e)?;
        }
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_patch_path_names_the_errata() {
        let mut table = serde_json::json!({"a": 1});
        let e = serde_json::json!({"id": "E-999", "table": "x",
            "patches": [{"op": "replace", "path": "/missing/deep", "value": 2}]});
        let err = apply_errata(&mut table, &e).unwrap_err().to_string();
        assert!(err.contains("E-999"), "{err}");
    }
}
