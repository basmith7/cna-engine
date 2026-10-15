# cna-engine

A rules core for the Land Game of *The Campaign for North Africa*, written in
Rust. Every ruling in the [`cna`](https://github.com/basmith7/cna) repo's
`rulings/` is a switch here, so a *ruleset* is the printed rules plus one
chosen option per ruling. The rules data is read directly from `cna`, which
is a git submodule at `vendor/cna`.

New here? Read [`ARCHITECTURE.md`](ARCHITECTURE.md) to orient.

On top of the core sit *probes*: small programs that measure how a ruling's
options differ in play (expected losses, how often a case comes up) and
write the result to `probes/<id>.json` for the `cna` decision board. See
[the design](docs/designs/2026-09-26-engine-probes-design.md).

```bash
git clone --recursive https://github.com/basmith7/cna-engine.git
cargo test
cargo run -p cna-probe -- run
```

## Playing in a browser

`web/` is a local, two-players-at-one-screen UI for Graziani's Offensive
on the same engine, compiled to WebAssembly (`crates/cna-web`, with the
`vendor/cna` data built in). Nothing is hosted: it runs on localhost only.
See [the design](docs/designs/2026-10-13-minimal-ui-design.md).

```bash
rustup target add wasm32-unknown-unknown   # once
cargo install wasm-pack --version 0.15.0   # once; Node 22 as well
cd web && npm ci && npm run wasm && npm run dev
```

Open the address Vite prints. The start screen offers a new game (with
its seed shown, so a game can be repeated), the autosave as "Continue",
or a save file. In play: click a piece in the "to place" tray and then a
shaded hex; click a stack and then a shaded hex to move it along the
engine's cheapest path (Shift-click builds a path of your own, Enter
sends it); every interrupt the engine raises (ZOC declarations,
reactions, the sealed combat steps, retreats) is a panel offering only
what the engine computed; the log is the engine's events in English; the
victory report opens when the game ends. Every order goes to the engine
as the side it is waiting on and an illegal one is refused with its case;
the UI holds no rules of its own. Save downloads the state JSON
(`graziani-gt<N>-os<M>.json`); Load takes one back and refuses a file
from another `vendor/cna` commit or with its invariants broken; the state
is also autosaved to the browser's `localStorage` after every order.

**Secrecy is hot-seat only.** The screen avoids *showing* the other side's
sealed combat answers until both are in, and a curtain asks for the
keyboard to change hands (and comes down when a save or the autosave is
opened); but the autosave, a downloaded save and (in
test builds) `window.__cna` hold the whole state. Real hidden information
waits for the server.

Checks: `npm run check` (svelte-check and tsc), `npm test` (Vitest),
`npm run build`, `npx playwright test` (headless Chromium against the
production build; the golden game `tests/games/graziani-short.json` is
played through by clicks and must end in the same state as `replay`).

## Playing headless

`cna-game` is the game state for one scenario, **Graziani's Offensive**
(SPI 60.22, Land Game only), played from `vendor/cna`'s data: set-up with
free placements, the sequence of play (§5, §7) with initiative and weather,
movement with CP, terrain, roads, stacking, zones of control, reaction and
reserves (§6, §8–10, §18), the six combat steps (§11–15) on the rules core,
and victory at the end of Game-Turn 6 (60.8, with R-110's route trace for
the supply clause). Every order is validated before anything changes; an
illegal one is rejected with the SPI case it breaks. When the game is over
the state carries the victory report: each side's level and every
objective checked. Phases and systems the engine passes through are in
`NOT_SIMULATED.md`. `cna-play` drives it from files:

```bash
cargo run -p cna-play -- new grazianis-offensive --seed 7 -o state.json
cargo run -p cna-play -- show state.json
cargo run -p cna-play -- apply state.json orders.json   # [{"player": "axis", "order": {...}}, ...]
cargo run -p cna-play -- log state.json -o log.json
cargo run -p cna-play -- replay log.json -o replayed.json
```

A log is the scenario, the ruleset, the seed, the `vendor/cna` commit and
the accepted orders; it replays to the same state byte for byte on the same
commit. `tests/games/` holds scripted games the tests replay:
`setup.json` (set-up and one OpStage passed through) and
`graziani-short.json` (set-up, two OpStages of moves, a reaction, barrages
and a close assault, then passes to the end; its final state's SHA-256 and
victory report are pinned). A scripted game pins the commit and the
ruleset defaults, so a submodule bump or a changed default fails its
replay test until the fixture is regenerated (`cargo test -p cna-game --
--ignored regenerate_setup_fixture` and `... regenerate_graziani_short_fixture`,
then pin the hash the latter prints).

## Probes

| File | Question |
|---|---|
| `probes/R-001.json` | When a unit that alternates a 4-DP push with rest collapses, counting the cohesion level or a DP tally |
| `probes/R-009.json` | How often the non-phasing reply barrage pins or destroys phasing infantry in a fortification, with and without the terrain shift |
| `probes/R-011.json` | Loss as a share of each side's own strength, by size ratio, under each loss base |
| `probes/R-012.json` | The defender's expected close-assault loss when fighting, against withholding everything |
| `probes/R-015.json` | Defensive anti-armour damage against armour assaulting out of rough ground up a slope, under each option |
| `probes/R-018.json` | What a supply dump costs under each option (text against charts) |
| `probes/chart-oddities.json` | How often a roll lands on a printed oddity of the Close Assault Results Table or the Morale Modifier Table |
| `probes/chart-vs-text.json` | The guarded-dump raid check, chart ("at least") against text ("above"), plus the repair-facility cost gaps |

`cargo run -p cna-probe -- check` fails when a committed probe is stale (the
`vendor/cna` commit moved, or the output changed); CI runs it. Rulings with
no switch yet are listed, with the reason, in `NOT_SIMULATED.md`.

To see the probes on the decision board, from a `cna` checkout:

```bash
python3 tools/decisions_page.py --probes ~/path/to/cna-engine/probes
```
