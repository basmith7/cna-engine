//! `chart-vs-text` (a decision-board item): the printed disagreements
//! between the charts and the rules text other than R-018.

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_rules::construction::{TEXT_FACILITY_REBUILD, TEXT_TEMP_FACILITY};
use cna_rules::dice::sums;

pub fn probe(ctx: &Ctx) -> Probe {
    // Guarded supply dump (SPI 27.91 chart, 27.5x text): the raider gets
    // past the guards on a two-dice sum at least (chart) or above (text)
    // the guards' raw close-assault defence.
    let defences: Vec<u8> = (2..=12).collect();
    let survive = |d: u8, at_least: bool| {
        100.0
            * sums()
                .filter(|(s, _)| if at_least { *s >= d } else { *s > d })
                .map(|(_, p)| p)
                .sum::<f64>()
    };
    let chart: Vec<f64> = defences.iter().map(|d| survive(*d, true)).collect();
    let text: Vec<f64> = defences.iter().map(|d| survive(*d, false)).collect();
    let (gi, gap) = chart
        .iter()
        .zip(&text)
        .map(|(c, t)| c - t)
        .enumerate()
        .fold((0, 0.0), |m, (i, g)| if g > m.1 { (i, g) } else { m });

    let row = |item, situation| {
        ctx.construction
            .row(item, situation)
            .unwrap_or_else(|| panic!("{item} {situation} on the Construction Chart"))
    };
    let temp = row("temporary-repair-facility", "build");
    let rebuild = row("repair-facility", "rebuild-level");
    let (tf, ts, tt) = TEXT_TEMP_FACILITY;
    let (rf, rs) = TEXT_FACILITY_REBUILD;
    let finding = format!(
        "Guarded dump raids: the chart's 'at least' lets the raider past the guards up to {gap:.1} percentage points more often than the text's 'above', at a defence of {} ({:.1} % against {:.1} %); the gap is exactly the chance of rolling the defence. Temporary repair facility: the chart asks {} fuel and {} stores in {} stage, the text (24.82) {tf} fuel and {ts} stores over {tt} stages. Facility rebuild: the chart asks {} fuel and {} stores, the text (24.84) {rf} fuel and {rs} stores.",
        defences[gi],
        chart[gi],
        text[gi],
        temp.supplies.fuel.unwrap_or(0),
        temp.supplies.stores.unwrap_or(0),
        temp.stages.unwrap_or(0),
        rebuild.supplies.fuel.unwrap_or(0),
        rebuild.supplies.stores.unwrap_or(0),
    );
    Probe {
        id: "chart-vs-text".into(),
        kind: Kind::Line,
        question: "Chart against text: how much the printed disagreements matter".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Guards' raw close-assault defence".into(),
            values: defences.iter().map(|d| d.to_string()).collect(),
        },
        y: Axis {
            label: "% of raids that get past the guards".into(),
            values: vec![],
        },
        series: vec![
            Series {
                option: None,
                label: "Chart (27.91): sum at least the defence".into(),
                values: chart,
            },
            Series {
                option: None,
                label: "Text (27.5x): sum above the defence".into(),
                values: text,
            },
        ],
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_rules::dice::sums;

    #[test]
    fn the_gap_is_the_chance_of_a_tie() {
        let p = probe(&Ctx::load().unwrap());
        let xs: Vec<String> = (2..=12).map(|n| n.to_string()).collect();
        assert_eq!(p.x.values, xs);
        let (chart, text) = (&p.series[0].values, &p.series[1].values);
        for (i, (_, prob)) in sums().enumerate() {
            assert!((chart[i] - text[i] - 100.0 * prob).abs() < 1e-9);
        }
        assert!((chart[5] - text[5] - 100.0 / 6.0).abs() < 1e-9);
        assert!(p.finding.contains("50 fuel") && p.finding.contains("10 fuel"));
    }
}
