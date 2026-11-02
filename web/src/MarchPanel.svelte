<script lang="ts">
  // Phase H (28.13): the side to act's prisoners that may march. Arm a
  // hex's march, click the path hex by hex on the map, then send; the
  // engine judges the path.
  import { marchable } from "./prisoners";
  import type { HexId, View } from "./types";

  export interface March {
    from: HexId;
    points: number;
    path: HexId[];
  }

  let {
    view,
    march = null,
    onarm,
    onsend,
    oncancel,
  }: {
    view: View;
    march?: March | null;
    onarm: (m: March) => void;
    onsend: () => void;
    oncancel: () => void;
  } = $props();

  const groups = $derived(marchable(view));
  let counts: Record<HexId, number> = $state({});
</script>

{#if groups.length > 0 || march}
  <section class="panel" data-testid="march-panel">
    <h2>Prisoners · march (28.13)</h2>
    {#if march}
      <p>
        Marching {march.points} from {march.from}{#if march.path.length > 0}: {march.path.join(" → ")}{:else}: click the hexes of the path, 8 CP as foot infantry{/if}.
      </p>
      <button type="button" data-testid="march-send" disabled={march.path.length === 0} onclick={onsend}>March</button>
      <button type="button" onclick={oncancel}>Cancel</button>
    {:else}
      <ul>
        {#each groups as g (g.hex)}
          <li>
            <label>{g.hex}: <input type="number" min="1" max={g.points} value={counts[g.hex] ?? g.points} oninput={(e) => (counts[g.hex] = Number((e.target as HTMLInputElement).value))} aria-label="Points to march from {g.hex}" /> of {g.points}</label>
            <button type="button" data-testid="march-arm" onclick={() => onarm({ from: g.hex, points: counts[g.hex] ?? g.points, path: [] })}>March…</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #efe8f6;
    border: 1px solid #b7a2cf;
    border-radius: 4px;
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  p {
    margin: 0 0 0.3rem;
  }
  button {
    font: inherit;
    padding: 0.2rem 0.5rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  input {
    width: 3.5rem;
  }
</style>
