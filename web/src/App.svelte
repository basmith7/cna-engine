<script lang="ts">
  import Counters from "./Counters.svelte";
  import { Engine, initEngine } from "./engine";
  import Map from "./Map.svelte";
  import OffMap from "./OffMap.svelte";
  import StackPanel from "./StackPanel.svelte";
  import TopBar from "./TopBar.svelte";
  import type { View } from "./types";

  let engine: Engine | undefined = $state();
  let view: View | undefined = $state();
  let hex: string | null = $state(null);
  let error: string | null = $state(null);

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
  {#if view}
    <TopBar {view} {hex} />
    <div class="body">
      <main class="board">
        <Map onhex={(h) => (hex = h)}>
          {#snippet overlays(scale)}
            <Counters view={view!} selected={hex} {scale} />
          {/snippet}
        </Map>
      </main>
      <aside class="side">
        <StackPanel {view} {hex} />
        <OffMap {view} />
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
    overflow-y: auto;
    padding: 0.6rem 0.8rem;
    box-sizing: border-box;
    background: #f4f0e6;
    border-left: 1px solid #d8d1c0;
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
      overflow: visible;
      border-left: none;
      border-top: 1px solid #d8d1c0;
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
