// Hand-written mirrors of the engine's serde JSON (cna-game: state.rs,
// order.rs, event.rs, piece.rs, clock.rs, victory.rs). Kebab-case tags
// as serde writes them. If these drift from the Rust, generating them
// (tsify / ts-rs) is the later option (slice plan, task 3).

export type Side = "axis" | "cw";
export type HexId = string;
export type PieceId = string;
export type Nation = "it" | "cw" | "de";

export type Phase =
  | "initiative-declaration"
  | "weather"
  | "organisation"
  | "arrival"
  | "cw-fleet"
  | "reserve-designation"
  | "movement-combat"
  | "truck-convoy"
  | "rail"
  | "repair"
  | "patrol";

export type Half = "A" | "B";

export type Segment =
  | "movement"
  | "breakdown"
  | "position-determination"
  | "barrage"
  | "retreat-before-assault"
  | "force-assignment"
  | "anti-armour"
  | "close-assault"
  | "reserve-release";

export interface Clock {
  game_turn: number;
  stage: number;
  phase: Phase;
  half: Half | null;
  segment: Segment | null;
}

export type Weather =
  | "normal"
  | "hot"
  | { sandstorm: { sheets: string[] } }
  | { rainstorm: { sheets: string[] } };

export type OffMapBox = "tripoli" | "tunisia" | "tripolitania" | "malta" | "italy-sicily" | "crete";
export type OffMapPlace = "tunis" | "gabes" | "tripoli" | "tripolitania" | "nofilia";

export type Country = "libya" | "egypt";

/** A free placement (cna-data `Placement`): one or more locator keys (a hex meets every one), then the narrowing fields. */
export interface Placement {
  hexes?: HexId[];
  place?: string | string[];
  box?: OffMapBox;
  within?: { of: HexId; hexes: number };
  region?: Country;
  sheets?: string[];
  west_of?: HexId;
  air_facility?: boolean;
  /** A hex holding a unit of this kind once the rest is placed (R-135, Rommel). */
  stacked_with?: "german-unit";
  not_within?: { of: string; hexes: number } | null;
  /** The whole block goes into one hex. */
  together?: boolean;
  /** Prose the engine cannot check; the owner honours it (59.2). */
  constraint?: string | null;
}

export type Location =
  | { hex: HexId }
  | { box: OffMapBox }
  | { transit: { from: OffMapPlace; to: OffMapPlace; done: number } }
  | { pending: Placement }
  /** A warship in port (30.1x). */
  | { port: string }
  | "eliminated";

export type PieceKind = "combat" | "hq" | "commander" | "supply-unit" | "dummy-supply-unit" | "raider" | "ship" | "blockship";
export type Equivalent = "brigade" | "battalion" | "company" | "hq" | "by-unit";
export type Reserve = "I" | "II";

/** A piece as a view shows it. An enemy's under limited intelligence (3.61) lacks the optional fields its owner sees (`ENEMY_PIECE_KEEPS` in cna-game's `view/state.rs`). */
export interface Piece {
  id: PieceId;
  side: Side;
  kind: PieceKind;
  nation: Nation | null;
  id_code: string | null;
  parent?: string | null;
  /** The unit this one is merged into (19.1x); absent when it stands alone. */
  attached_to?: string;
  /** A formation too depleted to count whole: it stacks one level smaller (9.26). */
  shell?: true;
  toe?: number;
  weapons?: { weapon: string; points: number }[];
  /** The characteristics cell as printed, e.g. "10" or "8*". */
  cpa: string | null;
  morale?: number | null;
  status?: "in-training" | "broken-down" | null;
  equivalent: Equivalent | null;
  garrison_of?: HexId;
  /** CPA Rommel lends it this OpStage (31.0, R-137); absent when none. */
  rommel_cpa?: number;
  /** A raider off the map in 27.31's sense: its owner alone sees it (the other side's view drops it). */
  hidden?: true;
  location: Location;
  cp_spent?: number;
  cp_enemy_half?: number;
  cohesion?: { level: number; dp_tally: number };
  contact: boolean;
  engaged: boolean;
  reserve?: Reserve;
  released?: Reserve;
  landed?: boolean;
  /** Breakdown points collected this OpStage, in halves (21.2x); absent for none. */
  breakdown_halves?: number;
  /** Motorisation points attached to a supply unit (32.31); absent for none. */
  motorised?: number;
  /** A supply unit's ammunition and fuel (32.1x); only on the side to act's own. */
  load?: { ammo: number; fuel: number };
  /** Added by `view()`: the characteristics chart's unit type, null off the chart. */
  unit_type: string | null;
  /** Added by `view()`: the OA unit's name and abbreviation, null for supply units. */
  name: string | null;
  abbreviation: string | null;
}

