//! Dice. Exact enumeration: every probability is a count over outcomes.

/// Two distinguishable dice read first-then-second (SPI 15.0: the larger
/// die, not the higher number, is read first). All 36 ordered pairs are
/// equally likely.
pub fn rolls() -> impl Iterator<Item = (u8, u8)> {
    (1..=6u8).flat_map(|a| (1..=6u8).map(move |b| (a, b)))
}

/// The probability of one ordered pair.
pub const P: f64 = 1.0 / 36.0;
