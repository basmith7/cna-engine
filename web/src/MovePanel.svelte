<script lang="ts">
  // The movement panel (spec, *Moving*): what the ticked stack may do
  // now, from the engine's `reachable`; the assault toggle; the custom
  // path shift-clicks build; and the key to the overlay. The engine's
  // refusal is shown as it is when the stack cannot move at all.
  import { label } from "./counters";
  import { cpLabel, OFF_MAP_PLACES } from "./moves";
  import { fuelSummary, sourceLabel } from "./supply";
  import type { HexId, Illegal, OffMapPlace, PieceId, Purpose, Reach, View } from "./types";

  let {
    view,
    purpose,
    ticked,
    reaches,
    fuelSources = [],
    fuelPick = $bindable(null),
    hovered = null,
    refusal,
    assault = $bindable(false),
    custom,
    onMap,
    offMap,
    onsend,
    onclear,
    onoffmap,
  }: {
    view: View;
    purpose: Purpose;
    ticked: PieceId[];
    reaches: Reach[];
    /** Abstract supply: the supply units that could pay the stack's fuel (32.17). */
    fuelSources?: PieceId[];
    /** The one the player picked to pay; null lets the engine take the nearest. */
    fuelPick?: PieceId | null;
    /** The hex under the pointer, for its CP and fuel. */
    hovered?: HexId | null;
    refusal: Illegal | null;
    assault?: boolean;
    custom: HexId[];
    /** The ticked stack stands on a hex: the overlay applies. */
    onMap: boolean;
    /** The ticked stack is in a box, in transit or at Nofilia: it may move off-map (8.8x). */
    offMap: boolean;
    onsend: () => void;
    onclear: () => void;
    onoffmap: (to: OffMapPlace) => void;
  } = $props();

  const TITLES: Record<Purpose, string> = { move: "Move", react: "Reaction move", "retreat-before-assault": "Retreat before assault", "forced-retreat": "Forced retreat", rail: "Rail" };
  const title = $derived(TITLES[purpose]);
  const shorts = $derived(reaches.filter((r) => r.short > 0).length);
  const zocStops = $derived(reaches.filter((r) => r.ends_in_zoc).length);
  const maybe = $derived(reaches.filter((r) => r.may_be_interrupted).length);
  const dearest = $derived(reaches.reduce((m, r) => Math.max(m, r.cp), 0));
  const fuel = $derived(fuelSummary(reaches));
  const under = $derived(hovered ? reaches.find((r) => r.hex === hovered) : undefined);
  const nameOf = (id: PieceId) => (view.pieces[id] ? label(view.pieces[id]).short : id);
</script>

<section class="panel" data-testid="move-panel" data-purpose={purpose}>
  <h2>{title} <span class="n">{ticked.length} piece{ticked.length === 1 ? "" : "s"} ticked</span></h2>
  {#if ticked.length === 0}
    <p class="hint">{purpose === "react" ? "Tick the pieces to react with." : purpose === "retreat-before-assault" ? "Click one of your stacks and tick the pieces to pull back." : "Click one of your stacks and tick the pieces to move."}</p>
  {:else if refusal}
    <p class="refusal" data-testid="move-refusal">Cannot move ({refusal.case}): {refusal.reason}</p>
  {:else if !onMap && !offMap}
    <p class="refusal" data-testid="move-refusal">The ticked pieces are not all in one place.</p>
  {:else if onMap}
    {#if purpose === "rail"}
      <p class="count" data-testid="reach-count">{reaches.length} hex{reaches.length === 1 ? "" : "es"} along the railway, no CP or fuel. Click one to go there.</p>
    {:else}
      <p class="count" data-testid="reach-count">{reaches.length} hex{reaches.length === 1 ? "" : "es"} reachable, up to {cpLabel(dearest)} CP. Click one to go there by the cheapest path.</p>
    {/if}
    {#if fuel}
      <div class="fuel" data-testid="fuel">
        <p>Fuel owed (32.22): <span data-testid="fuel-owed">{fuel}</span>{#if under}, <span data-testid="fuel-hovered">{under.fuel} to {under.hex}</span>{/if}.</p>
        {#if fuelSources.length === 0}
          <p class="refusal" data-testid="fuel-none">No supply unit in range has fuel: a move that owes some is refused.</p>
        {:else}
          <label>
            Paid by
            <select bind:value={fuelPick} data-testid="fuel-from">
              <option value={null}>the nearest in range</option>
              {#each fuelSources as id (id)}
                <option value={id}>{sourceLabel(view, id, nameOf)}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>
    {/if}
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
      {#if shorts > 0}<li><span class="swatch short"></span>short of the ordered distance: stands and pays ({shorts})</li>{/if}
    </ul>
  {/if}
  {#if offMap && purpose === "move"}
    <div class="offmap" data-testid="off-map-moves">
      <p>Off-map, by the whole OpStage (8.8x), towards:</p>
      <p class="buttons">
        {#each OFF_MAP_PLACES as { place, name } (place)}
          <button type="button" data-to={place} onclick={() => onoffmap(place)}>{name}</button>
        {/each}
      </p>
    </div>
  {/if}
  <div class="custom" data-testid="custom">
    {#if custom.length > 0}
      <p>Custom path: <span class="hexes">{custom.join(" → ")}</span></p>
      <p class="buttons">
        <button type="button" onclick={onsend} data-testid="send-path">Send (Enter)</button>
        <button type="button" onclick={onclear}>Clear (Esc)</button>
      </p>
    {:else if ticked.length > 0 && onMap && !refusal}
      <p class="hint">Shift-click hexes to build your own path (also the way onto a hex your own units hold, which a plain click selects); Enter sends it and the engine judges it.</p>
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
  .fuel {
    margin: 0.2rem 0 0.3rem;
  }
  .fuel select {
    font: inherit;
    max-width: 100%;
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
  .swatch.short { background: rgba(255, 183, 3, 0.5); border: 2px dashed #c77700; }
  .custom {
    margin-top: 0.3rem;
  }
  .hexes {
    font-variant-numeric: tabular-nums;
    color: #6a1b9a;
  }
  .buttons button {
    font: inherit;
    margin: 0 0.3rem 0.3rem 0;
  }
  .offmap {
    margin-top: 0.3rem;
    padding-top: 0.3rem;
    border-top: 1px solid #8fc9bf;
  }
</style>
