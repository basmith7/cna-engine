//! Errata from `cna/data/errata/E-*.json`: RFC 6902 patches applied to a
//! table at load.

use crate::source::Source;
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

/// Fails unless the errata names a table in `tables` (the file names of
/// `data/tables/`), so a misspelt `table` cannot leave an errata
/// silently unapplied.
pub fn check_target(errata: &Value, tables: &[String]) -> Result<()> {
    let id = errata["id"].as_str().unwrap_or("?");
    let Some(table) = errata["table"].as_str() else {
        anyhow::bail!("{id}: no `table` field");
    };
    if !tables.iter().any(|t| *t == format!("{table}.json")) {
        anyhow::bail!("{id}: names table `{table}`, which does not exist");
    }
    Ok(())
}

/// Reads `data/tables/<name>.json` from `src` and applies every errata
/// file whose `table` is `name`, in id order.
pub fn load_table(src: &dyn Source, name: &str) -> Result<Value> {
    let rel = format!("tables/{name}.json");
    let mut table: Value = serde_json::from_str(&src.read(&rel)?)
        .with_context(|| format!("parsing {rel}"))?;
    let tables = src.list("tables")?;
    let mut files: Vec<_> = src
        .list("errata")?
        .into_iter()
        .filter(|f| f.ends_with(".json"))
        .collect();
    files.sort();
    for f in files {
        let rel = format!("errata/{f}");
        let e: Value =
            serde_json::from_str(&src.read(&rel)?).with_context(|| format!("parsing {rel}"))?;
        check_target(&e, &tables)?;
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

    #[test]
    fn errata_for_a_missing_table_names_the_errata() {
        let tables = crate::source::Fs.list("tables").unwrap();
        let e = serde_json::json!({"id": "E-998", "table": "no-such-table", "patches": []});
        let err = check_target(&e, &tables).unwrap_err().to_string();
        assert!(err.contains("E-998"), "{err}");
        let e = serde_json::json!({"id": "E-997", "patches": []});
        assert!(check_target(&e, &tables).is_err());
    }
}
