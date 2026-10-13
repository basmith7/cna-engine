<script lang="ts">
  import { clockLine } from "./clock";
  import Counters from "./Counters.svelte";
  import { label } from "./counters";
  import Curtain from "./Curtain.svelte";
  import { curtainAfter } from "./curtain";
  import { Engine, initEngine } from "./engine";
  import { formatEvent } from "./format";
  import { withEvents, withIllegal, type LogEntry } from "./log";
  import LogPanel from "./LogPanel.svelte";
  import Map from "./Map.svelte";
  import OffMap from "./OffMap.svelte";
  import Placements from "./Placements.svelte";
  import SetupPanel from "./SetupPanel.svelte";
  import StackPanel from "./StackPanel.svelte";
  import Toast from "./Toast.svelte";
  import TopBar from "./TopBar.svelte";
  import type { HexId, Order, PieceId, Side, View } from "./types";

  let engine: Engine | undefined = $state();
  let view: View | undefined = $state();
  let hex: string | null = $state(null);
  let error: string | null = $state(null);
  /** The tray piece picked to place; UI-only state (spec, *State sync*). */
  let picked: PieceId | null = $state(null);
  /** The engine's last refusal, until the toast goes or the next order. */
  let illegal: string | null = $state(null);
  /** The log panel: event lines under clock headers, refusals in red. */
  let log: LogEntry[] = $state([]);
  /** The side the curtain is down for, until they click it away. */
  let curtain: Side | null = $state(null);

  // Re-read from the engine whenever the view is replaced: the engine is
  // not reactive, the view standing in for its version.
  const placements: HexId[] = $derived.by(() => (view && picked && engine ? engine.placements(picked) : []));
  const pickedPiece = $derived(view && picked ? (view.pieces[picked] ?? null) : null);

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
      return false;
    }
    illegal = null;
    const before = view;
    const name = (id: PieceId) => (before.pieces[id] ? label(before.pieces[id]).short : id);
    view = engine.view();
    log = withEvents(
      log,
      clockLine(before),
      r.ok.map((e) => formatEvent(e, name)),
    );
    curtain = curtainAfter(before.to_act, view.to_act);
    if (curtain) picked = null;
    return true;
  }

  /** The next piece of the same tray, so a run of placements is one click each. */
  function nextToPlace(v: View, side: string): PieceId | null {
    const p = Object.values(v.pieces).find((p) => p.side === side && typeof p.location === "object" && "pending" in p.location);
    return p?.id ?? null;
  }

  function onhex(h: HexId) {
    hex = h;
    if (!view || !picked || !(view.pending && "set-up" in view.pending)) return;
    const side = view.to_act;
    if (send({ place: { piece: picked, hex: h } })) {
      picked = view!.to_act === side && side ? nextToPlace(view!, side) : null;
    }
  }

  function onpick(id: PieceId) {
    picked = picked === id ? null : id;
  }

  initEngine()
    .then(() => {
      engine = Engine.newGame(7);
      view = engine.view();
      if (import.meta.env.MODE === "test") {
        // Test builds only: lets Playwright compare the DOM with the engine.
        (window as unknown as { __cna: unknown }).__cna = {
          view: () => engine!.view(),
          save: () => engine!.save(),
        };
      }
    })
    .catch((e: unknown) => {
      error = e instanceof Error ? e.message : String(e);
    });
</script>

<div class="app">
  {#if curtain}
    <Curtain side={curtain} ondismiss={() => (curtain = null)} />
  {/if}
  {#if illegal}
    <Toast text={illegal} ondone={() => (illegal = null)} />
  {/if}
  {#if view}
    <TopBar {view} {hex} />
    <div class="body">
      <main class="board">
        <Map {onhex} ongoto={(h) => (hex = h)}>
          {#snippet overlays(scale)}
            <Placements hexes={placements} />
            <Counters view={view!} selected={hex} {scale} />
          {/snippet}
        </Map>
      </main>
      <aside class="side">
        <div class="panels">
          <SetupPanel {view} picked={pickedPiece} hexes={placements} />
          <StackPanel {view} {hex} />
          <OffMap {view} {picked} {onpick} />
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
