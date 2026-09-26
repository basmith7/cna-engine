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
