// The engine behind the page, as one interface whatever runs it: here
// `cna-web`'s wasm `Game` (`LocalEngine`), its JSON strings parsed into
// the types of `types.ts`. Every call is async so a server can stand
// behind the same interface. The engine state is the only game state;
// the UI reads `view()` whole after every accepted order.
import init, { Game } from "./pkg/cna_web.js";
import type { ApplyResult, HexId, Order, PieceId, PreviewResult, Purpose, ReachResult, SealedOptionsResult, Side, View } from "./types";

/** A scenario a new game can start (`Game.scenarios()`). */
export type Scenario = { slug: string; name: string };

/** The scenario the start screen offers first. */
export const DEFAULT_SCENARIO = "grazianis-offensive";

let ready: Promise<void> | undefined;

/** Loads the wasm once. */
export function initEngine(): Promise<void> {
  ready ??= init().then(() => undefined);
  return ready;
}

/** One game, as the UI asks it things and sends it orders. */
export interface Engine {
  view(): Promise<View>;
  /** The state JSON (spec, *Save and load*). */
  save(): Promise<string>;
  /** The hexes `place` would accept for a pending piece of the side setting up; empty for any other piece. */
  placements(piece: PieceId): Promise<HexId[]>;
  /** Every hex `pieces` (one stack of `side`'s) can end a move of `purpose` in, or why the stack cannot move at all. */
  reachable(side: Side, pieces: PieceId[], purpose: Purpose): Promise<ReachResult>;
  /** The supply units that could pay the fuel of a move by `pieces`; empty unless `side` is to act and owns them. */
  fuelSources(side: Side, pieces: PieceId[]): Promise<PieceId[]>;
  /** What `side` may answer the pending sealed step with, or why it may not answer now. */
  sealedOptions(side: Side): Promise<SealedOptionsResult>;
  /** What `apply` would answer `order` with, the game untouched; `hidden` if it rolls dice, which are the game's next rolls. */
  preview(side: Side, order: Order): Promise<PreviewResult>;
  apply(side: Side, order: Order): Promise<ApplyResult>;
}

/** The engine in the page: the wasm `Game`. */
export class LocalEngine implements Engine {
  /** The wasm game itself, for test builds' synchronous `window.__cna`. */
  private constructor(readonly game: Game) {}

  /** A new game of `scenario` (a slug from `scenarios()`). */
  static newGame(scenario: string, seed: number): LocalEngine {
    return new LocalEngine(Game.newGame(scenario, BigInt(seed)));
  }

  /** The scenarios a new game can start. */
  static scenarios(): Scenario[] {
    return JSON.parse(Game.scenarios()) as Scenario[];
  }

  /** The scenario's name, or its slug for one not offered. */
  static scenarioName(slug: string): string {
    return LocalEngine.scenarios().find((s) => s.slug === slug)?.name ?? slug;
  }

  /** Throws (the engine's message) for a save of a scenario not embedded or of another data commit. */
  static load(json: string): LocalEngine {
    return new LocalEngine(Game.load(json));
  }

  static rulesCommit(): string {
    return Game.rulesCommit();
  }

  async view(): Promise<View> {
    return JSON.parse(this.game.view()) as View;
  }

  async save(): Promise<string> {
    return this.game.save();
  }

  async placements(piece: PieceId): Promise<HexId[]> {
    return JSON.parse(this.game.placements(piece)) as HexId[];
  }

  async reachable(side: Side, pieces: PieceId[], purpose: Purpose): Promise<ReachResult> {
    return JSON.parse(this.game.reachable(side, JSON.stringify(pieces), purpose)) as ReachResult;
  }

  async fuelSources(side: Side, pieces: PieceId[]): Promise<PieceId[]> {
    return JSON.parse(this.game.fuelSources(side, JSON.stringify(pieces))) as PieceId[];
  }

  async sealedOptions(side: Side): Promise<SealedOptionsResult> {
    return JSON.parse(this.game.sealedOptions(side)) as SealedOptionsResult;
  }

  async preview(side: Side, order: Order): Promise<PreviewResult> {
    return JSON.parse(this.game.preview(side, JSON.stringify(order))) as PreviewResult;
  }

  async apply(side: Side, order: Order): Promise<ApplyResult> {
    return JSON.parse(this.game.apply(side, JSON.stringify(order))) as ApplyResult;
  }
}
