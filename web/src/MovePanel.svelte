<script lang="ts">
  // The movement panel (spec, *Moving*): what the ticked stack may do
  // now, from the engine's `reachable`; the assault toggle; the custom
  // path shift-clicks build; and the key to the overlay. The engine's
  // refusal is shown as it is when the stack cannot move at all.
  import { cpLabel } from "./moves";
  import type { HexId, Illegal, PieceId, Purpose, Reach } from "./types";

  let {
    purpose,
    ticked,
    reaches,
    refusal,
    assault = $bindable(false),
    custom,
    onsend,
    onclear,
  }: {
    purpose: Purpose;
    ticked: PieceId[];
    reaches: Reach[];
    refusal: Illegal | null;
    assault?: boolean;
    custom: HexId[];
    onsend: () => void;
    onclear: () => void;
  } = $props();

  const title = $derived(purpose === "react" ? "Reaction move" : "Move");
  const zocStops = $derived(reaches.filter((r) => r.ends_in_zoc).length);
  const maybe = $derived(reaches.filter((r) => r.may_be_interrupted).length);
  const dearest = $derived(reaches.reduce((m, r) => Math.max(m, r.cp), 0));
</script>

<section class="panel" data-testid="move-panel" data-purpose={purpose}>
  <h2>{title} <span class="n">{ticked.length} piece{ticked.length === 1 ? "" : "s"} ticked</span></h2>
  {#if ticked.length === 0}
    <p class="hint">{purpose === "react" ? "Tick the pieces to react with." : "Click one of your stacks and tick the pieces to move."}</p>
  {:else if refusal}
    <p class="refusal" data-testid="move-refusal">Cannot move ({refusal.case}): {refusal.reason}</p>
  {:else}
    <p class="count" data-testid="reach-count">{reaches.length} hex{reaches.length === 1 ? "" : "es"} reachable, up to {cpLabel(dearest)} CP. Click one to go there by the cheapest path.</p>
    {#if purpose === "move"}
      <label class="assault">
        <input type="checkbox" bind:checked={assault} data-testid="assault" />
        Assault: the stack means to attack whoever it ends next to
      </label>
    {/if}
    <ul class="key">
      <li><span class="swatch reach"></span>reachable, CP in the hex</li>
      <li><span class="swatch dp"></span>earns DP (+n)</li>
      {#if zocStops > 0}<li><span class="swatch zoc"></span>enemy ZOC: the move ends there ({zocStops})</li>{/if}
      {#if maybe > 0}<li><span class="swatch maybe"></span>the engine may stop the stack short ({maybe})</li>{/if}
    </ul>
  {/if}
  <div class="custom" data-testid="custom">
    {#if custom.length > 0}
      <p>Custom path: <span class="hexes">{custom.join(" → ")}</span></p>
      <p class="buttons">
        <button type="button" onclick={onsend} data-testid="send-path">Send (Enter)</button>
        <button type="button" onclick={onclear}>Clear (Esc)</button>
      </p>
    {:else if ticked.length > 0}
      <p class="hint">Shift-click hexes to build your own path; Enter sends it and the engine judges it.</p>
    {/if}
  </div>
</section>

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #e6f4f1;
    border: 1px solid #8fc9bf;
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
    margin: 0 0 0.3rem;
  }
  .hint {
    color: #555;
  }
  .refusal {
    color: #a4161a;
  }
  .assault {
    display: block;
    margin: 0.2rem 0 0.4rem;
  }
  .key {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 12px;
    color: #444;
  }
  .swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 0.4rem;
    vertical-align: -2px;
    background: rgba(42, 157, 143, 0.4);
    border: 1px solid #1b6f66;
  }
  .swatch.dp { border: 2px dashed #c0392b; }
  .swatch.zoc { background: rgba(224, 123, 0, 0.5); border: 2px solid #7a3e00; }
  .swatch.maybe { border: 2px dotted #1b6f66; }
  .custom {
    margin-top: 0.3rem;
  }
  .hexes {
    font-variant-numeric: tabular-nums;
    color: #6a1b9a;
  }
  .buttons button {
    font: inherit;
    margin-right: 0.3rem;
  }
</style>
