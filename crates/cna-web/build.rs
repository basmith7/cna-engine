//! Embeds `vendor/cna/data` in the crate: every file directly inside the
//! directories `cna_game::data::Data` reads, as `include_str!`s, and the
//! submodule's commit as `RULES_COMMIT`, so a save made in the browser
//! names the data it was played on.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directories `Data` reads. Files directly inside them only: the
/// loaders list nothing deeper (`map/raw`, `map/corrections` are cna's
/// own working files).
const DIRS: [&str; 5] = ["map", "tables", "oa", "scenarios", "errata"];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/cna");
    let data = root.join("data");
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for dir in DIRS {
        let path = data.join(dir);
        println!("cargo:rerun-if-changed={}", path.display());
        let entries = std::fs::read_dir(&path).unwrap_or_else(|e| {
            panic!("{}: {e} (run: git submodule update --init)", path.display())
        });
        for e in entries {
            let e = e.unwrap();
            if e.file_type().unwrap().is_file() {
                let name = e.file_name().into_string().unwrap();
                files.push((format!("{dir}/{name}"), e.path().canonicalize().unwrap()));
            }
        }
    }
    files.sort();

    let mut out =
        String::from("/// `(path relative to vendor/cna/data, contents)`, sorted by path.\n");
    out.push_str("pub static FILES: &[(&str, &str)] = &[\n");
    for (rel, path) in &files {
        writeln!(out, "    ({rel:?}, include_str!({:?})),", path.display()).unwrap();
    }
    out.push_str("];\n");
    writeln!(out, "/// The `vendor/cna` commit the files above came from.\npub const RULES_COMMIT: &str = {:?};", rules_commit(&root)).unwrap();
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(dest, out).unwrap();
}

/// `git -C vendor/cna rev-parse HEAD`, as `cna_data::rules_commit` does
/// natively. Re-run when the submodule's checkout moves.
fn rules_commit(root: &Path) -> String {
    if let Ok(dir) = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--git-dir"])
        .output()
    {
        let dir = String::from_utf8_lossy(&dir.stdout).trim().to_owned();
        let git_dir = if Path::new(&dir).is_absolute() {
            PathBuf::from(dir)
        } else {
            root.join(dir)
        };
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("running git");
    assert!(
        out.status.success(),
        "git rev-parse HEAD failed in vendor/cna"
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}
