//! Close assault resolution (SPI 15.0–15.89) as exact dice distributions.

use crate::dice::{P, rolls};
use crate::ruleset::{R011, R012, Ruleset};
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

/// The column a basic differential reads (15.2x): `-11` for −11 and
/// worse, `+17` for +17 and better.
pub fn column_for(t: &CloseAssault, differential: i32) -> &str {
    t.columns
        .iter()
        .find(|c| {
            c.min.is_none_or(|m| differential >= m) && c.max.is_none_or(|m| differential <= m)
        })
        .map(|c| c.id.as_str())
        .expect("the columns cover every differential")
}

/// `loss_pct` with the printed gaps decided: the defender's +2 column
/// reads 34–36 as 5 % (R-023). Any other gap is read as no loss, which
/// `only_declared_gaps_exist` guarantees never happens.
pub fn loss_pct_decided(t: &CloseAssault, side: Side, column: &str, reading: u8) -> u8 {
    match loss_pct(t, side, column, reading) {
        Lookup::Pct(p) => p,
        Lookup::Gap if side == Side::Defender && column == "+2" && (34..=36).contains(&reading) => {
            5
        }
        Lookup::Gap | Lookup::Overlap(_) => 0,
    }
}

/// Whether the summed dice land on a line of the column (15.7x).
pub fn on_line(t: &CloseAssault, line: &str, column: &str, sum: u8) -> bool {
    matches!(t.sums[line].get(column), Some(Some(sums)) if sums.contains(&sum))
}

/// What one side's single roll says (15.7x): read sequentially for the
/// percentage, summed for the engaged or retreat line and again for
/// the captured line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideRoll {
    pub pct: u8,
    /// Attacker's half only.
    pub engaged: bool,
    /// Defender's half only: hexes to retreat, 0 for none.
    pub retreat: u32,
    /// This side's losses are partly prisoners.
    pub captured: bool,
}

/// The game's sampling entry point: one side's result on one reading.
pub fn resolve_side(t: &CloseAssault, side: Side, column: &str, reading: u8) -> SideRoll {
    let sum = reading / 10 + reading % 10;
    let pct = loss_pct_decided(t, side, column, reading);
    match side {
        Side::Attacker => SideRoll {
            pct,
            engaged: on_line(t, "engaged", column, sum),
            retreat: 0,
            captured: on_line(t, "capture_attacker", column, sum),
        },
        Side::Defender => SideRoll {
            pct,
            engaged: false,
            retreat: [(3, "retreat_3"), (2, "retreat_2"), (1, "retreat_1")]
                .into_iter()
                .find(|(_, line)| on_line(t, line, column, sum))
                .map_or(0, |(n, _)| n),
            captured: on_line(t, "capture_defender", column, sum),
        },
    }
}

/// Moves `by` columns (positive toward `+17`), clamping at the end columns.
pub fn shift(t: &CloseAssault, column: &str, by: i32) -> String {
    let i = t.column_index(column).expect("known column") as i32;
    let j = (i + by).clamp(0, t.columns.len() as i32 - 1) as usize;
    t.columns[j].id.clone()
}

/// Raw points a side loses per percentage point of its result, as a share
/// of its own raw points (SPI 15.83b; R-011). 1 under option 1; the combined
/// total over the side's own under option 2.
pub fn loss_base_factor(rules: &Ruleset, side: Side, attacker_raw: f64, defender_raw: f64) -> f64 {
    let own = match side {
        Side::Attacker => attacker_raw,
        Side::Defender => defender_raw,
    };
    match rules.r011 {
        R011::OwnRaw => 1.0,
        R011::CombinedRaw => (attacker_raw + defender_raw) / own,
    }
}

/// Whether a defender who commits nothing may stay put and pay 10 % per hex
/// not retreated (SPI 15.29, 15.82; R-012), given whether a three-hex
/// retreat path exists.
pub fn may_buy_out(rules: &Ruleset, path_exists: bool) -> bool {
    match rules.r012 {
        R012::MustRetreat => false,
        R012::BuyOut => true,
        R012::NoPathNoWithhold => path_exists,
    }
}

