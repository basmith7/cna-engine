//! Set-up works a player places (59.12): the `construction` rows with a
//! `side` (Rommel's Arrival's Tobruk forts, 61.6; The Last Chance's
//! Bardia forts, 63.76) and the minefield sets (63.76–63.77), with the
//! `placed_works` module on. Each becomes a `WorkQuota` its side fills
//! with `Order::PlaceWork` during its set-up; the set-up does not pass on
//! until every required quota is full, and a side with only optional
//! (`up_to`) room left ends it with `Pass`. Whether a field is real is
//! the owner's secret, as for one laid in play (`construction::Minefield`).

use crate::construction::{Item, Minefield};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::place::{allowed, area};
use crate::state::{GameState, Pending};
use cna_data::map::{HexId, Terrain};
use cna_data::scenario::{MinefieldSet, Side, Works};
use serde::{Deserialize, Serialize};

/// What a side still places at set-up: the scenario's fortification row
/// or minefield set `index`, and what it has placed of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkQuota {
    pub side: Side,
    /// A minefield set; else a fortification row.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub minefields: bool,
    /// The row's or set's index in the scenario's `construction` block.
    pub index: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placed: Vec<HexId>,
    /// Of `placed`, the real fields (minefield sets only).
    #[serde(default, skip_serializing_if = "crate::raiders::is_zero")]
    pub real: u32,
    /// An optional set its side declined the rest of (`Pass`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
}

/// `Map::within` results by centre and radius, kept across one query.
type Near = std::collections::BTreeMap<(HexId, u32), std::collections::BTreeSet<HexId>>;

/// Whether a minefield could be laid in this terrain (24.35, R-134).
fn mine_terrain(t: Option<Terrain>) -> bool {
    matches!(t, Some(Terrain::Clear | Terrain::Gravel | Terrain::Rough))
}

/// Whether a fortification could stand in this terrain (24.44).
fn fort_terrain(t: Option<Terrain>) -> bool {
    !matches!(
        t,
        Some(
            Terrain::Sea
                | Terrain::MajorCity
                | Terrain::SaltMarsh
                | Terrain::Delta
                | Terrain::Mountain
                | Terrain::Desert
        ) | None
    )
}

