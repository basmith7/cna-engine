# Mission 5: desert raiders and the raid on Rommel (§27)

Spec: `docs/designs/2026-10-13-land-turn-design.md`, whose non-goals left
§27 to "its own plan written when it starts". With #100–#103 merged,
Graziani's land rules are complete bar the LRDG raid on Rommel (27.6x),
and §27 is the first land rule outside Graziani's that a scenario sets up
(Rommel's Arrival places one LRDG at Jalo). Every decision here is the
autopilot's under Brian's delegation of 2026-09-26 and is reversible.

**Goal.** A player can form a raider, move it unseen by the enemy, risk
being spotted, and raid Rommel. A networked view never shows an enemy
raider's hex unless the raider is on the map (27.31).

**Non-goals.** Raids on the pipeline, airfields, aircraft and truck convoys
(27.5x). Under the abstract block water is ignored and the Air and
Logistics Games are not built, so none of those targets exists. Commandos
(§28: Layforce, the SAS), which reuse hidden movement (27.88, E-028),
come in a later plan. Hot-seat play stays open-handed: one browser shows
both sides.

## Blocked on cna

Two requests are filed under **Requests for cna** in PROGRESS.md:

1. **No raider counters in the data.** Rommel's Arrival's Jalo placement
   has `"units": []` and a note that "the OA sheets print no LRDG
   counter". The engine needs the two LRDG counters and Sonderkommando
   Almasy as units (CPA 50, a recce vehicle, no ratings, 0 SP), even if
   they are marked as made by 27.13 rather than printed.
2. **Raiders under the abstract block.** 32.11 says to ignore all
   ammunition, fuel and water rules. Does a raider still have to call at
   a dump, oasis or well each game-turn (27.34), and die if it does not?
   Is a 32.1x supply unit ("shown by a supply dump counter") a supply
   dump that 27.5x can raid?

Task 1 can start before the answers by using a test-only raider piece.
Task 3 waits on request 1.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Piece | `PieceKind::Raider`: CPA 50, no ratings, 0 SP, no ZOC, never breaks down, never fights | 27.12, 27.32, 27.38 |
| Hidden | `Piece.hidden: bool` (`#[serde(default, skip_serializing_if = "Not::not")]`), set while off the map in 27.31's sense. The enemy view drops a hidden piece and every event that names it, the same way 3.61 removes enemy-only fields | 27.31; the view module already removes what the enemy may not see |
| Revealed | A raid, a spotting roll and forming each clear `hidden` for the rest of the stage. Moving off the map again sets it | 27.31 |
| Spotting | Rolled by the engine, on its own stream, when a raider enters an enemy-controlled hex. An enemy unit entering a raider's hex takes an order to spend 1 CP and roll. A 6 eliminates the raider | 27.41–27.44 |
| Raid on Rommel | `Order::RaidRommel`: an LRDG next to Rommel's hex (no unit enters an enemy-held hex, 8.1x) spends 10 CP and rolls on `raid-on-rommel.json`. Four attempts in a campaign game (a scenario whose slug names a campaign: cna has no flag), one elsewhere | 27.6x, 27.92 |
| Supply calls | Wait for cna's answer to request 2; until then raiders need none (32.11) and a NOT_SIMULATED row says so | 32.11 |

## Tasks

### Task 1: the engine (test-only raider piece)

- [x] Failing tests: an enemy view hides a raider and its `Moved`
  events; a raider that enters an enemy ZOC hex is spotted on a 6 and
  survives on 1–5; a raid on Rommel spends 10 CP and applies each row
  of the table; a fifth attempt in a campaign game is refused.
- [x] `Piece.hidden` and the view filtering: the enemy's state view and
  events drop a hidden piece and everything naming it, open information
  or not (`view::state::strip_hidden`). An event is judged against the
  state after its order, so a raider that moves hidden and is revealed
  in the same order shows that move.
- [x] `PieceKind::Raider`, spotting, `Order::RaidRommel`.
- [x] `NOT_SIMULATED.md`: R-135's row loses "the LRDG raid (27.9x)".
  New rows for the 27.5x targets and for supply calls under the abstract
  block.

### Task 2: the browser

- [x] Your own raiders are drawn with a hidden marker. Enemy raiders
  show only when on the map. A *Raid Rommel* button appears in the
  move panel. A Playwright test raids Rommel.

### Task 3: forming raiders (waits on cna request 1)

- [x] Forming in the organisation phase from a unit at maximum morale
  (27.13), and re-forming four turns after elimination (27.16, E-026).
  `Order::FormRaider` (engine); a raider that moves is hidden again
  (27.31). Morale training (17.3x) is not in the engine, so "at maximum
  morale" reads as "not in training".
- [x] The browser: a *Form raider* control in phase C.

### Task 4: raids on supply dumps (R-149)

- [x] `Order::RaidDump` on a §32 supply unit (27.55, R-149); the move
  panel offers a *Raid the dump* button per enemy supply unit the
  engine's preview accepts.

### Task 5: the enemy's spotting try (27.42–27.44)

- [x] When an enemy combat unit's move enters a hidden raider's hex, the
  raider's owner announces it (27.42): an event tells the mover that a
  raider is there, without naming it. The move stops there so the
  mover may answer, as a reaction stops it (8.5x); it may walk on with
  a new order.
- [x] `Order::TrySpot { piece }`: that phasing combat unit, in the
  hex, spends 1 CP and the engine rolls one die for the raider's owner;
  a 6 eliminates the raider (`lose_raider`). Once per stage per phasing
  unit (27.44); the mover may decline by simply not ordering it. The
  hex is the one it was told of (`spot_chances`), so a refusal never
  tells a unit where a raider is. Server previews run on
  `view::as_known_to`, the board without the enemy's hidden pieces.
- [ ] Browser: a *Try to spot* button in the move panel while the
  announcement stands. Playwright by clicks.
