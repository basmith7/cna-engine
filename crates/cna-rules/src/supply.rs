//! Abstract-supply expenditure rates (SPI 32.21, 32.22): what a unit
//! owes a supply unit in range for fuel and ammunition.

use cna_data::characteristics::Equivalent;

/// What burns fuel when it moves by land (32.22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelClass {
    Tank,
    /// Gun-class and reconnaissance, camels excepted.
    GunOrRecce,
    CamelRecce,
    /// Motorisation points, which pay by what they carry.
    Points,
    /// Everything else, on foot or not.
    Other,
}

/// What motorisation points are carrying (32.22, 32.5x).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrying {
    Nothing,
    /// Infantry, not replacements.
    Infantry,
    Gun,
    RealSupplyUnit,
    /// Replacements, a dummy supply unit or other cargo.
    Other,
}

/// What a unit is doing when it spends ammunition (32.21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Defend,
    Barrage,
}

/// Fuel owed for one land move in a stage. A brigade, a division or an
/// HQ pays by its components, so the caller passes those.
pub fn fuel_owed(class: FuelClass, equivalent: Equivalent, carrying: Carrying) -> u32 {
    match (class, equivalent) {
        (FuelClass::Tank, Equivalent::Battalion) => 2,
        (FuelClass::Tank, Equivalent::Company) => 1,
        (FuelClass::GunOrRecce, Equivalent::Battalion) => 1,
        (FuelClass::Points, _) => match carrying {
            Carrying::Infantry | Carrying::Gun | Carrying::RealSupplyUnit => 1,
            Carrying::Nothing | Carrying::Other => 0,
        },
        _ => 0,
    }
}

/// Ammunition owed by one unit at the instant of combat, per
/// battalion-equivalent; a company-equivalent or larger pays by its
/// components, so the caller passes those.
pub fn ammo_owed(equivalent: Equivalent, role: Role, phasing: bool) -> u32 {
    if equivalent != Equivalent::Battalion {
        return 0;
    }
    let base = match role {
        Role::Defend => 1,
        Role::Barrage => 2,
    };
    if phasing { base * 2 } else { base }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Equivalent::{Battalion, Brigade, Company, Hq};
    use FuelClass::*;

    #[test]
    fn fuel_rows_of_32_22() {
        let n = Carrying::Nothing;
        // (class, equivalent, carrying, owed)
        let rows = [
            (Tank, Battalion, n, 2),
            (Tank, Company, n, 1),
            (GunOrRecce, Battalion, n, 1),
            (GunOrRecce, Company, n, 0),
            (CamelRecce, Battalion, n, 0),
            (CamelRecce, Company, n, 0),
            (Other, Battalion, n, 0),
            (Other, Company, n, 0),
            (Points, Battalion, Carrying::Infantry, 1),
            (Points, Battalion, Carrying::Gun, 1),
            (Points, Battalion, Carrying::RealSupplyUnit, 1),
            (Points, Battalion, Carrying::Other, 0),
            (Points, Battalion, n, 0),
            // composites pay by their components, never as a whole
            (Tank, Brigade, n, 0),
            (Tank, Hq, n, 0),
        ];
        for (class, eq, carrying, owed) in rows {
            assert_eq!(
                fuel_owed(class, eq, carrying),
                owed,
                "{class:?} {eq:?} {carrying:?}"
            );
        }
    }

    #[test]
    fn ammunition_rows_of_32_21() {
        let rows = [
            (Battalion, Role::Defend, false, 1),
            (Battalion, Role::Barrage, false, 2),
            (Battalion, Role::Defend, true, 2),
            (Battalion, Role::Barrage, true, 4),
            (Company, Role::Defend, false, 0),
            (Company, Role::Barrage, true, 0),
            (Brigade, Role::Defend, false, 0),
        ];
        for (eq, role, phasing, owed) in rows {
            assert_eq!(
                ammo_owed(eq, role, phasing),
                owed,
                "{eq:?} {role:?} {phasing}"
            );
        }
    }
}
