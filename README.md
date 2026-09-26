# cna-engine

A rules core for the Land Game of *The Campaign for North Africa*, written in
Rust. Every ruling in the [`cna`](https://github.com/basmith7/cna) repo's
`rulings/` is a switch here, so a *ruleset* is the printed rules plus one
chosen option per ruling. The rules data is read directly from `cna`, which
is a git submodule at `vendor/cna`.

On top of the core sit *probes*: small programs that measure how a ruling's
options differ in play (expected losses, how often a case comes up) and
write the result to `probes/<id>.json` for the `cna` decision board. See
[the design](docs/designs/2026-09-26-engine-probes-design.md).

```bash
git clone --recursive https://github.com/basmith7/cna-engine.git
cargo test
cargo run -p cna-probe -- run
```

## Probes

| File | Question |
|---|---|
| `probes/R-009.json` | How often the non-phasing reply barrage pins or destroys phasing infantry in a fortification, with and without the terrain shift |
| `probes/R-011.json` | Loss as a share of each side's own strength, by size ratio, under each loss base |
| `probes/R-012.json` | The defender's expected close-assault loss when fighting, against withholding everything |
| `probes/R-015.json` | Defensive anti-armour damage against armour assaulting out of rough ground up a slope, under each option |
| `probes/chart-oddities.json` | How often a roll lands on a printed oddity of the Close Assault Results Table |

`cargo run -p cna-probe -- check` fails when a committed probe is stale (the
`vendor/cna` commit moved, or the output changed); CI runs it. Rulings with
no switch yet are listed, with the reason, in `NOT_SIMULATED.md`.

To see the probes on the decision board, from a `cna` checkout:

```bash
python3 tools/decisions_page.py --probes ~/path/to/cna-engine/probes
```
