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
    /// Committed to a close assault, either side.
    Assault,
    Barrage,
}

/// The battalion-equivalents a single counter stands for when it pays
/// by components (32.21, 32.22): a brigade-equivalent counter is three
/// battalions (the 15.5x size ladder's reading of 9.4); anything else
/// pays as itself, once. Returns the equivalent each component pays as
/// and how many there are.
pub fn components(equivalent: Equivalent) -> (Equivalent, u32) {
    match equivalent {
        Equivalent::Brigade => (Equivalent::Battalion, 3),
        other => (other, 1),
    }
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
        Role::Assault => 1,
        Role::Barrage => 2,
    };
    if phasing { base * 2 } else { base }
}

/// R-121: a company-equivalent pays half a battalion-equivalent's
/// ammunition, a side's companies in one combat paying together with a
/// half point left over rounded up. Returns what the `k`-th company
/// (0-based) of the side pays, so that `n` companies pay
/// `ceil(n * rate / 2)` in all.
pub fn company_ammo_share(role: Role, phasing: bool, k: u32) -> u32 {
    let rate = ammo_owed(Equivalent::Battalion, role, phasing);
    ((k + 1) * rate).div_ceil(2) - (k * rate).div_ceil(2)
}

/// What a simplified Axis convoy carries, or loses (32.61).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cargo {
    pub supply_units: u32,
    pub motorisation: u32,
    pub replacements: u32,
}

/// A bombed convoy's losses at `percent` (32.65): below 20 % only
/// motorisation (fractions down) and replacement points (fractions up)
/// are hit; at 20 % or more everything, fractions down, but at least one
/// of each item carried. The chart's 5 % is read with its 10 %: the rule
/// names only 10 % and "20 % or more".
pub fn convoy_losses(percent: u32, cargo: Cargo) -> Cargo {
    let down = |n: u32| n * percent / 100;
    let up = |n: u32| (n * percent).div_ceil(100);
    if percent == 0 {
        return Cargo::default();
    }
    if percent < 20 {
        return Cargo {
            supply_units: 0,
            motorisation: down(cargo.motorisation),
            replacements: up(cargo.replacements),
        };
    }
    let at_least_one = |n: u32| if n > 0 { down(n).max(1) } else { 0 };
    Cargo {
        supply_units: at_least_one(cargo.supply_units),
        motorisation: at_least_one(cargo.motorisation),
        replacements: at_least_one(cargo.replacements),
    }
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
    fn a_brigade_counter_pays_as_three_battalions() {
        assert_eq!(components(Brigade), (Battalion, 3));
        assert_eq!(components(Company), (Company, 1));
        assert_eq!(components(Hq), (Hq, 1));
    }

    #[test]
    fn ammunition_rows_of_32_21() {
        let rows = [
            (Battalion, Role::Assault, false, 1),
            (Battalion, Role::Barrage, false, 2),
            (Battalion, Role::Assault, true, 2),
            (Battalion, Role::Barrage, true, 4),
            (Company, Role::Assault, false, 0),
            (Company, Role::Barrage, true, 0),
            (Brigade, Role::Assault, false, 0),
        ];
        for (eq, role, phasing, owed) in rows {
            assert_eq!(
                ammo_owed(eq, role, phasing),
                owed,
                "{eq:?} {role:?} {phasing}"
            );
        }
    }

    // R-121: n companies of a side in one combat pay ceil(n * rate / 2)
    // together, the first company taking the rounded-up half.
    #[test]
    fn companies_pay_half_a_battalion_together_r121() {
        let shares = |role, phasing, n: u32| -> Vec<u32> {
            (0..n)
                .map(|k| company_ammo_share(role, phasing, k))
                .collect()
        };
        assert_eq!(shares(Role::Assault, false, 3), vec![1, 0, 1]);
        assert_eq!(shares(Role::Assault, true, 3), vec![1, 1, 1]);
        assert_eq!(shares(Role::Barrage, false, 2), vec![1, 1]);
        assert_eq!(shares(Role::Barrage, true, 1), vec![2]);
        for (role, phasing) in [
            (Role::Assault, false),
            (Role::Assault, true),
            (Role::Barrage, false),
            (Role::Barrage, true),
        ] {
            let rate = ammo_owed(Battalion, role, phasing);
            for n in 0..7 {
                let total: u32 = shares(role, phasing, n).iter().sum();
                assert_eq!(total, (n * rate).div_ceil(2), "{role:?} {phasing} {n}");
            }
        }
    }

    #[test]
    fn a_convoy_hit_for_10_percent_loses_points_only_32_65() {
        let cargo = Cargo {
            supply_units: 3,
            motorisation: 19,
            replacements: 4,
        };
        // Motorisation down, replacements up, no supply unit.
        assert_eq!(
            convoy_losses(10, cargo),
            Cargo {
                supply_units: 0,
                motorisation: 1,
                replacements: 1
            }
        );
        assert_eq!(convoy_losses(5, cargo).supply_units, 0);
        assert_eq!(convoy_losses(0, cargo), Cargo::default());
    }

    #[test]
    fn twenty_percent_or_more_hits_everything_at_least_one_each_32_65() {
        let cargo = Cargo {
            supply_units: 3,
            motorisation: 20,
            replacements: 0,
        };
        // 20 % of 3 supply units is 0.6: down to 0, but at least one.
        assert_eq!(
            convoy_losses(20, cargo),
            Cargo {
                supply_units: 1,
                motorisation: 4,
                replacements: 0
            }
        );
        assert_eq!(convoy_losses(50, cargo).supply_units, 1);
        assert_eq!(
            convoy_losses(
                40,
                Cargo {
                    supply_units: 5,
                    ..cargo
                }
            )
            .supply_units,
            2
        );
    }
}
