<script lang="ts">
  // Phase E (30.1x): the Commonwealth's warships. Pick a ship the engine
  // offers, then click a marked sea or coastal hex to place it at sea
  // this OpStage; Pass, in the phase controls, keeps the rest in port and
  // ends the phase. The engine judges every station.
  import { shipName, shipsAtSea, shipsInPort } from "./counters";
  import type { PieceId, View } from "./types";

  let {
    view,
    picked = $bindable(null),
  }: {
    view: View;
    /** The ship picked to station; the app sends the order on a hex click. */
    picked?: PieceId | null;
  } = $props();

  const offered = $derived(new Set(view.station_options?.ships ?? []));
  const inPort = $derived(shipsInPort(view));
  const atSea = $derived([...shipsAtSea(view)].flatMap(([hex, ps]) => ps.map((p) => ({ p, hex }))));
  const record = (id: PieceId) => view.fleet?.[id] ?? {};
  function why(id: PieceId): string {
    const r = record(id);
    if (r.fired) return "fired: back to Alexandria";
    if (r.owed) return `${r.owed} more OpStage${r.owed === 1 ? "" : "s"} in port`;
    if ((r.at_sea ?? 0) >= 3) return "three OpStages out";
    return "";
  }
</script>

{#if view.station_options}
  <section class="panel" data-testid="fleet-panel">
    <h2>Mediterranean Fleet · phase E (30.1x)</h2>
    <p>{picked ? `Click a marked hex to place ${shipName(picked)} at sea.` : "Pick a ship, then click a marked sea or coastal hex. Pass keeps the rest in Alexandria."}</p>
    {#if atSea.length > 0}
      <h3>At sea</h3>
      <ul data-testid="ships-at-sea">
        {#each atSea as { p, hex } (p.id)}
          <li data-unit={p.id}>
            <label>
              <input type="radio" name="ship" checked={picked === p.id} disabled={!offered.has(p.id)} onchange={() => (picked = p.id)} aria-label="Station {shipName(p.id)}" />
              {shipName(p.id)} at {hex}{record(p.id).stationed ? " (placed this phase)" : ""}{why(p.id) ? ` · ${why(p.id)}` : ""}
            </label>
          </li>
        {/each}
      </ul>
    {/if}
    <h3>In Alexandria</h3>
    <ul data-testid="ships-in-port">
      {#each inPort as p (p.id)}
        <li data-unit={p.id}>
          <label>
            <input type="radio" name="ship" checked={picked === p.id} disabled={!offered.has(p.id)} onchange={() => (picked = p.id)} aria-label="Station {shipName(p.id)}" />
            {shipName(p.id)}{why(p.id) ? ` · ${why(p.id)}` : ""}
          </label>
        </li>
      {:else}
        <li>None.</li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #e4ebf2;
    border: 1px solid #8fa6bf;
    border-radius: 4px;
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  h3 {
    margin: 0.3rem 0 0.1rem;
    font-size: 13px;
  }
  p {
    margin: 0 0 0.3rem;
  }
  ul {
    list-style: none;
    margin: 0 0 0.3rem;
    padding: 0;
  }
</style>
