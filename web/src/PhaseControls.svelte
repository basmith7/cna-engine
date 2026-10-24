<script lang="ts">
  // The phase controls (spec, *Phase controls*): the orders a phase
  // takes as buttons, shown by the clock alone and never pre-filtered
  // by the rules: an illegal click is the engine's refusal in a toast.
  // While something is pending its own panel holds the answers, Pass
  // included, so the controls stand down.
  import { sideName } from "./clock";
  import { label } from "./counters";
  import type { HexId, PieceId, Side, View } from "./types";

  let {
    view,
    ticked,
    hex = null,
    onorder,
  }: {
    view: View;
    ticked: PieceId[];
    /** The selected hex, for orders about a whole hex. */
    hex?: HexId | null;
    onorder: (
      o:
        | "end-segment"
        | "pass"
        | { "declare-initiative": { player_a: Side } }
        | { "designate-reserve": { pieces: PieceId[] } }
        | { "release-reserve": { pieces: PieceId[] } }
        | { "attach-motorisation": { piece: PieceId } }
        | { "detach-motorisation": { piece: PieceId } }
        | { "transfer-load": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
        | { "destroy-supply": { hex: HexId } },
    ) => void;
  } = $props();

  const c = $derived(view.clock);
  const initiative = $derived(c.phase === "initiative-declaration");
  const designate = $derived(c.phase === "reserve-designation");
  const release = $derived(c.phase === "movement-combat" && c.segment === "reserve-release");
  const endSegment = $derived(c.phase === "movement-combat" && (c.segment === "movement" || c.segment === "close-assault" || c.segment === "reserve-release"));
  const n = $derived(ticked.length);
  // Phase C under abstract supply: the first ticked supply unit takes or gives up its motorisation points (32.31).
  const organise = $derived(c.phase === "organisation");
  const supply = $derived(ticked.find((id) => view.pieces[id]?.kind === "supply-unit" || view.pieces[id]?.kind === "dummy-supply-unit") ?? null);
  const motorised = $derived(supply ? (view.pieces[supply].motorised ?? 0) > 0 : false);
  // Two ticked real supply units: the first gives the second ammunition and fuel (32.1x).
  const loaded = $derived(ticked.filter((id) => view.pieces[id]?.load));
  // The movement segment: the selected hex holds the side to act's supply units, which it may destroy before moving anything (32.1x).
  const scuttle = $derived(
    c.phase === "movement-combat" && c.segment === "movement" && view.moved.length === 0 && hex !== null
      ? Object.values(view.pieces).some((p) => p.side === view.to_act && (p.kind === "supply-unit" || p.kind === "dummy-supply-unit") && typeof p.location === "object" && "hex" in p.location && p.location.hex === hex)
      : false,
  );
  let ammo = $state(0);
  let fuel = $state(0);
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
    {#if organise}
      {#if supply === null}
        <span class="hint" data-testid="organise-hint">Tick a supply unit to attach or detach its motorisation points (32.31).</span>
      {:else if motorised}
        <button type="button" data-order="detach-motorisation" onclick={() => onorder({ "detach-motorisation": { piece: supply } })}>Detach motorisation</button>
      {:else}
        <button type="button" data-order="attach-motorisation" onclick={() => onorder({ "attach-motorisation": { piece: supply } })}>Attach motorisation</button>
      {/if}
      {#if loaded.length >= 2}
        <span class="transfer" data-testid="transfer">
          <input type="number" min="0" bind:value={ammo} aria-label="Ammunition to move" data-transfer-ammo />
          <input type="number" min="0" bind:value={fuel} aria-label="Fuel to move" data-transfer-fuel />
          <button type="button" data-order="transfer-load" onclick={() => onorder({ "transfer-load": { from: loaded[0], to: loaded[1], ammo, fuel } })}>Move ammo/fuel {label(view.pieces[loaded[0]]).short} → {label(view.pieces[loaded[1]]).short}</button>
        </span>
      {/if}
    {/if}
    {#if scuttle && hex}
      <button type="button" data-order="destroy-supply" onclick={() => onorder({ "destroy-supply": { hex } })}>Destroy supply at {hex}</button>
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
  .transfer input {
    width: 4rem;
    font: 13px system-ui, sans-serif;
  }
  .hint {
    font: 12px system-ui, sans-serif;
    color: #555;
    align-self: center;
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
