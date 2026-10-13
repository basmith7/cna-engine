use std::path::PathBuf;

pub mod anti_armour;
pub mod assault_size;
pub mod barrage;
pub mod characteristics;
pub mod close_assault;
pub mod construction;
pub mod cp;
pub mod errata;
pub mod initiative;
pub mod map;
pub mod morale;
pub mod oa;
pub mod off_map;
pub mod prisoners;
pub mod scenario;
pub mod schedule;
pub mod source;
pub mod stacking;
pub mod terrain;
pub mod time;
pub mod weapons;
pub mod weather;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn cna_root() -> PathBuf {
    repo_root().join("vendor/cna")
}

/// The commit `vendor/cna` is checked out at: what every probe and saved
/// game records as `rules_commit`.
pub fn rules_commit() -> anyhow::Result<String> {
    use anyhow::Context;
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cna_root())
        .args(["rev-parse", "HEAD"])
        .output()
        .context("running git")?;
    if !out.status.success() {
        anyhow::bail!("git rev-parse HEAD failed in vendor/cna");
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submodule_is_checked_out() {
        assert!(
            cna_root()
                .join("data/tables/close-assault-results.json")
                .is_file(),
            "run: git submodule update --init"
        );
    }
}