/// Whether a defender may commit nothing at all (R-012).
pub fn may_withhold(rules: &Ruleset, path_exists: bool) -> bool {
    path_exists || rules.r012 == R012::BuyOut
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

    #[test]
    fn column_for_covers_every_differential() {
        let t = t();
        assert_eq!(column_for(&t, -20), "-11");
        assert_eq!(column_for(&t, -11), "-11");
        assert_eq!(column_for(&t, -10), "-8");
        assert_eq!(column_for(&t, 0), "0");
        assert_eq!(column_for(&t, 6), "+5");
        assert_eq!(column_for(&t, 16), "+14");
        assert_eq!(column_for(&t, 40), "+17");
    }

    #[test]
    fn resolve_side_agrees_with_the_exact_outcome() {
        let t = t();
        for side in [Side::Attacker, Side::Defender] {
            for c in &t.columns {
                let o = outcome(&t, side, &c.id);
                let (mut pct, mut retreat) = (0.0, 0.0);
                for (a, b) in crate::dice::rolls() {
                    let r = resolve_side(&t, side, &c.id, a * 10 + b);
                    pct += P * r.pct as f64;
                    retreat += P * r.retreat as f64;
                }
                // The one gap (defender +2, three readings) is decided
                // as 5 % by R-023, so the mean is that much higher.
                let decided = if side == Side::Defender && c.id == "+2" {
                    3.0 * P * 5.0
                } else {
                    0.0
                };
                assert!(
                    (pct - o.expected_pct - decided).abs() < 1e-9,
                    "{} {}",
                    side.key(),
                    c.id
                );
                assert!(
                    (retreat - o.expected_retreat_hexes).abs() < 1e-9,
                    "{} {}",
                    side.key(),
                    c.id
                );
            }
        }
        // Lines by hand: at -11 the attacker is engaged on 10-12 and his
        // losses captured on 2-7; the defender at +2 retreats a hex on
        // 5-7 and loses prisoners on a 2.
        assert!(resolve_side(&t, Side::Attacker, "-11", 46).engaged);
        assert!(resolve_side(&t, Side::Attacker, "-11", 16).captured);
        assert!(!resolve_side(&t, Side::Attacker, "-11", 16).engaged);
        assert_eq!(resolve_side(&t, Side::Defender, "+2", 34).retreat, 1);
        assert_eq!(resolve_side(&t, Side::Defender, "+2", 34).pct, 5);
        assert!(resolve_side(&t, Side::Defender, "+2", 11).captured);
    }

    #[test]
    fn ten_thousand_seeded_samples_match_the_distribution() {
        use crate::dice::sample;
        let t = t();
        let o = outcome(&t, Side::Defender, "+4");
        let mut rng = sample::rng(15);
        let n = 10_000;
        let (mut pct, mut retreat) = (0u32, 0u32);
        for _ in 0..n {
            let r = resolve_side(&t, Side::Defender, "+4", sample::reading(&mut rng));
            pct += r.pct as u32;
            retreat += r.retreat;
        }
        assert!(
            (pct as f64 / n as f64 - o.expected_pct).abs() < 0.5,
            "{o:?}"
        );
        assert!((retreat as f64 / n as f64 - o.expected_retreat_hexes).abs() < 0.02);
        let mut rng = sample::rng(15);
        let r = sample::reading(&mut rng);
        assert_eq!(r, 13);
        assert_eq!(
            resolve_side(&t, Side::Defender, "+4", r),
            SideRoll {
                pct: loss_pct_decided(&t, Side::Defender, "+4", 13),
                engaged: false,
                retreat: resolve_side(&t, Side::Defender, "+4", 13).retreat,
                captured: on_line(&t, "capture_defender", "+4", 4),
            }
        );
    }

    #[test]
    fn r011_switch_changes_the_base() {
        let own = Ruleset::default();
        let comb = Ruleset {
            r011: R011::CombinedRaw,
            ..own
        };
        assert_eq!(loss_base_factor(&own, Side::Attacker, 1.0, 4.0), 1.0);
        assert_eq!(loss_base_factor(&comb, Side::Attacker, 1.0, 4.0), 5.0);
        assert_eq!(loss_base_factor(&comb, Side::Defender, 1.0, 4.0), 1.25);
    }

    #[test]
    fn r012_switch_decides_withholding() {
        let r = |r012| Ruleset {
            r012,
            ..Ruleset::default()
        };
        // (option, path exists) -> (may withhold, may buy out)
        for (o, path, w, b) in [
            (R012::MustRetreat, true, true, false),
            (R012::MustRetreat, false, false, false),
            (R012::BuyOut, true, true, true),
            (R012::BuyOut, false, true, true),
            (R012::NoPathNoWithhold, true, true, true),
            (R012::NoPathNoWithhold, false, false, false),
        ] {
            assert_eq!(
                (may_withhold(&r(o), path), may_buy_out(&r(o), path)),
                (w, b),
                "{o:?} {path}"
            );
        }
    }
}
