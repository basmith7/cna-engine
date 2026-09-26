//! R-015: does terrain weaken the non-phasing player's anti-armour fire at
//! armour assaulting out of protective ground (SPI 14.0, 14.32, 14.33)?

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_data::terrain::Shift;
use cna_rules::anti_armour::{expected_damage, terrain_shift};
use cna_rules::ruleset::{R015, Ruleset};

fn anti_armour_shift(ctx: &Ctx, kind: &str, terrain: &str) -> i32 {
    match ctx.terrain.shifts(kind, terrain).map(|s| &s.anti_armour) {
        Some(Shift::Cols(n)) => *n,
        other => panic!("{kind} {terrain} anti-armour shift: {other:?}"),
    }
}

pub fn probe(ctx: &Ctx) -> Probe {
    let hex = anti_armour_shift(ctx, "hex", "rough");
    let side = anti_armour_shift(ctx, "hexside", "up-slope");
    let points: Vec<u32> = (1..=16).collect();
    let series: Vec<Series> = [
        (R015::OwnHexBoth, "Own hex shifts it (option 1)"),
        (R015::PhasingOnly, "No shift (option 2)"),
        (
            R015::HexAndHexsideBoth,
            "Own hex and hexside shift it (option 3)",
        ),
    ]
    .into_iter()
    .map(|(opt, label)| {
        let rules = Ruleset {
            r015: opt,
            ..Ruleset::default()
        };
        // Fired by the non-phasing player at the assaulting armour.
        let shift = terrain_shift(&rules, false, hex, side);
        Series {
            option: Some(opt as u8),
            label: label.into(),
            values: points
                .iter()
                .map(|p| expected_damage(&ctx.anti_armour, *p, shift, false))
                .collect(),
        }
    })
    .collect();
    let base: f64 = series[1].values.iter().sum();
    let cut = |s: &Series| {
        let total: f64 = s.values.iter().sum();
        let worst = s
            .values
            .iter()
            .zip(&series[1].values)
            .map(|(v, b)| b - v)
            .fold(0.0, f64::max);
        (100.0 * (base - total) / base, worst)
    };
    let (c1, w1) = cut(&series[0]);
    let (c3, w3) = cut(&series[2]);
    let finding = format!(
        "For armour assaulting out of a rough hex up a slope, option 1 cuts the defender's expected anti-armour damage by {c1:.0} % summed over 1-16 points (at most {w1:.1} damage points), and option 3 by {c3:.0} % (at most {w3:.1}). Option 2 leaves it unshifted. Phasing fire is the same under every option. Assumes the firing side has at least five raw points (actual points are a fraction of raw), so the starred 0 column is reached only by shifts (SPI 14.33)."
    );
    Probe {
        id: "R-015".into(),
        kind: Kind::Line,
        question: "Defensive anti-armour fire at armour assaulting out of rough ground, up a slope"
            .into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Actual anti-armour points (non-phasing)".into(),
            values: points.iter().map(|p| p.to_string()).collect(),
        },
        y: Axis {
            label: "Expected damage points".into(),
            values: vec![],
        },
        series,
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_rules::anti_armour::expected_damage;

    #[test]
    fn options_order_and_option_2_is_unshifted() {
        let ctx = Ctx::load().unwrap();
        let p = probe(&ctx);
        let xs: Vec<String> = (1..=16).map(|n| n.to_string()).collect();
        assert_eq!(p.x.values, xs);
        let s = |o| {
            &p.series
                .iter()
                .find(|s| s.option == Some(o))
                .unwrap()
                .values
        };
        for i in 0..16 {
            let flat = expected_damage(&ctx.anti_armour, i as u32 + 1, 0, false);
            assert!((s(2)[i] - flat).abs() < 1e-9);
            assert!(s(3)[i] <= s(1)[i] && s(1)[i] <= s(2)[i]);
        }
        assert!(s(3)[15] < s(2)[15]);
    }
}
