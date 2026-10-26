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
  | "eliminated";

export type PieceKind = "combat" | "hq" | "supply-unit" | "dummy-supply-unit";
export type Equivalent = "brigade" | "battalion" | "company" | "hq" | "by-unit";
export type Reserve = "I" | "II";

export interface Piece {
  id: PieceId;
  side: Side;
  kind: PieceKind;
  nation: Nation | null;
  id_code: string | null;
  parent: string | null;
  toe: number;
  weapons?: { weapon: string; points: number }[];
  /** The characteristics cell as printed, e.g. "10" or "8*". */
  cpa: string | null;
  morale: number | null;
  status: "in-training" | "broken-down" | null;
  equivalent: Equivalent | null;
  garrison_of?: HexId;
  location: Location;
  cp_spent: number;
  cp_enemy_half: number;
  cohesion: { level: number; dp_tally: number };
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
export type RetreatReason = "assault-result" | "withheld" | "must-attack";

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
  | { "attach-motorisation": { piece: PieceId } }
  | { "detach-motorisation": { piece: PieceId } }
  | { "transfer-load": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
  | { rail: { pieces: PieceId[]; path: HexId[] } }
  | { repair: { hex: HexId; weapon: string; fuel_from?: PieceId } }
  | "pass";

/** Broken-down TOE points of one weapon system in a hex (21.4x). */
export interface Wreck {
  side: Side;
  nation?: string | null;
  weapon: string;
  points: number;
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
}

export type BarrageResult = "no-effect" | "pinned" | { lose: number } | "unresolved";

export interface AssaultRoll {
  reading: number;
  pct: number;
  raw_lost: number;
  toe_lost: number;
  engaged: boolean;
  retreat: number;
  captured: boolean;
}

/** The 46 event variants of `cna_game::event::Event`, externally tagged. */
export type Event =
  | { "phase-started": { clock: Clock } }
  | { "phase-skipped": { clock: Clock } }
  | { "die-rolled": { purpose: string; value: number } }
  | { "initiative-determined": { holder: Side; rolled: boolean } }
  | { "initiative-declared": { player_a: Side } }
  | { "weather-set": { weather: Weather } }
  | { placed: { piece: PieceId; hex: HexId } }
  | "set-up-complete"
  | { arrived: { pieces: PieceId[]; at: Location } }
  | { "moved-off-map": { piece: PieceId; location: Location } }
  | { moved: { pieces: PieceId[]; from: HexId; to: HexId; cost: number } }
  | { disorganised: { piece: PieceId; dp: number; level: number } }
  | { "move-interrupted": { pieces: PieceId[]; at: HexId; before: HexId; declare: HexId[] } }
  | { "zoc-declared": { hex: HexId; exerts: boolean } }
  | { "reaction-offered": { pieces: PieceId[]; at: HexId } }
  | { "contact-broken": { piece: PieceId; cost: number } }
  | { "reserve-designated": { pieces: PieceId[] } }
  | { "reserve-released": { pieces: PieceId[]; from: Reserve } }
  | { "reserve-flipped": { pieces: PieceId[] } }
  | { sealed: { side: Side; step: SealedStep } }
  | { "gun-positions-set": { side: Side; forward: PieceId[] } }
  | { "barrage-plotted": { side: Side; fires: Fire[] } }
  | {
      barrage: {
        side: Side;
        firers: PieceId[];
        target: PieceId;
        points: number;
        shift: number;
        reading: number;
        result: BarrageResult;
      };
    }
  | { "retreated-before-assault": { pieces: PieceId[]; to: HexId } }
  | { "anti-armour": { side: Side; target: HexId; points: number; shift: number; reading: number; damage: number } }
  | {
      assault: {
        target: HexId;
        attackers: PieceId[];
        probe: boolean;
        attacker_raw: number;
        defender_raw: number;
        basic: number;
        shifts: number;
        column: string;
        attacker: AssaultRoll;
        defender: AssaultRoll;
      };
    }
  | { prisoners: { side: Side; points: number } }
  | { surrendered: { pieces: PieceId[] } }
  | { engaged: { pieces: PieceId[] } }
  | { "retreat-ordered": { pieces: PieceId[]; hexes: number; reason: RetreatReason } }
  | { "retreat-bought-out": { pieces: PieceId[]; short: number } }
  | { losses: { piece: PieceId; points: number; toe: number } }
  | { "fuel-spent": { piece: PieceId; from: PieceId; fuel: number } }
  | { "ammunition-spent": { piece: PieceId; from: PieceId; ammo: number } }
  | { "ammunition-short": { piece: PieceId } }
  | { "supply-unit-spent": { piece: PieceId } }
  | { "supply-destroyed": { piece: PieceId } }
  | { "motorisation-attached": { piece: PieceId; points: number } }
  | { "motorisation-detached": { piece: PieceId; points: number } }
  | { "motorisation-lost": { side: Side; percent: number; of: number; points: number } }
  | { "load-transferred": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
  | { railed: { pieces: PieceId[]; from: HexId; to: HexId } }
  | { "field-repaired": { hex: HexId; weapon: string; attempted: number; die: number; repaired: number; fuel_from?: PieceId } }
  | { "wrecks-captured": { by: PieceId; hex: HexId; points: number } }
  | { "broke-down": { piece: PieceId; weapon: string; reading: number; percent: number; points: number } }
  | { eliminated: { piece: PieceId } }
  | { "forces-assigned": { side: Side; assignments: Assignment[] } }
  | { "combat-cp": { piece: PieceId; cost: number } }
  | { "combat-cp-refunded": { piece: PieceId; cp: number; dp: number } }
  | { captured: { piece: PieceId; hex: HexId } }
  | { "game-over": { report: VictoryReport } };

export interface Illegal {
  case: string;
  reason: string;
}

export type ApplyResult = { ok: Event[] } | { illegal: Illegal };

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
  /** Breakdown: broken-down vehicles by hex (21.4x), both sides', in the open. */
  broken_down?: Record<HexId, Wreck[]>;
  /** Repaired points by weapon system waiting as replacements (22.5), the side to act's own only. */
  repaired?: Partial<Record<Side, Record<string, number>>>;
  to_act: Side | null;
  /** The hexes the phasing player still has to assault in the close assault step; empty at any other moment. */
  assaults_to_resolve: HexId[];
}