impl WorkQuota {
    fn set<'a>(&self, works: &'a Works) -> Option<&'a MinefieldSet> {
        self.minefields
            .then(|| works.minefield_sets.get(self.index))
            .flatten()
    }

    /// How many it holds in all: a fort row's count, a set's real and
    /// dummy together.
    fn total(&self, works: &Works) -> u32 {
        match self.set(works) {
            Some(s) => s.real + s.dummy,
            None => works.fortifications.get(self.index).map_or(0, |r| r.count),
        }
    }

    fn optional(&self, works: &Works) -> bool {
        self.set(works).is_some_and(|s| s.up_to)
    }

    fn full(&self, works: &Works) -> bool {
        self.placed.len() as u32 >= self.total(works)
    }

    /// Still holding up its side's set-up: required and not full, or
    /// optional, open and not full.
    fn open(&self, works: &Works) -> bool {
        !self.full(works) && !(self.optional(works) && self.closed)
    }

    fn required_open(&self, works: &Works) -> bool {
        !self.full(works) && !self.optional(works)
    }

    /// Why `item` may not go into `hex` under this quota, judged on its
    /// area and counts alone.
    fn refusal(
        &self,
        data: &Data,
        works: &Works,
        item: Item,
        hex: &HexId,
        near: &mut Near,
    ) -> Option<String> {
        if self.full(works) {
            return Some("its set-up works are all placed".into());
        }
        if self.placed.contains(hex) {
            return Some(format!("{hex} already has one"));
        }
        let terrain = data.map.hex(hex).map(|h| h.terrain);
        let Some(set) = self.set(works) else {
            let row = works.fortifications.get(self.index)?;
            if item != Item::Fortification {
                return Some("this row places fortifications".into());
            }
            if !allowed(data, &row.placement, hex) {
                return Some(format!("{hex} is outside the row's placement"));
            }
            if !fort_terrain(terrain) {
                return Some(format!("no fortification stands in {hex}'s terrain"));
            }
            return None;
        };
        let real = match item {
            Item::RealMinefield => true,
            Item::DummyMinefield => false,
            _ => return Some("this set places minefields".into()),
        };
        let dummies = self.placed.len() as u32 - self.real;
        if real && self.real >= set.real {
            return Some(format!("all {} real minefields are placed", set.real));
        }
        if !real && dummies >= set.dummy {
            return Some(format!("all {} dummy minefields are placed", set.dummy));
        }
        if set.hexes.contains(hex) {
            return None;
        }
        if let Some(more) = &set.more {
            let extra = self
                .placed
                .iter()
                .filter(|h| !set.hexes.contains(h))
                .count() as u32;
            if extra >= more.count {
                return Some(format!(
                    "all {} of the owner's choice are placed",
                    more.count
                ));
            }
            if !allowed(data, &more.placement, hex) {
                return Some(format!("{hex} is outside the set's placement"));
            }
        } else if let Some(p) = &set.placement {
            if !allowed(data, p, hex) {
                return Some(format!("{hex} is outside the set's placement"));
            }
            if let Some(b) = &set.beyond
                && near
                    .entry((b.of.clone(), b.hexes))
                    .or_insert_with(|| data.map.within(&b.of, b.hexes))
                    .contains(hex)
            {
                return Some(format!("{hex} is within {} hexes of {}", b.hexes, b.of));
            }
        } else {
            return Some(format!("{hex} is not one of the set's hexes"));
        }
        (!mine_terrain(terrain)).then(|| format!("no minefield could be laid in {hex}'s terrain"))
    }
}

impl GameState {
    /// The scenario's player-placed works as quotas, with the module on.
    pub(crate) fn start_placed_works(&mut self, data: &Data) -> anyhow::Result<()> {
        if !self.modules.placed_works {
            return Ok(());
        }
        let works = data.scenario.works()?;
        for (index, row) in works.fortifications.iter().enumerate() {
            if let Some(side) = row.side {
                self.works_to_place.push(WorkQuota {
                    side,
                    minefields: false,
                    index,
                    placed: Vec::new(),
                    real: 0,
                    closed: false,
                });
            }
        }
        for (index, set) in works.minefield_sets.iter().enumerate() {
            self.works_to_place.push(WorkQuota {
                side: set.side,
                minefields: true,
                index,
                placed: Vec::new(),
                real: 0,
                closed: false,
            });
        }
        Ok(())
    }

    /// Whether `side` still has works to place, or optional ones it has
    /// not declined.
    pub(crate) fn works_open(&self, data: &Data, side: Side) -> bool {
        if self.works_to_place.is_empty() {
            return false;
        }
        let Ok(works) = data.scenario.works() else {
            return false;
        };
        self.works_to_place
            .iter()
            .any(|q| q.side == side && q.open(&works))
    }

    /// Where `place_work` would accept `item` from `side` now, in id
    /// order: empty when that side is not setting up.
    pub fn work_placements(&self, data: &Data, side: Side, item: Item) -> Vec<HexId> {
        if self.pending != Some(Pending::SetUp { side }) {
            return Vec::new();
        }
        let Ok(works) = data.scenario.works() else {
            return Vec::new();
        };
        let mut v: Vec<HexId> = Vec::new();
        let mut near = Near::new();
        for q in self.works_to_place.iter().filter(|q| q.side == side) {
            let candidates: Vec<HexId> = match q.set(&works) {
                None => works
                    .fortifications
                    .get(q.index)
                    .map(|r| area(data, &r.placement))
                    .unwrap_or_default(),
                Some(s) => {
                    let mut c = s.hexes.clone();
                    if let Some(m) = &s.more {
                        c.extend(area(data, &m.placement));
                    }
                    if let Some(p) = &s.placement {
                        c.extend(area(data, p));
                    }
                    c
                }
            };
            v.extend(candidates.into_iter().filter(|h| {
                self.check_work(data, &works, side, item, h, &mut near)
                    .is_ok()
            }));
        }
        v.sort();
        v.dedup();
        v
    }

