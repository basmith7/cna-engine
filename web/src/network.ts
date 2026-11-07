// The engine behind an online game: the server's HTTP API
// (`crates/cna-server/src/api.rs`) and its live stream (`stream.rs`).
// The server judges every order; this keeps the seat's view, its
// `version` (which the next order carries), and the events the stream
// has shown, and tells subscribers when any of them change. The server
// sends no order numbers: they would count the orders the view hides.
import type { Engine } from "./engine";
import type { GameStatus, MyGame } from "./lobby";
import type { ApplyResult, Event, HexId, Illegal, Order, PieceId, PreviewResult, Purpose, ReachResult, SealedOptionsResult, Side, View } from "./types";

/** What `GET /api/games/{id}` answers. */
type GameBody = { side: Side; version: string; open: boolean; status: GameStatus; view: View };

/** One stream message (`stream.rs`): the events of the orders since the last, as this seat may see them, and the view after with its `version`. */
type StreamMessage = { version: string; events: Event[]; view: View };

/** An entry of `GET /api/games?mine`; only the fields read here. */
type MineEntry = Pick<MyGame, "id" | "opponent">;

/** The illegal-order case `apply` answers when another order landed first. */
export const STALE_CASE = "stale";
export const MOVED_ON = "the game moved on";

class HttpError extends Error {
  constructor(
    readonly status: number,
    readonly body: unknown,
  ) {
    super(errorText(body) ?? `the server answered ${status}`);
  }
}

function errorText(body: unknown): string | undefined {
  const e = (body as { error?: unknown } | null)?.error;
  return typeof e === "string" ? e : undefined;
}

function illegalOf(body: unknown): Illegal | undefined {
  const i = (body as { illegal?: unknown } | null)?.illegal;
  return i && typeof i === "object" ? (i as Illegal) : undefined;
}

export class NetworkEngine implements Engine {
  /** The seat this account holds. */
  side!: Side;
  /** Whether the game is played with open information. */
  open = false;
  /** The game's status when last read: only an `active` game takes orders. */
  status: GameStatus = "active";
  /** The `version` of the view held, which the next order carries: a hash of the view, not a count. */
  version = "";
  /** Versions held before the current one: a stream message still carrying one was overtaken by a reload. */
  private readonly past = new Set<string>();
  /** The opponent's name, once known; `null` while the seat is empty or the lobby list could not be read. */
  opponent: string | null = null;
  /** Every event the stream has delivered, in order. */
  readonly log: Event[] = [];
  private current!: View;
  private source: EventSource | undefined;
  private listeners = new Set<() => void>();
  private readonly base: string;

  private constructor(readonly gameId: string) {
    this.base = `/api/games/${encodeURIComponent(gameId)}`;
  }

  /** Loads the game and opens its stream. Throws (the server's message) if the game cannot be read. */
  static async open(gameId: string): Promise<NetworkEngine> {
    const e = new NetworkEngine(gameId);
    await e.refresh();
    await e.loadOpponent();
    e.source = new EventSource(`${e.base}/stream`, { withCredentials: true });
    e.source.onmessage = (m: MessageEvent<string>) => e.receive(m.data);
    e.source.addEventListener("seats", () => void e.seatsChanged());
    return e;
  }

  /** Calls `f` whenever the view, `version` or log changes; returns the unsubscribe. */
  onChange(f: () => void): () => void {
    this.listeners.add(f);
    return () => this.listeners.delete(f);
  }

  /** Closes the stream; the engine answers no more messages. */
  close(): void {
    if (this.source) this.source.onmessage = null;
    this.source?.close();
    this.source = undefined;
    this.listeners.clear();
  }

  /** Whether it is this seat's move. */
  get myMove(): boolean {
    return this.current.to_act === this.side;
  }

  /** The top bar's line: "Your move", "Waiting for <name>", or `null` when nobody is to act. */
  get turnText(): string | null {
    if (this.status === "open") return "Waiting for an opponent to join";
    if (this.status === "abandoned") return "Abandoned: a player left";
    if (this.status === "stale") return "Stopped: the rules changed under this game";
    const to = this.current.to_act;
    if (to === null) return null;
    if (to === this.side) return "Your move";
    return `Waiting for ${this.opponent ?? "your opponent"}`;
  }

  async view(): Promise<View> {
    return this.current;
  }

  /** The full state; the server gives it only once the game is over (it throws its message before). */
  async save(): Promise<string> {
    const r = await this.request("GET", "/save");
    if (!r.ok) throw new HttpError(r.status, await body(r));
    return r.text();
  }

