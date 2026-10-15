//! Rewrites the generated blocks of `ARCHITECTURE.md`, or with `--check`
//! fails if they have drifted. Today there is one block, `crate-graph`: the
//! workspace's crate dependencies as a Mermaid flowchart, from
//! `cargo metadata`.
//!
//! ```text
//! cargo run --manifest-path tools/archdoc/Cargo.toml            # rewrite
//! cargo run --manifest-path tools/archdoc/Cargo.toml -- --check # CI
//! ```

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const BLOCK: &str = "crate-graph";

/// A Mermaid flowchart of the workspace members and the normal (non-dev,
/// non-build) dependencies between them. Dependencies on outside crates are
/// left out: the graph is the workspace's own layering.
fn render(metadata: &Value) -> Result<String> {
    let packages = metadata["packages"]
        .as_array()
        .context("metadata has no packages")?;
    let names: BTreeSet<&str> = packages.iter().filter_map(|p| p["name"].as_str()).collect();
    let mut edges = BTreeSet::new();
    for p in packages {
        let from = p["name"].as_str().context("package without a name")?;
        for d in p["dependencies"].as_array().into_iter().flatten() {
            let Some(to) = d["name"].as_str() else { continue };
            // `kind` is null for a normal dependency.
            if d["kind"].is_null() && names.contains(to) {
                edges.insert((from, to));
            }
        }
    }
    let mut out = String::from("```mermaid\nflowchart LR\n");
    for n in &names {
        out.push_str(&format!("    {}[\"{n}\"]\n", id(n)));
    }
    // Arrows point from a crate to what it depends on.
    for (from, to) in edges {
        out.push_str(&format!("    {} --> {}\n", id(from), id(to)));
    }
    out.push_str("```\n");
    Ok(out)
}

/// A Mermaid node id: crate names have hyphens.
fn id(name: &str) -> String {
    name.replace('-', "_")
}

/// `doc` with the text between the block's markers replaced by `body`.
fn splice(doc: &str, name: &str, body: &str) -> Result<String> {
    let open = format!("<!-- generated:{name} -->\n");
    let close = format!("<!-- /generated:{name} -->");
    let start = doc
        .find(&open)
        .with_context(|| format!("no `{}` marker", open.trim()))?
        + open.len();
    let end = start
        + doc[start..]
            .find(&close)
            .with_context(|| format!("no `{close}` marker"))?;
    Ok(format!("{}{}{}", &doc[..start], body, &doc[end..]))
}

fn workspace_root() -> PathBuf {
    // tools/archdoc -> the repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn main() -> Result<()> {
    let check = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--check") => true,
        Some(other) => bail!("usage: archdoc [--check] (got {other})"),
    };
    let root = workspace_root();
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .context("running cargo metadata")?;
    if !out.status.success() {
        bail!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr));
    }
    let metadata: Value = serde_json::from_slice(&out.stdout)?;
    let path = root.join("ARCHITECTURE.md");
    let doc = std::fs::read_to_string(&path).context("reading ARCHITECTURE.md")?;
    let new = splice(&doc, BLOCK, &render(&metadata)?)?;
    if new == doc {
        println!("ARCHITECTURE.md: {BLOCK} is current");
    } else if check {
        bail!(
            "ARCHITECTURE.md: the {BLOCK} block is stale; run `cargo run --manifest-path tools/archdoc/Cargo.toml`"
        );
    } else {
        std::fs::write(&path, new)?;
        println!("ARCHITECTURE.md: {BLOCK} rewritten");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta() -> Value {
        json!({"packages": [
            {"name": "app", "dependencies": [
                {"name": "lib-a", "kind": null},
                {"name": "serde", "kind": null},
                {"name": "test-only", "kind": "dev"}]},
            {"name": "lib-a", "dependencies": []},
            {"name": "test-only", "dependencies": []},
        ]})
    }

    #[test]
    fn graph_has_workspace_edges_only() {
        let g = render(&meta()).unwrap();
        assert!(g.contains("app --> lib_a"), "{g}");
        assert!(!g.contains("serde"), "outside crates are left out: {g}");
        assert!(!g.contains("app --> test_only"), "dev edges are left out: {g}");
        assert!(g.contains("lib_a[\"lib-a\"]"), "{g}");
    }

    #[test]
    fn splice_replaces_only_between_markers_and_is_idempotent() {
        let doc = "top\n<!-- generated:x -->\nold\n<!-- /generated:x -->\nbottom\n";
        let once = splice(doc, "x", "new\n").unwrap();
        assert_eq!(once, "top\n<!-- generated:x -->\nnew\n<!-- /generated:x -->\nbottom\n");
        assert_eq!(splice(&once, "x", "new\n").unwrap(), once);
        assert!(splice("no markers", "x", "").is_err());
    }
}
