# Mission 5: Axis convoy bombing under abstract supply (32.6x)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 4, *Arrivals*);
slice 4 (`2026-10-24-m5-slice-4-arrivals.md`) landed Axis convoys in the
Tripoli box and left bombing out for want of lane data. cna now has the
lanes (`convoy-lanes.json`, 56.1) and rulings R-126, R-127 and R-128.
Every decision here is the autopilot's under Brian's delegation of
2026-09-26 and is reversible.

**Goal.** An Axis convoy can be sunk on its way: the Commonwealth's
westward advance turns into bomb points against it, two dice on the Air
Bombardment Table's convoy rows decide the loss, and dummies count only
what landed.

**Non-goals.** Lanes beyond Tripoli (Benghazi, Tobruk, Bizerta): they need
ports as destinations, captured ports cancelling convoys (56.15) and
Bizerta's activation roll (55.17); motorisation and replacement points by
convoy (convoys carry only supply units today, so a 5 % or 10 % result
sinks nothing); tank and gun losses.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Lanes | `cna-data::convoy::ConvoyLanes` loads `convoy-lanes.json`; the Axis convoy sails lane 2 (West Sicily to Tripoli), the only open lane whose destination the engine lands supply at | Lane 2 is open from the start; the others wait for port destinations |
| "Full division" (§19) | Engine reading: a Commonwealth HQ on the map whose OA echelon is division, as the air lift's R-124 check reads it. The westernmost by sheet, then column | The engine does not track a division's strength against its TOE; an HQ on the map is the reversible stand-in |
| Bomb points | `ConvoyBombing::bomb_points(2, sheet, col)` for that HQ; none reached is no attack (R-126), no dice drawn | R-126 |
| Dice | Two dice, read in order, from a stream of their own seeded by game seed, turn and a bombing salt, like planning's | Bombing never shifts the dice of play, so replays stay byte-equal |
| Losses | 5 % and 10 % hit nothing (no motorisation or replacement points aboard, R-127); 20 %+ sinks that share of the supply units, fractions down, at least one | 32.65, R-127 |
| Dummies | One per three supply units that land, after bombing | R-128 |

## Tasks

### Task 1: lanes as data

- [x] Failing test: `ConvoyLanes` loads six lanes; lane 2 ends in the
  Tripoli box and is open from the start.

### Task 2: bombing at the convoy stage

- [x] Failing tests: no division west of the first threshold, no attack and
  no event; a division far west, a 20 %+ result sinks at least one; dummies
  are counted from survivors; the event records lane, bomb points, reading
  and losses.
- [x] Implement in `cna-game::convoy::supply_arrivals`.

### Task 3: docs and gates

- [x] `NOT_SIMULATED.md`, design Status; goldens re-pinned if they move.

## Testing

`cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`,
`cargo run -p cna-probe -- check`.
