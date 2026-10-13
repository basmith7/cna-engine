<svelte:options namespace="svg" />

<script lang="ts">
  // The counters layer: one group per occupied hex, rebuilt from the view
  // alone (spec, *State sync*). Every piece has its own `data-unit`
  // element at its hex; under the top counter the rest show as offset
  // edges, and a badge gives the count.
  import Counter from "./Counter.svelte";
  import { COUNTER, markers, stacks } from "./counters";
  import { centre } from "./geom";
  import type { View } from "./types";

  let { view, selected = null, scale = 1 }: { view: View; selected?: string | null; scale?: number } = $props();

  const groups = $derived([...stacks(view)]);
  const S = COUNTER;
  /** Zoomed out, a stack is still a mark the eye can find: at least this many pixels wide. */
  const MIN_PX = 16;
  const grow = $derived(Math.max(1, MIN_PX / (S * scale)));
  /** How far each counter beneath the top one peeks out. */
  const STEP = S * 0.08;
  const MAX_SHOWN = 4;
</script>

{#each groups as [hex, pieces] (hex)}
  {@const [cx, cy] = centre(hex)}
  {@const shown = Math.min(pieces.length, MAX_SHOWN)}
  <g class="stack" class:selected={hex === selected} data-stack={hex} data-count={pieces.length} transform="translate({cx} {cy}) scale({grow}) translate({-S / 2} {-S / 2 - (shown - 1) * STEP * 0.5})">
    {#each pieces.slice(1).reverse() as p, i (p.id)}
      {@const depth = pieces.length - 1 - i}
      {@const k = Math.min(depth, MAX_SHOWN - 1)}
      <g class="under side-{p.side}" data-unit={p.id} data-hex={hex} transform="translate({k * STEP} {k * STEP})">
        <rect width={S} height={S} rx={S * 0.06} />
      </g>
    {/each}
    <g data-unit={pieces[0].id} data-hex={hex}>
      <Counter piece={pieces[0]} markers={markers(pieces[0], view)} />
    </g>
    {#if pieces.length > 1}
      <g class="badge" transform="translate({S * 0.98} {S * 0.02})">
        <circle r={S * 0.17} />
        <text y={S * 0.06}>{pieces.length}</text>
      </g>
    {/if}
  </g>
{/each}

<style>
  .stack {
    cursor: pointer;
  }
  .under rect {
    fill: #efe8d8;
    stroke-width: 0.8;
  }
  .under.side-axis rect { stroke: #7a2e1f; }
  .under.side-cw rect { stroke: #1f3a5f; }
  .badge circle {
    fill: #1f2a33;
    stroke: #fff;
    stroke-width: 0.6;
  }
  .badge text {
    font: 700 5px system-ui, sans-serif;
    fill: #fff;
    text-anchor: middle;
    pointer-events: none;
  }
  .selected :global(.body),
  .selected .under rect {
    stroke: #ffb703;
    stroke-width: 1.6;
  }
</style>
