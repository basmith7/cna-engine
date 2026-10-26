# Mission 5, slice 6: attachment, shells, dumps and prisoners (phase C, §19)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 6). Every
decision here is the autopilot's under Brian's delegation of 2026-09-26
and is reversible.

**Goal.** Formations become real: a battalion is either merged into its
parent's counter or standing on its own, and the player decides which.
Today the engine reads "attached" as "stacked with an HQ it is assigned
to", so a division can never be stripped, a battalion never lent to
another brigade, and no formation is ever a shell. Explicit attachment
lets a group fight the 1940 campaign's improvised columns, makes 9.26's
shells count in stacking and close assault, and gives convoy bombing
(32.66) and later rules their "full division".

**Non-goals.** Reassignment (19.2x: free slots, the Commonwealth
battalion shuffle, historical shifts, the armour limits by turn): units
keep the assignment the OA gives them. Rebuilding depleted units (19.6x),
battle groups (19.7x: Graziani's Italian battle groups have no counters,
R-124) and ad hoc anti-tank units (19.8x). Replacements (§20) beyond
what the repaired-points pool already holds.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| State | `Piece.attached_to: Option<UnitId>`, the parent it is merged into; `parent` stays the assignment (19.1x: a unit may be assigned to one parent and attached to another). Saves without the field read as today: attached where the unit stands with its parent (serde default plus a load-time fill) | 19.11–19.14; old logs replay |
| Set-up | An assigned unit placed in its parent's hex starts attached (4.25) | 4.25 |
| Movement | A move of a parent carries everything attached to it, listed or not; a move listing an attached unit without its parent is refused ("detach it first"). Reactions and retreats likewise | 19.4x: the parent's counter stands for it |
| Orders | `Order::Attach { piece, to }` and `Order::Detach { piece }`: same hex; assigned units attach or detach at any time for 1 CP each (unit and parent); a non-assigned unit attaches only in the reorganisation segment for 2 CP and detaches there or in a movement segment for 1; a unit detached in a movement segment stops unless it goes on stacked with the parent | 19.41–19.47 |
| Limits | Non-assigned attachments within `max-attachments.json` by nation and parent type (two free company-equivalents on top); nesting counts a unit attached to an attached parent against the larger one (substitutes excepted) | 19.5, 19.46 |
| Shells | `GameState::is_shell(unit)` by 9.26's table, shells not counting toward a parent's quota; stacking reads a shell one level smaller; close assault's differentiation likewise | 9.26–9.28 |
| Convoy bombing | Waits on cna's lane table (request on `main`); with shells built, "westernmost full Commonwealth division" is a query | 32.66 |
| Dumps (24.9) | In phase C's construction segment, costed by `cna-rules::construction::dump_cost` under R-018 | 24.9 |
| Prisoners (§28) | Guards, the phase-H march, `prisoners-captured.json` | §28 |

## Crates touched

- `cna-data`: `formation-organisation.json` and `max-attachments.json`
  readers.
- `cna-game`: `piece.rs` (`attached_to`), `setup.rs` (4.25), a new
  `attachment.rs` (orders, limits, shells), `cycle.rs` (stacking by
  attachment instead of `parent_on`), `movement.rs` (carrying attached
  units; `cpa()`'s subtree minimum over attached units only), `reaction.rs`
  and `combat/retreat.rs` (the same carry), `convoy.rs` (later: bombing).
- `cna-web`: attached units nested under their parent in the stack panel;
  Attach and Detach in phase C and the movement segment.

## Testing

- Rule tests per case: 4.25 set-up, each row of 19.4x's cost table, the
  segment limits, 19.5's caps by nation and the free companies, 9.26's
  three shell rows and the shell-in-quota example, moves carrying and
  refusing.
- Golden games: both scripts list whole stacks, so they should replay;
  any hash change re-pins in one commit with the reason.
- Playwright: detach a battalion from its brigade in phase C, move it
  alone in G, attach it back.

## Tasks

Resliced 2026-10-26: Tasks 1–2 and the Attach/Detach controls they need
(a lone attached battalion can no longer move, so the UI must split it
off) merge as PR #44; Tasks 3–5 follow in their own PRs. Task 3 waits
on cna's HQ organisation types.

### Task 1: attachment as state

- [x] `Piece.attached_to`, set at set-up (4.25) and filled on load for
  older saves; stacking (9.1x) and the HQ's CPA read it instead of
  "stacked with its parent". Golden games unchanged (logs replay; the
  state hashes re-pin for the new fields).

### Task 2: orders and costs

- [x] `Order::Attach` and `Order::Detach` with 19.4x's costs and
  segments; moves carry attached units and refuse a lone attached one.

### Task 3: limits and shells

- [x] `max-attachments.json` caps for non-assigned units, nesting;
  `is_shell` (9.26), stacking one level smaller (`cna-game::limits`).
  Shells judge the HQ's assigned units in the game, not the 19.3 chart.
- [x] A shell battalion or brigade-equivalent counts one size smaller in
  close assault's size table (9.28, 15.5x); a battalion's printed
  strength reads its ID code's row where the sheet lists no TOE.
- [ ] Open: division-sized formations in the size table (assault sizes
  read single pieces, not an HQ with its attached brigades), the 19.3
  chart as the shell denominator, substitution.

### Task 4: dumps and prisoners

- [ ] 24.9 dumps (waits on cna: under abstract supply the supply unit is the dump). §28 prisoner points and the march: slice 6b, `2026-10-26-m5-slice-6b-prisoners.md`; guards and escape after it.

### Task 5: UI, docs

- [ ] Stack panel nesting, Attach/Detach controls, e2e; README,
  `NOT_SIMULATED.md`, the spec's Status and the queue row.
