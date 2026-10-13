//! The CP cost of one step of movement (SPI 8.3x, 8.4x, 29.4x, 29.5x):
//! the hex entered plus the hexside crossed, priced from the Terrain
//! Effects Chart for the kind of unit moving, with the weather applied.
//! Pure: the map, the chart and the step come in, a price or a
//! prohibition comes out. A railroad gives no rate of its own
//! (footnote 5); it only opens swamp (8.37) and major rivers (footnote
//! 11) the way a road does.

use cna_data::cp::Cp;
use cna_data::map::{Hex, Hexside, HexsideFeature, Terrain as Fill};
use cna_data::terrain::{CpCell, CpValue, Terrain};
use std::fmt;

/// How a unit moves, for the chart's two columns and its footnotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mobility {
    /// A non-motorised unit: CPA 10 or less (8.1x).
    Foot,
    /// Any other vehicle unit: tanks, guns, motorised infantry.
    Vehicle,
    /// Light trucks and motorcycle units: no desert (footnote 3), salt
    /// marsh without a road (8.4x). No row of `unit-characteristics.json`
    /// is one yet, so `GameState::mobility` never produces it.
    LightVehicle,
    /// Recce units: salt marsh without a road (8.4x).
    Recce,
}

impl Mobility {
    pub fn motorised(self) -> bool {
        self != Mobility::Foot
    }
}

/// The weather over the hex entered (29.4x, 29.5x).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Weather {
    #[default]
    Normal,
    Sandstorm,
    Rainstorm,
}

/// One step: from a hex into an adjacent one across a hexside (plain
/// when `hexside` is none). `climbs` is true when the hexside's slope or
/// escarpment goes up into `to`.
#[derive(Debug, Clone, Copy)]
pub struct Step<'a> {
    pub from: &'a Hex,
    pub to: &'a Hex,
    pub hexside: Option<&'a Hexside>,
    pub climbs: bool,
    pub weather: Weather,
}

/// A step the chart forbids this unit, and the case that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prohibited {
    pub case: &'static str,
    pub reason: String,
}

impl fmt::Display for Prohibited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.reason, self.case)
    }
}

impl std::error::Error for Prohibited {}

fn no(case: &'static str, reason: impl Into<String>) -> Prohibited {
    Prohibited {
        case,
        reason: reason.into(),
    }
}

/// The rate a step is priced at, from the hexside's features and the
/// weather: a road gives the road rate and ignores other terrain (8.33);
/// a track halves it (E-031); an unfinished road is a track; in rain a
/// road is priced as a track (29.5x). Whether the step is *by road* for
/// the prohibitions that ask (swamp, delta, wadis and rivers in rain)
/// is a fact of the hexside the weather does not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Along {
    Road,
    Track,
    Country,
}

fn along(hexside: Option<&Hexside>, weather: Weather) -> Along {
    let Some(hs) = hexside else {
        return Along::Country;
    };
    match (hs.road(), hs.track(), weather) {
        (true, _, Weather::Rainstorm) => Along::Track,
        (true, _, _) => Along::Road,
        (false, true, _) => Along::Track,
        (false, false, _) => Along::Country,
    }
}

/// A chart price for one kind of unit; the cell's rules are the caller's.
fn price(chart: &Terrain, kind: &str, name: &str, motorised: bool) -> Result<Cp, Prohibited> {
    let cell = chart
        .cp(kind, name)
        .ok_or_else(|| no("8.37", format!("{name}: no CP column")))?;
    match cell {
        CpCell::Pair { non_mot, mot } => match if motorised { mot } else { non_mot } {
            CpValue::Cost(c) => Ok(*c),
            CpValue::Prohibited => Err(no(
                "8.37",
                format!(
                    "{name}: prohibited to {} units",
                    if motorised { "motorised" } else { "foot" }
                ),
            )),
            v => Err(no("8.37", format!("{name}: {v:?} is not a price"))),
        },
        CpCell::Ref(r) => Err(no("8.37", format!("{name}: {r}"))),
    }
}