    /// At `side`'s set-up with works to place: the hexes open to each
    /// item (empty lists dropped) and whether `Pass` would end its set-up.
    pub fn work_options(&self, data: &Data, side: Side) -> Option<serde_json::Value> {
        if self.pending != Some(Pending::SetUp { side })
            || !self.works_to_place.iter().any(|q| q.side == side)
        {
            return None;
        }
        let works = data.scenario.works().ok()?;
        let mut o = serde_json::Map::new();
        for item in [
            Item::Fortification,
            Item::RealMinefield,
            Item::DummyMinefield,
        ] {
            let hexes = self.work_placements(data, side, item);
            if !hexes.is_empty() {
                o.insert(
                    serde_json::to_value(item).ok()?.as_str()?.to_owned(),
                    serde_json::json!(hexes),
                );
            }
        }
        let left: u32 = self
            .works_to_place
            .iter()
            .filter(|q| q.side == side && q.required_open(&works))
            .map(|q| q.total(&works) - q.placed.len() as u32)
            .sum();
        let pieces_left = self
            .pieces
            .values()
            .any(|p| p.side == side && p.is_pending());
        o.insert("required_left".into(), serde_json::json!(left));
        o.insert(
            "may_pass".into(),
            serde_json::json!(left == 0 && !pieces_left),
        );
        Some(serde_json::Value::Object(o))
    }

    /// The quota of `side` that takes `item` in `hex`: the first, in
    /// scenario order, that accepts it.
    fn check_work(
        &self,
        data: &Data,
        works: &Works,
        side: Side,
        item: Item,
        hex: &HexId,
        near: &mut Near,
    ) -> Result<usize, Illegal> {
        if item.is_minefield() && self.minefields.contains_key(hex) {
            return Err(Illegal::new(
                "24.35",
                format!("{hex} already has a minefield"),
            ));
        }
        let mut why = None;
        for (i, q) in self.works_to_place.iter().enumerate() {
            if q.side != side || (q.closed && q.optional(works)) {
                continue;
            }
            match q.refusal(data, works, item, hex, near) {
                None => return Ok(i),
                // The reason worth giving is a quota of the item's kind.
                Some(r) if q.minefields == item.is_minefield() => why = why.or(Some(r)),
                Some(_) => {}
            }
        }
        Err(Illegal::new(
            "59.12",
            why.unwrap_or_else(|| format!("{side} has no set-up works of that kind to place")),
        ))
    }

    /// `Order::PlaceWork`: the side setting up places a fortification
    /// level or a real or dummy minefield its scenario gives it.
    pub(crate) fn place_work(
        &mut self,
        data: &Data,
        player: Side,
        item: Item,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        if self.pending != Some(Pending::SetUp { side: player }) {
            return Err(Illegal::new("59.2", format!("{player} is not setting up")));
        }
        let works = data
            .scenario
            .works()
            .map_err(|e| Illegal::new("59.12", format!("{e:#}")))?;
        let i = self.check_work(data, &works, player, item, hex, &mut Near::new())?;
        let q = &mut self.works_to_place[i];
        q.placed.push(hex.clone());
        let mut events = Vec::new();
        if item == Item::Fortification {
            let level = works.fortifications[q.index].level;
            let at = self.fortifications.entry(hex.clone()).or_insert(0);
            *at = (*at).max(level);
            events.push(Event::FortificationBuilt {
                side: player,
                hex: hex.clone(),
                level: *at,
            });
        } else {
            let real = item == Item::RealMinefield;
            if real {
                q.real += 1;
            }
            self.minefields.insert(
                hex.clone(),
                Minefield {
                    side: player,
                    real,
                    revealed: false,
                },
            );
            events.push(Event::MinefieldLaid {
                side: player,
                hex: hex.clone(),
            });
        }
        events.extend(self.advance_set_up(data));
        Ok(events)
    }

