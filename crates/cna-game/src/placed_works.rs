//! Set-up works a player places (59.12): the `construction` rows with a
//! `side` (Rommel's Arrival's Tobruk forts, 61.6; The Last Chance's
//! Bardia forts, 63.76) and the minefield sets (63.76–63.77), with the
//! `placed_works` module on. Each becomes a `WorkQuota` its side fills
//! with `Order::PlaceWork` during its set-up; the set-up does not pass on
//! until every required quota is full, and a side with only optional
//! (`up_to`) room left ends it with `Pass`: Rommel's Arrival's and
//! Bardia's forts are a maximum (R-161). Whether a field is real is
//! the owner's secret, as for one laid in play (`construction::Minefield`).
//!
//! A work in a hex the scenario prints stands as printed; one whose hex
//! the owner chooses goes only where §24 would let it be started, enemy
//! ZOC included, judged against the finished set-up (R-162). A side
//! places against the enemy units already set up; once both sides are
//! done, a chosen work in an enemy ZOC is taken off and its side's set-up
//! reopens to place it again (`void_works_in_enemy_zoc`).

use crate::construction::{Item, Minefield};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::place::{Near, allowed_near, area};
use crate::state::{GameState, Pending};
use crate::zoc::Zoc;
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
        match self.set(works) {
            Some(s) => s.up_to,
            None => works
                .fortifications
                .get(self.index)
                .is_some_and(|r| r.up_to),
        }
    }

    /// Whether `hex` is one the owner chose rather than one the scenario
    /// prints: every fortification row with a side, and a minefield set's
    /// fields beyond its list (R-162).
    fn chosen(&self, works: &Works, hex: &HexId) -> bool {
        self.set(works).is_none_or(|s| !s.hexes.contains(hex))
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
            if !allowed_near(data, &row.placement, hex, near) {
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
            if !allowed_near(data, &more.placement, hex, near) {
                return Some(format!("{hex} is outside the set's placement"));
            }
        } else if let Some(p) = &set.placement {
            if !allowed_near(data, p, hex, near) {
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
        let candidates = self.work_candidates(data, &works, side);
        self.accepting(data, &works, side, item, &candidates, &mut Near::new())
    }

    /// Every hex one of `side`'s quotas lists or whose placement takes
    /// in, in id order and once each.
    fn work_candidates(&self, data: &Data, works: &Works, side: Side) -> Vec<HexId> {
        let mut c: Vec<HexId> = Vec::new();
        for q in self.works_to_place.iter().filter(|q| q.side == side) {
            match q.set(works) {
                None => {
                    if let Some(r) = works.fortifications.get(q.index) {
                        c.extend(area(data, &r.placement));
                    }
                }
                Some(s) => {
                    c.extend(s.hexes.iter().cloned());
                    if let Some(m) = &s.more {
                        c.extend(area(data, &m.placement));
                    }
                    if let Some(p) = &s.placement {
                        c.extend(area(data, p));
                    }
                }
            }
        }
        c.sort();
        c.dedup();
        c
    }

    /// Of `candidates`, those `place_work` would accept `item` in.
    fn accepting(
        &self,
        data: &Data,
        works: &Works,
        side: Side,
        item: Item,
        candidates: &[HexId],
        near: &mut Near,
    ) -> Vec<HexId> {
        candidates
            .iter()
            .filter(|h| self.check_work(data, works, side, item, h, near).is_ok())
            .cloned()
            .collect()
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
        let candidates = self.work_candidates(data, &works, side);
        let mut near = Near::new();
        let mut o = serde_json::Map::new();
        for item in [
            Item::Fortification,
            Item::RealMinefield,
            Item::DummyMinefield,
        ] {
            let hexes = self.accepting(data, &works, side, item, &candidates, &mut near);
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
            .any(|p| p.side == side && p.is_pending())
            || self.motorisation_to_place.contains_key(&side);
        let optional: u32 = self
            .works_to_place
            .iter()
            .filter(|q| q.side == side && q.optional(&works) && q.open(&works))
            .map(|q| q.total(&works) - q.placed.len() as u32)
            .sum();
        o.insert("required_left".into(), serde_json::json!(left));
        o.insert("optional_left".into(), serde_json::json!(optional));
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
            // A quota of the other kind always refuses, and the reason
            // worth giving is one of the item's kind.
            if q.side != side
                || (q.closed && q.optional(works))
                || q.minefields != item.is_minefield()
            {
                continue;
            }
            let r = q.refusal(data, works, item, hex, near).or_else(|| {
                (q.chosen(works, hex) && self.enemy_controlled(data, hex, side))
                    .then(|| format!("{hex} is in an enemy ZOC (24.35, 24.44, R-162)"))
            });
            match r {
                None => return Ok(i),
                Some(r) => why = why.or(Some(r)),
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
        let motorisation_left = self.motorisation_to_place.contains_key(&player);
        if self.pending != Some(Pending::SetUp { side: player })
            || pieces_left
            || required
            || motorisation_left
        {
            return Err(Illegal::new(
                "59.2",
                "a side passes at set-up only with every piece, motorisation point and required work placed",
            ));
        }
        for q in self.works_to_place.iter_mut().filter(|q| q.side == player) {
            q.closed = true;
        }
        Ok(self.advance_set_up(data))
    }

    /// Whether the enemy holds `hex` or exerts a ZOC into it, for `side`.
    fn enemy_controlled(&self, data: &Data, hex: &HexId, side: Side) -> bool {
        self.holder(hex) == Some(side.other()) || self.zoc_at(data, hex, side) != Zoc::Clear
    }

    /// Once both sides have set up: every work its owner chose a hex for
    /// that now stands in an enemy ZOC is taken off, and its quota opens
    /// again (R-162). A fortification level comes off only down to none:
    /// no printed row shares a hex with a chosen one.
    fn void_works_in_enemy_zoc(&mut self, data: &Data) -> Vec<Event> {
        let Ok(works) = data.scenario.works() else {
            return Vec::new();
        };
        let mut voided = Vec::new();
        for (i, q) in self.works_to_place.iter().enumerate() {
            for h in &q.placed {
                if q.chosen(&works, h) && self.enemy_controlled(data, h, q.side) {
                    voided.push((i, h.clone()));
                }
            }
        }
        let mut events = Vec::new();
        for (i, h) in voided {
            let q = &mut self.works_to_place[i];
            q.placed.retain(|x| *x != h);
            q.closed = false;
            if q.minefields {
                if self.minefields.remove(&h).is_some_and(|m| m.real) {
                    q.real -= 1;
                }
            } else {
                let level = works.fortifications[q.index].level;
                if self.fortifications.get(&h).is_some_and(|&l| l <= level) {
                    self.fortifications.remove(&h);
                }
            }
            events.push(Event::SetUpWorkVoided {
                side: q.side,
                hex: h,
                minefield: q.minefields,
            });
        }
        events
    }

    /// After a set-up order: the next side to set up, or play begins.
    pub(crate) fn advance_set_up(&mut self, data: &Data) -> Vec<Event> {
        self.pending = self.next_set_up_with(data);
        let mut events = Vec::new();
        if self.pending.is_none() {
            events.extend(self.void_works_in_enemy_zoc(data));
            self.pending = self.next_set_up_with(data);
            self.works_reopened = self.pending.is_some();
        }
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
            fill_works(data, &mut s, Side::Axis);
        }
        place_pieces(data, &mut s, side);
        assert_eq!(s.pending, Some(Pending::SetUp { side }));
        s
    }

    /// `side` places its works at the first hexes the engine offers, then
    /// passes, until its set-up is over.
    fn fill_works(data: &Data, s: &mut GameState, side: Side) -> Vec<Event> {
        let mut events = Vec::new();
        while s.pending == Some(Pending::SetUp { side }) {
            let order = s
                .work_placements(data, side, Item::Fortification)
                .into_iter()
                .chain(s.work_placements(data, side, Item::RealMinefield))
                .chain(s.work_placements(data, side, Item::DummyMinefield))
                .next()
                .map_or(Order::Pass, |h| {
                    let item = if s
                        .work_placements(data, side, Item::Fortification)
                        .contains(&h)
                    {
                        Item::Fortification
                    } else if s
                        .work_placements(data, side, Item::RealMinefield)
                        .contains(&h)
                    {
                        Item::RealMinefield
                    } else {
                        Item::DummyMinefield
                    };
                    Order::PlaceWork { item, hex: h }
                });
            events.extend(s.apply(data, side, order).unwrap());
        }
        events
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

    /// A stack of `side`'s that exerts a ZOC, moved next to `target`
    /// where its ZOC reaches it.
    fn exert_on(data: &Data, s: &mut GameState, side: Side, target: &HexId) {
        let from = s
            .pieces
            .values()
            .filter(|p| p.side == side)
            .filter_map(|p| match &p.location {
                crate::piece::Location::Hex(h) => Some(h.clone()),
                _ => None,
            })
            .find(|h| s.holder(h) == Some(side) && s.could_exert(data, h))
            .expect("an exerting stack");
        let stack: Vec<crate::piece::PieceId> =
            s.pieces_at(&from).iter().map(|p| p.id.clone()).collect();
        for n in data.map.neighbours(target) {
            for id in &stack {
                s.put(id, crate::piece::Location::Hex(n.clone()));
            }
            if s.zoc_at(data, target, side.other()) != Zoc::Clear {
                return;
            }
        }
        panic!("no neighbour of {target} carries a ZOC into it");
    }

    #[test]
    fn rommels_arrival_forts_are_up_to_four_r_161() {
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Cw }));
        let events = s.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert!(events.contains(&Event::SetUpComplete));
        assert!(s.fortifications.is_empty(), "{:?}", s.fortifications);
    }

    #[test]
    fn last_chance_axis_may_stop_at_one_bardia_fort_r_161() {
        let data = Data::load("the-last-chance").unwrap();
        let mut s = ready(&data, Side::Axis);
        s.apply(&data, Side::Axis, work(Item::Fortification, "C4220"))
            .unwrap();
        // The minefields are still required; the forts are not.
        let o = s.work_options(&data, Side::Axis).unwrap();
        let fields: u64 = data
            .scenario
            .works()
            .unwrap()
            .minefield_sets
            .iter()
            .filter(|m| m.side == Side::Axis && !m.up_to)
            .map(|m| u64::from(m.real + m.dummy))
            .sum();
        assert_eq!(o["required_left"], serde_json::json!(fields));
    }

    #[test]
    fn a_chosen_fort_hex_in_enemy_zoc_is_refused_r_162() {
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        let site = s.work_placements(&data, Side::Cw, Item::Fortification)[0].clone();
        exert_on(&data, &mut s, Side::Axis, &site);
        assert!(
            !s.work_placements(&data, Side::Cw, Item::Fortification)
                .contains(&site)
        );
        let err = s
            .apply(&data, Side::Cw, work(Item::Fortification, site.as_str()))
            .unwrap_err();
        assert!(err.reason.contains("enemy ZOC"), "{err:?}");
    }

    #[test]
    fn a_work_the_enemy_set_up_next_to_is_placed_again_r_162() {
        let data = Data::load("the-last-chance").unwrap();
        let mut s = ready(&data, Side::Cw);
        let works = data.scenario.works().unwrap();
        let fort = s
            .works_to_place
            .iter()
            .find(|q| q.side == Side::Axis && !q.minefields)
            .and_then(|q| q.placed.first().cloned())
            .expect("an Axis fort placed at Bardia");
        assert!(works.fortifications.iter().any(|r| r.up_to));
        exert_on(&data, &mut s, Side::Cw, &fort);
        let events = fill_works(&data, &mut s, Side::Cw);
        assert!(
            events.contains(&Event::SetUpWorkVoided {
                side: Side::Axis,
                hex: fort.clone(),
                minefield: false,
            }),
            "{events:?}"
        );
        assert!(!events.contains(&Event::SetUpComplete));
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Axis }));
        assert_eq!(s.fortifications.get(&fort), None);
        // Only works move now: the Axis knows the whole Commonwealth
        // set-up, so its pieces stay put.
        let axis = s
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && matches!(p.location, crate::piece::Location::Hex(_)))
            .unwrap()
            .id
            .clone();
        let err = s
            .apply(&data, Side::Axis, Order::Unplace { piece: axis })
            .unwrap_err();
        assert!(err.reason.contains("pieces stay"), "{err:?}");
        assert!(
            !s.work_placements(&data, Side::Axis, Item::Fortification)
                .contains(&fort)
        );
        // The forts are a maximum: the Axis may leave it off.
        let events = s.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert!(events.contains(&Event::SetUpComplete), "{events:?}");
    }

    #[test]
    fn no_set_up_fort_in_a_major_city_24_44() {
        // A city keeps its own level (2): a fort there would lower it.
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 1, Modules::ALL).unwrap();
        place_pieces(&data, &mut s, Side::Axis);
        place_pieces(&data, &mut s, Side::Cw);
        let tobruk = hex("C4807");
        assert!(
            !s.work_placements(&data, Side::Cw, Item::Fortification)
                .contains(&tobruk)
        );
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
        // The forts are a maximum (R-161): set-up holds only for the
        // optional works the Axis has not declined.
        assert_eq!(s.pending, Some(Pending::SetUp { side: Side::Axis }));
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

    /// The placements offered are every hex on the map `place_work`'s
    /// check accepts, judged one hex at a time with nothing kept between
    /// them: the candidate areas miss none, and the circles kept across
    /// one query change no answer.
    #[test]
    fn work_placements_are_the_whole_map_judged_hex_by_hex() {
        let data = Data::load("the-last-chance").unwrap();
        let works = data.scenario.works().unwrap();
        for side in [Side::Axis, Side::Cw] {
            let s = ready(&data, side);
            let options = s.work_options(&data, side).unwrap();
            for item in [
                Item::Fortification,
                Item::RealMinefield,
                Item::DummyMinefield,
            ] {
                let mut each: Vec<HexId> = data
                    .map
                    .hexes()
                    .map(|h| h.id.clone())
                    .filter(|h| {
                        s.check_work(&data, &works, side, item, h, &mut Near::new())
                            .is_ok()
                    })
                    .collect();
                each.sort();
                let offered = s.work_placements(&data, side, item);
                assert_eq!(offered, each, "{side} {item:?}");
                let key = serde_json::to_value(item).unwrap();
                assert_eq!(
                    options.get(key.as_str().unwrap()),
                    (!offered.is_empty())
                        .then(|| serde_json::json!(offered))
                        .as_ref(),
                    "{side} {item:?}"
                );
            }
        }
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
