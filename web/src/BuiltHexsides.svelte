<svelte:options namespace="svg" />

<script lang="ts">
  // Roads and railroads the scenario finishes at set-up (59.12, the
  // `roads_built` module): the base layer draws them as cna's map does,
  // unfinished, so this layer draws them finished over it.
  import { edgePath } from "./map";
  import type { HexId } from "./types";

  let { built }: { built: { roads: [HexId, HexId][]; railroads: [HexId, HexId][] } | undefined } = $props();
</script>

{#if built}
  <g class="built-hexsides" data-testid="built-hexsides">
    {#each built.roads as [a, b] (a + b)}
      <path class="hexside road built" data-a={a} data-b={b} d={edgePath(a, b)} />
    {/each}
    {#each built.railroads as [a, b] (a + b)}
      <path class="hexside railroad built" data-a={a} data-b={b} d={edgePath(a, b)} />
    {/each}
  </g>
{/if}

<style>
  .built-hexsides {
    pointer-events: none;
  }
</style>
