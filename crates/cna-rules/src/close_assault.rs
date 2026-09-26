//! Close assault resolution (SPI 15.0–15.89) as exact dice distributions.

use crate::dice::{P, rolls};
use cna_data::close_assault::{CloseAssault, Side};

/// What one side's half of the table says for one reading.
#[derive(Debug, PartialEq)]
pub enum Lookup {
    Pct(u8),
    /// The reading falls in no row: a printed gap.
    Gap,
    /// The reading falls in several rows (their percentages).
    Overlap(Vec<u8>),
}

pub fn loss_pct(t: &CloseAssault, side: Side, column: &str, reading: u8) -> Lookup {
    let hits: Vec<u8> = t.losses[side.key()]
        .iter()
        .filter_map(|(pct, cols)| match cols.get(column).copied().flatten() {
            Some(r) if r.contains(reading) => Some(pct.parse().expect("pct row key")),
            _ => None,
        })
        .collect();
    match hits.as_slice() {
        [] => Lookup::Gap,
        [p] => Lookup::Pct(*p),
        _ => Lookup::Overlap(hits),
    }
}

/// One side's result in one column, over all 36 readings.
#[derive(Debug, Clone, Copy)]
pub struct SideOutcome {
    /// Expected percentage loss over the readings that resolve.
    pub expected_pct: f64,
    /// Probability of a reading in a gap or an overlap; never counted as 0 %.
    pub unresolved: f64,
    /// Expected forced-retreat hexes (defender only; 0 for the attacker).
    pub expected_retreat_hexes: f64,
}

pub fn outcome(t: &CloseAssault, side: Side, column: &str) -> SideOutcome {
    let mut o = SideOutcome {
        expected_pct: 0.0,
        unresolved: 0.0,
        expected_retreat_hexes: 0.0,
    };
    for (a, b) in rolls() {
        match loss_pct(t, side, column, a * 10 + b) {
            Lookup::Pct(p) => o.expected_pct += P * p as f64,
            _ => o.unresolved += P,
        }
        if side == Side::Defender {
            for (hexes, line) in [(1.0, "retreat_1"), (2.0, "retreat_2"), (3.0, "retreat_3")] {
                if let Some(Some(sums)) = t.sums[line].get(column)
                    && sums.contains(&(a + b))
                {
                    o.expected_retreat_hexes += P * hexes;
                }
            }
        }
    }
    o
}

/// Moves `by` columns (positive toward `+17`), clamping at the end columns.
pub fn shift(t: &CloseAssault, column: &str, by: i32) -> String {
    let i = t.column_index(column).expect("known column") as i32;
    let j = (i + by).clamp(0, t.columns.len() as i32 - 1) as usize;
    t.columns[j].id.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::close_assault::CloseAssault;

    fn t() -> CloseAssault {
        CloseAssault::load().unwrap()
    }

    #[test]
    fn rolls_are_36() {
        assert_eq!(crate::dice::rolls().count(), 36);
    }

    #[test]
    fn only_declared_gaps_exist() {
        // Every (side, column, reading) resolves to exactly one row, except known_gaps.
        let t = t();
        let mut gaps = vec![];
        for side in [Side::Attacker, Side::Defender] {
            for c in &t.columns {
                for (a, b) in crate::dice::rolls() {
                    match loss_pct(&t, side, &c.id, a * 10 + b) {
                        Lookup::Pct(_) => {}
                        Lookup::Gap => {
                            gaps.push((side.key().to_string(), c.id.clone(), a * 10 + b))
                        }
                        Lookup::Overlap(rows) => {
                            panic!("{} {} {}: in rows {rows:?}", side.key(), c.id, a * 10 + b)
                        }
                    }
                }
            }
        }
        let declared: Vec<_> = t
            .known_gaps
            .iter()
            .flat_map(|g| {
                g.readings
                    .iter()
                    .map(move |r| (g.side.clone(), g.column.clone(), *r))
            })
            .collect();
        assert_eq!(gaps, declared);
    }

    #[test]
    fn gap_probability_is_reported_not_zero_loss() {
        let o = outcome(&t(), Side::Defender, "+2");
        assert!((o.unresolved - 3.0 / 36.0).abs() < 1e-12);
    }

    #[test]
    fn shift_clamps_at_ends() {
        let t = t();
        assert_eq!(shift(&t, "+14", 5), "+17");
        assert_eq!(shift(&t, "-8", -3), "-11");
        assert_eq!(shift(&t, "0", 2), "+2");
    }

    #[test]
    fn hand_computed_attacker_minus_11() {
        // Rows at -11, counted over the 36 readings: 50 % x5, 40 % x5, 30 % x5,
        // 25 % x3, 20 % x6, 15 % x6, 10 % x3, 5 % x3.
        let o = outcome(&t(), Side::Attacker, "-11");
        let expected =
            (5 * 50 + 5 * 40 + 5 * 30 + 3 * 25 + 6 * 20 + 6 * 15 + 3 * 10 + 3 * 5) as f64 / 36.0;
        assert!((o.expected_pct - expected).abs() < 1e-9, "{o:?}");
        assert_eq!(o.unresolved, 0.0);
    }
}
