<script lang="ts">
  import { clockLine, sideName } from "./clock";
  import Counters from "./Counters.svelte";
  import { label, stacks } from "./counters";
  import Curtain from "./Curtain.svelte";
  import { curtainAfter } from "./curtain";
  import EndDialog from "./EndDialog.svelte";
  import { type Engine, LocalEngine, initEngine } from "./engine";
  import { formatEvent } from "./format";
  import GamePanel from "./GamePanel.svelte";
  import { withEvents, withIllegal, type LogEntry } from "./log";
  import LogPanel from "./LogPanel.svelte";
  import Map from "./Map.svelte";
  import Marks from "./Marks.svelte";
  import MovePanel from "./MovePanel.svelte";
  import { allOffMapCapable, allOnMap, defaultTicked, extendPath, toggle, toggleOffMap } from "./moves";
  import { addStop, stopsOn, type RailStop } from "./rail";
  import OffMap from "./OffMap.svelte";
  import Pending from "./Pending.svelte";
  import PhaseControls from "./PhaseControls.svelte";
  import Placements from "./Placements.svelte";
  import Reachable from "./Reachable.svelte";
  import { AUTOSAVE_KEY, describeSave, saveName } from "./saves";
  import SetupPanel from "./SetupPanel.svelte";
  import StartScreen from "./StartScreen.svelte";
  import Lobby from "./Lobby.svelte";
  import { type Account, SIGN_IN_URL, me } from "./lobby";
  import JoinScreen from "./JoinScreen.svelte";
  import { NetworkEngine } from "./network";
  import { RETURN_KEY, parseRoute } from "./route";
  import MarchPanel, { type March } from "./MarchPanel.svelte";
  import TowPanel, { type Tow, towable } from "./TowPanel.svelte";
  import PatrolPanel from "./PatrolPanel.svelte";
  import FleetPanel from "./FleetPanel.svelte";
  import Ships from "./Ships.svelte";
  import { marchable } from "./prisoners";
  import StackPanel from "./StackPanel.svelte";
  import Toast from "./Toast.svelte";
  import TopBar from "./TopBar.svelte";
  import { SIZE, centre } from "./geom";
  import type { BuildItem, PreviewResult, HexId, Illegal, OffMapPlace, Order, Piece, PieceId, Purpose, Reach, SealedOptions, Side, View } from "./types";

  let engine: Engine | undefined = $state();
  let view: View | undefined = $state();
  let hex: string | null = $state(null);
  let error: string | null = $state(null);
  /** The wasm is in; without a game yet the start screen shows. */
  let loaded = $state(false);
  /** The signed-in account for "Play online", or null when signed out or there is no lobby server. */
  let account: Account | null = $state(null);
  /** The lobby shows in place of the start screen. */
  let inLobby = $state(location.pathname.startsWith("/lobby"));
  /** The page asked for: a game online, an invite, or the start screen. */
  const route = parseRoute(location.pathname, location.search);
  /** Whether `me()` has answered, so the invite page knows if a sign-in is needed. */
  let known = $state(false);
  me().then(
    (a) => {
      account = a;
      known = true;
      // Back from a sign-in an invite or a game page asked for: go back to it.
      let back: string | null = null;
      try {
        back = sessionStorage.getItem(RETURN_KEY);
        sessionStorage.removeItem(RETURN_KEY);
      } catch {
        back = null;
      }
      if (a && back && route.page === "start" && /^\/(join|game)\//.test(back)) location.replace(back);
    },
    () => {
      account = null;
      known = true;
    },
  );
  /** Sign-in lands on `/`: this page is where it should come back to. */
  function rememberPage() {
    try {
      sessionStorage.setItem(RETURN_KEY, location.pathname + location.search);
    } catch {
      // Without storage the player comes back to the start screen.
    }
  }
  /** The online game on the page, if it is one: no curtain, no autosave, the log from its stream. */
  const online = $derived(engine instanceof NetworkEngine ? engine : null);
  /** Whether this screen may send orders now: always at hot-seat, on its own move online. */
  const myTurn = $derived(!online || (!!view && view.to_act === online.side));
  /** The online game's "Your move" / "Waiting for …" line. */
  let turnText: string | null = $state(null);
  /** Why the online game could not be opened. */
  let onlineError: string | null = $state(null);
  /** The autosave's clock line on the start screen, or null when there is none. */
  let autosave: string | null = $state(null);
  /** The engine's refusal of the last file given to Load, until the next Load. */
  let loadError: string | null = $state(null);
  /** The tray piece picked to place; UI-only state (spec, *State sync*). */
  let picked: PieceId | null = $state(null);
  /** Pieces placed in this sitting of the set-up, which may be taken back (59.2). */
  let placedNow: PieceId[] = $state([]);
  /** The engine's last refusal, until the toast goes or the next order. */
  let illegal: string | null = $state(null);
  /** Counts refusals so a repeat of the same text is still a new toast. */
  let refusals = $state(0);
  /** The log panel: event lines under clock headers, refusals in red. */
  let log: LogEntry[] = $state([]);
  /** The side the curtain is down for, until they click it away. */
  let curtain: Side | null = $state(null);
  /** The pieces an order moves: the stack panel's ticks (spec, *Moving*). UI-only state. */
  let ticked: PieceId[] = $state([]);
  /** A supply unit armed to fly in phase C: the next hex clicked is where (32.37). */
  let flying: PieceId | null = $state(null);
  /** Prisoners armed to march (28.13): map clicks extend the path until it is sent. */
  let march: March | null = $state(null);
  // An armed march lapses when its hex can no longer march: phase H
  // ended, the other side is to act, or its points have gone.
  $effect(() => {
    if (march && (!view || !marchable(view).some((g) => g.hex === march!.from))) march = null;
  });
  /** Broken-down vehicles armed to tow (21.6x): map clicks extend the path until it is sent. */
  let tow: Tow | null = $state(null);
  // An armed tow lapses once its points can no longer be towed.
  $effect(() => {
    if (tow && (!view || !towable(view).some((g) => g.hex === tow!.hex && g.weapon === tow!.weapon))) tow = null;
  });
  /** The assault toggle for the next move. */
  let assault = $state(false);
  /** The custom path shift-clicks build; Enter sends it. */
  let custom: HexId[] = $state([]);
  /** Phase J: the stops the ticked stack's rail run makes on its way (8.77, R-005), and whether the next click adds one. */
  let railStops: RailStop[] = $state([]);
  let stopping = $state(false);
  /** The hex under the pointer, for the path preview. */
  let hover: HexId | null = $state(null);
  /** The end-of-game dialog, closed to look at the map and reopened from a button. */
  let reportOpen = $state(true);

  // Re-read from the engine whenever the view is replaced: the engine is
  // not reactive, the view standing in for its version.
  // The engine's answers are async: each query below reruns when its
  // inputs change and keeps only the answer to its latest run, cleared
  // meanwhile, so no answer outlives the view or the ticks it was for.
  let placements: HexId[] = $state([]);
  $effect(() => {
    const e = engine, p = picked;
    placements = [];
    if (!view || !p || !e) return;
    return answer(e.placements(p), (r) => (placements = r));
  });
  const pickedPiece = $derived(view && picked ? (view.pieces[picked] ?? null) : null);
  /** The set-up work armed in the set-up panel (59.12): a click places one. */
  let workItem: BuildItem | null = $state(null);
  const workHexes: HexId[] = $derived(!picked && workItem ? (view?.work_options?.[workItem] ?? []) : []);

  /** Hands `q`'s answer to `take` unless the effect that asked has rerun or gone first (its cleanup, returned). */
  function answer<T>(q: Promise<T>, take: (r: T) => void): () => void {
    let live = true;
    q.then((r) => {
      if (live) take(r);
    }).catch((e: unknown) => {
      if (live) error = e instanceof Error ? e.message : String(e);
    });
    return () => {
      live = false;
    };
  }

  /** What a click on a reachable hex would send: a move in a movement segment or the truck convoy phase, a reaction while one is offered, a retreat while one is asked. */
  const purpose: Purpose | null = $derived.by(() => {
    if (!view || !view.to_act || !myTurn) return null;
    if (view.pending) {
      if ("reaction" in view.pending) return "react";
      if ("retreat-before-assault" in view.pending) return "retreat-before-assault";
      if ("forced-retreat" in view.pending) return "forced-retreat";
      return null;
    }
    // Phase H (abstract supply): supply units move alone, by the same order.
    if (view.clock.phase === "truck-convoy") return "move";
    // Phase J (abstract supply): Commonwealth supply units ride the railway.
    if (view.clock.phase === "rail") return "rail";
    return view.clock.phase === "movement-combat" && view.clock.segment === "movement" ? "move" : null;
  });
  /** The engine's options for the pending sealed step, for the side to act (spec, *Sealed options*). */
  let sealed: { ok: SealedOptions } | { ok: null; illegal: Illegal } | null = $state(null);
  $effect(() => {
    const e = engine;
    sealed = null;
    if (!e || !view || !view.to_act || !myTurn || !view.pending || !("sealed" in view.pending)) return;
    return answer(e.sealedOptions(view.to_act), (r) => (sealed = "ok" in r ? r : { ok: null, illegal: r.illegal }));
  });
  /** The engine's preview of Pass while a forced retreat is pending: what standing still costs (spec, *Interrupts*). */
  let passPreview: PreviewResult | null = $state(null);
  $effect(() => {
    const e = engine;
    passPreview = null;
    if (!e || !view || !view.to_act || !myTurn || !view.pending || !("forced-retreat" in view.pending)) return;
    return answer(e.preview(view.to_act, "pass"), (r) => (passPreview = r));
  });
  /** The hexes the phasing player still has to assault: a click on one resolves it (spec, *Combat*). */
  const toAssault: HexId[] = $derived(view?.assaults_to_resolve ?? []);
  /** Phase L: the ticked patrol sources with their points, and the target clicked (16.x). */
  let patrolChosen: Record<PieceId, number> = $state({});
  let patrolPick: HexId | null = $state(null);
  /** The hexes every ticked source may patrol, as the engine offers them; marked, and a click picks one. */
  const patrolTargets: HexId[] = $derived.by(() => {
    const opts = (view?.patrol_options ?? []).filter((o) => o.piece in patrolChosen);
    if (opts.length === 0) return [];
    return opts[0].targets.filter((h) => opts.every((o) => o.targets.includes(h)));
  });
  const patrolTarget = $derived(patrolPick !== null && patrolTargets.includes(patrolPick) ? patrolPick : null);
  // Sources the engine no longer offers (sent, or phase L over) drop out.
  $effect(() => {
    const offered = new Set((view?.patrol_options ?? []).map((o) => o.piece));
    const kept = Object.fromEntries(Object.entries(patrolChosen).filter(([id]) => offered.has(id)));
    if (Object.keys(kept).length !== Object.keys(patrolChosen).length) patrolChosen = kept;
  });
  /** Phase E: the ship picked to station, and the hexes open to it, as the engine offers them (30.1x). */
  let shipPick: PieceId | null = $state(null);
  const stationHexes: HexId[] = $derived(shipPick !== null && view?.station_options?.ships.includes(shipPick) ? view.station_options.hexes : []);
  // A ship the engine no longer offers (phase E over) drops out.
  $effect(() => {
    if (shipPick !== null && !view?.station_options?.ships.includes(shipPick)) shipPick = null;
  });
  const wrecked: HexId[] = $derived(Object.keys(view?.broken_down ?? {}));
  let map: Map | undefined = $state();
  // A forced retreat is the ordered stack's, nobody else's: it stands
  // ticked and the map comes to it.
  $effect(() => {
    if (!view?.pending || !("forced-retreat" in view.pending)) return;
    const pieces = view.pending["forced-retreat"].pieces.filter((id) => {
      const l = view!.pieces[id]?.location;
      return typeof l === "object" && "hex" in l;
    });
    if (pieces.join(",") !== ticked.join(",")) {
      ticked = pieces;
      const l = pieces.length > 0 ? view.pieces[pieces[0]].location : null;
      if (l && typeof l === "object" && "hex" in l) map?.goTo(l.hex);
    }
  });
  /** Whether the ticked stack stands on a hex (the overlay applies) or could go off-map (8.8x). */
  const onMap = $derived(view ? allOnMap(view, ticked) : false);
  const offMap = $derived(view ? allOffMapCapable(view, ticked) : false);
  /** The engine's answer for the ticked stack: the hexes, or why it cannot move. */
  let reach: { ok: Reach[]; illegal: null } | { ok: []; illegal: Illegal } = $state({ ok: [], illegal: null });
  $effect(() => {
    const e = engine, p = purpose, t = ticked;
    reach = { ok: [], illegal: null };
    if (!e || !view || !view.to_act || !p || !onMap) return;
    return answer(e.reachable(view.to_act, t, p), (r) => (reach = "ok" in r ? { ok: r.ok, illegal: null } : { ok: [], illegal: r.illegal }));
  });
  const reaches = $derived(reach.ok);
  /** Who could pay the ticked stack's fuel (32.17), and the player's pick; none picked lets the engine take the nearest. */
  let fuelSources: PieceId[] = $state([]);
  $effect(() => {
    const e = engine, t = ticked;
    fuelSources = [];
    if (!e || !view?.to_act || purpose !== "move" || !onMap || reaches.length === 0) return;
    return answer(e.fuelSources(view.to_act, t), (r) => (fuelSources = r));
  });
  /** A lone ticked raider that may raid Rommel now: the engine's preview of the raid does not refuse it (27.6x). */
  let raid: PieceId | null = $state(null);
  $effect(() => {
    const e = engine, t = ticked;
    raid = null;
    if (!e || !view?.to_act || purpose !== "move" || t.length !== 1 || view.pieces[t[0]]?.kind !== "raider") return;
    const piece = t[0];
    return answer(e.preview(view.to_act, { "raid-rommel": { piece } }), (r) => (raid = "illegal" in r ? null : piece));
  });
  /** Enemy supply units a lone ticked raider may raid as dumps now: the engine's preview does not refuse them (27.55, R-149). */
  let dumps: { target: PieceId; hex: HexId }[] = $state([]);
  $effect(() => {
    const e = engine, t = ticked, v = view;
    dumps = [];
    if (!e || !v?.to_act || purpose !== "move" || t.length !== 1 || v.pieces[t[0]]?.kind !== "raider") return;
    const piece = t[0], side = v.to_act;
    const hexOf = (p: Piece | undefined) => (p && typeof p.location === "object" && "hex" in p.location ? p.location.hex : null);
    const at = hexOf(v.pieces[piece]);
    if (!at) return;
    const [ax, ay] = centre(at);
    const near = Object.values(v.pieces).flatMap((p) => {
      const h = hexOf(p);
      if (!h || p.side === side || (p.kind !== "supply-unit" && p.kind !== "dummy-supply-unit")) return [];
      const [x, y] = centre(h);
      return Math.hypot(x - ax, y - ay) < 2 * SIZE ? [{ target: p.id, hex: h }] : [];
    });
    if (near.length === 0) return;
    return answer(
      Promise.all(near.map((d) => e.preview(side, { "raid-dump": { piece, target: d.target } }))),
      (rs) => (dumps = near.filter((_, i) => !("illegal" in rs[i]))),
    );
  });
  let fuelPick: PieceId | null = $state(null);
  const fuelFrom = $derived(fuelPick && fuelSources.includes(fuelPick) ? fuelPick : null);
  const occupied = $derived(new Set(view ? stacks(view).keys() : []));
  /** The hexes the ZOC declaration asks about, outlined on the map. */
  const asked: HexId[] = $derived(view?.pending && "declare-zoc" in view.pending ? view.pending["declare-zoc"].hexes : []);
  /** Where the ticked stack stands, for drawing paths from it. */
  const from: HexId | null = $derived.by(() => {
    const p = view && ticked.length > 0 ? view.pieces[ticked[0]] : null;
    return p && typeof p.location === "object" && "hex" in p.location ? p.location.hex : null;
  });

  /**
   * Every order goes to the engine as the side it is waiting on; the UI
   * never picks one. On `ok` the events go to the log and the view is
   * replaced whole; on `illegal` nothing changes but the toast and a red
   * log line. Orders go one at a time: each is sent once the one before
   * it has been answered and its view taken, as the side then to act.
   */
  function send(order: Order): Promise<boolean> {
    const run = orders.then(() => sendNow(order));
    orders = run.catch(() => false);
    return run;
  }
  let orders: Promise<unknown> = Promise.resolve();

  async function sendNow(order: Order): Promise<boolean> {
    const e = engine;
    if (!e || !view || !view.to_act) return false;
    // Online, orders go as this seat; the server would refuse the other side's.
    if (e instanceof NetworkEngine && view.to_act !== e.side) {
      illegal = "Not your move: the other side is to act.";
      refusals++;
      return false;
    }
    const r = await e.apply(view.to_act, order);
    // Another game was opened while the order was out: it is not this one's.
    if (engine !== e) return false;
    if ("illegal" in r) {
      illegal = `Illegal (${r.illegal.case}): ${r.illegal.reason}`;
      // A refused sealed answer stays out of the log: the engine's reason
      // repeats the answer, and the log is the other side's to read next.
      if (!("sealed" in (view.pending ?? {}))) log = withIllegal(log, illegal);
      refusals++;
      return false;
    }
    illegal = null;
    flying = null;
    const before = view;
    const next = await e.view();
    // A piece the order brought into play (a formed raider) is named from the view after it.
    const name = (id: PieceId) => {
      const p = before.pieces[id] ?? next.pieces[id];
      return p ? label(p).long : id;
    };
    if (engine !== e) return false;
    view = next;
    // Online, the stream logs every order, this seat's own included.
    if (!(e instanceof NetworkEngine)) {
      log = withEvents(
        log,
        clockLine(before),
        r.ok.map((e) => formatEvent(e, name)),
      );
      curtain = curtainAfter(before.to_act, view.to_act);
    }
    custom = [];
    railStops = [];
    stopping = false;
    autosaveNow();
    if (curtain) {
      picked = null;
      ticked = [];
    } else {
      // The ticked pieces stay ticked where they now stand, so a stack
      // can be walked on click by click; pieces gone from the map drop out.
      ticked = ticked.filter((id) => {
        const l = view!.pieces[id]?.location;
        return typeof l === "object" && ("hex" in l || "box" in l || "transit" in l);
      });
      if (ticked.length > 0) {
        const l = view.pieces[ticked[0]].location;
        if (typeof l === "object" && "hex" in l) hex = l.hex;
      }
    }
    return true;
  }

  /** The ticked stack along `path`, as the order the moment calls for. */
  async function sendPath(path: HexId[]): Promise<boolean> {
    if (!purpose || ticked.length === 0) return false;
    const order: Order =
      purpose === "react"
        ? { react: { pieces: ticked, path } }
        : purpose === "retreat-before-assault"
          ? { "retreat-before-assault": { pieces: ticked, path } }
          : purpose === "forced-retreat"
            ? { retreat: { path } }
            : purpose === "rail"
              ? { rail: { pieces: ticked, path, ...(stopsOn(railStops, path).length > 0 ? { stops: stopsOn(railStops, path) } : {}) } }
              : { move: { pieces: ticked, path, assault, ...(fuelFrom ? { fuel_from: fuelFrom } : {}) } };
    const ok = await send(order);
    if (ok) assault = false;
    return ok;
  }

  /** The next piece of the same tray, so a run of placements is one click each. */
  function nextToPlace(v: View, side: string): PieceId | null {
    const p = Object.values(v.pieces).find((p) => p.side === side && typeof p.location === "object" && "pending" in p.location);
    return p?.id ?? null;
  }

  async function onhex(h: HexId, shift = false) {
    if (!view) return;
    if (!picked && workItem && view.work_options && workHexes.includes(h)) {
      hex = h;
      if (await send({ "place-work": { item: workItem, hex: h } }) && !(view?.work_options?.[workItem] ?? []).length) workItem = null;
      return;
    }
    if (picked && view.pending && "set-up" in view.pending) {
      hex = h;
      const side = view.to_act;
      const piece = picked;
      if (await send({ place: { piece, hex: h } })) {
        placedNow = view!.pending && "set-up" in view!.pending && view!.to_act === side ? [...placedNow, piece] : [];
        picked = view!.to_act === side && side ? nextToPlace(view!, side) : null;
      }
      return;
    }
    // An armed air lift: the click is its destination, legal or not,
    // while its supply unit is still ticked in phase C.
    if (flying && (view.clock.phase !== "organisation" || !ticked.includes(flying))) flying = null;
    if (flying) {
      const piece = flying;
      flying = null;
      send({ "air-lift": { piece, to: { hex: h } } });
      return;
    }
    // An armed prisoner march: each click is the path's next hex.
    if (march) {
      march = { ...march, path: [...march.path, h] };
      return;
    }
    // An armed tow: each click is the path's next hex.
    if (tow) {
      tow = { ...tow, path: [...tow.path, h] };
      return;
    }
    // Phase E: with a ship picked, the click is its station; the engine judges it.
    if (!view.pending && shipPick !== null && view.station_options) {
      const ship = shipPick;
      hex = h;
      if (await send({ station: { ship, hex: h } })) shipPick = null;
      return;
    }
    // Phase L: a click on a marked hex is the patrol's target.
    if (!view.pending && patrolTargets.includes(h)) {
      patrolPick = h;
      hex = h;
      return;
    }
    // The close assault step: a click on a hex still to assault resolves it.
    if (!view.pending && toAssault.includes(h)) {
      send({ assault: { target: h } });
      return;
    }
    // Phase J: an armed stop takes the clicked hex of the line.
    if (purpose === "rail" && stopping && ticked.length > 0) {
      if (reaches.some((r) => r.hex === h)) railStops = addStop(railStops, h);
      stopping = false;
      return;
    }
    // A shift-click extends the custom path; the engine judges it on Enter.
    if (shift && purpose && ticked.length > 0) {
      custom = extendPath(custom, h);
      return;
    }
    // A click on a reachable hex is the move, unless the hex holds the
    // player's own pieces, which it selects instead (moving onto them
    // is a shift-click path): a stack must never move by a click meant
    // to look at another.
    const r = purpose && custom.length === 0 ? reaches.find((r) => r.hex === h) : undefined;
    if (r && !ownStackAt(h)) {
      sendPath(r.path);
      return;
    }
    select(h);
  }

  function ownStackAt(h: HexId): boolean {
    return !!view && defaultTicked(view, h, view.to_act).length > 0;
  }

  /** Selects a hex: its own pieces start ticked for the side to act. */
  function select(h: HexId) {
    hex = h;
    custom = [];
    railStops = [];
    ticked = view ? defaultTicked(view, h, view.to_act) : [];
  }

  function ontoggle(id: PieceId) {
    ticked = toggle(ticked, id);
    custom = [];
    railStops = [];
  }

  /** A click on a piece in a box or in transit: ticks it with the others of its place. */
  function ontick(id: PieceId) {
    if (!view) return;
    ticked = toggleOffMap(view, ticked, id);
    custom = [];
  }

  function onoffmap(to: OffMapPlace) {
    send({ "move-off-map": { pieces: ticked, to } });
  }

  function onpick(id: PieceId) {
    picked = picked === id ? null : id;
  }

  function onkey(e: KeyboardEvent) {
    if ((e.target as HTMLElement | null)?.tagName === "INPUT") return;
    if (e.key === "Enter" && custom.length > 0) {
      e.preventDefault();
      sendPath(custom);
    } else if (e.key === "Escape") {
      if (custom.length > 0) custom = [];
      else {
        ticked = [];
        hex = null;
      }
    }
  }

  /**
   * Puts a game on the screen with the UI-only state cleared: a new game, a
   * loaded save, the autosave. `curtained` drops the curtain for the side
   * to act, for a game whoever opened it may not be entitled to see.
   */
  async function start(e: Engine, curtained = false) {
    const first = await e.view();
    engine = e;
    view = first;
    hex = null;
    picked = null;
    ticked = [];
    flying = null;
    custom = [];
    log = [];
    curtain = curtained ? curtainAfter(null, view.to_act) : null;
    reportOpen = true;
    illegal = null;
    autosaveNow();
  }

  /** The state JSON into `localStorage` (spec, *Save and load*), after every accepted order; a full store is not an error worth stopping play for. */
  async function autosaveNow() {
    if (!engine || engine instanceof NetworkEngine) return;
    try {
      localStorage.setItem(AUTOSAVE_KEY, await engine.save());
    } catch {
      // Private mode or a full store: play goes on without the autosave.
    }
  }

  /**
   * Hands a save's text to the engine; on its refusal the current game, if
   * any, is kept and the reason shown. A player's load opens behind the
   * curtain (`curtained`): whoever loads may be the side not to act.
   */
  async function loadJson(json: string, curtained = true): Promise<boolean> {
    try {
      await start(LocalEngine.load(json), curtained);
      loadError = null;
      return true;
    } catch (e) {
      loadError = `This file was not loaded: ${e instanceof Error ? e.message : String(e)}`;
      return false;
    }
  }

  function loadFile(file: File) {
    file
      .text()
      .then((json) => loadJson(json))
      .catch((e: unknown) => {
        loadError = `This file could not be read: ${e instanceof Error ? e.message : String(e)}`;
      });
  }

  /** Downloads the state JSON as `saveName`'s file. */
  async function saveFile() {
    if (!engine || !view) return;
    const name = saveName(view);
    const blob = new Blob([await engine.save()], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  async function continueAutosave() {
    const json = localStorage.getItem(AUTOSAVE_KEY);
    if (json === null) return;
    if (!(await loadJson(json))) autosave = null;
  }

  /**
   * Opens the online game `id` (spec, *Web client*): the seat's view,
   * replaced whenever the stream brings an order, either side's; the
   * stream's events go to the log under the clock they happened at.
   */
  async function openOnline(id: string) {
    let e: NetworkEngine;
    try {
      e = await NetworkEngine.open(id);
    } catch (err) {
      onlineError = err instanceof Error ? err.message : String(err);
      return;
    }
    await start(e);
    turnText = e.turnText;
    let logged = e.log.length;
    e.onChange(async () => {
      if (engine !== e) return;
      const before = view;
      const next = await e.view();
      if (engine !== e) return;
      const fresh = e.log.slice(logged);
      logged = e.log.length;
      if (before && fresh.length > 0) {
        const name = (id: PieceId) => {
          const p = before.pieces[id] ?? next.pieces[id];
          return p ? label(p).long : id;
        };
        log = withEvents(log, clockLine(before), fresh.map((ev) => formatEvent(ev, name)));
      }
      view = next;
      turnText = e.turnText;
    });
  }

  initEngine()
    .then(() => {
      loaded = true;
      if (route.page === "game") openOnline(route.id);
      const stored = localStorage.getItem(AUTOSAVE_KEY);
      autosave = stored === null ? null : describeSave(stored);
      if (import.meta.env.MODE === "test") {
        // Test builds only: lets Playwright compare the DOM with the engine
        // and start from a fixture state (what `save()` wrote). Hot-seat
        // only: it reads the wasm game synchronously.
        const game = () => (engine as LocalEngine).game;
        (window as unknown as { __cna: unknown }).__cna = {
          view: () => JSON.parse(game().view()) as View,
          save: () => game().save(),
          /** How many orders the engine has accepted: the e2e driver's sync point after each click. */
          orders: () => (JSON.parse(game().save()) as { log: unknown[] }).log.length,
          sealedOptions: () => JSON.parse(game().sealedOptions((JSON.parse(game().view()) as View).to_act!)) as unknown,
          /** A fixture state, opened without the curtain. */
          load: async (json: string) => {
            if (!(await loadJson(json, false))) throw new Error(loadError ?? "not loaded");
          },
        };
      }
    })
    .catch((e: unknown) => {
      error = e instanceof Error ? e.message : String(e);
    });
</script>

<svelte:window onkeydown={onkey} />

<div class="app">
  {#if curtain}
    <Curtain side={curtain} ondismiss={() => (curtain = null)} />
  {/if}
  {#if illegal}
    {#key refusals}
      <Toast text={illegal} ondone={() => (illegal = null)} />
    {/key}
  {/if}
  {#if view && engine}
    {#if view.over && reportOpen}
      <EndDialog {view} onclose={() => (reportOpen = false)} />
    {/if}
    <TopBar {view} {hex} title={LocalEngine.scenarioName(view.scenario)} />
    <div class="body">
      <main class="board">
        <Map bind:this={map} {onhex} onhover={(h) => (hover = h)} ongoto={(h) => (hex = h)}>
          {#snippet overlays(scale)}
            <Placements hexes={picked ? placements : workHexes} />
            <Marks hexes={asked} kind="zoc" />
            <Marks hexes={toAssault} kind="assault" />
            <Marks hexes={patrolTargets} kind="patrol" />
            <Marks hexes={stationHexes} kind="station" />
            <Marks hexes={wrecked} kind="wreck" />
            <Reachable {reaches} {from} part="fill" />
            <Counters view={view!} selected={hex} {scale} />
            <Ships view={view!} />
            <Reachable {reaches} {from} hovered={hover} {custom} {occupied} part="labels" />
          {/snippet}
        </Map>
      </main>
      <aside class="side">
        <div class="panels">
          {#if online}
            <section class="panel online" data-testid="online">
              <h2>Game {online.gameId} · you play {sideName(online.side)}</h2>
              <p><strong data-testid="turn">{turnText ?? "Nobody is to act"}</strong>{online.opponent && turnText === "Your move" ? ` · against ${online.opponent}` : ""}{online.open ? " · open information" : ""}</p>
              <p><a href="/lobby">Lobby</a></p>
            </section>
          {:else}
            <GamePanel error={loadError} onsave={saveFile} onload={loadFile} />
          {/if}
          {#if view.over}
            <section class="panel over" data-testid="over">
              <h2>Game over</h2>
              <p>The engine takes no more orders (60.22). <button type="button" data-testid="show-report" onclick={() => (reportOpen = true)}>Show the victory report</button></p>
            </section>
          {/if}
          {#if toAssault.length > 0}
            <section class="panel assault" data-testid="assault-panel">
              <h2>Close assault · {toAssault.length} to resolve</h2>
              <p>Click an outlined hex to resolve the assault assigned against it: {toAssault.join(", ")}. Pass ends the step once every one is resolved (15.2x).</p>
            </section>
          {/if}
          {#if myTurn}
          <PhaseControls {view} {ticked} {hex} {flying} onfly={(p) => (flying = p)} onorder={(o) => send(o)} />
          <MarchPanel
            {view}
            {march}
            onarm={(m) => (march = m)}
            onsend={async () => {
              if (march && (await send({ "march-prisoners": { from: march.from, path: march.path, points: march.points } }))) march = null;
            }}
            oncancel={() => (march = null)}
          />
          <TowPanel
            {view}
            {tow}
            onarm={(t) => (tow = t)}
            onsend={async () => {
              if (tow && (await send({ tow: { hex: tow.hex, weapon: tow.weapon, points: tow.points, path: tow.path } }))) tow = null;
            }}
            oncancel={() => (tow = null)}
          />
          <PatrolPanel
            {view}
            bind:chosen={patrolChosen}
            target={patrolTarget}
            onsend={async (o) => {
              if (await send(o)) {
                patrolChosen = {};
                patrolPick = null;
              }
            }}
          />
          <FleetPanel {view} bind:picked={shipPick} />
          <SetupPanel
            {view}
            picked={pickedPiece}
            hexes={placements}
            {workItem}
            onwork={(item) => {
              picked = null;
              workItem = item;
            }}
            onpass={() => {
              workItem = null;
              send("pass");
            }}
          />
          {#if view.pending}
            <Pending {view} {ticked} {ontoggle} onorder={(o) => send(o)} {sealed} {reaches} pass={passPreview} />
          {/if}
          {/if}
          {#if purpose}
            <MovePanel {view} {purpose} {ticked} {reaches} {fuelSources} bind:fuelPick hovered={hover} refusal={reach.illegal} bind:assault {custom} {onMap} {offMap} {raid} {dumps} onsend={() => sendPath(custom)} onclear={() => (custom = [])} {onoffmap} onorder={(o) => send(o)} bind:railStops bind:stopping />
          {/if}
          <StackPanel {view} {hex} {ticked} {ontoggle} takeBack={placedNow} onorder={async (o) => {
            if ((await send(o)) && typeof o === "object" && "unplace" in o) {
              placedNow = placedNow.filter((id) => id !== o.unplace.piece);
              picked = o.unplace.piece;
            }
          }} />
          <OffMap {view} {picked} {onpick} {ticked} {ontick} />
        </div>
        <LogPanel entries={log} />
      </aside>
    </div>
  {:else if error}
    <p class="status error">The engine failed to load: {error}</p>
  {:else if route.page === "game" && onlineError}
    <p class="status error" role="alert">Game {route.id} could not be opened: {onlineError}.
      {#if known && !account}<a href={SIGN_IN_URL} onclick={rememberPage}>Sign in with Discord</a> ·{/if}
      <a href="/">Start screen</a></p>
  {:else if route.page === "game"}
    <p class="status">Opening game {route.id}…</p>
  {:else if route.page === "join" && known}
    <JoinScreen {account} id={route.id} invite={route.invite} />
  {:else if loaded && inLobby && account}
    <Lobby {account} onback={() => { inLobby = false; me().then((a) => (account = a), () => (account = null)); }} />
  {:else if loaded}
    <StartScreen {account} onlobby={() => (inLobby = true)} {autosave} error={loadError} oncontinue={continueAutosave} scenarios={LocalEngine.scenarios()} onnew={(scenario, seed) => start(LocalEngine.newGame(scenario, seed))} onload={loadFile} />
  {:else}
    <p class="status">Loading the engine…</p>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .board {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .side {
    width: 340px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 0.6rem 0.8rem;
    box-sizing: border-box;
    background: #f4f0e6;
    border-left: 1px solid #d8d1c0;
  }
  /* The panels scroll above the log, which keeps its own strip at the foot. */
  .panels {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .side :global(.log) {
    flex: none;
    height: 28%;
    min-height: 120px;
    margin-top: 0.6rem;
  }
  /* Narrow screens: the panel goes under the map and the page scrolls. */
  @media (max-width: 760px) {
    .app {
      height: auto;
      min-height: 100vh;
    }
    .body {
      flex-direction: column;
    }
    .board {
      height: 60vh;
      flex: none;
    }
    .side {
      width: auto;
      display: block;
      border-left: none;
      border-top: 1px solid #d8d1c0;
    }
    .panels {
      overflow: visible;
    }
    .side :global(.log) {
      height: 40vh;
    }
  }
  .status {
    padding: 1rem;
    font: 15px system-ui, sans-serif;
  }
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    border-radius: 4px;
  }
  .panel h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  .panel p {
    margin: 0;
  }
  .panel.assault {
    background: #fbe3e1;
    border: 1px solid #d9847d;
  }
  .panel.online {
    background: #e9e4d8;
    border: 1px solid #b9b2a2;
  }
  .panel.online p + p {
    margin-top: 0.3rem;
  }
  .panel.over {
    background: #e9e4d8;
    border: 1px solid #b9b2a2;
  }
  .panel button {
    font: inherit;
    padding: 0.2rem 0.5rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
  .error {
    color: #a4161a;
  }
</style>
