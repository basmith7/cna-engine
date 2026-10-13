<script lang="ts">
  // The set-up: who is placing, what the picked piece's placement
  // allows (in words; the hexes are the engine's `placements`) and the
  // constraint prose the owner must honour (spec, *Placements*).
  import { sideName } from "./clock";
  import { label, nameParts } from "./counters";
  import { placementWords } from "./setup";
  import type { HexId, Piece, Side, View } from "./types";

  let {
    view,
    picked,
    hexes,
  }: { view: View; picked: Piece | null; hexes: HexId[] } = $props();

  const side: Side | null = $derived(view.pending && "set-up" in view.pending ? view.pending["set-up"].side : null);
  const left = $derived(Object.values(view.pieces).filter((p) => p.side === side && typeof p.location === "object" && "pending" in p.location).length);
  const placement = $derived(picked && typeof picked.location === "object" && "pending" in picked.location ? picked.location.pending : null);
</script>

{#if side}
  <section class="setup" data-testid="setup">
    <h2>Set-up · {sideName(side)} <span class="n">{left} to place</span></h2>
    {#if picked && placement}
      {@const name = nameParts(label(picked).long)}
      <p class="picked" data-unit={picked.id}>
        <strong>{name.text}{#if name.sup}<sup>{name.sup}</sup>{/if}</strong>
        <span class="where">{placementWords(placement)}</span>
        {#if placement.constraint}
          <span class="also">also: {placement.constraint}</span>
        {/if}
        <span class="count" data-testid="placement-count">{hexes.length} hexes allowed. Click one to place it.</span>
      </p>
    {:else}
      <p class="hint">Click a piece in the tray, then a highlighted hex.</p>
    {/if}
  </section>
{/if}

<style>
  .setup {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #fff4d6;
    border: 1px solid #e9c46a;
    border-radius: 4px;
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  .n {
    font-weight: 400;
    color: #666;
    margin-left: 0.3rem;
  }
  p {
    margin: 0;
  }
  .picked span {
    display: block;
  }
  .where {
    color: #333;
  }
  .also {
    color: #7a2e1f;
    font-style: italic;
  }
  .count {
    color: #666;
    font-size: 12px;
    margin-top: 0.2rem;
  }
  .hint {
    color: #555;
  }
</style>
