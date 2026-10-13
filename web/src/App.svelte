<script lang="ts">
  import { Engine, initEngine } from "./engine";
  import Map from "./Map.svelte";
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
    <main class="board">
      <Map onhex={(h) => (hex = h)} />
    </main>
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
  .board {
    flex: 1;
    min-height: 0;
  }
  .status {
    padding: 1rem;
    font: 15px system-ui, sans-serif;
  }
  .error {
    color: #a4161a;
  }
</style>