export type SealedStep = "gun-positions" | "barrage" | "assign-forces";
export type RetreatReason = "assault-result" | "withheld" | "must-attack" | "rommel-alone";

export type Pending =
  | { "set-up": { side: Side } }
  | { "declare-zoc": { side: Side; hexes: HexId[]; at_start: boolean } }
  | { reaction: { side: Side; eligible: PieceId[] } }
  | { sealed: { step: SealedStep; waiting: Side[] } }
  | { "retreat-before-assault": { side: Side } }
  | {
      "forced-retreat": {
        side: Side;
        pieces: PieceId[];
        hexes: number;
        from: HexId[];
        reason: RetreatReason;
        raw: number;
        overrun: boolean;
      };
    };

export interface Fire {
  piece: PieceId;
  points: number;
  target: PieceId;
}

export interface Allocation {
  target?: HexId;
  points: number;
}

export interface Assignment {
  piece: PieceId;
  anti_armour?: Allocation[];
  close_assault?: Allocation[];
}

export type Order =
  | { place: { piece: PieceId; hex: HexId } }
  | { unplace: { piece: PieceId } }
  | { "place-work": { item: WorkItem; hex: HexId } }
  | { "place-motorisation": { hex: HexId; points: number } }
  | { "declare-initiative": { player_a: Side } }
  | { move: { pieces: PieceId[]; path: HexId[]; assault?: boolean; fuel_from?: PieceId } }
  | { react: { pieces: PieceId[]; path: HexId[] } }
  | { "declare-zoc": { exerting: HexId[] } }
  | { "move-off-map": { pieces: PieceId[]; to: OffMapPlace } }
  | { "gun-positions": { forward: PieceId[] } }
  | { barrage: { fires: Fire[] } }
  | { "retreat-before-assault": { pieces: PieceId[]; path: HexId[] } }
  | { "assign-forces": { assignments: Assignment[] } }
  | { assault: { target: HexId } }
  | { retreat: { path: HexId[] } }
  | { "designate-reserve": { pieces: PieceId[] } }
  | { "release-reserve": { pieces: PieceId[] } }
  | "end-segment"
  | { "destroy-supply": { hex: HexId } }
  | { attach: { piece: PieceId; to: PieceId } }
  | { detach: { piece: PieceId } }
  | { reassign: { piece: PieceId } }
  | { "swap-battalions": { piece: PieceId; with: PieceId } }
  | { "shift-parent": { piece: PieceId; to: PieceId | null } }
  | { "attach-motorisation": { piece: PieceId } }
  | { "detach-motorisation": { piece: PieceId } }
  | { "transfer-load": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
  | { rail: { pieces: PieceId[]; path: HexId[]; stops?: { at: HexId; off?: PieceId[]; on?: PieceId[] }[] } }
  | { "air-lift": { piece: PieceId; to: Location; fuel_from?: PieceId } }
  | { "convoy-lanes": { lanes: number[] } }
  | { repair: { hex: HexId; weapon: string; fuel_from?: PieceId; unit?: PieceId } }
  | { "facility-repair": { hex: HexId; weapon: string; fuel_from?: PieceId; unit?: PieceId } }
  | { tow: { hex: HexId; weapon: string; points: number; path: HexId[] } }
  | { "repair-destroyed-tanks": { hex: HexId; weapon: string; points: number; fuel_from?: PieceId } }
  | { "march-prisoners": { from: HexId; path: HexId[]; points: number } }
  | { absorb: { piece: PieceId; weapon: string; points: number } }
  | { build: { item: BuildItem; hex: HexId; builders: PieceId[] } }
  | { "detail-guards": { piece: PieceId; points: number } }
  | { "release-guards": { piece: PieceId; points: number } }
  | { "raid-rommel": { piece: PieceId } }
  | { "pick-withdrawal": { fills: string; pieces: PieceId[] } }
  | { "withdraw-voluntarily": { piece: PieceId } }
  | { "return-withdrawn": { piece: PieceId } }
  | { "form-raider": { raider: PieceId; from: PieceId } }
  | { "raid-dump": { piece: PieceId; target: PieceId } }
  | { "try-spot": { piece: PieceId } }
  | { "release-anti-armour": { piece: PieceId; target: HexId; points: number } }
  /** Phase L: a patrol of each source's points against `target` (16.x). */
  | { patrol: { sources: [PieceId, number][]; target: HexId; exclude?: PieceId[] } }
  | { station: { ship: PieceId; hex: HexId } }
  /** Phase C, the Axis: note the chariot raid on Alexandria for the seventh OpStage after (30.41). */
  | "plan-chariot-raid"
  /** Phase C: note a dummy tank formation, secretly, in `hex` (16.4x). */
  | { "place-dummy-tank": { hex: HexId } }
  | "pass";

/** What a construction project builds (24.1x). */
export type BuildItem = "fortification" | "real-minefield" | "dummy-minefield" | "clear-minefield";
/** A set-up work a side places (59.12): any build item but clearing a field. */
export type WorkItem = Exclude<BuildItem, "clear-minefield">;

/** A project under way in a hex (24.1x): its builders and the stages counted so far. */
export interface Project {
  side: Side;
  /** The owner's secret (3.61): absent from the other side's view. */
  item?: BuildItem;
  builders: PieceId[];
  stages?: number;
}

/** Broken-down TOE points of one weapon system in a hex (21.4x). */
/** A repair facility's kind (22.31). */
export type FacilityKind = "temporary" | "major";

/** Phase K: a facility roll the viewer may make, for the points that began the OpStage in the hex (22.3x, `cna_game::facility::FacilityOffer`). */
export interface FacilityOffer {
  hex: HexId;
  weapon: string;
  points: number;
  kind: FacilityKind;
}

/** Phase K: destroyed tanks the viewer may attempt at its facility, up to `points` (22.4x, `cna_game::destroyed::DestroyedTankOffer`). */
export interface DestroyedTankOffer {
  hex: HexId;
  weapon: string;
  points: number;
  kind: FacilityKind;
}

export interface Wreck {
  side: Side;
  nation?: string | null;
  weapon: string;
  points: number;
  /** How many of `points` broke down this OpStage: no facility repairs them until the next (22.31). */
  this_stage?: number;
  /** How many of `points` were towed in this phase K: no roll and no second tow until the next OpStage (21.6x, 22.13). */
  towed?: number;
  /** The unit these points broke down with, which repair returns them to (61.38); none for loose points. */
  unit?: PieceId | null;
}

export type VictoryLevel = string;

export interface Condition {
  side: Side;
  level: VictoryLevel;
  objective: string;
  hexes: HexId[];
  any_hex: boolean;
  held: HexId[];
  unsupplied: PieceId[];
  met: boolean;
}

export interface VictoryReport {
  axis: VictoryLevel | null;
  cw: VictoryLevel | null;
  checked: Condition[];
  unchecked: string[];
  /** A points victory's places (absent for a levels victory). */
  scored?: Scored[];
  points?: { axis: number; cw: number };
  /** An automatic win that ended the game early (64.71, 64.72). */
  automatic?: string;
}

export interface Scored {
  objective: string;
  hexes: HexId[];
  holder: Side | null;
  points: number;
  /** The holder's units there that failed the end-of-game supply test (64.73). */
  unsupplied?: PieceId[];
}

export type BarrageResult = "no-effect" | "pinned" | { lose: number } | "unresolved";

export interface AssaultRoll {
  reading: number;
  pct: number;
  /** Withheld from the other side online (3.61). */
  raw_lost?: number;
  toe_lost?: number;
  engaged: boolean;
  retreat: number;
  captured: boolean;
}

/** The 96 event variants of `cna_game::event::Event`, externally tagged. */
export type Event =
  | { "phase-started": { clock: Clock } }
  | { "phase-skipped": { clock: Clock } }
  | { "die-rolled": { purpose: string; value: number } }
  | { "initiative-determined": { holder: Side; rolled: boolean } }
  | { "initiative-declared": { player_a: Side } }
  | { "weather-set": { weather: Weather } }
  | { placed: { piece: PieceId; hex: HexId } }
  | { unplaced: { piece: PieceId } }
  | "set-up-complete"
  | "rommel-recalled"
  | { "raider-spotted": { piece: PieceId; hex: HexId; roll: number; eliminated: boolean } }
  /** Enemy combat units entered `hex`, where a raider of the other side stands (27.42); `pieces` may each try to spot it. */
  | { "raider-announced": { hex: HexId; pieces: PieceId[] } }
  /** A spotting try (27.43): one die per raider in `hex`, in id order; a 6 eliminates. */
  | { "spot-tried": { piece: PieceId; hex: HexId; rolls: number[]; eliminated: PieceId[] } }
  | { "rommel-raided": { piece: PieceId; hex: HexId; roll: number; result: RaidResult; lrdg_eliminated: boolean } }
  | { "raider-formed": { piece: PieceId; from: PieceId; hex: HexId } }
  /** A raid on a supply unit as a dump (27.55, R-149); the losses and the guards' strength reach the dump's owner alone. */
  | { "dump-raided": { piece: PieceId; target: PieceId; hex: HexId; guard_roll: number | null; guard_strength?: number | null; roll: number | null; dummy: boolean; ammo_lost?: number; fuel_lost?: number; removed: boolean; raider_eliminated: boolean } }
  /** A patrol's three dice (16.x): public, its descriptions (each unit's stated TOE) included. */
  | { patrolled: PatrolReport }
  | { stationed: { ship: PieceId; hex: HexId } }
  | { "in-port": { ships: PieceId[] } }
  /** The Axis's roll for its strikes on the fleet (32.72): its secret, sent to the Axis alone. */
  | { "fleet-strikes-rolled": { strikes: number } }
  /** An Axis strike on the ships in Alexandria (32.73, 32.74): gun points laid per ship, and those sunk. */
  | { "fleet-struck": { column: number; reading: number; damage: number; hits?: Record<PieceId, number>; sunk?: PieceId[] } }
  /** The Axis noted its chariot raid (30.41): its secret, sent to the Axis alone. */
  | { "chariot-raid-planned": { game_turn: number; stage: number } }
  /** A dummy tank formation noted (16.42), or gone with its formation: the owner's secret, sent to it alone. */
  | { "dummy-tank-placed": { side: Side; hex: HexId } }
  | { "dummy-tank-lost": { side: Side; hex: HexId } }
  /** A dummy tank formation close-assaulted, or left with no real armour under anti-armour fire, dies in the open (16.44, 16.45). */
  | { "dummy-tank-exposed": { side: Side; hex: HexId } }
  /** The chariot raid on Alexandria (30.4x): the table's die, the dice of damage it gave, and the gun points laid per ship. */
  | { "chariot-raid": { roll: number; dice: number; damage: number; hits?: Record<PieceId, number>; sunk?: PieceId[] } }
  /** A damaged ship got a gun point back after six OpStages in Alexandria (30.37). */
  | { "ship-repaired": { ship: PieceId; guns: number } }
  | "bizerta-opened"
  | { "convoy-lanes-set": { lanes: number[] } }
  | { "convoy-cancelled": { lane: number } }
  | { arrived: { pieces: PieceId[]; at: Location } }
  | { "moved-off-map": { piece: PieceId; location: Location } }
  | { moved: { pieces: PieceId[]; from: HexId; to: HexId; cost?: number } }
  | { disorganised: { piece: PieceId; dp: number; level: number } }
  | { "move-interrupted": { pieces: PieceId[]; at: HexId; before: HexId; declare: HexId[] } }
  | { "zoc-declared": { hex: HexId; exerts: boolean } }
  | { "reaction-offered": { pieces: PieceId[]; at: HexId } }
  | { "contact-broken": { piece: PieceId; cost?: number } }
  | { "reserve-designated": { pieces: PieceId[] } }
  | { "reserve-released": { pieces: PieceId[]; from: Reserve } }
  | { "reserve-flipped": { pieces: PieceId[] } }
  | { sealed: { side: Side; step: SealedStep } }
  | { "gun-positions-set": { side: Side; forward: PieceId[] } }
  | { "barrage-plotted": { side: Side; fires?: Fire[] } }
  | {
      barrage: {
        side: Side;
        /** Absent for the enemy's barrage, online (3.61). */
        firers?: PieceId[];
        target: PieceId;
        points: number;
        shift: number;
        reading: number;
        result: BarrageResult;
      };
    }
  | { "retreated-before-assault": { pieces: PieceId[]; to: HexId } }
  | { "anti-armour": { side: Side; target: HexId; points?: number; shift?: number; reading: number; damage: number } }
  | { "anti-armour-released": { side: Side; piece: PieceId; target: HexId; points: number } }
  | {
      assault: {
        target: HexId;
        attackers: PieceId[];
        probe: boolean;
        attacker_raw?: number;
        defender_raw?: number;
        basic: number;
        shifts: number;
        column: string;
        attacker: AssaultRoll;
        defender: AssaultRoll;
      };
    }
  | { prisoners: { side: Side; points?: number } }
  | { surrendered: { pieces: PieceId[] } }
  | { engaged: { pieces: PieceId[] } }
  | { "retreat-ordered": { pieces: PieceId[]; hexes: number; reason: RetreatReason } }
  | { "retreat-bought-out": { pieces: PieceId[]; short: number } }
  | { losses: { piece: PieceId; points: number; toe: number; motorisation?: number } }
  | { "fuel-spent": { piece: PieceId; from: PieceId; fuel: number } }
  | { "ammunition-spent": { piece: PieceId; from: PieceId; ammo: number } }
  | { "ammunition-short": { piece: PieceId } }
  | { "supply-unit-spent": { piece: PieceId } }
  | { "supply-destroyed": { piece: PieceId } }
  | { attached: { piece: PieceId; to: PieceId } }
  | { detached: { piece: PieceId; from: PieceId } }
  /** Its parent was eliminated: the unit is independent (19.2x). */
  | { unassigned: { piece: PieceId; from: PieceId } }
  /** Assigned to `to` (none: independent), leaving `from` (19.2x). */
  | { reassigned: { piece: PieceId; to: PieceId | null; from: PieceId | null } }
  | { "motorisation-attached": { piece: PieceId; points: number } }
  | { "motorisation-detached": { piece: PieceId; points: number } }
  | { "supply-planned": { side: Side; die: number; units: number; arrives: number } }
  | { "motorisation-lost": { side: Side; percent: number; of: number; points: number } }
  | { "load-transferred": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
  | { railed: { pieces: PieceId[]; from: HexId; to: HexId } }
  | { "air-lifted": { piece: PieceId; from: Location; to: Location; fuel_from?: PieceId } }
  | { "field-repaired": { hex: HexId; weapon: string; attempted: number; die: number; repaired: number; fuel_from?: PieceId } }
  | { "facility-repaired": { hex: HexId; weapon: string; kind: FacilityKind; attempted: number; die: number; repaired: number; fuel_from?: PieceId } }
  | { towed: { side: Side; from: HexId; to: HexId; weapon: string; points: number } }
  | { "wrecks-captured": { by: PieceId; hex: HexId; points: number } }
  | { "tanks-destroyed": { piece: PieceId; hex: HexId; weapon: string; points: number } }
  | { "destroyed-tanks-captured": { by: PieceId; hex: HexId; points: number } }
  | { "destroyed-tanks-repaired": { hex: HexId; weapon: string; kind: FacilityKind; attempted: number; dice: number[]; repaired: number; junked: number; fuel_from?: PieceId } }
  | { "prisoners-taken": { side: Side; hex: HexId; points?: number } }
  | { "equipment-captured": { side: Side; hex: HexId; weapon: string; points?: number } }
  | { "prisoners-marched": { side: Side; path: HexId[]; points?: number; cp?: number } }
  | { "prisoners-gone": { side: Side; hex: HexId; points?: number } }
  | { "prisoners-left-map": { side: Side; points?: number; stages: number } }
  | { "convoy-bombed": { lane: number; bomb_points: number; reading: number; percent: number; units?: number; sunk?: number } }
  | { "construction-begun": { side: Side; item?: BuildItem; hex: HexId; builders: PieceId[] } }
  | { "construction-lost": { side: Side; item?: BuildItem; hex: HexId } }
  | { "fortification-built": { side: Side; hex: HexId; level: number } }
  | { "minefield-laid": { side: Side; hex: HexId } }
  | { "minefield-revealed": { hex: HexId; real: boolean } }
  | { "minefield-removed": { hex: HexId } }
  | { "set-up-work-voided": { side: Side; hex: HexId; minefield: boolean } }
  | { "guards-detailed": { piece: PieceId; hex: HexId; points?: number } }
  | { "guards-released": { piece: PieceId; hex: HexId; points?: number } }
  | { "prisoners-escaped": { side: Side; hex: HexId; rejoined: number; escapees: number } }
  | { "prisoners-freed": { side: Side; hex: HexId; points: number } }
  | { absorbed: { piece: PieceId; weapon: string; points: number; cp: number } }
  | { "broke-down": { piece: PieceId; weapon: string; reading: number; percent: number; points: number } }
  | { eliminated: { piece: PieceId } }
  | { withdrawn: { pieces: PieceId[]; motorisation: number } }
  | { "withdrawal-failed": { pieces: PieceId[]; reason: string } }
  | { "withdrawal-picked": { fills: string; pieces: PieceId[] } }
  | { "withdrawn-voluntarily": { piece: PieceId } }
  | { "returned-from-withdrawal": { piece: PieceId } }
  | { "forces-assigned": { side: Side; assignments?: Assignment[] } }
  | { "combat-cp": { piece: PieceId; cost: number } }
  | { "combat-cp-refunded": { piece: PieceId; cp: number; dp: number } }
  | { captured: { piece: PieceId; hex: HexId } }
  | { "game-over": { report: VictoryReport } };

/** A row of the Raid on Rommel Table (27.92, `cna_data::raid::RaidResult`). */
export type RaidResult = "axis-initiative-3-temporary" | "lrdg-eliminated" | "none" | "rommel-removed";

export interface Illegal {
  case: string;
  reason: string;
}

export type ApplyResult = { ok: Event[] } | { illegal: Illegal };

/** A preview's answer: `apply`'s, or `hidden` when the order rolls dice (the game's next rolls) or answers a sealed step (it would reveal the other side's answer). */
export type PreviewResult = ApplyResult | { hidden: "dice" | "sealed" };

/** What a legal-move query is for: the order the path will be sent as (`cna_game::reach::Purpose`). */
export type Purpose = "move" | "react" | "retreat-before-assault" | "forced-retreat" | "rail";

/** A hex a stack can end in (`cna_game::reach::Reach`). */
export interface Reach {
  /** The fuel the move owes under abstract supply, the whole stack's (32.22); 0 with the module off. */
  fuel: number;
  hex: HexId;
  /** What the stack pays in all, breaking off included. */
  cp: number;
  /** The hexes entered, cheapest first. */
  path: HexId[];
  /** The most DP any piece in the stack earns on the way. */
  dp: number;
  /** A known enemy ZOC: the move ends there. */
  ends_in_zoc: boolean;
  /** The engine may stop the stack short of it (an undeclared ZOC or a reaction on the way). */
  may_be_interrupted: boolean;
  /** A forced retreat only: hexes short of the order, paid for at 10 % of the raw points each (15.82). */
  short: number;
}

export type ReachResult = { ok: Reach[] } | { illegal: Illegal };

/** A gun-class unit that may take a position (`cna_game::combat::options::Gun`). */
export interface Gun {
  piece: PieceId;
  next_to_enemy: boolean;
}

/** A unit that may barrage, with its TOE and the enemy units it may target (`cna_game::combat::options::Firer`). */
export interface Firer {
  piece: PieceId;
  points: number;
  forward: boolean;
  targets: PieceId[];
  /** Abstract supply: the ammunition it owes (32.21), and whether no supply unit in range can pay it; 0 and false with the module off. */
  ammo: number;
  ammo_short: boolean;
  /** A ship (30.2x): the one point count it fires at each target; absent for a land unit. */
  ship_points?: Record<PieceId, number>;
}

/** A unit that may commit TOE points in step 4 (`cna_game::combat::options::Committer`). */
export interface Committer {
  piece: PieceId;
  points: number;
  anti_armour: boolean;
  anti_armour_targets: HexId[];
  close_assault_targets: HexId[];
  one_assault: boolean;
  /** Abstract supply: the ammunition it owes (32.21), and whether no supply unit in range can pay it; 0 and false with the module off. */
  ammo: number;
  ammo_short: boolean;
}

/** What the side to act may answer the pending sealed step with (`cna_game::combat::options::SealedOptions`). */
export type SealedOptions =
  | { "gun-positions": { guns: Gun[] } }
  | { barrage: { firers: Firer[] } }
  | { "assign-forces": { phasing: boolean; units: Committer[] } };

export type SealedOptionsResult = { ok: SealedOptions } | { illegal: Illegal };

/** `Game.view()`: the state minus `log` and `rng`, plus `to_act` and each piece's labels. */
export interface View {
  scenario: string;
  rules_commit: string;
  ruleset: Record<string, number>;
  clock: Clock;
  initiative: Side | null;
  player_a: Side | null;
  weather: Weather;
  pieces: Record<PieceId, Piece>;
  pending: Pending | null;
  cycle: number;
  moved: PieceId[];
  zoc: Record<HexId, boolean>;
  continuing: PieceId[];
  shifted: PieceId[];
  /** `CombatSegment`; its sets serialise as arrays. The other side's sealed answers are redacted while the step is pending. */
  combat: {
    forward?: PieceId[];
    plots?: Partial<Record<Side, Fire[]>>;
    assignments?: Partial<Record<Side, Assignment[]>>;
    pinned?: PieceId[];
    retreated?: PieceId[];
    [k: string]: unknown;
  };
  over: VictoryReport | null;
  /** Abstract supply: motorisation points by hex id (or box), the side to act's own only. */
  motorisation?: Partial<Record<Side, Record<string, number>>>;
  /** Set-up: unattached motorisation points the viewer still places, each quota with the hexes it may go to (61.72, R-160). */
  motorisation_to_place?: Partial<Record<Side, { hexes: HexId[]; points: number }[]>>;
  /** Breakdown: broken-down vehicles by hex (21.4x), both sides', in the open. */
  broken_down?: Record<HexId, Wreck[]>;
  /** Destroyed-tank markers by hex (14.5x): the viewer's own only. */
  destroyed_tanks?: Record<HexId, Wreck[]>;
  /** Prisoner points by captor and hex (28.1x): open to both sides. */
  prisoners?: Partial<Record<Side, Record<HexId, number>>>;
  /** Prisoner points each captor has sent out of play (28.15). */
  prisoners_out?: Partial<Record<Side, number>>;
  /** Fortification levels changed from the map's (25.1x); a major city absent here is at its own. */
  fortifications?: Record<HexId, number>;
  /** Minefields by hex (26.1x); `real` is absent from the other side's until revealed. */
  minefields?: Record<HexId, { side: Side; real?: boolean; revealed?: boolean }>;
  /** Projects under way by hex (24.1x): open. */
  projects?: Record<HexId, Project>;
  /** Guard points by owner and hex (28.21): open. */
  guards?: Partial<Record<Side, Record<HexId, number>>>;
  /** Escapees by their own side and hex (28.25): open. */
  escapees?: Partial<Record<Side, Record<HexId, number>>>;
  /** Prisoner points each side has had back by escape or liberation (28.24). */
  escaped?: Partial<Record<Side, number>>;
  /** The Axis's convoy lanes in the order it fills them (32.63); the Axis's alone. Absent: the Tripoli lane. */
  convoy_lanes?: number[];
  /** Whether the Axis has rolled its 12 for Bizerta (55.17). */
  bizerta_open?: boolean;
  /** Phasing combat units told of an enemy raider in the hex they entered, and that hex (27.42): each may try to spot it there. */
  spot_chances?: Record<PieceId, HexId>;
  /** Units that have tried to spot a raider this OpStage, once each (27.44). */
  spot_tried?: PieceId[];
  /** Hexes prisoners marched into this phase H: they go no further (28.13). */
  marched?: HexId[];
  /** Repaired points by hex and weapon system waiting for a unit there to absorb them (22.5, 20.41), the side to act's own only. */
  repaired?: Partial<Record<Side, Record<HexId, Record<string, number>>>>;
  to_act: Side | null;
  /** The hexes the phasing player still has to assault in the close assault step; empty at any other moment. */
  assaults_to_resolve: HexId[];
  /** Close assault step: anti-armour points the phasing viewer aimed at a hex that held no armour and may move to the assault on it, at most `points` each (14.23). */
  anti_armour_releases?: { piece: PieceId; target: HexId; points: number }[];
  /** Phase C: the viewer's raider counters it may form now, each with a unit that may give the TOE point (27.13). */
  raider_formings?: { raider: PieceId; name: string | null; from: PieceId }[];
  /** Phase C: the viewer's units whose O/A notes date a shift of parent to this month, and the parent each joins (null: independent); the engine judges attachment and slots (19.24). */
  parent_shifts?: { piece: PieceId; to: PieceId | null }[];
  /** Phase C: the Axis viewer may plan its chariot raid now (30.41). */
  chariot_raid_plannable?: boolean;
  /** Phase C: the hexes the viewer may note a dummy tank formation in now (16.4x). */
  dummy_tank_options?: HexId[];
  /** The viewer's own dummy tank formations by side, its secret (16.4): the enemy's never reach it. */
  dummy_tanks?: Partial<Record<Side, HexId[]>>;
  /** The Commonwealth viewer, to act: the withdrawal picks it may make this OpStage (20.82, `cna_game::withdraw::PickOption`). */
  withdrawal_pick_options?: WithdrawalPick[];
  /** The Commonwealth, to act in a campaign game: the units it may withdraw voluntarily, those it may bring back, and its net withdrawal half-points (20.9, 64.75). */
  /** Pieces out of the game by withdrawal, as they left: the Commonwealth's own view (20.8x, 64.75). */
  withdrawn?: Record<PieceId, Piece>;
  voluntary_options?: { withdrawable: PieceId[]; returnable: PieceId[]; halves: number };
  /** Phase L: the viewer's units that may patrol now, the points each may send and the hexes it may send them against (16.x). */
  patrol_options?: PatrolSource[];
  /** Phase E: the Commonwealth's ships that may be placed at sea and the hexes open to them (30.1x, `cna_game::fleet::StationOptions`). */
  station_options?: { ships: PieceId[]; hexes: HexId[] };
  /** Set-up: the hexes open to each of the viewer's set-up works, how many required ones are left, and whether a pass ends its set-up (59.12). */
  work_options?: WorkOptions;
  /** The scenario's roads and railroads finished at set-up, which the base map draws unfinished (`roads_built`). */
  built_hexsides?: { roads: [HexId, HexId][]; railroads: [HexId, HexId][] };
  /** Phase K: the facility rolls the phasing viewer may make (22.3x). */
  facility_repairs?: FacilityOffer[];
  /** Phase K: the destroyed tanks the phasing viewer may attempt (22.4x). */
  destroyed_tank_repairs?: DestroyedTankOffer[];
  /** Each warship's port record (30.1x), with the fleet module on. */
  fleet?: Record<PieceId, { at_sea?: number; owed?: number; stationed?: true; fired?: true }>;
}

/** A withdrawal's pick of `count` units of a slot kind (`cna_game::withdraw::PickOption`): `candidates` in the order the engine would take them, `picked` the pick made. */
export type WithdrawalPick = { fills: string; count: number; candidates: PieceId[]; picked?: PieceId[] };

/** One unit the phasing player may patrol with (`cna_game::patrol::PatrolSource`). */
export type PatrolSource = { piece: PieceId; hex: HexId; points: number; targets: HexId[] };

/** One patrol's dice and what came of them (`cna_game::patrol::PatrolReport`). */
export type PatrolReport = {
  side: Side;
  target: HexId;
  sources: [PieceId, number][];
  /** None when the target held no combat unit (16.31). */
  survival_die?: number;
  killed: number;
  captured: number;
  /** None for a patrol wiped out. */
  recon_die?: number;
  described?: { piece: PieceId; toe: number }[];
  objective_die: number;
  objective_killed: number;
  objective_captured: number;
};

/** Where the side setting up may place its set-up works (`cna_game::placed_works`). */
export type WorkOptions = { [K in WorkItem]?: HexId[] } & {
  required_left: number;
  /** Works the side may still place by choice (`up_to` rows and sets, R-161). */
  optional_left: number;
  may_pass: boolean;
};
