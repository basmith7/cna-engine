<svelte:options namespace="svg" />

<script lang="ts">
  // Hexes a panel is asking about, outlined on the map (the ZOC
  // declaration's hexes; the hexes still to assault in the close assault
  // step, which a click assaults). Only those hexes are in this layer.
  import { polygonPoints } from "./geom";
  import type { HexId } from "./types";

  let { hexes, kind }: { hexes: HexId[]; kind: string } = $props();
</script>

<g class="marks kind-{kind}" data-testid="marks" data-kind={kind} data-count={hexes.length}>
  {#each hexes as h (h)}
    <polygon class="mark" data-hex={h} points={polygonPoints(h)} />
  {/each}
</g>

<style>
  .mark {
    fill: #ffb703;
    fill-opacity: 0.25;
    stroke: #c77700;
    stroke-width: 1.6;
    stroke-dasharray: 3 2;
    pointer-events: none;
  }
  .kind-assault .mark {
    fill: #b3261e;
    fill-opacity: 0.3;
    stroke: #7a1410;
    stroke-width: 2;
    stroke-dasharray: none;
  }
</style>
