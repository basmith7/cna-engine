<script lang="ts">
  // The pieces not on a hex (spec, *Off-map pieces*): each side's "to
  // place" tray, the six off-map boxes and the units in transit, each
  // piece as a small counter. The tray of the side setting up is
  // clickable: a click picks the piece to place (spec, *Placements*).
  import Counter from "./Counter.svelte";
  import { BOX_NAMES, COUNTER, label, offMap } from "./counters";
  import type { Piece, PieceId, Side, View } from "./types";

  let {
    view,
    picked = null,
    onpick,
  }: { view: View; picked?: PieceId | null; onpick?: (id: PieceId) => void } = $props();

  /** The side whose tray may be clicked: the one the engine is waiting on to set up. */
  const placing: Side | null = $derived(view.pending && "set-up" in view.pending ? view.pending["set-up"].side : null);

  const off = $derived(offMap(view));
  const transitLine = (p: Piece) => {
    const t = (p.location as { transit: { from: string; to: string; done: number } }).transit;
    return `${t.from} → ${t.to}, ${t.done} done`;
  };
</script>

{#snippet counters(pieces: Piece[], where: string)}
  <div class="counters" data-where={where} data-count={pieces.length}>
    {#each pieces as p (p.id)}
      <svg class="mini" viewBox="0 0 {COUNTER} {COUNTER}" data-unit={p.id} role="img" aria-label={label(p).long}>
        <title>{label(p).long}</title>
        <Counter piece={p} />
      </svg>
    {/each}
  </div>
{/snippet}

{#snippet tray(pieces: Piece[], side: Side)}
  <div class="counters" data-where="tray-{side}" data-count={pieces.length}>
    {#each pieces as p (p.id)}
      <button
        type="button"
        class="pick"
        class:picked={p.id === picked}
        data-unit={p.id}
        disabled={side !== placing}
        aria-pressed={p.id === picked}
        title={label(p).long}
        onclick={() => onpick?.(p.id)}
      >
        <svg class="mini" viewBox="0 0 {COUNTER} {COUNTER}" role="img" aria-label={label(p).long}>
          <Counter piece={p} />
        </svg>
      </button>
    {/each}
  </div>
{/snippet}

<section class="offmap" data-testid="off-map">
  {#each ["axis", "cw"] as const as side (side)}
    {#if off.tray[side].length > 0}
      <h3 data-testid="tray-{side}">To place · {side === "axis" ? "Axis" : "Commonwealth"} <span class="n">{off.tray[side].length}</span></h3>
      {@render tray(off.tray[side], side)}
    {/if}
  {/each}
  {#if off.transit.length > 0}
    <h3>In transit <span class="n">{off.transit.length}</span></h3>
    <ul class="transit">
      {#each off.transit as p (p.id)}
        <li data-unit={p.id}>{label(p).short} · {transitLine(p)}</li>
      {/each}
    </ul>
  {/if}
  <h3>Off-map boxes</h3>
  {#each off.boxes as { box, pieces } (box)}
    <h4>{BOX_NAMES[box]} <span class="n">{pieces.length}</span></h4>
    {#if pieces.length > 0}
      {@render counters(pieces, `box-${box}`)}
    {/if}
  {/each}
</section>

<style>
  .offmap {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
  }
  h3 {
    margin: 0.8rem 0 0.3rem;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #333;
  }
  h4 {
    margin: 0.4rem 0 0.2rem;
    font-size: 13px;
    font-weight: 600;
    color: #444;
  }
  .n {
    font-weight: 400;
    color: #777;
    margin-left: 0.3rem;
  }
  .counters {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }
  .mini {
    width: 44px;
    height: 44px;
    display: block;
  }
  .pick {
    padding: 0;
    margin: 0;
    border: 2px solid transparent;
    border-radius: 4px;
    background: none;
    cursor: pointer;
    line-height: 0;
  }
  .pick:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .pick:not(:disabled):hover {
    border-color: #e9c46a;
  }
  .pick.picked {
    border-color: #ffb703;
    background: #fff4d6;
  }
  .transit {
    margin: 0;
    padding-left: 1.2rem;
  }
</style>
