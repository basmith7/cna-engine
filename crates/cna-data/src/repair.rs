//! The Vehicle Repair Table (`vehicle-repair.json`, SPI 22.8).

use crate::close_assault::Range;
use anyhow::Result;
use serde::Deserialize;

/// One die result: what field repair and the facilities return.
#[derive(Debug, Clone, Deserialize)]
pub struct RepairRow {
    pub die: Range,
    // Every column is none where the chart has "na".
    /// Truck points repaired in the field (2: two light or medium, or
    /// one heavy).
    pub field_truck: Option<u32>,
    pub field_armoured_car: Option<u32>,
    /// Percent of one tank type repaired in the field.
    pub field_tank_percent: Option<u32>,
    pub temporary_facility_percent: Option<u32>,
    pub major_facility_percent: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VehicleRepair {
    pub rows: Vec<RepairRow>,
}

impl VehicleRepair {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src,
            "vehicle-repair",
        )?)?)
    }

    /// The row for a modified die roll, if the chart has one.
    pub fn row(&self, die: u8) -> Option<&RepairRow> {
        self.rows.iter().find(|r| r.die.contains(die))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_tank_column_reads_25_then_10_then_nothing() {
        let t = VehicleRepair::load().unwrap();
        let pct = |d| t.row(d).and_then(|r| r.field_tank_percent);
        assert_eq!(pct(1), Some(25));
        assert_eq!(pct(2), Some(10));
        assert_eq!(pct(4), Some(10));
        assert_eq!(pct(5), Some(0));
        assert_eq!(pct(7), None);
    }
}
