<script lang="ts">
  // The phase controls (spec, *Phase controls*): the orders a phase
  // takes as buttons, shown by the clock alone and never pre-filtered
  // by the rules: an illegal click is the engine's refusal in a toast.
  // While something is pending its own panel holds the answers, Pass
  // included, so the controls stand down.
  import { sideName } from "./clock";
  import type { PieceId, Side, View } from "./types";

  let {
    view,
    ticked,
    onorder,
  }: {
    view: View;
    ticked: PieceId[];
    onorder: (o: "end-segment" | "pass" | { "declare-initiative": { player_a: Side } } | { "designate-reserve": { pieces: PieceId[] } } | { "release-reserve": { pieces: PieceId[] } }) => void;
  } = $props();

  const c = $derived(view.clock);
  const initiative = $derived(c.phase === "initiative-declaration");
  const designate = $derived(c.phase === "reserve-designation");
  const release = $derived(c.phase === "movement-combat" && c.segment === "reserve-release");
  const endSegment = $derived(c.phase === "movement-combat" && (c.segment === "movement" || c.segment === "close-assault" || c.segment === "reserve-release"));
  const n = $derived(ticked.length);
</script>

{#if view.pending === null && view.to_act}
  <div class="controls" data-testid="phase-controls">
    {#if initiative}
      {#each ["axis", "cw"] as const as side (side)}
        <button type="button" data-order="declare-initiative" data-side={side} onclick={() => onorder({ "declare-initiative": { player_a: side } })}>{sideName(side)} is Player A</button>
      {/each}
    {/if}
    {#if designate}
      <button type="button" data-order="designate-reserve" onclick={() => onorder({ "designate-reserve": { pieces: ticked } })}>Designate reserve ({n} ticked)</button>
    {/if}
    {#if release}
      <button type="button" data-order="release-reserve" onclick={() => onorder({ "release-reserve": { pieces: ticked } })}>Release reserve ({n} ticked)</button>
    {/if}
    {#if endSegment}
      <button type="button" data-order="end-segment" onclick={() => onorder("end-segment")}>End segment</button>
    {/if}
    <button type="button" data-order="pass" onclick={() => onorder("pass")}>Pass</button>
  </div>
{/if}

<style>
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.6rem;
  }
  button {
    font: 14px system-ui, sans-serif;
    padding: 0.25rem 0.6rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover {
    background: #34434f;
  }
  button[data-order="pass"] {
    background: #f2efe6;
    color: #1b1b1b;
    border-color: #b9b2a2;
  }
  button[data-order="pass"]:hover {
    background: #e4dfd2;
  }
</style>
