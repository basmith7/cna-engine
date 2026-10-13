<script lang="ts">
  import { clockLine } from "./clock";
  import Counters from "./Counters.svelte";
  import { label, stacks } from "./counters";
  import Curtain from "./Curtain.svelte";
  import { curtainAfter } from "./curtain";
  import { Engine, initEngine } from "./engine";
  import { formatEvent } from "./format";
  import { withEvents, withIllegal, type LogEntry } from "./log";
  import LogPanel from "./LogPanel.svelte";
  import Map from "./Map.svelte";
  import Marks from "./Marks.svelte";
  import MovePanel from "./MovePanel.svelte";
  import { allOffMapCapable, allOnMap, defaultTicked, extendPath, toggle, toggleOffMap } from "./moves";
  import OffMap from "./OffMap.svelte";
  import Pending from "./Pending.svelte";
  import PhaseControls from "./PhaseControls.svelte";
  import Placements from "./Placements.svelte";
  import Reachable from "./Reachable.svelte";
  import SetupPanel from "./SetupPanel.svelte";
  import StackPanel from "./StackPanel.svelte";
  import Toast from "./Toast.svelte";
  import TopBar from "./TopBar.svelte";
  import type { HexId, Illegal, OffMapPlace, Order, PieceId, Purpose, Reach, Side, View } from "./types";

  let engine: Engine | undefined = $state();
  let view: View | undefined = $state();
  let hex: string | null = $state(null);
  let error: string | null = $state(null);
  /** The tray piece picked to place; UI-only state (spec, *State sync*). */
  let picked: PieceId | null = $state(null);
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
  /** The assault toggle for the next move. */
  let assault = $state(false);
  /** The custom path shift-clicks build; Enter sends it. */
  let custom: HexId[] = $state([]);
  /** The hex under the pointer, for the path preview. */
  let hover: HexId | null = $state(null);

  // Re-read from the engine whenever the view is replaced: the engine is
  // not reactive, the view standing in for its version.
  const placements: HexId[] = $derived.by(() => (view && picked && engine ? engine.placements(picked) : []));
  const pickedPiece = $derived(view && picked ? (view.pieces[picked] ?? null) : null);

  /** What a click on a reachable hex would send: a move in a movement segment, a reaction while one is offered. */
  const purpose: Purpose | null = $derived.by(() => {
    if (!view || !view.to_act) return null;
    if (view.pending && "reaction" in view.pending) return "reaction" in view.pending ? "react" : null;
    if (view.pending) return null;
    return view.clock.phase === "movement-combat" && view.clock.segment === "movement" ? "move" : null;
  });
  /** Whether the ticked stack stands on a hex (the overlay applies) or could go off-map (8.8x). */
  const onMap = $derived(view ? allOnMap(view, ticked) : false);
  const offMap = $derived(view ? allOffMapCapable(view, ticked) : false);
  /** The engine's answer for the ticked stack: the hexes, or why it cannot move. */
  const reach: { ok: Reach[]; illegal: null } | { ok: []; illegal: Illegal } = $derived.by(() => {
    if (!engine || !view || !view.to_act || !purpose || !onMap) return { ok: [], illegal: null };
    const r = engine.reachable(view.to_act, ticked, purpose);
    return "ok" in r ? { ok: r.ok, illegal: null } : { ok: [], illegal: r.illegal };
  });
  const reaches = $derived(reach.ok);
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
   * log line.
   */
  function send(order: Order): boolean {
    if (!engine || !view || !view.to_act) return false;
    const r = engine.apply(view.to_act, order);
    if ("illegal" in r) {
      illegal = `Illegal (${r.illegal.case}): ${r.illegal.reason}`;
      log = withIllegal(log, illegal);
      refusals++;
      return false;
    }
    illegal = null;
    const before = view;
    const name = (id: PieceId) => (before.pieces[id] ? label(before.pieces[id]).long : id);
    view = engine.view();
    log = withEvents(
      log,
      clockLine(before),
      r.ok.map((e) => formatEvent(e, name)),
    );
    curtain = curtainAfter(before.to_act, view.to_act);
    custom = [];
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
  function sendPath(path: HexId[]): boolean {
    if (!purpose || ticked.length === 0) return false;
    const ok = purpose === "react" ? send({ react: { pieces: ticked, path } }) : send({ move: { pieces: ticked, path, assault } });
    if (ok) assault = false;
    return ok;
  }

  /** The next piece of the same tray, so a run of placements is one click each. */
  function nextToPlace(v: View, side: string): PieceId | null {
    const p = Object.values(v.pieces).find((p) => p.side === side && typeof p.location === "object" && "pending" in p.location);
    return p?.id ?? null;
  }

  function onhex(h: HexId, shift = false) {
    if (!view) return;
    if (picked && view.pending && "set-up" in view.pending) {
      hex = h;
      const side = view.to_act;
      if (send({ place: { piece: picked, hex: h } })) {
        picked = view!.to_act === side && side ? nextToPlace(view!, side) : null;
      }
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
    ticked = view ? defaultTicked(view, h, view.to_act) : [];
  }

  function ontoggle(id: PieceId) {
    ticked = toggle(ticked, id);
    custom = [];
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

  initEngine()
    .then(() => {
      engine = Engine.newGame(7);
      view = engine.view();
      if (import.meta.env.MODE === "test") {
        // Test builds only: lets Playwright compare the DOM with the engine
        // and start from a fixture state (what `save()` wrote).
        (window as unknown as { __cna: unknown }).__cna = {
          view: () => engine!.view(),
          save: () => engine!.save(),
          load: (json: string) => {
            engine = Engine.load(json);
            view = engine.view();
            hex = null;
            picked = null;
            ticked = [];
            custom = [];
            log = [];
            curtain = null;
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
  {#if view}
    <TopBar {view} {hex} />
    <div class="body">
      <main class="board">
        <Map {onhex} onhover={(h) => (hover = h)} ongoto={(h) => (hex = h)}>
          {#snippet overlays(scale)}
            <Placements hexes={placements} />
            <Marks hexes={asked} kind="zoc" />
            <Reachable {reaches} {from} part="fill" />
            <Counters view={view!} selected={hex} {scale} />
            <Reachable {reaches} {from} hovered={hover} {custom} {occupied} part="labels" />
          {/snippet}
        </Map>
      </main>
      <aside class="side">
        <div class="panels">
          <PhaseControls {view} {ticked} onorder={(o) => send(o)} />
          <SetupPanel {view} picked={pickedPiece} hexes={placements} />
          {#if view.pending}
            <Pending {view} {ticked} {ontoggle} onorder={(o) => send(o)} />
          {/if}
          {#if purpose}
            <MovePanel {purpose} {ticked} {reaches} refusal={reach.illegal} bind:assault {custom} {onMap} {offMap} onsend={() => sendPath(custom)} onclear={() => (custom = [])} {onoffmap} />
          {/if}
          <StackPanel {view} {hex} {ticked} {ontoggle} />
          <OffMap {view} {picked} {onpick} {ticked} {ontick} />
        </div>
        <LogPanel entries={log} />
      </aside>
    </div>
  {:else if error}
    <p class="status error">The engine failed to load: {error}</p>
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
  .error {
    color: #a4161a;
  }
</style>
