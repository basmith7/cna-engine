//! R-011: whose raw points form the percentage-loss base (SPI 15.83b), by
//! the size ratio of the two sides.

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_data::close_assault::Side;
use cna_rules::close_assault::{loss_base_factor, outcome, shift};
use cna_rules::ruleset::{R011, Ruleset};

const RATIOS: [(u32, u32); 7] = [(1, 4), (1, 3), (1, 2), (1, 1), (2, 1), (3, 1), (4, 1)];

/// The column for equal actual strength after the double raw strength shift
/// (SPI 15.51): two columns toward a side with at least twice the other's.
fn column(ctx: &Ctx, a: u32, d: u32) -> String {
    let by = if a >= 2 * d {
        2
    } else if d >= 2 * a {
        -2
    } else {
        0
    };
    shift(&ctx.table, "0", by)
}

pub fn probe(ctx: &Ctx) -> Probe {
    let mut series = vec![];
    let mut capped = false;
    let mut worst = (1.0f64, String::new(), String::new());
    for side in [Side::Attacker, Side::Defender] {
        for (opt, name) in [
            (R011::OwnRaw, "own raw"),
            (R011::CombinedRaw, "combined raw"),
        ] {
            let rules = Ruleset {
                r011: opt,
                ..Ruleset::default()
            };
            let values = RATIOS
                .iter()
                .map(|&(a, d)| {
                    let pct = outcome(&ctx.table, side, &column(ctx, a, d)).expected_pct;
                    let factor = loss_base_factor(&rules, side, a as f64, d as f64);
                    if factor > worst.0 {
                        worst = (factor, format!("{a}:{d}"), side.key().to_string());
                    }
                    let v = pct * factor;
                    capped |= v > 100.0;
                    v.min(100.0)
                })
                .collect();
            let who = match side {
                Side::Attacker => "Attacker",
                Side::Defender => "Defender",
            };
            series.push(Series {
                option: Some(opt as u8),
                label: format!("{who}, {name} (option {})", opt as u8),
                values,
            });
        }
    }
    let (wi, ws) = (
        RATIOS
            .iter()
            .position(|(a, d)| format!("{a}:{d}") == worst.1)
            .expect("worst ratio"),
        if worst.2 == "attacker" { 0 } else { 2 },
    );
    let finding = format!(
        "With equal actual strength (column 0 before the double-strength shift), option 2 multiplies the smaller side's loss by the combined total over its own raw points: up to {:.0}x for the {} at {}, whose expected loss goes from {:.1} % of its strength to {:.1} %.{}",
        worst.0,
        worst.2,
        worst.1,
        series[ws].values[wi],
        series[ws + 1].values[wi],
        if capped {
            " Some option 2 values exceed 100 % of the side's strength and are capped at 100."
        } else {
            " No value reaches 100 % of a side's strength in expectation."
        }
    );
    Probe {
        id: "R-011".into(),
        kind: Kind::Line,
        question: "Loss as a share of each side's own strength, by size ratio (column 0 before the double-strength shift)".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Raw strength, attacker:defender".into(),
            values: RATIOS.iter().map(|(a, d)| format!("{a}:{d}")).collect(),
        },
        y: Axis {
            label: "Expected loss, % of the side's own raw points".into(),
            values: vec![],
        },
        series,
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::close_assault::CloseAssault;

    #[test]
    fn combined_base_multiplies_small_side_losses() {
        let ctx = Ctx {
            table: CloseAssault::load().unwrap(),
        };
        let p = probe(&ctx);
        assert_eq!(
            p.x.values,
            ["1:4", "1:3", "1:2", "1:1", "2:1", "3:1", "4:1"]
        );
        let own = p
            .series
            .iter()
            .find(|s| s.label == "Attacker, own raw (option 1)")
            .unwrap();
        let comb = p
            .series
            .iter()
            .find(|s| s.label == "Attacker, combined raw (option 2)")
            .unwrap();
        // At 1:4 the combined base is 5x the attacker's own.
        assert!((comb.values[0] - own.values[0] * 5.0).abs() < 1e-9);
    }
}
