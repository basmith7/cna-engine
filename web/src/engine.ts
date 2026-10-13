// The engine in the page: `cna-web`'s wasm `Game`, with its JSON strings
// parsed into the types of `types.ts`. The engine state is the only game
// state; the UI reads `view()` whole after every accepted order.
import init, { Game } from "./pkg/cna_web.js";
import type { ApplyResult, HexId, Order, PieceId, Side, View } from "./types";

let ready: Promise<void> | undefined;

/** Loads the wasm once. */
export function initEngine(): Promise<void> {
  ready ??= init().then(() => undefined);
  return ready;
}

export class Engine {
  private constructor(private readonly game: Game) {}

  static newGame(seed: number): Engine {
    return new Engine(Game.newGame(BigInt(seed)));
  }

  /** Throws (the engine's message) for a save of another scenario or data commit. */
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

  apply(side: Side, order: Order): ApplyResult {
    return JSON.parse(this.game.apply(side, JSON.stringify(order))) as ApplyResult;
  }
}
