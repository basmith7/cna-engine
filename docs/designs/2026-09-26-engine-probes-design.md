# cna-engine: rules core and ruling probes

**Date:** 2026-09-26
**Sub-project:** 6 of the digital CNA roadmap (engine), slice 1.
**Status:** proposed.

Brian delegated the design decisions on 2026-09-26 ("make most of the
decisions, especially if they are easily changeable"). Every decision in this
file was made on that basis and can be reversed with one PR. The decision
policy for rulings is in the last section.

## Goal

The Land Game rules of *The Campaign for North Africa* (restated in the `cna`
repo) contain contradictions and gaps. Each one is a ruling in
`cna/rulings/`, and each ruling offers numbered options. This project builds:

1. **A rules core in Rust**, where every ruling option is a switch. A
   *ruleset* is the printed rules plus one chosen option per ruling.
2. **Probes**: small programs that measure how the options differ in play,
   such as expected losses, how often a case comes up, or supply cost.
3. **Probe results on the decision board**, as a chart inside each ruling's
   card, so a ruling is decided with its evidence next to it.

The core is the first piece of the eventual engine, so it is built as that
engine's foundation and not as throwaway scripts. It grows only as far as the
probes need, one rules system at a time.

## Non-goals for slice 1

Movement, the map, supply, weather, the air and logistics games, AI players,
whole-game simulation, a server and a UI. A whole-game simulator cannot
settle rulings without players that play competently. That is a research
problem for later, and none of slice 1 depends on it.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Repo | `cna-engine`, separate from `cna` | The rules repo's design keeps engine code out of it. |
| Licence | MIT, matching `cna`'s `tools/` | |
| Language | Rust, stable, edition 2024, one cargo workspace | Fast and deterministic; it can compile to WebAssembly for a browser client later. |
| Rules data | `cna` as a git submodule at `vendor/cna`, pinned to a commit | Moving to newer rules is a deliberate bump. Every probe result records the commit it ran against. |
| Tables | Read directly from `vendor/cna/data/tables/*.json` with serde; errata applied at load from `data/errata/*.json` (JSON-Patch `replace` ops) | No second copy of any table. |
| Randomness | Exact enumeration of dice outcomes by default (a sequential two-dice reading has 36 outcomes, a dice sum has 11). Monte Carlo with a seeded ChaCha8 RNG only where the state space is too large to enumerate | Exact probabilities, reproducible, and fast enough. |
| Ruling switches | One enum per ruling in `cna-rules::ruleset`. The variant numbers match the option numbers in the ruling file. The default is the option the printed text supports, as each ruling's Rationale identifies it | A ruleset serialises as `{"R-012": 1, "R-013": 2}`, the same numbers used on the board. |
| Coverage | A test reads `vendor/cna/rulings/R-*.md`. Every ruling must either have a switch or be listed in `NOT_SIMULATED.md` with a one-line reason | Rulings cannot silently go unmodelled. |

### Crates

- **`cna-data`**: loads and validates the tables and applies errata. It has
  no game logic.
- **`cna-rules`**: pure resolution functions (for example
  `close_assault(&Ruleset, &Assault) -> Distribution<Outcome>`), plus
  `Ruleset`. It does no I/O.
- **`cna-probe`**: a binary. `cna-probe run` writes `probes/<ruling>.json` for
  every probe. `cna-probe check` fails when a committed result is stale (a
  different submodule commit, or the output no longer matches).

### Probe output (`probes/R-012.json`)

```json
{
  "ruling": "R-012",
  "question": "Expected strength lost: fight, or withhold everything",
  "rules_commit": "<vendor/cna sha>",
  "engine_commit": "<sha>",
  "x": {"label": "Close-assault differential", "values": [-11, -10, ..., 17]},
  "y": {"label": "Defender loss, % of raw points"},
  "series": [
    {"option": 1, "label": "Retreat in full (3 DP)", "values": [...]},
    {"option": 2, "label": "15.82 buy-out", "values": [...]}
  ],
  "finding": "One or two plain sentences: where the options differ and by how much."
}
```

The `finding` is written by the probe's code from the numbers, not by hand.

### The board

`cna/tools/decisions_page.py` gains `--probes <dir>`. For each ruling with a
probe file, the board draws an inline SVG chart (one line per option, using
the board's own palette, no chart library) and prints the finding, just under
the options. Rulings without a probe look as they do now.

## Slices

1. **Close assault (§15).** R-011 (whose raw points form the loss base),
   R-012 (withhold vs fight), R-013 (ammunition for pinned units), and the two
   printed oddities on the close-assault table ("13-18" and the 34–36 gap),
   including how often a roll actually lands on them.
2. **Barrage and anti-armour (§12, §14).** R-009, R-015.
3. **Cohesion (§6, §17).** R-001, and the morale-table oddity (no cell for 56).
4. **Construction costs (§24).** R-018, and the chart-vs-text cost
   disagreements.

The water rulings (R-020–R-022) wait for the Logistics Game (§47–58) to be
restated in `cna`. That is sub-project 4.

## Testing

- Golden tests: a sample of cells per table, checked against the JSON source
  and with errata applied.
- Distribution tests: every outcome distribution sums to 1. Each ruling
  switch must change the output of at least one probe; a switch that changes
  nothing is a bug. A ruling with no probe yet is listed in `NOT_SIMULATED.md`
  and gets no switch.
- One test per probe, on a hand-computed case.
- CI: `cargo test`, `cargo clippy -D warnings`, `cna-probe check`.

## Who builds it

The autopilot, as Mission 1 of `cna-engine`: its own `AUTOPILOT.md` and
`docs/autopilot/PROGRESS.md`, and a cron entry calling
`~/Scripts/claude-autopilot.sh` for this repo, the same way `cna` is run. One
PR per slice.

## Decision policy for rulings (the `cna` side)

Brian wants to be out of the loop wherever a decision is cheap to reverse.
Rulings are cheap to reverse: every ruling can be disputed and superseded
under `cna/rulings/README.md`, and in the engine it is one switch. So:

- **Rulings a probe covers** (slices 1–4): once the probe has landed, the
  autopilot decides. It picks an option, writes the Decision and Rationale
  citing the probe's finding, sets `status: accepted`, and edits the rules
  prose, one PR per ruling as the README requires.
- **Interpretive rulings** that a probe cannot settle (R-002–R-008, R-010,
  R-014, R-016, R-017, R-019): the autopilot decides them now, on the text
  and consistency with rulings already accepted, in the same way.
- **Map, chart and site questions** on the board: the autopilot adopts the
  recommended default for each one (in general, whichever keeps the data
  as read or as printed) and records it.
- **Only Brian** can answer questions of fact that need his printed copy:
  river classes, hill bands, which railways were finished, SPI 20.67, the
  two formation-chart reads, and the rows the scan cannot settle. Each stays
  as read until he looks. The board shows only these, marked optional.
- `cna/rulings/README.md` records that rulings are decided under Brian's
  delegation and stay open to dispute.
