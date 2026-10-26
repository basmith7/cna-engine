<script lang="ts">
  // What is in the clicked hex (spec, *Moving*): the side to act's own
  // pieces carry a tick, and the ticked ones are the stack an order
  // moves. The app owns the ticks; this only shows and toggles them.
  import { glyphFor, label, markers, nameParts, stacks } from "./counters";
  import { hexInfo } from "./map";
  import { attachmentOrder, tickable } from "./moves";
  import { itemName, itemStages } from "./construction";
  import { escapeesAt, guardPhase, guardsAt, prisonersAt } from "./prisoners";
  import { repairedAt } from "./replacements";
  import { loadLabel, motorisationAt } from "./supply";
  import type { Order, PieceId, View } from "./types";

  let {
    view,
    hex,
    ticked = [],
    ontoggle,
    onorder,
  }: { view: View; hex: string | null; ticked?: PieceId[]; ontoggle?: (id: PieceId) => void; onorder?: (o: Order) => void } = $props();

  const info = $derived(hex ? hexInfo(hex) : null);
  const pieces = $derived(hex ? (stacks(view).get(hex) ?? []) : []);
  const points = $derived(hex ? motorisationAt(view, hex) : null);
  const wrecks = $derived(hex ? (view.broken_down?.[hex] ?? []) : []);
  const prisoners = $derived(hex ? prisonersAt(view, hex) : []);
  const repaired = $derived(hex ? repairedAt(view, hex) : []);
  const guards = $derived(hex ? guardsAt(view, hex) : []);
  const project = $derived(hex ? view.projects?.[hex] : undefined);
  const minefield = $derived(hex ? view.minefields?.[hex] : undefined);
  /** The side to act's ticked pieces here, the builders a Fortify order names (24.41). */
  const builders = $derived(ticked.filter((id) => pieces.some((p) => p.id === id && p.side === view.to_act)));
  const escapees = $derived(hex ? escapeesAt(view, hex) : []);
  /** The side to act's combat units in the hex, which detail guards or take them back (28.21, 28.22). */
  const guarders = $derived(pieces.filter((p) => p.side === view.to_act && p.kind === "combat"));
  let guarding: { piece: PieceId | null; points: number } = $state({ piece: null, points: 1 });
  /** The side to act's units in the hex, which may absorb repaired points in phase C (20.41). */
  const absorbers = $derived(pieces.filter((p) => p.side === view.to_act && (p.kind === "combat" || p.kind === "hq")));
  let absorbing: Record<string, { piece: PieceId | null; points: number }> = $state({});
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
    {#each wrecks as w, i (i)}
      <p class="wreck side-{w.side}" data-testid="wreck">Broken down: {w.points} {w.weapon} ({sideName(w.side)})</p>
    {/each}
    {#each prisoners as g (g.side)}
      <p class="prisoners side-{g.side}" data-testid="prisoners">{g.points} prisoner point{g.points === 1 ? "" : "s"} held by the {sideName(g.side)}</p>
    {/each}
    {#if project}
      <p class="prisoners side-{project.side}" data-testid="project">{sideName(project.side)} {project.item ? itemName(project.item) : "construction"} under way{#if project.item}: {project.stages ?? 0} of {itemStages(project.item)} stage{itemStages(project.item) === 1 ? "" : "s"}{/if}</p>
    {/if}
    {#if minefield}
      <p class="prisoners side-{minefield.side}" data-testid="minefield">{sideName(minefield.side)} minefield{#if minefield.real === false} (dummy){/if}</p>
    {/if}
    {#if onorder && view.clock?.phase === "organisation" && !project && builders.length >= 2}
      <p class="guarding">
        <button type="button" data-order="build" title="An engineer and a 3+ TOE infantry battalion, three stages spending no CP (24.41)" onclick={() => hex && onorder({ build: { item: "fortification", hex, builders } })}>Fortify with the ticked</button>
      </p>
    {/if}
    {#if onorder && view.clock?.phase === "organisation" && !project && !minefield && builders.length === 1}
      <p class="guarding">
        <button type="button" data-order="lay-real" title="One stage; 15 ammunition from a supply unit here (24.33)" onclick={() => hex && onorder({ build: { item: "real-minefield", hex, builders } })}>Lay a minefield</button>
        <button type="button" data-order="lay-dummy" title="One stage, nothing spent (24.31)" onclick={() => hex && onorder({ build: { item: "dummy-minefield", hex, builders } })}>Lay a dummy</button>
      </p>
    {/if}
    {#each guards as g (g.side)}
      <p class="prisoners side-{g.side}" data-testid="guards">{g.points} guard point{g.points === 1 ? "" : "s"} of the {sideName(g.side)}</p>
    {/each}
    {#each escapees as g (g.side)}
      <p class="prisoners side-{g.side}" data-testid="escapees">{g.points} {sideName(g.side)} escapee point{g.points === 1 ? "" : "s"}</p>
    {/each}
    {#if onorder && guardPhase(view) && guarders.length > 0}
      {@const pick = { piece: guarding.piece && guarders.some((p) => p.id === guarding.piece) ? guarding.piece : guarders[0].id, points: guarding.points }}
      <p class="guarding" data-testid="guard-controls">
        Guards:
        <select aria-label="Unit for guards" value={pick.piece} onchange={(e) => (guarding = { ...pick, piece: (e.target as HTMLSelectElement).value })}>
          {#each guarders as a (a.id)}
            <option value={a.id}>{label(a).short}</option>
          {/each}
        </select>
        <input type="number" min="1" value={pick.points} aria-label="Guard points" oninput={(e) => (guarding = { ...pick, points: Number((e.target as HTMLInputElement).value) })} />
        <button type="button" data-order="detail-guards" title="Infantry TOE points become guards, one per five prisoners on the march (28.17, 28.21)" onclick={() => onorder({ "detail-guards": { piece: pick.piece, points: pick.points } })}>Detail</button>
        <button type="button" data-order="release-guards" title="Guards here rejoin the unit (28.22)" onclick={() => onorder({ "release-guards": { piece: pick.piece, points: pick.points } })}>Release</button>
      </p>
    {/if}
    {#each repaired as r (r.weapon)}
      {@const pick = absorbing[r.weapon] ?? { piece: absorbers[0]?.id ?? null, points: r.points }}
      <p class="repaired" data-testid="repaired" data-weapon={r.weapon}>
        {r.points} repaired {r.weapon} point{r.points === 1 ? "" : "s"} waiting
        {#if onorder && view.clock?.phase === "organisation" && absorbers.length > 0}
          <select aria-label="Unit to absorb {r.weapon}" value={pick.piece} onchange={(e) => (absorbing[r.weapon] = { ...pick, piece: (e.target as HTMLSelectElement).value })}>
            {#each absorbers as a (a.id)}
              <option value={a.id}>{label(a).short}</option>
            {/each}
          </select>
          <input type="number" min="1" max={r.points} value={pick.points} aria-label="Points of {r.weapon} to absorb" oninput={(e) => (absorbing[r.weapon] = { ...pick, points: Number((e.target as HTMLInputElement).value) })} />
          <button type="button" data-order="absorb" title="A CP per two points from the unit and its parent (19.61)" onclick={() => pick.piece && onorder({ absorb: { piece: pick.piece, weapon: r.weapon, points: pick.points } })}>Absorb</button>
        {/if}
      </p>
    {/each}
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
                {#if p.breakdown_halves}
                  · <span data-testid="breakdown-points" title="Breakdown points this OpStage (21.2x)">BP {p.breakdown_halves / 2}</span>
                {/if}
              {:else if loadLabel(p)}
                · <span data-testid="load">{loadLabel(p)}</span>
              {/if}
              {#each markers(p, view) as m (m)}
                <span class="marker {m}">{m.replace("-", " ")}</span>
              {/each}
              {#if p.attached_to && view.pieces[p.attached_to]}
                · <span data-testid="attached">attached to {label(view.pieces[p.attached_to]).short}</span>
              {/if}
            </span>
            {#if onorder}
              {@const order = attachmentOrder(view, p)}
              {#if order}
                <button type="button" class="attach" data-order={"detach" in order ? "detach" : "attach"} title="A CP from the unit and its parent (19.4x)" onclick={() => onorder(order)}>{"detach" in order ? "Detach" : "Attach"}</button>
              {/if}
            {/if}
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
  .repaired {
    margin: 0 0 0.3rem;
    font-size: 12px;
    color: #1f4d3a;
  }
  .repaired input,
  .guarding input {
    width: 3rem;
  }
  .guarding {
    margin: 0 0 0.3rem;
    font-size: 12px;
  }
  .prisoners {
    margin: 0 0 0.3rem;
    font-size: 12px;
    color: #5b3f80;
  }
  .wreck {
    margin: 0 0 0.3rem;
    font-size: 12px;
    color: #7a3b16;
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
  .attach {
    grid-column: 2 / -1;
    justify-self: start;
    margin-top: 0.2rem;
    font-size: 12px;
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
