//! Morale modification (SPI 17.2x) read from the Morale Modifier Table.

pub use cna_data::morale::MoraleModifier;

/// What the table says for one cohesion level and reading.
#[derive(Debug, PartialEq)]
pub enum MoraleLookup {
    /// `+4` … `-4`, or `surrender`.
    Modifier(String),
    /// The reading falls in no cell: a printed gap.
    Gap,
    /// The reading falls in several cells.
    Overlap(Vec<String>),
}

pub fn lookup(t: &MoraleModifier, level: i32, reading: u8) -> MoraleLookup {
    let mut hits: Vec<String> = t
        .row(level)
        .modifier
        .iter()
        .filter(|(_, r)| r.is_some_and(|r| r.contains(reading)))
        .map(|(m, _)| m.clone())
        .collect();
    match hits.len() {
        0 => MoraleLookup::Gap,
        1 => MoraleLookup::Modifier(hits.remove(0)),
        _ => MoraleLookup::Overlap(hits),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::rolls;

    #[test]
    fn only_declared_gaps_exist() {
        let t = MoraleModifier::load().unwrap();
        let mut gaps = vec![];
        for row in &t.rows {
            for (a, b) in rolls() {
                match lookup(&t, row.level, a * 10 + b) {
                    MoraleLookup::Modifier(_) => {}
                    MoraleLookup::Gap => gaps.push((row.level, a * 10 + b)),
                    MoraleLookup::Overlap(m) => panic!("{} {}: in {m:?}", row.level, a * 10 + b),
                }
            }
        }
        let declared: Vec<_> = t
            .known_gaps
            .iter()
            .flat_map(|g| g.readings.iter().map(move |r| (g.level, *r)))
            .collect();
        assert_eq!(gaps, declared);
    }

    #[test]
    fn minus_17_always_surrenders() {
        let t = MoraleModifier::load().unwrap();
        for (a, b) in rolls() {
            assert_eq!(
                lookup(&t, -30, a * 10 + b),
                MoraleLookup::Modifier("surrender".into())
            );
        }
    }
}
