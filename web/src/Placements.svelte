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
    /* A cool hue, so it reads over the sand-coloured terrain; light enough
       that the terrain under it still shows at the fitted view. */
    fill: #7b4fd1;
    fill-opacity: 0.3;
    stroke: #4c2a9a;
    stroke-width: 0.6;
    pointer-events: none;
  }
</style>
