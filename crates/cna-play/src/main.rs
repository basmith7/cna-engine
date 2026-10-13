//! `cna-play`: the headless game from the command line. Files in, files
//! out; the engine itself never touches the file system, so the reading,
//! the writing and `Data::load` (which asks git for the `vendor/cna`
//! commit) all happen here.

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use cna_game::data::Data;
use cna_game::log::{Log, replay};
use cna_game::order::LoggedOrder;
use cna_game::piece::Location;
use cna_game::state::GameState;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start a scenario and write its state.
    New {
        /// A scenario slug, e.g. `grazianis-offensive`.
        scenario: String,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(short, long, default_value = "state.json")]
        output: PathBuf,
    },
    /// Apply a file of orders (`[{"player": ..., "order": ...}, ...]`)
    /// to a state, in place, printing the events. Stops at the first
    /// illegal order without writing.
    Apply { state: PathBuf, orders: PathBuf },
    /// Write a state's log.
    Log {
        state: PathBuf,
        #[arg(short, long, default_value = "log.json")]
        output: PathBuf,
    },
    /// Rebuild a state from a log.
    Replay {
        log: PathBuf,
        #[arg(short, long, default_value = "state.json")]
        output: PathBuf,
    },
    /// A text summary of a state.
    Show { state: PathBuf },
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// A state file with the scenario's data: the scenario is read from the
/// file first, since loading a state checks it against the data.
fn load_state(path: &Path) -> Result<(Data, GameState)> {
    let json = read(path)?;
    let v: serde_json::Value = serde_json::from_str(&json).context("parsing the state")?;
    let Some(scenario) = v["scenario"].as_str() else {
        bail!("{}: no scenario", path.display());
    };
    let data = Data::load(scenario)?;
    let state = GameState::from_json(&json, &data)?;
    Ok((data, state))
}

fn write_state(path: &Path, state: &GameState) -> Result<()> {
    std::fs::write(path, state.to_json()?).with_context(|| format!("writing {}", path.display()))
}

fn show(state: &GameState) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", state.clock));
    match state.to_act() {
        Some(side) => out.push_str(&format!("waiting on {side}")),
        None => out.push_str("game over"),
    }
    if let Some(p) = &state.pending {
        out.push_str(&format!(" ({p:?})"));
    }
    out.push('\n');
    if let Some(i) = state.initiative {
        out.push_str(&format!("initiative: {i}"));
        if let Some(a) = state.player_a {
            out.push_str(&format!(", player A: {a}"));
        }
        out.push('\n');
    }
    out.push_str(&format!("weather: {:?}\n", state.weather));
    out.push_str(&format!("orders logged: {}\n", state.log.len()));
    let pending = state.pieces.values().filter(|p| p.is_pending()).count();
    out.push_str(&format!(
        "pieces: {}, pending: {pending}\n",
        state.pieces.len()
    ));
    let mut by_place: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in state.pieces.values() {
        let key = match &p.location {
            Location::Hex(h) => h.to_string(),
            Location::Box(b) => format!("box {b:?}"),
            Location::Transit { from, to, done } => format!("{from} to {to}, {done} stages along"),
            Location::Pending(_) => format!("pending ({})", p.side),
            Location::Eliminated => "eliminated".into(),
        };
        by_place.entry(key).or_default().push(p.id.to_string());
    }
    for (place, ids) in by_place {
        out.push_str(&format!("{place}: {} ({})\n", ids.len(), ids.join(", ")));
    }
    out
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::New {
            scenario,
            seed,
            output,
        } => {
            let data = Data::load(&scenario)?;
            let state = GameState::new(&data, seed)?;
            write_state(&output, &state)?;
            println!("{}", show(&state));
        }
        Cmd::Apply { state, orders } => {
            let (data, mut game) = load_state(&state)?;
            let list: Vec<LoggedOrder> =
                serde_json::from_str(&read(&orders)?).context("parsing the orders")?;
            for (i, o) in list.iter().enumerate() {
                match game.apply(&data, o.player, o.order.clone()) {
                    Ok(events) => {
                        for e in events {
                            println!("{}", serde_json::to_string(&e)?);
                        }
                    }
                    Err(e) => bail!("order {} ({:?}): {e}", i + 1, o.order),
                }
            }
            write_state(&state, &game)?;
        }
        Cmd::Log { state, output } => {
            let (_, game) = load_state(&state)?;
            let json = serde_json::to_string_pretty(&game.to_log())? + "\n";
            std::fs::write(&output, json)
                .with_context(|| format!("writing {}", output.display()))?;
        }
        Cmd::Replay { log, output } => {
            let log: Log = serde_json::from_str(&read(&log)?).context("parsing the log")?;
            let data = Data::load(&log.scenario)?;
            let state = replay(&data, &log)?;
            write_state(&output, &state)?;
            println!("{}", show(&state));
        }
        Cmd::Show { state } => {
            let (_, game) = load_state(&state)?;
            print!("{}", show(&game));
        }
    }
    Ok(())
}
