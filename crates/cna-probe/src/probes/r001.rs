//! R-001: is the −26 collapse threshold a cohesion level or a count of
//! disorganisation points (SPI 6.26, 17.5)?

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_rules::cohesion::{COLLAPSE, Unit};
use cna_rules::ruleset::{R001, Ruleset};

const STAGES: u32 = 24;
/// DP earned in each pushing stage: CP spent over the CPA (SPI 6.21).
const PUSH_DP: u32 = 4;

pub fn probe(_ctx: &Ctx) -> Probe {
    // A unit that pushes PUSH_DP CP over its CPA, then rests a whole stage,
    // and so on. The ledger is the same under both options; only what
    // collapses it differs.
    let mut unit = Unit::default();
    let mut history = vec![];
    for stage in 1..=STAGES {
        if stage % 2 == 1 {
            unit.earn_dp(PUSH_DP);
        } else {
            unit.rest();
        }
        history.push(unit);
    }
    let collapse = |r001| {
        let rules = Ruleset {
            r001,
            ..Ruleset::default()
        };
        history
            .iter()
            .position(|u| u.collapsed(&rules))
            .map(|i| i + 1)
    };
    let describe = |c: Option<usize>| match c {
        Some(s) => format!("collapses at stage {s}"),
        None => format!("never collapses in {STAGES} stages"),
    };
    let lowest = history.iter().map(|u| u.level).min().unwrap_or(0);
    let finding = format!(
        "A unit that pushes {PUSH_DP} CP over its CPA and then rests, stage after stage, {} under option 1 (its level never falls below {lowest}) but {} under option 2, because a DP tally with no reset only grows. The rules give option 2 no reset, and the ruling notes one would have to be invented; any reset rule would move its collapse later.",
        describe(collapse(R001::CohesionLevel)),
        describe(collapse(R001::DpTally)),
    );
    Probe {
        id: "R-001".into(),
        kind: Kind::Line,
        question: format!(
            "A unit alternating a {PUSH_DP}-DP push with a stage of rest: when does it collapse (threshold {COLLAPSE})?"
        ),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Operations Stage".into(),
            values: (1..=STAGES).map(|s| s.to_string()).collect(),
        },
        y: Axis {
            label: "Collapse measure (collapse at 26)".into(),
            values: vec![],
        },
        series: vec![
            Series {
                option: Some(R001::CohesionLevel as u8),
                label: "Minus the cohesion level, floored at 0 (option 1)".into(),
                values: history.iter().map(|u| (-u.level).max(0) as f64).collect(),
            },
            Series {
                option: Some(R001::DpTally as u8),
                label: "DP tally, never reset (option 2)".into(),
                values: history.iter().map(|u| u.dp_tally as f64).collect(),
            },
        ],
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_stays_shallow_while_the_tally_collapses_at_stage_13() {
        let p = probe(&Ctx::load().unwrap());
        let xs: Vec<String> = (1..=24).map(|n| n.to_string()).collect();
        assert_eq!(p.x.values, xs);
        let s = |o| {
            &p.series
                .iter()
                .find(|s| s.option == Some(o))
                .unwrap()
                .values
        };
        assert!(s(1).iter().all(|v| *v <= 4.0));
        let first = s(2).iter().position(|v| *v >= 26.0).unwrap();
        assert_eq!(p.x.values[first], "13");
        assert!(p.finding.contains("13"));
    }
}
