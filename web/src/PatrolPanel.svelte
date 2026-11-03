<script lang="ts">
  // Phase L (16.x): the side to act's units that may patrol, as the
  // engine offers them. Tick sources and set each one's points, click a
  // marked hex for the target, then send; the engine judges the patrol
  // (16.13's limits on points pooled from several units included).
  // Pass, in the phase controls, ends the phase.
  import { label } from "./counters";
  import type { HexId, Order, PatrolSource, PieceId, View } from "./types";

  let {
    view,
    chosen = $bindable({}),
    target = null,
    onsend,
  }: {
    view: View;
    /** The ticked sources and the points each sends. */
    chosen?: Record<PieceId, number>;
    /** The hex clicked on the map, if one of the ticked sources' targets. */
    target?: HexId | null;
    onsend: (o: Order) => void;
  } = $props();

  const options: PatrolSource[] = $derived(view.patrol_options ?? []);
  const sources = $derived(Object.entries(chosen).filter(([, n]) => n > 0) as [PieceId, number][]);

  function tick(o: PatrolSource, on: boolean) {
    const next = { ...chosen };
    if (on) next[o.piece] = Math.min(next[o.piece] ?? o.points, o.points);
    else delete next[o.piece];
    chosen = next;
  }

  function setPoints(o: PatrolSource, n: number) {
    if (o.piece in chosen) chosen = { ...chosen, [o.piece]: n };
  }
</script>

{#if options.length > 0}
  <section class="panel" data-testid="patrol-panel">
    <h2>Patrols · phase L (16.x)</h2>
    <p>Tick units and their points (at most 2 leave a hex, 3 go against a target), then click a marked hex. Pass ends the phase.</p>
    <ul>
      {#each options as o (o.piece)}
        <li data-unit={o.piece}>
          <label>
            <input type="checkbox" checked={o.piece in chosen} onchange={(e) => tick(o, (e.target as HTMLInputElement).checked)} aria-label="Patrol with {label(view.pieces[o.piece]).short}" />
            {label(view.pieces[o.piece]).short} at {o.hex}
          </label>
          <input type="number" min="1" max={o.points} value={chosen[o.piece] ?? o.points} disabled={!(o.piece in chosen)} oninput={(e) => setPoints(o, Number((e.target as HTMLInputElement).value))} aria-label="Patrol points from {o.piece}" data-points />
          of {o.points}
        </li>
      {/each}
    </ul>
    <p data-testid="patrol-target">{target ? `Target: ${target}` : sources.length > 0 ? "Click a marked hex for the target." : "Tick a unit to see the hexes it may patrol."}</p>
    <button type="button" data-order="patrol" disabled={sources.length === 0 || target === null} onclick={() => target && onsend({ patrol: { sources, target } })}>Send patrol</button>
  </section>
{/if}

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #e8f0e4;
    border: 1px solid #9fb894;
    border-radius: 4px;
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  p {
    margin: 0 0 0.3rem;
  }
  ul {
    list-style: none;
    margin: 0 0 0.3rem;
    padding: 0;
  }
  li input[type="number"] {
    width: 3rem;
    font: inherit;
  }
  button {
    font: inherit;
    padding: 0.2rem 0.5rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
