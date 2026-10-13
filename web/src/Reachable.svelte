<svelte:options namespace="svg" />

<script lang="ts">
  // The reachable overlay (spec, *Legal moves*, *Moving*): the hexes the
  // engine says the ticked stack can end in, shaded, each with its CP
  // in the hex; a DP outline where the move earns DP, a heavy one where
  // it ends in an enemy ZOC, a dotted one where the engine may stop the
  // stack short. Hovering a hex draws the engine's path to it; a custom
  // path (shift-clicks) is drawn in its own colour. Two parts, so the
  // shading sits under the counters and the labels over them.
  import { COUNTER } from "./counters";
  import { centre, polygonPoints, SIZE } from "./geom";
  import { byHex, cpLabel } from "./moves";
  import type { HexId, Reach } from "./types";

  let {
    reaches,
    from,
    hovered = null,
    custom = [],
    occupied = new Set<HexId>(),
    part,
  }: {
    reaches: Reach[];
    from: HexId | null;
    hovered?: HexId | null;
    custom?: HexId[];
    /** Hexes holding a stack: their label goes on the counter's corner, not under it. */
    occupied?: Set<HexId>;
    part: "fill" | "labels";
  } = $props();

  const index = $derived(byHex(reaches));
  /** Plain hexes first, so a flagged hex's outline is not overdrawn by a plain neighbour's stroke. */
  const flagged = (r: Reach) => r.dp > 0 || r.ends_in_zoc || r.may_be_interrupted || r.short > 0;
  const ordered = $derived([...reaches].sort((a, b) => Number(flagged(a)) - Number(flagged(b))));
  const hoverPath = $derived(hovered ? (index.get(hovered)?.path ?? null) : null);

  function line(path: HexId[]): string {
    if (!from) return "";
    return [from, ...path].map((h) => centre(h).join(",")).join(" ");
  }
</script>

{#if part === "fill"}
  <g class="reachable" data-testid="reachable" data-count={reaches.length}>
    {#each ordered as r (r.hex)}
      <polygon
        class="reach"
        class:dp={r.dp > 0}
        class:zoc={r.ends_in_zoc}
        class:maybe={r.may_be_interrupted}
        class:short={r.short > 0}
        data-hex={r.hex}
        data-cp={r.cp}
        data-dp={r.dp}
        data-short={r.short}
        points={polygonPoints(r.hex)}
      />
    {/each}
  </g>
{:else}
  <g class="reach-labels">
    {#each reaches as r (r.hex)}
      {@const [cx, cy] = centre(r.hex)}
      {@const text = `${cpLabel(r.cp)}${r.dp > 0 ? ` +${r.dp}` : ""}${r.short > 0 ? ` −${r.short}h` : ""}`}
      {#if occupied.has(r.hex)}
        <!-- A pill on the top-right corner of the counter, the count badge's opposite. -->
        <g class="pill" class:dp={r.dp > 0} transform="translate({cx + COUNTER * 0.5} {cy - COUNTER * 0.5})">
          <rect x={-text.length * 2 - 2} y={-4} width={text.length * 4 + 4} height={8} rx={4} />
          <text class="cp" y={2.3} data-hex={r.hex}>{text}</text>
        </g>
      {:else}
        <text class="cp" class:dp={r.dp > 0} x={cx} y={cy + SIZE * 0.3} data-hex={r.hex}>{text}</text>
      {/if}
    {/each}
    {#if hoverPath}
      <polyline class="path hover" data-testid="hover-path" points={line(hoverPath)} />
    {/if}
    {#if custom.length > 0}
      <polyline class="path custom" data-testid="custom-path" points={line(custom)} />
      {#each custom as h, i (i)}
        {@const [cx, cy] = centre(h)}
        <circle class="custom-step" {cx} {cy} r={SIZE * 0.18} />
      {/each}
    {/if}
  </g>
{/if}

<style>
  .reach {
    fill: #2a9d8f;
    fill-opacity: 0.32;
    stroke: #1b6f66;
    stroke-width: 0.6;
    pointer-events: none;
  }
  .reach.dp {
    stroke: #c0392b;
    stroke-width: 1.4;
    stroke-dasharray: 4 2;
  }
  .reach.zoc {
    fill: #e07b00;
    stroke: #7a3e00;
    stroke-width: 2;
  }
  .reach.maybe {
    stroke-dasharray: 1.5 1.5;
    stroke-width: 1.4;
  }
  .reach.short {
    fill: #ffb703;
    stroke: #c77700;
    stroke-width: 1.4;
    stroke-dasharray: 4 2;
  }
  .cp {
    font: 700 6.5px system-ui, sans-serif;
    fill: #0b3d38;
    text-anchor: middle;
    paint-order: stroke;
    stroke: #fff;
    stroke-width: 1.6;
    stroke-linejoin: round;
    pointer-events: none;
  }
  .cp.dp {
    fill: #8b1e12;
  }
  /* Light, so it reads as a label and not as a second count badge. */
  .pill rect {
    fill: #fff;
    stroke: #0b3d38;
    stroke-width: 0.7;
  }
  .pill.dp rect {
    stroke: #8b1e12;
  }
  .pill .cp {
    stroke: none;
    font-size: 5.5px;
  }
  /* Far out the hex numbers go; so do these. */
  :global(.map.far) .reach-labels {
    display: none;
  }
  .path {
    fill: none;
    stroke-width: 2.2;
    stroke-linejoin: round;
    stroke-linecap: round;
    pointer-events: none;
  }
  .path.hover {
    stroke: #0b3d38;
    stroke-opacity: 0.85;
  }
  .path.custom {
    stroke: #6a1b9a;
    stroke-dasharray: 5 3;
  }
  .custom-step {
    fill: #6a1b9a;
    stroke: #fff;
    stroke-width: 0.8;
    pointer-events: none;
  }
</style>
