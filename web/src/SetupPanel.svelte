<script lang="ts">
  // The set-up: who is placing, what the picked piece's placement
  // allows (in words; the hexes are the engine's `placements`) and the
  // constraint prose the owner must honour (spec, *Placements*).
  import { sideName } from "./clock";
  import { label, nameParts } from "./counters";
  import { placementWords } from "./setup";
  import type { WorkItem, HexId, Piece, Side, View } from "./types";

  let {
    view,
    picked,
    hexes,
    workItem = null,
    onwork = () => {},
    onpass = () => {},
    motorQuotas = [],
    motorArmed = false,
    motorPoints = $bindable(30),
    onmotor = () => {},
  }: {
    view: View;
    picked: Piece | null;
    hexes: HexId[];
    workItem?: WorkItem | null;
    onwork?: (item: WorkItem) => void;
    onpass?: () => void;
    motorQuotas?: { hexes: HexId[]; points: number }[];
    motorArmed?: boolean;
    motorPoints?: number;
    onmotor?: () => void;
  } = $props();
  const motorLeft = $derived(motorQuotas.reduce((n, q) => n + q.points, 0));

  // Set-up works the side places (59.12): forts, real and dummy fields.
  const WORKS: [WorkItem, string][] = [
    ["fortification", "Fortification"],
    ["real-minefield", "Real minefield"],
    ["dummy-minefield", "Dummy minefield"],
  ];

  const side: Side | null = $derived(view.pending && "set-up" in view.pending ? view.pending["set-up"].side : null);
  const left = $derived(Object.values(view.pieces).filter((p) => p.side === side && typeof p.location === "object" && "pending" in p.location).length);
  const placement = $derived(picked && typeof picked.location === "object" && "pending" in picked.location ? picked.location.pending : null);
</script>

{#if side}
  <div class="pin">
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
      {:else if !view.work_options && left > 0}
        <p class="hint">Click a piece in the tray, then a highlighted hex.</p>
      {/if}
      {#if motorLeft > 0}
        <div class="works" data-testid="set-up-motorisation">
          <span class="where">Unattached motorisation: {motorLeft} points to place (61.72)</span><br />
          <label>Points per click <input type="number" min="1" max={motorLeft} bind:value={motorPoints} data-testid="motorisation-points" /></label>
          <button type="button" data-testid="place-motorisation" class:on={motorArmed} onclick={onmotor}>Place motorisation</button>
          {#if motorArmed}
            <span class="count">{new Set(motorQuotas.flatMap((q) => q.hexes)).size} hexes allowed. Click one to place them.</span>
          {/if}
        </div>
      {/if}
      {#if view.work_options}
        {@const w = view.work_options}
        <div class="works" data-testid="set-up-works">
          <span class="where">Set-up works: {w.required_left} required left</span>
          {#each WORKS as [item, name] (item)}
            {#if (w[item] ?? []).length > 0}
              <button type="button" data-work={item} class:on={workItem === item} onclick={() => onwork(item)}>{name}</button>
            {/if}
          {/each}
          {#if w.may_pass}
            <button type="button" data-testid="works-done" onclick={onpass}>Done</button>
          {/if}
          {#if workItem}
            <span class="count">{(w[workItem] ?? []).length} hexes allowed. Click one to place it.</span>
          {/if}
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  /* Stays at the top of the side column while the trays scroll under it;
     the gap below is opaque so nothing shows through it. */
  .pin {
    position: sticky;
    top: 0;
    z-index: 1;
    padding-bottom: 0.6rem;
    background: #f4f0e6;
  }
  .setup {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
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
  .works button {
    margin: 0.2rem 0.3rem 0 0;
  }
  .works input {
    width: 4.5em;
    margin-left: 0.3rem;
  }
  .works button.on {
    font-weight: 700;
    outline: 2px solid #7b4fd1;
  }
  .hint {
    color: #555;
  }
</style>