    /// `Pass` at set-up: a side with every piece and required work placed
    /// declines the optional works it has room for.
    pub(crate) fn decline_works(
        &mut self,
        data: &Data,
        player: Side,
    ) -> Result<Vec<Event>, Illegal> {
        let works = data
            .scenario
            .works()
            .map_err(|e| Illegal::new("59.12", format!("{e:#}")))?;
        let pieces_left = self
            .pieces
            .values()
            .any(|p| p.side == player && p.is_pending());
        let required = self
            .works_to_place
            .iter()
            .any(|q| q.side == player && q.required_open(&works));
        if self.pending != Some(Pending::SetUp { side: player }) || pieces_left || required {
            return Err(Illegal::new(
                "59.2",
                "a side passes at set-up only with every piece and required work placed",
            ));
        }
        for q in self.works_to_place.iter_mut().filter(|q| q.side == player) {
            q.closed = true;
        }
        Ok(self.advance_set_up(data))
    }

    /// After a set-up order: the next side to set up, or play begins.
    pub(crate) fn advance_set_up(&mut self, data: &Data) -> Vec<Event> {
        self.pending = self.next_set_up_with(data);
        let mut events = Vec::new();
        if self.pending.is_none() {
            events.push(Event::SetUpComplete);
            events.extend(self.start_play(data));
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::state::Modules;
    use crate::state::test_support::place_pieces;
    use crate::view::{Aliases, leaks, view_for};

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn work(item: Item, h: &str) -> Order {
        Order::PlaceWork { item, hex: hex(h) }
    }

    /// A new game with every module on and `side`'s pieces all placed
    /// (the Axis's whole set-up first for the Commonwealth), so only its
    /// works hold the set-up.
    fn ready(data: &Data, side: Side) -> GameState {
        let mut s = GameState::with_modules(data, 1, Modules::ALL).unwrap();
        if side == Side::Cw {
            place_pieces(data, &mut s, Side::Axis);
            while s.pending == Some(Pending::SetUp { side: Side::Axis }) {
                let order = s
                    .work_placements(data, Side::Axis, Item::Fortification)
                    .into_iter()
                    .chain(s.work_placements(data, Side::Axis, Item::RealMinefield))
                    .chain(s.work_placements(data, Side::Axis, Item::DummyMinefield))
                    .next()
                    .map_or(Order::Pass, |h| {
                        let item = if s
                            .work_placements(data, Side::Axis, Item::Fortification)
                            .contains(&h)
                        {
                            Item::Fortification
                        } else if s
                            .work_placements(data, Side::Axis, Item::RealMinefield)
                            .contains(&h)
                        {
                            Item::RealMinefield
                        } else {
                            Item::DummyMinefield
                        };
                        Order::PlaceWork { item, hex: h }
                    });
                s.apply(data, Side::Axis, order).unwrap();
            }
        }
        place_pieces(data, &mut s, side);
        assert_eq!(s.pending, Some(Pending::SetUp { side }));
        s
    }

    #[test]
    fn rommels_arrival_cw_places_four_forts_within_two_of_tobruk_61_6() {
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        // Outside the two hexes: refused.
        let far = s.apply(&data, Side::Cw, work(Item::Fortification, "C4021"));
        assert!(far.is_err(), "{far:?}");
        // The Axis may not place the Commonwealth's.
        assert!(
            s.apply(&data, Side::Axis, work(Item::Fortification, "C4807"))
                .is_err()
        );
        let ok = s.work_placements(&data, Side::Cw, Item::Fortification);
        assert!(ok.len() >= 4, "{ok:?}");
        let near = data.map.within(&hex("C4807"), 2);
        assert!(ok.iter().all(|h| near.contains(h)));
        // A minefield is not this scenario's.
        assert!(
            s.apply(&data, Side::Cw, work(Item::RealMinefield, ok[0].as_str()))
                .is_err()
        );
        for h in &ok[..3] {
            s.apply(&data, Side::Cw, work(Item::Fortification, h.as_str()))
                .unwrap();
            assert_eq!(s.fortifications.get(h), Some(&1));
        }
        // Not twice in one hex; set-up still waits on the fourth.
        assert!(
            s.apply(&data, Side::Cw, work(Item::Fortification, ok[0].as_str()))
                .is_err()
        );
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        let events = s
            .apply(&data, Side::Cw, work(Item::Fortification, ok[3].as_str()))
            .unwrap();
        assert!(events.contains(&Event::SetUpComplete));
        assert!(s.pending.is_none() || s.pending != Some(Pending::SetUp { side: Side::Cw }));
        // A fifth: over the count, and set-up is over anyway.
        assert!(
            s.apply(&data, Side::Cw, work(Item::Fortification, ok[4].as_str()))
                .is_err()
        );
    }

    #[test]
    fn no_set_up_fort_in_a_major_city_24_44() {
        // A city keeps its own level (2): a fort there would lower it.
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        let tobruk = hex("C4807");
        assert!(!s.work_placements(&data, Side::Cw, Item::Fortification).contains(&tobruk));
        assert!(
            s.apply(&data, Side::Cw, work(Item::Fortification, "C4807"))
                .is_err()
        );
        assert_eq!(s.fortifications.get(&tobruk), None);
        let data = Data::load("the-last-chance").unwrap();
        let s = ready(&data, Side::Axis);
        assert!(
            !s.work_placements(&data, Side::Axis, Item::Fortification)
                .contains(&hex("C4321")),
            "Bardia itself is not one of the four next to it"
        );
    }

    #[test]
    fn last_chance_axis_picks_three_of_bardias_four_63_76() {
        let data = Data::load("the-last-chance").unwrap();
        let mut s = ready(&data, Side::Axis);
        let bardia = ["C4220", "C4221", "C4320", "C4420"];
        assert!(
            s.apply(&data, Side::Axis, work(Item::Fortification, "C4807"))
                .is_err()
        );
        for h in &bardia[..3] {
            s.apply(&data, Side::Axis, work(Item::Fortification, h))
                .unwrap();
            assert_eq!(s.fortifications.get(&hex(h)), Some(&1));
        }
        let fourth = s
            .apply(&data, Side::Axis, work(Item::Fortification, bardia[3]))
            .unwrap_err();
        assert!(fourth.reason.contains("all placed"), "{fourth:?}");
    }

    #[test]
    fn last_chance_axis_lays_eleven_real_and_three_dummy_in_its_fourteen_63_77() {
        let data = Data::load("the-last-chance").unwrap();
        let mut s = ready(&data, Side::Axis);
        let works = data.scenario.works().unwrap();
        let listed = works.minefield_sets[1].hexes.clone();
        // Not a listed hex, nor near Tobruk, nor on sheets A-C: refused.
        assert!(
            s.apply(&data, Side::Axis, work(Item::RealMinefield, "D2001"))
                .is_err()
        );
        for (i, h) in listed.iter().enumerate() {
            let item = if i < 3 {
                Item::DummyMinefield
            } else {
                Item::RealMinefield
            };
            if i == 3 {
                // A fourth dummy is over the count.
                let e = s
                    .apply(&data, Side::Axis, work(Item::DummyMinefield, h.as_str()))
                    .unwrap_err();
                assert!(e.reason.contains("dummy"), "{e:?}");
            }
            s.apply(&data, Side::Axis, work(item, h.as_str())).unwrap();
            assert_eq!(s.minefields[h].real, item == Item::RealMinefield);
            assert_eq!(s.minefields[h].side, Side::Axis);
        }
        // The forts are still to place: set-up holds.
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Axis }));
        assert!(s.apply(&data, Side::Axis, Order::Pass).is_err());
        for h in ["C4220", "C4221", "C4320"] {
            s.apply(&data, Side::Axis, work(Item::Fortification, h))
                .unwrap();
        }
        // Only the optional sets are left: one near Tobruk, then decline.
        let near = s.work_placements(&data, Side::Axis, Item::RealMinefield);
        let tobruk = data.map.within(&hex("C4807"), 3);
        let first = near.iter().find(|h| tobruk.contains(h)).unwrap().clone();
        assert!(
            near.iter().all(|h| tobruk.contains(h) || h.sheet() <= 'C'),
            "{near:?}"
        );
        s.apply(&data, Side::Axis, work(Item::RealMinefield, first.as_str()))
            .unwrap();
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Axis }));
        s.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        // The Commonwealth sees the fields, not which are real.
        let aliases = Aliases::new([1; 32]);
        let v = view_for(&s, &data, &aliases, Some(Side::Cw), false).unwrap();
        for h in &listed {
            assert_eq!(v["minefields"][h.as_str()]["side"], "axis");
            assert!(v["minefields"][h.as_str()].get("real").is_none());
        }
        assert!(
            v.get("works_to_place").is_none_or(|w| w
                .as_array()
                .unwrap()
                .iter()
                .all(|q| q["side"] == "cw"))
        );
        assert!(
            leaks(&s, &v, Side::Axis).is_empty(),
            "{:?}",
            leaks(&s, &v, Side::Axis)
        );
        let full = serde_json::to_value(&s).unwrap();
        assert!(
            leaks(&s, &full, Side::Axis)
                .iter()
                .any(|l| l.contains("minefields")),
        );
    }

    #[test]
    fn last_chance_cw_fills_its_ten_and_five_more_on_sheet_e_63_76() {
        let data = Data::load("the-last-chance").unwrap();
        let mut s = ready(&data, Side::Cw);
        let works = data.scenario.works().unwrap();
        let listed = works.minefield_sets[0].hexes.clone();
        for h in &listed {
            s.apply(&data, Side::Cw, work(Item::RealMinefield, h.as_str()))
                .unwrap();
        }
        // An eleventh real one is over the count.
        let more = s.work_placements(&data, Side::Cw, Item::DummyMinefield);
        assert!(more.iter().all(|h| h.sheet() == 'E' && !listed.contains(h)));
        assert!(
            s.apply(&data, Side::Cw, work(Item::RealMinefield, more[0].as_str()))
                .is_err()
        );
        // Off sheet E: refused.
        assert!(
            s.apply(&data, Side::Cw, work(Item::DummyMinefield, "D2001"))
                .is_err()
        );
        for h in &more[..4] {
            s.apply(&data, Side::Cw, work(Item::DummyMinefield, h.as_str()))
                .unwrap();
        }
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        let events = s
            .apply(
                &data,
                Side::Cw,
                work(Item::DummyMinefield, more[4].as_str()),
            )
            .unwrap();
        assert!(events.contains(&Event::SetUpComplete));
        // The Axis view hides which are dummies.
        let aliases = Aliases::new([1; 32]);
        let v = view_for(&s, &data, &aliases, Some(Side::Axis), false).unwrap();
        assert!(v["minefields"][more[0].as_str()].get("real").is_none());
        assert!(leaks(&s, &v, Side::Cw).is_empty());
    }

    #[test]
    fn with_the_module_off_no_works_wait() {
        let data = Data::load("the-last-chance").unwrap();
        let off = Modules {
            placed_works: false,
            ..Modules::ALL
        };
        let mut s = GameState::with_modules(&data, 1, off).unwrap();
        assert!(s.works_to_place.is_empty());
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        assert_ne!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        assert!(s.minefields.is_empty());
    }
}
