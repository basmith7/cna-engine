<script lang="ts">
  import { clockLine } from "./clock";
  import Counters from "./Counters.svelte";
  import { label, stacks } from "./counters";
  import Curtain from "./Curtain.svelte";
  import { curtainAfter } from "./curtain";
  import EndDialog from "./EndDialog.svelte";
  import { Engine, initEngine } from "./engine";
  import { formatEvent } from "./format";
  import GamePanel from "./GamePanel.svelte";
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
  import { AUTOSAVE_KEY, describeSave, saveName } from "./saves";
  import SetupPanel from "./SetupPanel.svelte";
  import StartScreen from "./StartScreen.svelte";
  import StackPanel from "./StackPanel.svelte";
  import Toast from "./Toast.svelte";
  import TopBar from "./TopBar.svelte";
  import type { ApplyResult, HexId, Illegal, OffMapPlace, Order, PieceId, Purpose, Reach, SealedOptions, Side, View } from "./types";

  let engine: Engine | undefined = $state();
  let view: View | undefined = $state();
  let hex: string | null = $state(null);
  let error: string | null = $state(null);
  /** The wasm is in; without a game yet the start screen shows. */
  let loaded = $state(false);
  /** The autosave's clock line on the start screen, or null when there is none. */
  let autosave: string | null = $state(null);
  /** The engine's refusal of the last file given to Load, until the next Load. */
  let loadError: string | null = $state(null);
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
  /** A supply unit armed to fly in phase C: the next hex clicked is where (32.37). */
  let flying: PieceId | null = $state(null);
  /** The assault toggle for the next move. */
  let assault = $state(false);
  /** The custom path shift-clicks build; Enter sends it. */
  let custom: HexId[] = $state([]);
  /** The hex under the pointer, for the path preview. */
  let hover: HexId | null = $state(null);
  /** The end-of-game dialog, closed to look at the map and reopened from a button. */
  let reportOpen = $state(true);

  // Re-read from the engine whenever the view is replaced: the engine is
  // not reactive, the view standing in for its version.
  const placements: HexId[] = $derived.by(() => (view && picked && engine ? engine.placements(picked) : []));
  const pickedPiece = $derived(view && picked ? (view.pieces[picked] ?? null) : null);

  /** What a click on a reachable hex would send: a move in a movement segment or the truck convoy phase, a reaction while one is offered, a retreat while one is asked. */
  const purpose: Purpose | null = $derived.by(() => {
    if (!view || !view.to_act) return null;
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
  const sealed: { ok: SealedOptions } | { ok: null; illegal: Illegal } | null = $derived.by(() => {
    if (!engine || !view || !view.to_act || !view.pending || !("sealed" in view.pending)) return null;
    const r = engine.sealedOptions(view.to_act);
    return "ok" in r ? r : { ok: null, illegal: r.illegal };
  });
  /** The engine's preview of Pass while a forced retreat is pending: what standing still costs (spec, *Interrupts*). */
  const passPreview: ApplyResult | null = $derived.by(() => {
    if (!engine || !view || !view.to_act || !view.pending || !("forced-retreat" in view.pending)) return null;
    return engine.preview(view.to_act, "pass");
  });
  /** The hexes the phasing player still has to assault: a click on one resolves it (spec, *Combat*). */
  const toAssault: HexId[] = $derived(view?.assaults_to_resolve ?? []);
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
  const reach: { ok: Reach[]; illegal: null } | { ok: []; illegal: Illegal } = $derived.by(() => {
    if (!engine || !view || !view.to_act || !purpose || !onMap) return { ok: [], illegal: null };
    const r = engine.reachable(view.to_act, ticked, purpose);
    return "ok" in r ? { ok: r.ok, illegal: null } : { ok: [], illegal: r.illegal };
  });
  const reaches = $derived(reach.ok);
  /** Who could pay the ticked stack's fuel (32.17), and the player's pick; none picked lets the engine take the nearest. */
  const fuelSources: PieceId[] = $derived(engine && view?.to_act && purpose === "move" && onMap && reaches.length > 0 ? engine.fuelSources(view.to_act, ticked) : []);
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
   * log line.
   */
  function send(order: Order): boolean {
    if (!engine || !view || !view.to_act) return false;
    const r = engine.apply(view.to_act, order);
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
    const name = (id: PieceId) => (before.pieces[id] ? label(before.pieces[id]).long : id);
    view = engine.view();
    log = withEvents(
      log,
      clockLine(before),
      r.ok.map((e) => formatEvent(e, name)),
    );
    curtain = curtainAfter(before.to_act, view.to_act);
    custom = [];
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
  function sendPath(path: HexId[]): boolean {
    if (!purpose || ticked.length === 0) return false;
    const order: Order =
      purpose === "react"
        ? { react: { pieces: ticked, path } }
        : purpose === "retreat-before-assault"
          ? { "retreat-before-assault": { pieces: ticked, path } }
          : purpose === "forced-retreat"
            ? { retreat: { path } }
            : purpose === "rail"
              ? { rail: { pieces: ticked, path } }
              : { move: { pieces: ticked, path, assault, ...(fuelFrom ? { fuel_from: fuelFrom } : {}) } };
    const ok = send(order);
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
    // An armed air lift: the click is its destination, legal or not,
    // while its supply unit is still ticked in phase C.
    if (flying && (view.clock.phase !== "organisation" || !ticked.includes(flying))) flying = null;
    if (flying) {
      const piece = flying;
      flying = null;
      send({ "air-lift": { piece, to: { hex: h } } });
      return;
    }
    // The close assault step: a click on a hex still to assault resolves it.
    if (!view.pending && toAssault.includes(h)) {
      send({ assault: { target: h } });
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

  /**
   * Puts a game on the screen with the UI-only state cleared: a new game, a
   * loaded save, the autosave. `curtained` drops the curtain for the side
   * to act, for a game whoever opened it may not be entitled to see.
   */
  function start(e: Engine, curtained = false) {
    engine = e;
    view = e.view();
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
  function autosaveNow() {
    if (!engine) return;
    try {
      localStorage.setItem(AUTOSAVE_KEY, engine.save());
    } catch {
      // Private mode or a full store: play goes on without the autosave.
    }
  }

  /**
   * Hands a save's text to the engine; on its refusal the current game, if
   * any, is kept and the reason shown. A player's load opens behind the
   * curtain (`curtained`): whoever loads may be the side not to act.
   */
  function loadJson(json: string, curtained = true): boolean {
    try {
      start(Engine.load(json), curtained);
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

  /** Downloads the state JSON as `graziani-gt<N>-os<M>.json`. */
  function saveFile() {
    if (!engine || !view) return;
    const blob = new Blob([engine.save()], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = saveName(view);
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function continueAutosave() {
    const json = localStorage.getItem(AUTOSAVE_KEY);
    if (json === null) return;
    if (!loadJson(json)) autosave = null;
  }

  initEngine()
    .then(() => {
      loaded = true;
      const stored = localStorage.getItem(AUTOSAVE_KEY);
      autosave = stored === null ? null : describeSave(stored);
      if (import.meta.env.MODE === "test") {
        // Test builds only: lets Playwright compare the DOM with the engine
        // and start from a fixture state (what `save()` wrote).
        (window as unknown as { __cna: unknown }).__cna = {
          view: () => engine!.view(),
          save: () => engine!.save(),
          /** How many orders the engine has accepted: the e2e driver's sync point after each click. */
          orders: () => (JSON.parse(engine!.save()) as { log: unknown[] }).log.length,
          sealedOptions: () => engine!.sealedOptions(engine!.view().to_act!),
          /** A fixture state, opened without the curtain. */
          load: (json: string) => {
            if (!loadJson(json, false)) throw new Error(loadError ?? "not loaded");
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
    <TopBar {view} {hex} />
    <div class="body">
      <main class="board">
        <Map bind:this={map} {onhex} onhover={(h) => (hover = h)} ongoto={(h) => (hex = h)}>
          {#snippet overlays(scale)}
            <Placements hexes={placements} />
            <Marks hexes={asked} kind="zoc" />
            <Marks hexes={toAssault} kind="assault" />
            <Marks hexes={wrecked} kind="wreck" />
            <Reachable {reaches} {from} part="fill" />
            <Counters view={view!} selected={hex} {scale} />
            <Reachable {reaches} {from} hovered={hover} {custom} {occupied} part="labels" />
          {/snippet}
        </Map>
      </main>
      <aside class="side">
        <div class="panels">
          <GamePanel error={loadError} onsave={saveFile} onload={loadFile} />
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
          <PhaseControls {view} {ticked} {hex} {flying} onfly={(p) => (flying = p)} onorder={(o) => send(o)} />
          <SetupPanel {view} picked={pickedPiece} hexes={placements} />
          {#if view.pending}
            <Pending {view} {ticked} {ontoggle} onorder={(o) => send(o)} {sealed} {reaches} pass={passPreview} />
          {/if}
          {#if purpose}
            <MovePanel {view} {purpose} {ticked} {reaches} {fuelSources} bind:fuelPick hovered={hover} refusal={reach.illegal} bind:assault {custom} {onMap} {offMap} onsend={() => sendPath(custom)} onclear={() => (custom = [])} {onoffmap} />
          {/if}
          <StackPanel {view} {hex} {ticked} {ontoggle} onorder={(o) => send(o)} />
          <OffMap {view} {picked} {onpick} {ticked} {ontick} />
        </div>
        <LogPanel entries={log} />
      </aside>
    </div>
  {:else if error}
    <p class="status error">The engine failed to load: {error}</p>
  {:else if loaded}
    <StartScreen {autosave} error={loadError} oncontinue={continueAutosave} onnew={(seed) => start(Engine.newGame(seed))} onload={loadFile} />
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
