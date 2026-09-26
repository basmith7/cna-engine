//! `chart-oddities` (a decision-board item): how often a close-assault roll
//! lands on a printed oddity of the table (SPI 15.79).

use super::Ctx;
use crate::output::{Axis, Kind, Probe, Series};
use cna_rules::dice::rolls;

fn is_reading(n: u8) -> bool {
    (1..=6).contains(&(n / 10)) && (1..=6).contains(&(n % 10))
}

pub fn probe(ctx: &Ctx) -> Probe {
    let t = &ctx.table;
    let mut labels = vec![];
    let mut values = vec![];
    // Rows skipped by every range in a column (declared in known_gaps).
    for g in &t.known_gaps {
        let (from, to) = (
            g.readings.iter().min().copied().unwrap_or(0),
            g.readings.iter().max().copied().unwrap_or(0),
        );
        let hits = rolls()
            .filter(|(a, b)| g.readings.contains(&(a * 10 + b)))
            .count();
        labels.push(format!("{} {}: no row for {from}-{to}", g.side, g.column));
        values.push(100.0 * hits as f64 / 36.0);
    }
    // Ranges printed with an end that is not a reading, such as "13-18".
    // Only the readings that exist can be rolled, so the odd part never is.
    for (side, rows) in &t.losses {
        for (pct, cols) in rows {
            for (col, r) in cols {
                if let Some(r) = r
                    && !(is_reading(r.from) && is_reading(r.to))
                {
                    labels.push(format!(
                        "{side} {col}: {pct} % row reads {}-{}",
                        r.from, r.to
                    ));
                    values.push(0.0);
                }
            }
        }
    }
    let matter: Vec<String> = labels
        .iter()
        .zip(&values)
        .filter(|(_, v)| **v > 0.0)
        .map(|(l, v)| format!("{l} ({v:.1} % of rolls in that column)"))
        .collect();
    let never: Vec<&str> = labels
        .iter()
        .zip(&values)
        .filter(|(_, v)| **v == 0.0)
        .map(|(l, _)| l.as_str())
        .collect();
    let mut finding = String::new();
    if !matter.is_empty() {
        finding += &format!(
            "Comes up in play and needs a ruling: {}.",
            matter.join("; ")
        );
    }
    if !never.is_empty() {
        finding += &format!(
            "{}Cannot occur, since the odd part is not a reading two dice can make: {}.",
            if finding.is_empty() { "" } else { " " },
            never.join("; ")
        );
    }
    // Close assault only so far; the board item also names a Morale
    // Modifier cell, which the cohesion slice measures.
    finding += " The Morale Modifier cell on this item is not measured yet.";
    Probe {
        id: "chart-oddities".into(),
        kind: Kind::Bar,
        question: "How often does a roll land on a printed oddity?".into(),
        rules_commit: String::new(),
        engine_commit: String::new(),
        x: Axis {
            label: "Close Assault Results Table oddity".into(),
            values: labels,
        },
        y: Axis {
            label: "% of rolls".into(),
            values: vec![],
        },
        series: vec![Series {
            option: None,
            label: "% of rolls that land on the odd cell".into(),
            values,
        }],
        finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::close_assault::CloseAssault;

    #[test]
    fn the_13_18_cell_cannot_be_rolled_past_16_and_the_gap_is_3_in_36() {
        let p = probe(&Ctx {
            table: CloseAssault::load().unwrap(),
        });
        let s = &p.series[0];
        let i = p.x.values.iter().position(|x| x.contains("13-18")).unwrap();
        let j = p.x.values.iter().position(|x| x.contains("34-36")).unwrap();
        assert_eq!(s.values[i], 0.0); // readings 17 and 18 do not exist
        assert!((s.values[j] - 100.0 * 3.0 / 36.0).abs() < 1e-9);
    }
}
