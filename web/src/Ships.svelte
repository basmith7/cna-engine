<svelte:options namespace="svg" />

<script lang="ts">
  // The Commonwealth's warships at sea (30.1x): a navy hull with the
  // ship's name, drawn inside the top half of its hex, clear of the hex
  // number at the bottom, so a land stack on the same coastal hex stays
  // readable. Ships in port are listed in the
  // phase E panel, not drawn.
  import { COUNTER, shipName, shipsAtSea } from "./counters";
  import { centre } from "./geom";
  import type { View } from "./types";

  let { view }: { view: View } = $props();
  const groups = $derived([...shipsAtSea(view)]);
  const S = COUNTER;
</script>

<g class="ships" data-testid="ships" data-count={groups.reduce((n, [, ps]) => n + ps.length, 0)}>
  {#each groups as [hex, ships] (hex)}
    {@const [cx, cy] = centre(hex)}
    <g class="at" data-hex={hex} transform="translate({cx} {cy - S * 0.3})">
      {#each ships as p, i (p.id)}
        <g class="ship" class:fired={view.fleet?.[p.id]?.fired} data-unit={p.id} data-ship={p.id} data-hex={hex} transform="translate(0 {-i * S * 0.32})">
          <path d="M {-S * 0.45} {-S * 0.12} L {S * 0.45} {-S * 0.12} L {S * 0.32} {S * 0.14} L {-S * 0.36} {S * 0.14} Z" />
          <text y={S * 0.07}>{shipName(p.id)}</text>
        </g>
      {/each}
    </g>
  {/each}
</g>

<style>
  .ship {
    cursor: pointer;
  }
  .ship path {
    fill: #1f3a5f;
    stroke: #f2efe6;
    stroke-width: 0.6;
  }
  .ship.fired path {
    fill: #5d6b7a;
  }
  .ship text {
    font: 700 4.2px system-ui, sans-serif;
    fill: #f2efe6;
    text-anchor: middle;
    pointer-events: none;
  }
</style>
