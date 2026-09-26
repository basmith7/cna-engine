//! R-012: is withholding everything (SPI 15.29) with the 15.82 buy-out ever
//! cheaper for the defender than fighting?

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_data::close_assault::Side;
use cna_rules::close_assault::{may_buy_out, outcome};
use cna_rules::ruleset::{R012, Ruleset};

/// The buy-out: stay put and pay 10 % per hex of the three not retreated.
const BUY_OUT_PCT: f64 = 30.0;

pub fn probe(ctx: &Ctx) -> Probe {
    let n = ctx.table.columns.len();
    // What staying put costs a withholding defender with a retreat path:
    // the buy-out where allowed, otherwise nothing (it must retreat).
    let withhold = |r012: R012, label: &str| {
        let rules = Ruleset {
            r012,
            ..Ruleset::default()
        };
        let pct = if may_buy_out(&rules, true) {
            BUY_OUT_PCT
        } else {
            0.0
        };
        Series {
            option: Some(r012 as u8),
            label: label.into(),
            values: vec![pct; n],
        }
    };
    let cols: Vec<String> = ctx.table.columns.iter().map(|c| c.id.clone()).collect();
    let fight: Vec<f64> = cols
        .iter()
        .map(|c| outcome(&ctx.table, Side::Defender, c).expected_pct)
        .collect();
    // The first column from which fighting costs more than the buy-out in
    // every column to its right.
    let from = (0..cols.len())
        .find(|&i| fight[i..].iter().all(|f| *f > BUY_OUT_PCT))
        .filter(|_| fight.iter().any(|f| *f > BUY_OUT_PCT));
    let finding = match from {
        None if fight.iter().all(|f| *f <= BUY_OUT_PCT) => {
            let (i, worst) = fight
                .iter()
                .enumerate()
                .fold((0, 0.0), |m, (i, f)| if *f > m.1 { (i, *f) } else { m });
            format!(
                "Fighting costs the defender less than the {BUY_OUT_PCT:.0} % buy-out on average in every column, so under option 2 withholding everything is never cheaper in expectation; the closest is column {} ({worst:.1} % when fighting). The buy-out's appeal is certainty and avoiding Engaged, not a lower average loss.",
                cols[i]
            )
        }
        None => format!(
            "Fighting costs the defender more than {BUY_OUT_PCT:.0} % on average in some columns, but not in a run up to the last column; see the chart."
        ),
        Some(i) => format!(
            "Under option 2 (and option 3 when a retreat path exists) a defender facing column {} or worse loses less by withholding everything and paying {BUY_OUT_PCT:.0} % than by fighting: expected loss when fighting is {:.1} % at {} and {:.1} % at {}. Under option 1 withholding costs no strength at all, only the 3 DP and the three hexes.",
            cols[i],
            fight[i],
            cols[i],
            fight[cols.len() - 1],
            cols[cols.len() - 1]
        ),
    };
    Probe {
        id: "R-012".into(),
        kind: Kind::Line,
        question: "Defender's expected loss when fighting, against withholding everything".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Final close-assault column".into(),
            values: cols.clone(),
        },
        y: Axis {
            label: "Defender loss, % of raw points".into(),
            values: vec![],
        },
        series: vec![
            Series {
                option: None,
                label: "Fight (expected)".into(),
                values: fight,
            },
            withhold(R012::MustRetreat, "Withhold: retreat 3 hexes, 0 % (+3 DP)"),
            // Option 3 equals option 2 whenever a retreat path exists (the
            // ruling's option 3 keeps the buy-out), so it is not plotted.
            withhold(R012::BuyOut, "Withhold: stay, pay 30 % (+3 DP)"),
        ],
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::close_assault::CloseAssault;

    #[test]
    fn buy_out_is_flat_30_and_fight_matches_outcome() {
        let ctx = Ctx {
            table: CloseAssault::load().unwrap(),
        };
        let p = probe(&ctx);
        assert_eq!(p.x.values.len(), 18);
        let buy = p.series.iter().find(|s| s.option == Some(2)).unwrap();
        assert!(buy.values.iter().all(|v| *v == 30.0));
        let fight = p.series.iter().find(|s| s.option.is_none()).unwrap();
        let o = cna_rules::close_assault::outcome(&ctx.table, Side::Defender, "+11");
        assert!((fight.values[15] - o.expected_pct).abs() < 1e-9);
        assert!(p.finding.contains("column"));
    }
}
