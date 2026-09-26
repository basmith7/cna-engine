//! R-009: does the target's terrain shift the non-phasing player's reply
//! barrage too (SPI 12.33, 14.0)?

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_data::terrain::Shift;
use cna_rules::barrage::{outcome, terrain_shift};
use cna_rules::ruleset::{R009, Ruleset};

/// The barrage shift of a level-two fortification, from the chart.
pub fn fortification_shift(ctx: &Ctx) -> i32 {
    match ctx
        .terrain
        .shifts("fortification", "fortification-2")
        .map(|s| &s.barrage)
    {
        Some(Shift::Cols(n)) => *n,
        other => panic!("fortification-2 barrage shift: {other:?}"),
    }
}

pub fn probe(ctx: &Ctx) -> Probe {
    let fort = fortification_shift(ctx);
    let bands = &ctx.barrage.bands;
    let mut series = vec![];
    let mut losses = vec![];
    for (opt, label) in [
        (R009::EitherSide, "Terrain shifts the reply (option 1)"),
        (R009::NonPhasingOnly, "Reply at full strength (option 2)"),
    ] {
        let rules = Ruleset {
            r009: opt,
            ..Ruleset::default()
        };
        // The reply is fired by the non-phasing player.
        let shift = terrain_shift(&rules, fort, false);
        let outs: Vec<_> = bands
            .iter()
            .map(|b| outcome(&ctx.barrage, "infantry", b.min, shift))
            .collect();
        losses.push(outs.iter().map(|o| o.expected_loss).collect::<Vec<_>>());
        series.push(Series {
            option: Some(opt as u8),
            label: label.into(),
            values: outs.iter().map(|o| 100.0 * o.p_effect).collect(),
        });
    }
    let (gap_i, gap) = series[1]
        .values
        .iter()
        .zip(&series[0].values)
        .map(|(two, one)| two - one)
        .enumerate()
        .fold((0, f64::MIN), |m, (i, g)| if g > m.1 { (i, g) } else { m });
    let (loss_i, loss_gap) = losses[1]
        .iter()
        .zip(&losses[0])
        .map(|(two, one)| two - one)
        .enumerate()
        .fold((0, f64::MIN), |m, (i, g)| if g > m.1 { (i, g) } else { m });
    let finding = format!(
        "Against phasing infantry in a level-two fortification (shifted {} bands under option 1), the reply barrage pins or destroys up to {gap:.0} percentage points more often under option 2, at {} points ({:.0} % against {:.0} %). Expected strength destroyed per barrage is up to {loss_gap:.2} TOE points higher under option 2 (at {} points).",
        -fort, bands[gap_i].id, series[1].values[gap_i], series[0].values[gap_i], bands[loss_i].id,
    );
    Probe {
        id: "R-009".into(),
        kind: Kind::Line,
        question: "Reply barrage against phasing infantry in a level-two fortification: how much does the terrain shift matter?".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Actual barrage points".into(),
            values: bands.iter().map(|b| b.id.clone()).collect(),
        },
        y: Axis {
            label: "% of barrages that pin or destroy".into(),
            values: vec![],
        },
        series,
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_rules::barrage::outcome;

    #[test]
    fn options_match_shifted_and_unshifted_outcomes() {
        let ctx = Ctx::load().unwrap();
        let p = probe(&ctx);
        assert_eq!(p.x.values.len(), 9);
        let fort = fortification_shift(&ctx);
        assert_eq!(fort, -2);
        let one = p.series.iter().find(|s| s.option == Some(1)).unwrap();
        let two = p.series.iter().find(|s| s.option == Some(2)).unwrap();
        for (i, b) in ctx.barrage.bands.iter().enumerate() {
            let shifted = outcome(&ctx.barrage, "infantry", b.min, fort).p_effect * 100.0;
            let flat = outcome(&ctx.barrage, "infantry", b.min, 0).p_effect * 100.0;
            assert!((one.values[i] - shifted).abs() < 1e-9);
            assert!((two.values[i] - flat).abs() < 1e-9);
        }
    }
}
