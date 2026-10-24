<script lang="ts">
  // What is in the clicked hex (spec, *Moving*): the side to act's own
  // pieces carry a tick, and the ticked ones are the stack an order
  // moves. The app owns the ticks; this only shows and toggles them.
  import { glyphFor, label, markers, nameParts, stacks } from "./counters";
  import { hexInfo } from "./map";
  import { tickable } from "./moves";
  import { loadLabel, motorisationAt } from "./supply";
  import type { PieceId, View } from "./types";

  let {
    view,
    hex,
    ticked = [],
    ontoggle,
  }: { view: View; hex: string | null; ticked?: PieceId[]; ontoggle?: (id: PieceId) => void } = $props();

  const info = $derived(hex ? hexInfo(hex) : null);
  const pieces = $derived(hex ? (stacks(view).get(hex) ?? []) : []);
  const points = $derived(hex ? motorisationAt(view, hex) : null);
  const sideName = (s: string) => (s === "axis" ? "Axis" : "Commonwealth");
</script>

<section class="panel" data-testid="stack-panel">
  {#if hex && info}
    <h2>
      <span class="hex">{hex}</span>
      <span class="terrain">{info.place ? `${info.place} · ` : ""}{info.terrain}{info.coastal ? ", coastal" : ""}</span>
    </h2>
    {#if points !== null}
      <p class="motorisation" data-testid="motorisation">{points} motorisation point{points === 1 ? "" : "s"} here</p>
    {/if}
    {#if pieces.length === 0}
      <p class="empty">Nothing here.</p>
    {:else}
      <ol>
        {#each pieces as p (p.id)}
          {@const text = label(p)}
          {@const name = nameParts(text.long)}
          <li class="piece side-{p.side}" class:ticked={ticked.includes(p.id)} data-unit={p.id}>
            {#if tickable(p, view.to_act)}
              <input type="checkbox" class="tick" checked={ticked.includes(p.id)} aria-label="Move {text.long}" onchange={() => ontoggle?.(p.id)} />
            {:else}
              <span class="tick"></span>
            {/if}
            <span class="abbr">{text.short}</span>
            <span class="name">{name.text}{#if name.sup}<sup title="Engineering capability">{name.sup}</sup>{/if}</span>
            <span class="facts">
              {sideName(p.side)} · {p.unit_type ?? glyphFor(p.kind, p.unit_type)}
              {#if p.kind !== "supply-unit" && p.kind !== "dummy-supply-unit"}
                · TOE {p.toe} · CPA {p.cpa ?? "–"} · morale {p.morale ?? "–"} · cohesion {p.cohesion.level}
              {:else if loadLabel(p)}
                · <span data-testid="load">{loadLabel(p)}</span>
              {/if}
              {#each markers(p, view) as m (m)}
                <span class="marker {m}">{m.replace("-", " ")}</span>
              {/each}
            </span>
          </li>
        {/each}
      </ol>
    {/if}
  {:else}
    <p class="empty">Click a hex to see what is in it.</p>
  {/if}
</section>

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
  }
  h2 {
    margin: 0 0 0.4rem;
    font-size: 15px;
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
  }
  .hex {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
  }
  .terrain {
    color: #555;
    font-weight: 400;
  }
  .motorisation {
    margin: 0 0 0.3rem;
    font-size: 12px;
    color: #555;
  }
  .empty {
    margin: 0;
    color: #666;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .piece {
    padding: 0.3rem 0.4rem;
    border-left: 4px solid;
    margin-bottom: 0.25rem;
    background: #fbf8f0;
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 0 0.5rem;
  }
  .piece.ticked {
    background: #e6f4f1;
  }
  .tick {
    margin: 0;
    width: 15px;
    align-self: center;
  }
  .piece.side-axis { border-color: #7a2e1f; }
  .piece.side-cw { border-color: #1f3a5f; }
  .abbr {
    font-weight: 700;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .facts {
    grid-column: 2 / -1;
    color: #555;
    font-size: 12px;
  }
  .marker {
    display: inline-block;
    margin-left: 0.3rem;
    padding: 0 0.3rem;
    border-radius: 3px;
    color: #fff;
    background: #555;
  }
  .marker.reserve-I, .marker.reserve-II { background: #b8860b; }
  .marker.contact { background: #e07b00; }
  .marker.engaged { background: #b3261e; }
  .marker.forward { background: #2e7d32; }
</style>
