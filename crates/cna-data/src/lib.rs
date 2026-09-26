use std::path::PathBuf;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn cna_root() -> PathBuf {
    repo_root().join("vendor/cna")
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
