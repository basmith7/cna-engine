<script lang="ts">
  // A Commonwealth withdrawal that takes units of a kind the player
  // picks (20.82, R-154), as the engine offers it: tick the units to
  // send and pick them. The engine judges the pick; one not made is
  // filled at the check by the engine, and the ticks start at the units
  // it would take. Each picked unit must stand in Cairo or Alexandria
  // at the check with 75 % of its TOE, or it is eliminated (R-156).
  import { label } from "./counters";
  import type { Location, Order, PieceId, View, WithdrawalPick } from "./types";

  let {
    view,
    onsend,
  }: {
    view: View;
    onsend: (o: Order) => Promise<boolean>;
  } = $props();

  const options: WithdrawalPick[] = $derived(view.withdrawal_pick_options ?? []);
  // The ticks changed since the last pick, by slot kind.
  let ticks: Record<string, PieceId[]> = $state({});
  const ticked = (o: WithdrawalPick) => ticks[o.fills] ?? o.picked ?? o.candidates.slice(0, o.count);
  const kind = (fills: string) => fills.replaceAll("-", " ");
  const name = (id: PieceId) => {
    const p = view.pieces[id] ?? view.withdrawn?.[id];
    return p ? label(p).short : id;
  };
  function where(l: Location | undefined): string {
    if (typeof l !== "object") return "";
    if ("hex" in l) return `at ${l.hex}`;
    if ("box" in l) return `in the ${l.box} box`;
    return "";
  }

  function tick(o: WithdrawalPick, id: PieceId, on: boolean) {
    const now = ticked(o).filter((t) => t !== id);
    // Kept in the engine's order, so a pick reads as the list does.
    const next = on ? o.candidates.filter((c) => c === id || now.includes(c)) : now;
    ticks = { ...ticks, [o.fills]: next };
  }

  async function pick(o: WithdrawalPick) {
    if (await onsend({ "pick-withdrawal": { fills: o.fills, pieces: ticked(o) } })) {
      const { [o.fills]: _, ...rest } = ticks;
      ticks = rest;
    }
  }
</script>

{#each options as o (o.fills)}
  {@const mine = ticked(o)}
  <section class="panel" data-testid="withdrawal-panel" data-fills={o.fills}>
    <h2>Withdrawal · GT{view.clock.game_turn}/{view.clock.stage} (20.82)</h2>
    <p>
      This OpStage's withdrawal takes {o.count} {kind(o.fills)} units of your choice. At its end each must stand in Cairo or Alexandria with 75 % of its TOE, or it is eliminated.
    </p>
    <p data-testid="withdrawal-picked">
      {o.picked ? `Picked: ${o.picked.map(name).join(", ")}.` : `Not picked: the engine will take ${o.count} itself, as ticked now (Cairo and Alexandria first, then the strongest).`}
    </p>
    <ul>
      {#each o.candidates as id (id)}
        <li data-unit={id}>
          <label>
            <input type="checkbox" checked={mine.includes(id)} onchange={(e) => tick(o, id, (e.target as HTMLInputElement).checked)} aria-label="Withdraw {name(id)}" />
            {name(id)} {where(view.pieces[id]?.location)}{view.pieces[id]?.toe !== undefined ? `, TOE ${view.pieces[id].toe}` : ""}
          </label>
        </li>
      {/each}
    </ul>
    <button type="button" data-order="pick-withdrawal" onclick={() => pick(o)}>Pick for withdrawal ({mine.length} of {o.count} ticked)</button>
  </section>
{/each}

{#if view.voluntary_options && view.voluntary_options.returnable.length > 0}
  {@const v = view.voluntary_options}
  <section class="panel" data-testid="voluntary-panel">
    <h2>Withdrawn voluntarily (64.75)</h2>
    <p>Withdrawal points so far: {v.halves / 2}. Each whole Game-Turn away earns half a point, at most three a unit; bringing one back to Cairo costs two, and it may not go again for 26 Game-Turns.</p>
    <ul>
      {#each v.returnable as id (id)}
        <li data-unit={id}>
          {name(id)}
          <button type="button" data-order="return-withdrawn" onclick={() => onsend({ "return-withdrawn": { piece: id } })}>Bring back</button>
        </li>
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
    background: #f1ebde;
    border: 1px solid #c2b28f;
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
    max-height: 14rem;
    overflow-y: auto;
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
</style>
