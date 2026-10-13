<svelte:options namespace="svg" />

<script lang="ts">
  // The placement overlay: the hexes the engine would accept for the
  // tray piece picked, shaded over the base layer. Only those hexes are
  // in this layer; the base is never touched (spec, *Map rendering*).
  import { polygonPoints } from "./geom";
  import type { HexId } from "./types";

  let { hexes }: { hexes: HexId[] } = $props();
</script>

<g class="placements" data-testid="placements" data-count={hexes.length}>
  {#each hexes as h (h)}
    <polygon class="placement" data-hex={h} points={polygonPoints(h)} />
  {/each}
</g>

<style>
  .placement {
    /* Light enough that the terrain under it still reads at the fitted view. */
    fill: #ffb703;
    fill-opacity: 0.32;
    stroke: #c77d00;
    stroke-width: 0.6;
    pointer-events: none;
  }
</style>
