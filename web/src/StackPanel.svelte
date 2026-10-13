<script lang="ts">
  // Read-only this slice: what is in the clicked hex (spec, *Moving*
  // adds the ticks and orders in slice 4).
  import { glyphFor, label, markers, stacks } from "./counters";
  import { hexInfo } from "./map";
  import type { View } from "./types";

  let { view, hex }: { view: View; hex: string | null } = $props();

  const info = $derived(hex ? hexInfo(hex) : null);
  const pieces = $derived(hex ? (stacks(view).get(hex) ?? []) : []);
  const sideName = (s: string) => (s === "axis" ? "Axis" : "Commonwealth");
</script>

<section class="panel" data-testid="stack-panel">
  {#if hex && info}
    <h2>
      <span class="hex">{hex}</span>
      <span class="terrain">{info.place ? `${info.place} · ` : ""}{info.terrain}{info.coastal ? ", coastal" : ""}</span>
    </h2>
    {#if pieces.length === 0}
      <p class="empty">Nothing here.</p>
    {:else}
      <ol>
        {#each pieces as p (p.id)}
          {@const text = label(p)}
          <li class="piece side-{p.side}" data-unit={p.id}>
            <span class="abbr">{text.short}</span>
            <span class="name">{text.long}</span>
            <span class="facts">
              {sideName(p.side)} · {p.unit_type ?? glyphFor(p.kind, p.unit_type)}
              {#if p.kind !== "supply-unit" && p.kind !== "dummy-supply-unit"}
                · TOE {p.toe} · CPA {p.cpa ?? "–"} · morale {p.morale ?? "–"} · cohesion {p.cohesion.level}
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
    grid-template-columns: auto 1fr;
    gap: 0 0.5rem;
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
    grid-column: 1 / -1;
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
