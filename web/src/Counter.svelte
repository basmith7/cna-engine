<svelte:options namespace="svg" />

<script lang="ts">
  // One counter, drawn from the piece (spec, *Counters*): a band in the
  // nation's colour, a NATO frame with a branch glyph, the size mark,
  // the abbreviation and ID code, TOE and CPA, and the small markers.
  // Drawn in a COUNTER x COUNTER box with its origin at the top-left.
  import { COUNTER, glyphFor, label, sizeMark, type Marker } from "./counters";
  import type { Piece } from "./types";

  let { piece, markers = [], muted = false }: { piece: Piece; markers?: Marker[]; muted?: boolean } = $props();

  const S = COUNTER;
  const glyph = $derived(glyphFor(piece.kind, piece.unit_type));
  const text = $derived(label(piece));
  const size = $derived(sizeMark(piece.equivalent));
  const isSupply = $derived(glyph === "supply");
  const nation = $derived(piece.nation ?? piece.side);

  // The frame: a NATO rectangle in the middle of the counter.
  const fw = S * 0.5;
  const fh = S * 0.32;
  const fx = (S - fw) / 2;
  const fy = S * 0.36;
  const cx = S / 2;
  const cy = fy + fh / 2;
  const toe = $derived(isSupply ? "" : String(piece.toe));
  const cpa = $derived(piece.cpa ?? "");
</script>

<g class="counter side-{piece.side} nation-{nation}" class:muted>
  <rect class="body" width={S} height={S} rx={S * 0.06} />
  <rect class="band" width={S} height={S * 0.24} rx={S * 0.06} />
  <text class="abbr" x={S * 0.07} y={S * 0.17}>{text.short}</text>
  {#if piece.id_code}<text class="code" x={S * 0.93} y={S * 0.17}>{piece.id_code}</text>{/if}
  {#if size}<text class="size" x={cx} y={fy - S * 0.06}>{size}</text>{/if}
  <rect class="frame" x={fx} y={fy} width={fw} height={fh} />
  {#if glyph === "infantry"}
    <path class="glyph" d="M {fx} {fy} L {fx + fw} {fy + fh} M {fx + fw} {fy} L {fx} {fy + fh}" />
  {:else if glyph === "armour"}
    <ellipse class="glyph" {cx} {cy} rx={fw * 0.32} ry={fh * 0.32} />
  {:else if glyph === "artillery"}
    <circle class="glyph filled" {cx} {cy} r={fh * 0.22} />
  {:else if glyph === "recce"}
    <path class="glyph" d="M {fx} {fy + fh} L {fx + fw} {fy}" />
  {:else if glyph === "engineer"}
    <path class="glyph" d="M {fx + fw * 0.2} {fy + fh * 0.7} V {fy + fh * 0.3} H {fx + fw * 0.8} V {fy + fh * 0.7} M {cx} {fy + fh * 0.3} V {fy + fh * 0.7}" />
  {:else if glyph === "supply"}
    <path class="glyph" d="M {fx + fw * 0.2} {fy + fh * 0.3} H {fx + fw * 0.8} M {fx + fw * 0.2} {fy + fh * 0.5} H {fx + fw * 0.8} M {fx + fw * 0.2} {fy + fh * 0.7} H {fx + fw * 0.8}" />
  {:else if glyph === "hq" || glyph === "anti-tank" || glyph === "anti-air" || glyph === "commander"}
    <text class="glyph-text" x={cx} y={cy + fh * 0.17}>{glyph === "hq" ? "HQ" : glyph === "anti-tank" ? "AT" : glyph === "anti-air" ? "AA" : "C"}</text>
  {:else}
    <text class="glyph-text" x={cx} y={cy + fh * 0.17}>?</text>
  {/if}
  <text class="toe" x={S * 0.07} y={S * 0.93}>{toe}</text>
  <text class="cpa" x={S * 0.93} y={S * 0.93}>{cpa}</text>
  {#each markers as m, i (m)}
    <g class="marker {m}" transform="translate({S * 0.96} {S * 0.3 + i * S * 0.2})">
      <circle r={S * 0.09} />
      <text y={S * 0.035}>{m === "reserve-I" ? "I" : m === "reserve-II" ? "II" : m === "contact" ? "C" : m === "engaged" ? "E" : m === "pinned" ? "P" : "F"}</text>
    </g>
  {/each}
</g>

<style>
  .counter text {
    font-family: system-ui, sans-serif;
    pointer-events: none;
    fill: #1b1b1b;
  }
  .body {
    fill: #f6f0e2;
    stroke-width: 0.8;
  }
  .side-axis .body { stroke: #7a2e1f; }
  .side-cw .body { stroke: #1f3a5f; }
  /* Nation bands: Axis warm, Commonwealth cool. */
  .nation-it .band { fill: #d8b06e; }
  .nation-de .band { fill: #b4a27a; }
  .nation-axis .band { fill: #d8b06e; }
  .nation-cw .band { fill: #86a9cc; }
  .abbr { font-size: 4.2px; font-weight: 600; }
  .code { font-size: 3.6px; text-anchor: end; }
  .size { font-size: 3.4px; text-anchor: middle; font-weight: 600; }
  .frame { fill: #fff; stroke-width: 0.7; }
  .side-axis .frame, .side-axis .glyph { stroke: #7a2e1f; }
  .side-cw .frame, .side-cw .glyph { stroke: #1f3a5f; }
  .glyph { fill: none; stroke-width: 0.8; }
  .glyph.filled { fill: currentColor; stroke: none; }
  .side-axis .glyph.filled { fill: #7a2e1f; }
  .side-cw .glyph.filled { fill: #1f3a5f; }
  .glyph-text { font-size: 4px; font-weight: 700; text-anchor: middle; }
  .toe { font-size: 4px; }
  .cpa { font-size: 4px; text-anchor: end; }
  .marker circle { stroke: #fff; stroke-width: 0.5; }
  .marker text { font-size: 3px; font-weight: 700; text-anchor: middle; fill: #fff; }
  .marker.reserve-I circle, .marker.reserve-II circle { fill: #b8860b; }
  .marker.contact circle { fill: #e07b00; }
  .marker.engaged circle { fill: #b3261e; }
  .marker.pinned circle { fill: #555; }
  .marker.forward circle { fill: #2e7d32; }
  .muted { opacity: 0.55; }
</style>
