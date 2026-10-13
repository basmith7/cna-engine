<script lang="ts">
  // One panel per `Pending` variant (spec, *Interrupts*), switched
  // exhaustively through `pendingKind`: the ZOC declaration ticks the
  // hexes asked about; the reaction ticks the eligible pieces (the move
  // panel and overlay then take over) or passes; the three sealed steps
  // are `SealedPanel`'s, remounted for each side and step; retreat
  // before assault picks a stack and a shaded hex, or passes; a forced
  // retreat shows the order and its pieces, a shaded hex answers it, or
  // Pass with what the engine's preview says it does.
  import { label } from "./counters";
  import { pendingKind, pendingTitle, zocQuestion } from "./pending";
  import { passConsequence, retreatOrderWords } from "./sealed";
  import SealedPanel from "./SealedPanel.svelte";
  import type { ApplyResult, HexId, Illegal, Order, PieceId, Reach, SealedOptions, View } from "./types";

  let {
    view,
    ticked,
    ontoggle,
    onorder,
    sealed = null,
    reaches = [],
    pass = null,
  }: {
    view: View;
    ticked: PieceId[];
    ontoggle: (id: PieceId) => void;
    onorder: (o: Order) => void;
    /** The engine's options for the pending sealed step, for the side to act. */
    sealed?: { ok: SealedOptions } | { ok: null; illegal: Illegal } | null;
    /** The hexes the ticked stack may retreat to, for the counts. */
    reaches?: Reach[];
    /** The engine's preview of Pass while a forced retreat is pending. */
    pass?: ApplyResult | null;
  } = $props();

  const pending = $derived(view.pending!);
  const kind = $derived(pendingKind(pending));

  /** The ZOC ticks: every hex asked about starts as exerting, the usual answer. */
  let exerting: HexId[] = $state([]);
  let askedAbout = "";
  $effect(() => {
    if ("declare-zoc" in pending) {
      const key = pending["declare-zoc"].hexes.join(",");
      if (key !== askedAbout) {
        askedAbout = key;
        exerting = [...pending["declare-zoc"].hexes];
      }
    }
  });
  function toggleHex(h: HexId) {
    exerting = exerting.includes(h) ? exerting.filter((x) => x !== h) : [...exerting, h];
  }
  const name = (id: PieceId) => (view.pieces[id] ? label(view.pieces[id]).long : id);
</script>

{#if kind !== "set-up"}
  <section class="panel kind-{kind}" data-testid="pending" data-kind={kind}>
    <h2>{pendingTitle(pending)}</h2>
    {#if "declare-zoc" in pending}
      {@const d = pending["declare-zoc"]}
      <p>{zocQuestion(d)}</p>
      <ul class="hexes">
        {#each d.hexes as h (h)}
          <li>
            <label><input type="checkbox" data-hex={h} checked={exerting.includes(h)} onchange={() => toggleHex(h)} /> {h} exerts a ZOC</label>
          </li>
        {/each}
      </ul>
      <p class="buttons"><button type="button" data-order="declare-zoc" onclick={() => onorder({ "declare-zoc": { exerting } })}>Declare</button></p>
    {:else if "reaction" in pending}
      {@const d = pending.reaction}
      <p>An enemy stack has moved next to these units; they may react now (8.5x): tick the ones to move, then click a shaded hex. Pass declines for the rest.</p>
      <ul class="pieces">
        {#each d.eligible as id (id)}
          <li data-unit={id}>
            <label><input type="checkbox" checked={ticked.includes(id)} onchange={() => ontoggle(id)} /> {name(id)}</label>
          </li>
        {/each}
      </ul>
      <p class="buttons"><button type="button" data-order="pass" onclick={() => onorder("pass")}>Pass: no (more) reaction</button></p>
    {:else if "sealed" in pending}
      {#if sealed}
        {#key `${view.to_act}:${pending.sealed.step}`}
          <SealedPanel {view} options={sealed} {onorder} />
        {/key}
      {/if}
    {:else if "retreat-before-assault" in pending}
      <p>Before the assault you may pull stacks back (13.2x): click one of your stacks, tick the pieces, then click a shaded hex. A stack next to no enemy combat unit goes one hex, or further for 4 CP at most (13.22). Pass ends the step.</p>
      {#if ticked.length > 0}
        <p class="count" data-testid="retreat-count">{reaches.length} hex{reaches.length === 1 ? "" : "es"} to retreat to for the {ticked.length} ticked.</p>
      {/if}
      <p class="buttons"><button type="button" data-order="pass" onclick={() => onorder("pass")}>Pass: no (more) retreat</button></p>
    {:else if "forced-retreat" in pending}
      {@const d = pending["forced-retreat"]}
      <p data-testid="retreat-order">{retreatOrderWords(d)}</p>
      <ul class="pieces">
        {#each d.pieces as id (id)}
          <li data-unit={id}>{name(id)}</li>
        {/each}
      </ul>
      <p class="count" data-testid="retreat-count">{reaches.length} hex{reaches.length === 1 ? "" : "es"} shaded; click one to retreat there{reaches.some((r) => r.short > 0) ? " (an amber one stops short and pays)" : ""}.</p>
      {#if pass}
        {@const c = passConsequence(pass, name)}
        <p class="consequence" data-testid="pass-consequence">{c.text}</p>
        <p class="buttons"><button type="button" data-order="pass" onclick={() => onorder("pass")}>{c.button}</button></p>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .panel {
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
  p {
    margin: 0 0 0.3rem;
  }
  ul {
    margin: 0 0 0.3rem;
    padding: 0;
    list-style: none;
  }
  li {
    padding: 0.1rem 0;
  }
  .hexes label {
    font-variant-numeric: tabular-nums;
  }
  .buttons button {
    font: inherit;
    padding: 0.25rem 0.6rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
  .count {
    color: #444;
  }
  .consequence {
    color: #7a3e00;
  }
</style>
