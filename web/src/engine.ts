// The engine in the page: `cna-web`'s wasm `Game`, with its JSON strings
// parsed into the types of `types.ts`. The engine state is the only game
// state; the UI reads `view()` whole after every accepted order.
import init, { Game } from "./pkg/cna_web.js";
import type { ApplyResult, HexId, Order, PieceId, Purpose, ReachResult, SealedOptionsResult, Side, View } from "./types";

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

export class Engine {
  private constructor(private readonly game: Game) {}

  /** A new game of `scenario` (a slug from `scenarios()`). */
  static newGame(scenario: string, seed: number): Engine {
    return new Engine(Game.newGame(scenario, BigInt(seed)));
  }

  /** The scenarios a new game can start. */
  static scenarios(): Scenario[] {
    return JSON.parse(Game.scenarios()) as Scenario[];
  }

  /** The scenario's name, or its slug for one not offered. */
  static scenarioName(slug: string): string {
    return Engine.scenarios().find((s) => s.slug === slug)?.name ?? slug;
  }

  /** Throws (the engine's message) for a save of a scenario not embedded or of another data commit. */
  static load(json: string): Engine {
    return new Engine(Game.load(json));
  }

  static rulesCommit(): string {
    return Game.rulesCommit();
  }

  view(): View {
    return JSON.parse(this.game.view()) as View;
  }

  save(): string {
    return this.game.save();
  }

  /** The hexes `place` would accept for a pending piece of the side setting up; empty for any other piece. */
  placements(piece: PieceId): HexId[] {
    return JSON.parse(this.game.placements(piece)) as HexId[];
  }

  /** Every hex `pieces` (one stack of `side`'s) can end a move of `purpose` in, or why the stack cannot move at all. */
  reachable(side: Side, pieces: PieceId[], purpose: Purpose): ReachResult {
    return JSON.parse(this.game.reachable(side, JSON.stringify(pieces), purpose)) as ReachResult;
  }

  /** The supply units that could pay the fuel of a move by `pieces`; empty unless `side` is to act and owns them. */
  fuelSources(side: Side, pieces: PieceId[]): PieceId[] {
    return JSON.parse(this.game.fuelSources(side, JSON.stringify(pieces))) as PieceId[];
  }

  /** What `side` may answer the pending sealed step with, or why it may not answer now. */
  sealedOptions(side: Side): SealedOptionsResult {
    return JSON.parse(this.game.sealedOptions(side)) as SealedOptionsResult;
  }

  /** What `apply` would answer `order` with, the game untouched (dice included: exactly the order's events). */
  preview(side: Side, order: Order): ApplyResult {
    return JSON.parse(this.game.preview(side, JSON.stringify(order))) as ApplyResult;
  }

  apply(side: Side, order: Order): ApplyResult {
    return JSON.parse(this.game.apply(side, JSON.stringify(order))) as ApplyResult;
  }
}
