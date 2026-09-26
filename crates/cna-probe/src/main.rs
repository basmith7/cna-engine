mod output;
mod probes;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Runs the ruling probes and checks that `probes/` is fresh.
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write `probes/<id>.json` for every probe.
    Run,
    /// Fail if any committed probe file is stale, missing or orphaned.
    Check,
}

/// Not HEAD: embedding the engine's own commit would make every commit
/// stale its own output.
const ENGINE_COMMIT: &str = "see git log";

fn probes_dir() -> PathBuf {
    cna_data::repo_root().join("probes")
}

fn rules_commit() -> Result<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cna_data::cna_root())
        .args(["rev-parse", "HEAD"])
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!("git rev-parse HEAD failed in vendor/cna");
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

/// Every probe as (file name, pretty JSON with a trailing newline).
fn render_all(rules: &str, engine: &str) -> Result<Vec<(String, String)>> {
    let ctx = probes::Ctx::load()?;
    probes::all()
        .into_iter()
        .map(|(id, f)| {
            let mut p = f(&ctx);
            assert_eq!(p.id, id, "probe registered under the wrong id");
            p.rules_commit = rules.to_string();
            p.engine_commit = engine.to_string();
            Ok((
                format!("{id}.json"),
                serde_json::to_string_pretty(&p)? + "\n",
            ))
        })
        .collect()
}

/// A probe file with `engine_commit` removed, for comparison.
fn comparable(json: &str) -> Result<serde_json::Value> {
    let mut v: serde_json::Value = serde_json::from_str(json)?;
    if let Some(o) = v.as_object_mut() {
        o.remove("engine_commit");
    }
    Ok(v)
}

fn stale_files(rendered: &[(String, String)]) -> Result<Vec<String>> {
    let dir = probes_dir();
    let mut problems = vec![];
    for (name, contents) in rendered {
        match std::fs::read_to_string(dir.join(name)) {
            Err(_) => problems.push(format!("{name}: missing")),
            Ok(disk) => {
                if comparable(&disk).ok() != Some(comparable(contents)?) {
                    problems.push(format!("{name}: stale"));
                }
            }
        }
    }
    if dir.is_dir() {
        for e in std::fs::read_dir(&dir)? {
            let name = e?.file_name().into_string().unwrap_or_default();
            if name.ends_with(".json") && !rendered.iter().any(|(n, _)| *n == name) {
                problems.push(format!("{name}: no probe writes it"));
            }
        }
    }
    problems.sort();
    Ok(problems)
}

fn main() -> Result<()> {
    let rendered = render_all(&rules_commit()?, ENGINE_COMMIT)?;
    match Cli::parse().cmd {
        Cmd::Run => {
            std::fs::create_dir_all(probes_dir())?;
            for (name, contents) in &rendered {
                std::fs::write(probes_dir().join(name), contents)?;
                println!("wrote probes/{name}");
            }
        }
        Cmd::Check => {
            let problems = stale_files(&rendered)?;
            if !problems.is_empty() {
                for p in &problems {
                    eprintln!("probes/{p}");
                }
                bail!("probes/ is out of date: run `cargo run -p cna-probe -- run`");
            }
            println!("probes/ is up to date ({} files)", rendered.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staleness_follows_rules_commit() {
        let a = render_all("aaa", "e").unwrap();
        let b = render_all("bbb", "e").unwrap();
        assert_ne!(a, b);
        assert!(a.iter().any(|(n, _)| n == "R-012.json"));
    }

    #[test]
    fn every_switch_changes_its_probe() {
        // Spec: a switch that changes no probe output is a bug.
        let ctx = probes::Ctx::load().unwrap();
        let all = probes::all();
        for id in cna_rules::ruleset::SWITCHED {
            let (_, f) = all
                .iter()
                .find(|(pid, _)| pid == id)
                .unwrap_or_else(|| panic!("{id} has a switch but no probe"));
            let p = f(&ctx);
            let options: Vec<_> = p.series.iter().filter(|s| s.option.is_some()).collect();
            let differ = options.iter().any(|a| {
                options
                    .iter()
                    .any(|b| a.option != b.option && a.values != b.values)
            });
            assert!(differ, "{id}: no two options differ in its probe");
        }
    }
}
