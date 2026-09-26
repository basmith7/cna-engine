//! Dice. Exact enumeration: every probability is a count over outcomes.

/// Two distinguishable dice read first-then-second (SPI 15.0: the larger
/// die, not the higher number, is read first). All 36 ordered pairs are
/// equally likely.
pub fn rolls() -> impl Iterator<Item = (u8, u8)> {
    (1..=6u8).flat_map(|a| (1..=6u8).map(move |b| (a, b)))
}

/// The probability of one ordered pair.
pub const P: f64 = 1.0 / 36.0;

/// The same two dice summed: (sum, probability) for 2 to 12.
pub fn sums() -> impl Iterator<Item = (u8, f64)> {
    (2..=12u8).map(|s| (s, rolls().filter(|(a, b)| a + b == s).count() as f64 * P))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_are_2_to_12_and_add_to_1() {
        let s: Vec<_> = sums().collect();
        assert_eq!(s.len(), 11);
        assert_eq!((s[0].0, s[10].0), (2, 12));
        assert!((s.iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((s[5].1 - 6.0 / 36.0).abs() < 1e-12);
        assert_eq!(s[5].0, 7);
    }
}
