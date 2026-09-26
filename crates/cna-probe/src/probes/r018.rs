//! R-018: what a supply dump costs, text (SPI 24.9) against charts (SPI
//! 24.17, 6.3).

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_rules::construction::dump_cost;
use cna_rules::ruleset::{R018, Ruleset};

/// An illustrative dump network: real dumps and dummies.
const REAL: u32 = 10;
const DUMMIES: u32 = 5;

pub fn probe(ctx: &Ctx) -> Probe {
    let mut series = vec![];
    let mut totals = vec![];
    for (opt, label) in [
        (R018::Text, "Text, 24.9 (option 1)"),
        (R018::Charts, "Charts, 24.17 and 6.3 (option 2)"),
        (
            R018::CpChartsStoresText,
            "CP from charts, stores from text (option 3)",
        ),
    ] {
        let rules = Ruleset {
            r018: opt,
            ..Ruleset::default()
        };
        let real = dump_cost(&rules, &ctx.construction, false);
        let dummy = dump_cost(&rules, &ctx.construction, true);
        totals.push((
            opt as u8,
            REAL * real.cp + DUMMIES * dummy.cp,
            REAL * real.stores + DUMMIES * dummy.stores,
        ));
        series.push(Series {
            option: Some(opt as u8),
            label: label.into(),
            values: vec![real.cp as f64, real.stores as f64, dummy.cp as f64],
        });
    }
    let each: Vec<String> = totals
        .iter()
        .map(|(o, cp, stores)| format!("option {o}: {cp} CP and {stores} stores"))
        .collect();
    let finding = format!(
        "The options agree on a real dump's CP and differ on a dummy's CP ({} or {}) and a real dump's stores ({} or {}; stores count only in the Logistics Game). {REAL} real dumps and {DUMMIES} dummies cost {}.",
        series[0].values[2],
        series[1].values[2],
        series[0].values[1],
        series[1].values[1],
        each.join("; ")
    );
    Probe {
        id: "R-018".into(),
        kind: Kind::Bar,
        question: "What a supply dump costs under each option".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Cost".into(),
            values: vec![
                "Real dump: CP".into(),
                "Real dump: stores".into(),
                "Dummy dump: CP".into(),
            ],
        },
        y: Axis {
            label: "CP or stores".into(),
            values: vec![],
        },
        series,
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_follow_dump_cost() {
        let ctx = Ctx::load().unwrap();
        let p = probe(&ctx);
        assert_eq!(
            p.x.values,
            ["Real dump: CP", "Real dump: stores", "Dummy dump: CP"]
        );
        for o in [R018::Text, R018::Charts, R018::CpChartsStoresText] {
            let rules = Ruleset {
                r018: o,
                ..Ruleset::default()
            };
            let real = dump_cost(&rules, &ctx.construction, false);
            let dummy = dump_cost(&rules, &ctx.construction, true);
            let s = p.series.iter().find(|s| s.option == Some(o as u8)).unwrap();
            assert_eq!(
                s.values,
                [real.cp as f64, real.stores as f64, dummy.cp as f64]
            );
        }
    }
}
