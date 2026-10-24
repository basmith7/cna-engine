//! Breakdown checks (SPI 21.2x, 21.3x): which column a group of vehicles
//! reads, what one reading breaks, and when a stopped unit must check.

pub use cna_data::breakdown::Breakdown;

/// Weather's shift on a check (21.37): hot, one column right; a
/// sandstorm, one right when at least half the move's CP was spent on a
/// map under it. Rain acts on the terrain values instead.
pub fn weather_shift(hot: bool, mostly_in_sandstorm: bool) -> i32 {
    i32::from(hot) + i32::from(mostly_in_sandstorm)
}

/// The column a check reads for `points` accumulated, shifted by the
/// BAR and weather together (21.31–21.33). `None`: no check is made
/// (3 or fewer points, 21.2x) or the shift carries it below 4–10 (no
/// breakdown). Past 71+ the shift stops at 71+.
pub fn column(t: &Breakdown, points: f64, shift: i32) -> Option<usize> {
    let base = t.band_index(points.ceil() as u32)?;
    if base == 0 {
        return None;
    }
    let shifted = (base as i32 + shift).min(t.table.bands.len() as i32 - 1);
    (shifted >= 1).then_some(shifted as usize)
}

/// The percentage of the group that breaks down on one sequential
/// reading (11..=66) in `column`.
pub fn percent(t: &Breakdown, column: usize, reading: u8) -> u32 {
    let id = &t.table.bands[column].id;
    t.table
        .cells
        .iter()
        .find(|c| &c.column == id && c.dice.contains(reading))
        .map_or(0, |c| c.percent)
}

/// TOE points lost from a group of `points` at `percent` (21.34):
/// fractions up, but a single point ignores 10 %.
pub fn losses(points: u32, percent: u32) -> u32 {
    if points == 1 && percent == 10 {
        return 0;
    }
    (points * percent).div_ceil(100)
}

/// Whether a unit stopping with `points` must check (21.2x): more than
/// 3, and in a higher unshifted column than its last check this stage.
pub fn must_check(t: &Breakdown, points: f64, last: Option<usize>) -> bool {
    let Some(now) = t.band_index(points.ceil() as u32).filter(|c| *c > 0) else {
        return false;
    };
    last.is_none_or(|l| now > l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Breakdown {
        Breakdown::load().unwrap()
    }

    fn id(t: &Breakdown, c: Option<usize>) -> Option<&str> {
        c.map(|c| t.table.bands[c].id.as_str())
    }

    #[test]
    fn the_worked_example_of_21_3x() {
        // 30 trucks (2 left) and 20 cruiser points rated 1R stop at 35
        // points in hot weather.
        let t = table();
        let hot = weather_shift(true, false);
        let trucks = column(&t, 35.0, t.adjustment("truck").unwrap().shift + hot);
        assert_eq!(id(&t, trucks), Some("21-30"));
        assert_eq!(id(&t, column(&t, 35.0, 1 + hot)), Some("51-60"));
        // 10 % of 30 trucks is 3; 33 % of 20 points is 6.6, so 7.
        assert_eq!(losses(30, 10), 3);
        assert_eq!(losses(20, 33), 7);
    }

    #[test]
    fn fractions_round_up_and_the_ends_of_the_table_hold() {
        let t = table();
        // 20.5 points read the 21-30 column (21.31).
        assert_eq!(id(&t, column(&t, 20.5, 0)), Some("21-30"));
        // Three points never check; shifted below 4-10, nothing breaks.
        assert_eq!(column(&t, 3.0, 2), None);
        assert_eq!(column(&t, 15.0, -2), None);
        // 71+ counts as one column.
        assert_eq!(id(&t, column(&t, 65.0, 3)), Some("71+"));
        // A single point ignores 10 %.
        assert_eq!(losses(1, 10), 0);
        assert_eq!(losses(1, 25), 1);
    }

    #[test]
    fn a_unit_checks_again_only_in_a_higher_column_21_2x() {
        let t = table();
        assert!(!must_check(&t, 3.0, None));
        assert!(must_check(&t, 15.0, None));
        let first = t.band_index(15);
        assert!(!must_check(&t, 17.0, first));
        assert!(must_check(&t, 27.0, first));
    }

    #[test]
    fn under_four_to_ten_nothing_breaks_and_from_sixty_one_every_roll_does() {
        let t = table();
        let at = |c: &str| t.table.bands.iter().position(|b| b.id == c).unwrap();
        let readings = (1..=6u8).flat_map(|a| (1..=6u8).map(move |b| a * 10 + b));
        assert!(readings.clone().all(|r| percent(&t, 0, r) == 0));
        assert!(readings.clone().all(|r| percent(&t, at("61-70"), r) > 0));
        assert!(readings.clone().any(|r| percent(&t, at("71+"), r) == 75));
        assert!([65, 66].iter().all(|r| percent(&t, at("4-10"), *r) >= 25));
    }
}