/// The CP one unit pays for `step`, or why it may not take it.
pub fn step_cost(chart: &Terrain, mobility: Mobility, step: &Step) -> Result<Cp, Prohibited> {
    let motorised = mobility.motorised();
    let mode = along(step.hexside, step.weather);
    let to = step.to.terrain;
    let from = step.from.terrain;
    let road = step.hexside.is_some_and(|hs| hs.has(HexsideFeature::Road));
    let rail = step
        .hexside
        .is_some_and(|hs| hs.has(HexsideFeature::Railroad));
    let road_or_rail = road || rail;

    // The hex entered (8.37 and its footnotes).
    if to == Fill::Sea {
        return Err(no("8.37", format!("{} is sea", step.to.id)));
    }
    if to == Fill::Swamp && !road_or_rail {
        return Err(no(
            "8.37",
            format!("{} is swamp: road or railroad only", step.to.id),
        ));
    }
    if to == Fill::Desert && mobility == Mobility::LightVehicle {
        return Err(no(
            "8.4x",
            format!("{} is desert: no light trucks or motorcycles", step.to.id),
        ));
    }
    if mobility == Mobility::Vehicle
        && (to == Fill::SaltMarsh || from == Fill::SaltMarsh)
        && mode == Along::Country
    {
        return Err(no(
            "8.4x",
            format!(
                "salt marsh at {}: vehicles enter and leave by road or track only",
                if to == Fill::SaltMarsh {
                    &step.to.id
                } else {
                    &step.from.id
                }
            ),
        ));
    }
    // In a rainstorm vehicles enter delta by road or rail only, and
    // those already in a delta hex leave it the same way (29.5x).
    if motorised
        && (to == Fill::Delta || from == Fill::Delta)
        && step.weather == Weather::Rainstorm
        && !road_or_rail
    {
        return Err(no(
            "29.5x",
            format!(
                "{} is delta in a rainstorm: vehicles enter and leave by road or rail only",
                if to == Fill::Delta {
                    &step.to.id
                } else {
                    &step.from.id
                }
            ),
        ));
    }

    let mut cost = match mode {
        Along::Road => price(chart, "hex", "road", motorised)?,
        // Swamp has no price of its own: it is entered by road or
        // railroad only (8.37), and the chart gives the railroad no rate
        // (footnote 5), so a track or a railroad into it is priced as
        // the road.
        _ if to == Fill::Swamp => price(chart, "hex", "road", motorised)?,
        Along::Track => price(chart, "hex", to.name(), motorised)?.halved(),
        Along::Country => price(chart, "hex", to.name(), motorised)?,
    };

    // The hexside crossed. On a road every feature is ignored except an
    // escarpment for vehicles (8.33), a flooded wadi (footnote 10) and a
    // major river for foot units (footnote 11). On a track the crossing
    // cost is halved, except a vehicle going down an escarpment
    // (footnote 8).
    if let Some(hs) = step.hexside {
        for f in &hs.features {
            let side = match f {
                HexsideFeature::Escarpment => {
                    if motorised && step.climbs {
                        return Err(no("8.4x", "no vehicle crosses an escarpment upward"));
                    }
                    if motorised && mode == Along::Country {
                        return Err(no(
                            "8.4x",
                            "vehicles go down an escarpment only by road or track",
                        ));
                    }
                    let name = if step.climbs {
                        "up-escarpment"
                    } else {
                        "down-escarpment"
                    };
                    match (mode, motorised) {
                        (Along::Road, false) => continue,
                        (Along::Track, false) => price(chart, "hexside", name, false)?.halved(),
                        _ => price(chart, "hexside", name, motorised)?,
                    }
                }
                HexsideFeature::Slope => {
                    let name = if step.climbs {
                        "up-slope"
                    } else {
                        "down-slope"
                    };
                    match mode {
                        Along::Road => continue,
                        Along::Track => price(chart, "hexside", name, motorised)?.halved(),
                        Along::Country => price(chart, "hexside", name, motorised)?,
                    }
                }
                HexsideFeature::Ridge => match mode {
                    Along::Road => continue,
                    Along::Track => price(chart, "hexside", "ridge", motorised)?.halved(),
                    Along::Country => price(chart, "hexside", "ridge", motorised)?,
                },
                HexsideFeature::Wadi => match (road, step.weather, mode) {
                    (true, Weather::Rainstorm, _) => Cp::whole(2),
                    (true, _, _) => continue,
                    (false, Weather::Rainstorm, _) => {
                        return Err(no("29.5x", "a wadi in a rainstorm is crossed by road only"));
                    }
                    (false, _, Along::Track) => {
                        price(chart, "hexside", "wadi", motorised)?.halved()
                    }
                    (false, _, _) => price(chart, "hexside", "wadi", motorised)?,
                },
                HexsideFeature::MajorRiver => {
                    if motorised && !road_or_rail {
                        return Err(no(
                            "8.4x",
                            "vehicles cross a major river only where a road or railroad crosses it",
                        ));
                    }
                    if step.weather == Weather::Rainstorm && !road_or_rail {
                        return Err(no(
                            "29.5x",
                            "a river in a rainstorm is crossed by road or rail only",
                        ));
                    }
                    // Footnote 11: by road or railroad a vehicle pays
                    // nothing for the river; a foot unit pays the crossing.
                    match (road_or_rail, motorised, mode) {
                        (true, true, _) => continue,
                        (true, false, _) => price(chart, "hexside", "major-river", false)?,
                        (false, _, Along::Track) => {
                            price(chart, "hexside", "major-river", motorised)?.halved()
                        }
                        (false, _, _) => price(chart, "hexside", "major-river", motorised)?,
                    }
                }
                HexsideFeature::MinorRiver => {
                    if step.weather == Weather::Rainstorm && !road_or_rail {
                        return Err(no(
                            "29.5x",
                            "a river in a rainstorm is crossed by road or rail only",
                        ));
                    }
                    match mode {
                        Along::Road => continue,
                        Along::Track => price(chart, "hexside", "minor-river", motorised)?.halved(),
                        Along::Country => price(chart, "hexside", "minor-river", motorised)?,
                    }
                }
                HexsideFeature::Lake => {
                    return Err(no("8.37", "no unit crosses a lake hexside"));
                }
                HexsideFeature::Road
                | HexsideFeature::UnfinishedRoad
                | HexsideFeature::Railroad
                | HexsideFeature::UnfinishedRailroad
                | HexsideFeature::Track
                | HexsideFeature::Coast => continue,
            };
            cost += side;
        }
    }

    // Sandstorm: double, except on delta hexes (29.4x).
    if step.weather == Weather::Sandstorm && to != Fill::Delta {
        cost = cost.doubled();
    }
    Ok(cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::map::{HexId, Map};

    fn fixtures() -> (Map, Terrain) {
        (Map::load().unwrap(), Terrain::load().unwrap())
    }

    fn cost(
        map: &Map,
        chart: &Terrain,
        mobility: Mobility,
        from: &str,
        to: &str,
        weather: Weather,
    ) -> Result<Cp, Prohibited> {
        let (a, b) = (HexId::parse(from).unwrap(), HexId::parse(to).unwrap());
        assert!(map.adjacent(&a, &b), "{from} and {to} are not adjacent");
        let hexside = map.hexside(&a, &b);
        let step = Step {
            from: map.hex(&a).unwrap(),
            to: map.hex(&b).unwrap(),
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(&a, &b)),
            weather,
        };
        step_cost(chart, mobility, &step)
    }

    #[test]
    fn hand_priced_steps_on_sheet_c() {
        let (map, chart) = fixtures();
        let foot = |a, b| cost(&map, &chart, Mobility::Foot, a, b, Weather::Normal);
        let mot = |a, b| cost(&map, &chart, Mobility::Vehicle, a, b, Weather::Normal);
        // Clear to clear across a plain hexside: 2 either way.
        assert_eq!(foot("C3922", "C3923"), Ok(Cp::whole(2)));
        assert_eq!(mot("C3922", "C3923"), Ok(Cp::whole(2)));
        // Into rough (the coast hexside is free): 3 foot, 4 motorised.
        assert_eq!(foot("C4122", "C4221"), Ok(Cp::whole(3)));
        assert_eq!(mot("C4122", "C4221"), Ok(Cp::whole(4)));
        // Up a slope into clear: 2 + 2 foot, 2 + 4 motorised; down: 2 + 1, 2 + 2.
        assert_eq!(foot("C4121", "C4220"), Ok(Cp::whole(4)));
        assert_eq!(mot("C4121", "C4220"), Ok(Cp::whole(6)));
        assert_eq!(foot("C4220", "C4121"), Ok(Cp::whole(3)));
        assert_eq!(mot("C4220", "C4121"), Ok(Cp::whole(4)));
        // Down the Halfaya escarpment by track (C4020 to C4121): the track
        // halves the clear hex (1) and the foot crossing (4 to 2); a
        // vehicle pays the full 8 (footnote 8).
        assert_eq!(foot("C4020", "C4121"), Ok(Cp::whole(3)));
        assert_eq!(mot("C4020", "C4121"), Ok(Cp::whole(9)));
        // Up it by the same track: foot 1 + 3; a vehicle never climbs.
        assert_eq!(foot("C4121", "C4020"), Ok(Cp::whole(4)));
        assert_eq!(mot("C4121", "C4020").unwrap_err().case, "8.4x");
        // The escarpment below Halfaya with no track: foot 2 + 6 up, 2 + 4
        // down; vehicles neither way.
        assert_eq!(foot("C3922", "C3921"), Ok(Cp::whole(8)));
        assert_eq!(foot("C3921", "C3922"), Ok(Cp::whole(6)));
        assert_eq!(mot("C3922", "C3921").unwrap_err().case, "8.4x");
        assert_eq!(mot("C3921", "C3922").unwrap_err().case, "8.4x");
        // Along the coast road: 1 foot, ½ motorised, the slope ignored.
        assert_eq!(foot("C4020", "C4120"), Ok(Cp::whole(1)));
        assert_eq!(mot("C4020", "C4120"), Ok(Cp::HALF));
        assert_eq!(mot("C4020", "C4021"), Ok(Cp::HALF));
        assert_eq!(foot("C4021", "C4020"), Ok(Cp::whole(1)));
        // A track into rough halves it: 1½ foot, 2 motorised (E-031).
        assert_eq!(foot("C0707", "C0708"), Ok(Cp::halves(3)));
        assert_eq!(mot("C0707", "C0708"), Ok(Cp::whole(2)));
        // A plain wadi: 1 foot, 4 motorised, on top of the clear hex.
        assert_eq!(foot("C0821", "C0822"), Ok(Cp::whole(3)));
        assert_eq!(mot("C0821", "C0822"), Ok(Cp::whole(6)));
    }

    #[test]
    fn prohibitions_by_unit_kind() {
        let (map, chart) = fixtures();
        let go = |m, a, b| cost(&map, &chart, m, a, b, Weather::Normal);
        // Desert: light trucks and motorcycles never; everyone else pays.
        assert_eq!(
            go(Mobility::LightVehicle, "C0224", "C0124")
                .unwrap_err()
                .case,
            "8.4x"
        );
        assert_eq!(go(Mobility::Vehicle, "C0224", "C0124"), Ok(Cp::whole(4)));
        assert_eq!(go(Mobility::Recce, "C0224", "C0124"), Ok(Cp::whole(4)));
        assert_eq!(go(Mobility::Foot, "C0224", "C0124"), Ok(Cp::whole(3)));
        // Salt marsh off the track: vehicles no; recce, light trucks and
        // foot yes. By the track, every vehicle.
        assert_eq!(
            go(Mobility::Vehicle, "C0127", "C0128").unwrap_err().case,
            "8.4x"
        );
        assert_eq!(
            go(Mobility::Vehicle, "C0128", "C0127").unwrap_err().case,
            "8.4x"
        );
        assert_eq!(go(Mobility::Recce, "C0127", "C0128"), Ok(Cp::whole(2)));
        assert_eq!(
            go(Mobility::LightVehicle, "C0127", "C0128"),
            Ok(Cp::whole(2))
        );
        assert_eq!(go(Mobility::Foot, "C0127", "C0128"), Ok(Cp::whole(3)));
        assert_eq!(go(Mobility::Vehicle, "C0227", "C0228"), Ok(Cp::whole(1)));
        // Sea is never entered.
        assert_eq!(
            go(Mobility::Foot, "C3923", "C4022").unwrap_err().case,
            "8.37"
        );
    }

    #[test]
    fn weather_changes_the_price() {
        let (map, chart) = fixtures();
        let storm = |m, a, b| cost(&map, &chart, m, a, b, Weather::Sandstorm);
        let rain = |m, a, b| cost(&map, &chart, m, a, b, Weather::Rainstorm);
        // A sandstorm doubles everything, roads included.
        assert_eq!(storm(Mobility::Foot, "C3922", "C3923"), Ok(Cp::whole(4)));
        assert_eq!(storm(Mobility::Vehicle, "C4020", "C4120"), Ok(Cp::whole(1)));
        assert_eq!(storm(Mobility::Foot, "C0707", "C0708"), Ok(Cp::whole(3)));
        // Rain makes the road a track (½ of clear = 1 for a vehicle) and
        // closes wadis off the road.
        assert_eq!(rain(Mobility::Vehicle, "C4020", "C4120"), Ok(Cp::whole(1)));
        assert_eq!(
            rain(Mobility::Foot, "C0821", "C0822").unwrap_err().case,
            "29.5x"
        );
        // The road is still a road for the rain prohibitions: a wadi is
        // crossed by it at +2 on top of the track rate (A2307|A2406: 1 +
        // 2), a river too (E1330|E1430, foot: ½ + 1½), and vehicles
        // enter delta by it (E3614|E3615: 4 halved).
        assert_eq!(rain(Mobility::Vehicle, "A2307", "A2406"), Ok(Cp::whole(3)));
        assert_eq!(rain(Mobility::Foot, "A2307", "A2406"), Ok(Cp::whole(3)));
        assert_eq!(rain(Mobility::Foot, "E1330", "E1430"), Ok(Cp::whole(2)));
        assert!(rain(Mobility::Vehicle, "E1330", "E1430").is_ok());
        assert_eq!(rain(Mobility::Vehicle, "E3614", "E3615"), Ok(Cp::whole(2)));
        // Off the road delta is closed to vehicles in rain, but not to
        // foot units.
        assert_eq!(
            rain(Mobility::Vehicle, "E3614", "E3514").unwrap_err().case,
            "29.5x"
        );
        assert_eq!(rain(Mobility::Foot, "E3614", "E3514"), Ok(Cp::whole(2)));
        // A vehicle already in delta stays put unless it leaves by road
        // or rail: E3514 to E3513 (clear) across a plain hexside is
        // refused in rain; a foot unit goes.
        assert_eq!(
            rain(Mobility::Vehicle, "E3514", "E3513").unwrap_err().case,
            "29.5x"
        );
        assert_eq!(rain(Mobility::Foot, "E3514", "E3513"), Ok(Cp::whole(2)));
    }

    #[test]
    fn a_seam_hexside_is_priced_like_any_other() {
        let (map, chart) = fixtures();
        // C0633 (rough) to D0701 (clear) across a seam slope; cna records
        // no `up` for seam slopes and says to read them as the feature
        // both ways, so it climbs in either direction: foot 2 + 2 into
        // clear, a vehicle 4 + 4 into rough.
        assert_eq!(
            cost(
                &map,
                &chart,
                Mobility::Foot,
                "C0633",
                "D0701",
                Weather::Normal
            ),
            Ok(Cp::whole(4))
        );
        assert_eq!(
            cost(
                &map,
                &chart,
                Mobility::Vehicle,
                "D0701",
                "C0633",
                Weather::Normal
            ),
            Ok(Cp::whole(8))
        );
    }

    #[test]
    fn a_railroad_opens_swamp_and_major_river_hexsides() {
        let (map, chart) = fixtures();
        let foot = |a, b, w| cost(&map, &chart, Mobility::Foot, a, b, w);
        let mot = |a, b, w| cost(&map, &chart, Mobility::Vehicle, a, b, w);
        // E3211 (clear) to E3312 (swamp) by railroad alone: the chart
        // allows swamp by road or railroad, priced as the road (8.37).
        assert_eq!(foot("E3211", "E3312", Weather::Normal), Ok(Cp::whole(1)));
        assert_eq!(mot("E3211", "E3312", Weather::Normal), Ok(Cp::HALF));
        // E2829|E2830: a major river with a railroad across it, delta on
        // both banks. A vehicle crosses free (footnote 11) and pays the
        // delta hex; a foot unit pays the river on top of the hex.
        assert_eq!(mot("E2829", "E2830", Weather::Normal), Ok(Cp::whole(4)));
        assert_eq!(foot("E2829", "E2830", Weather::Normal), Ok(Cp::whole(10)));
        // In a rainstorm the railroad still opens the river and the delta
        // to vehicles (29.5x).
        assert_eq!(mot("E2829", "E2830", Weather::Rainstorm), Ok(Cp::whole(4)));
    }
}