  async placements(piece: PieceId): Promise<HexId[]> {
    return this.get<HexId[]>(`/placements?piece=${encodeURIComponent(piece)}`);
  }

  async reachable(_side: Side, pieces: PieceId[], purpose: Purpose): Promise<ReachResult> {
    const q = `pieces=${encodeURIComponent(JSON.stringify(pieces))}&purpose=${encodeURIComponent(purpose)}`;
    return this.answer<ReachResult>("GET", `/reachable?${q}`);
  }

  /** The caller's supply units that could pay a move's fuel; empty unless the caller is to act. */
  async fuelSources(_side: Side, pieces: PieceId[]): Promise<PieceId[]> {
    const q = `pieces=${encodeURIComponent(JSON.stringify(pieces))}`;
    const r = await this.answer<{ ok: PieceId[] }>("GET", `/fuel-sources?${q}`);
    return r.ok ?? [];
  }

  async sealedOptions(_side: Side): Promise<SealedOptionsResult> {
    return this.answer<SealedOptionsResult>("GET", "/sealed-options");
  }

  async preview(_side: Side, order: Order): Promise<PreviewResult> {
    return this.answer<PreviewResult>("POST", "/preview", { order });
  }

  /** Sends `order` with the `version` held; on a stale one, reloads the game and answers "the game moved on". */
  async apply(side: Side, order: Order): Promise<ApplyResult> {
    const r = await this.request("POST", "/orders", { side, order, version: this.version });
    const b = await body(r);
    if (r.ok) {
      await this.refresh();
      return b as ApplyResult;
    }
    const illegal = illegalOf(b);
    if (illegal) return { illegal };
    if (r.status === 409 && (b as { stale?: unknown } | null)?.stale === true) {
      await this.refresh();
      return { illegal: { case: STALE_CASE, reason: MOVED_ON } };
    }
    return { illegal: { case: "server", reason: errorText(b) ?? `the server answered ${r.status}` } };
  }

  /** Reloads view and `version` from `GET /api/games/{id}`. */
  private async refresh(): Promise<void> {
    const g = await this.get<GameBody>("");
    this.side = g.side;
    this.open = g.open;
    this.status = g.status;
    this.hold(g.version);
    this.current = g.view;
    this.changed();
  }

  /** The opponent's name from the lobby list; the game plays on without it. */
  private async loadOpponent(): Promise<void> {
    try {
      const mine = await this.fetchJson<MineEntry[]>("/api/games?mine");
      this.opponent = mine.find((g) => String(g.id) === this.gameId)?.opponent ?? null;
    } catch {
      this.opponent = null;
    }
  }

  /** A player joined or left: reload the status and the opponent's name. A failed reload keeps what is shown. */
  private async seatsChanged(): Promise<void> {
    if (!this.source) return;
    try {
      await this.refresh();
      await this.loadOpponent();
    } catch {
      return;
    }
    this.changed();
  }

  /** One stream message: its events logged, and the view replaced whole unless a reload already moved past it. */
  private receive(data: string): void {
    const m = JSON.parse(data) as StreamMessage;
    this.log.push(...m.events);
    if (!this.past.has(m.version)) {
      this.current = m.view;
      this.hold(m.version);
    }
    this.changed();
  }

  private hold(version: string): void {
    if (version === this.version) return;
    if (this.version) this.past.add(this.version);
    this.past.delete(version);
    this.version = version;
  }

  private changed(): void {
    for (const f of this.listeners) f();
  }

  private request(method: "GET" | "POST", path: string, json?: unknown): Promise<Response> {
    return fetch(this.base + path, {
      method,
      credentials: "same-origin",
      headers: json === undefined ? { Accept: "application/json" } : { Accept: "application/json", "Content-Type": "application/json" },
      body: json === undefined ? undefined : JSON.stringify(json),
    });
  }

  private async get<T>(path: string): Promise<T> {
    return this.fetchJson<T>(this.base + path);
  }

  private async fetchJson<T>(url: string): Promise<T> {
    const r = await fetch(url, { credentials: "same-origin", headers: { Accept: "application/json" } });
    const b = await body(r);
    if (!r.ok) throw new HttpError(r.status, b);
    return b as T;
  }

  /** A query answered `{ok}` / `{hidden}` (200) or `{illegal}` (422); anything else throws. */
  private async answer<T>(method: "GET" | "POST", path: string, json?: unknown): Promise<T> {
    const r = await this.request(method, path, json);
    const b = await body(r);
    if (r.ok || illegalOf(b)) return b as T;
    throw new HttpError(r.status, b);
  }
}

async function body(r: Response): Promise<unknown> {
  try {
    return await r.json();
  } catch {
    return null;
  }
}
