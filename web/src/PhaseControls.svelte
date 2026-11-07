<script lang="ts">
  // The phase controls (spec, *Phase controls*): the orders a phase
  // takes as buttons, shown by the clock alone and never pre-filtered
  // by the rules: an illegal click is the engine's refusal in a toast.
  // While something is pending its own panel holds the answers, Pass
  // included, so the controls stand down.
  import { sideName } from "./clock";
  import { label } from "./counters";
  import { wreckOwners } from "./replacements";
  import { LANE_PORTS, laneName, toggleLane } from "./supply";
  import type { HexId, Location, PieceId, Side, View } from "./types";

  let {
    view,
    ticked,
    hex = null,
    flying = null,
    onfly,
    onorder,
  }: {
    view: View;
    ticked: PieceId[];
    /** The selected hex, for orders about a whole hex. */
    hex?: HexId | null;
    /** The supply unit armed to fly: the next hex clicked is its destination (32.37). */
    flying?: PieceId | null;
    onfly?: (piece: PieceId | null) => void;
    onorder: (
      o:
        | "end-segment"
        | "pass"
        | "plan-chariot-raid"
        | { "declare-initiative": { player_a: Side } }
        | { "designate-reserve": { pieces: PieceId[] } }
        | { "release-reserve": { pieces: PieceId[] } }
        | { "attach-motorisation": { piece: PieceId } }
        | { "detach-motorisation": { piece: PieceId } }
        | { "transfer-load": { from: PieceId; to: PieceId; ammo: number; fuel: number } }
        | { "destroy-supply": { hex: HexId } }
        | { "air-lift": { piece: PieceId; to: Location } }
        | { "convoy-lanes": { lanes: number[] } }
        | { repair: { hex: HexId; weapon: string; unit?: PieceId } }
        | { "facility-repair": { hex: HexId; weapon: string; unit?: PieceId } }
        | { "repair-destroyed-tanks": { hex: HexId; weapon: string; points: number } },
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
  // With no supply unit ticked, the first ticked combat unit: a foot unit takes a point per TOE point (32.54, R-123); the engine refuses the rest.
  const unit = $derived(supply === null ? (ticked.find((id) => view.pieces[id]?.kind === "combat") ?? null) : null);
  // Points short of its TOE (none, or a unit reinforced since) leave it walking: it may top up, and any it has it may give back.
  const unitPoints = $derived(unit ? (view.pieces[unit].motorised ?? 0) : 0);
  // Ticked units are the side to act's own, whose TOE the view always carries.
  const riding = $derived(unit ? unitPoints >= (view.pieces[unit].toe ?? Infinity) : false);
  // Two ticked real supply units: the first gives the second ammunition and fuel (32.1x).
  const loaded = $derived(ticked.filter((id) => view.pieces[id]?.load));
  // The movement segment: the selected hex holds the side to act's supply units, which it may destroy before moving anything (32.1x).
  const scuttle = $derived(
    c.phase === "movement-combat" && c.segment === "movement" && view.moved.length === 0 && hex !== null
      ? Object.values(view.pieces).some((p) => p.side === view.to_act && (p.kind === "supply-unit" || p.kind === "dummy-supply-unit") && typeof p.location === "object" && "hex" in p.location && p.location.hex === hex)
      : false,
  );
  // Phase K: one field-repair roll per broken-down type of the side to act's in the selected hex (22.2x).
  const repairs = $derived(
    c.phase === "repair" && hex !== null
      ? [...new Set((view.broken_down?.[hex] ?? []).filter((w) => w.side === view.to_act).map((w) => w.weapon))]
      : [],
  );
  // Phase K: the facility rolls the engine offers in the selected hex (22.3x).
  const facilityRepairs = $derived(c.phase === "repair" && hex !== null ? (view.facility_repairs ?? []).filter((o) => o.hex === hex) : []);
  // Phase K: destroyed tanks the engine offers to attempt in the selected hex, every point it can pay for (22.4x).
  const destroyedRepairs = $derived(c.phase === "repair" && hex !== null ? (view.destroyed_tank_repairs ?? []).filter((o) => o.hex === hex) : []);
  // Phase C, the Axis: the convoy lanes its supply sails by, filled in the order named (32.63); a click names or drops one.
  const lanes = $derived(view.convoy_lanes ?? []);
  /** A wreck owner's name on a repair button: its counter's short label, else its id. */
  const ownerName = (id: PieceId) => (view.pieces[id] ? label(view.pieces[id]).short : id);
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
      {#if supply === null && unit !== null}
        {#if !riding}
          <button type="button" data-order="attach-motorisation" onclick={() => onorder({ "attach-motorisation": { piece: unit } })}>Motorise {label(view.pieces[unit]).short}</button>
        {/if}
        {#if unitPoints > 0}
          <button type="button" data-order="detach-motorisation" onclick={() => onorder({ "detach-motorisation": { piece: unit } })}>Dismount {label(view.pieces[unit]).short}</button>
        {/if}
      {:else if supply === null}
        <span class="hint" data-testid="organise-hint">Tick a supply unit to attach or detach its motorisation points (32.31), or a foot unit to motorise it (32.54).</span>
      {:else if motorised}
        <button type="button" data-order="detach-motorisation" onclick={() => onorder({ "detach-motorisation": { piece: supply } })}>Detach motorisation</button>
      {:else}
        <button type="button" data-order="attach-motorisation" onclick={() => onorder({ "attach-motorisation": { piece: supply } })}>Attach motorisation</button>
        {#if flying === supply}
          <span class="hint" data-testid="fly-hint">Click the hex to fly {label(view.pieces[supply]).short} to (32.37).</span>
          {#if view.to_act === "axis"}
            {#each ["tripoli", "tunisia"] as const as box (box)}
              <button type="button" data-order="air-lift-box" data-box={box} onclick={() => { onfly?.(null); onorder({ "air-lift": { piece: supply, to: { box } } }); }}>Fly to the {box} box</button>
            {/each}
          {/if}
          <button type="button" data-order="air-lift-cancel" onclick={() => onfly?.(null)}>Cancel flight</button>
        {:else}
          <button type="button" data-order="air-lift" onclick={() => onfly?.(supply)}>Fly…</button>
        {/if}
      {/if}
      {#if view.to_act === "axis"}
        <span class="lanes" data-testid="convoy-lanes">
          Convoys: {lanes.length === 0 ? "lane 2 (Tripoli)" : lanes.map(laneName).join(" → ")}
          {#each Object.keys(LANE_PORTS).map(Number) as lane (lane)}
            <button type="button" data-order="convoy-lanes" data-lane={lane} aria-pressed={lanes.includes(lane)} onclick={() => onorder({ "convoy-lanes": { lanes: toggleLane(lanes, lane) } })}>{lane} {LANE_PORTS[lane]}</button>
          {/each}
        </span>
      {/if}
      {#if view.chariot_raid_plannable}
        <!-- 30.41: the engine offers it only when the Axis may plan it now; the plan stays secret until made. -->
        <button type="button" data-order="plan-chariot-raid" title="Strikes the ships in Alexandria in the seventh OpStage from now (30.4x)" onclick={() => onorder("plan-chariot-raid")}>Plan chariot raid</button>
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
    {#if c.phase === "repair" && repairs.length === 0}
      <span class="hint" data-testid="repair-hint">Select a hex with your broken-down vehicles to try field repair (22.2x) or, at a repair facility, facility repair (22.3x).</span>
    {/if}
    {#each facilityRepairs as o (o.weapon)}
      <button type="button" data-order="facility-repair" data-weapon={o.weapon} onclick={() => onorder({ "facility-repair": { hex: o.hex, weapon: o.weapon } })}>Repair {o.points} {o.weapon} at the {o.kind} facility</button>
      {#each wreckOwners(view, o.hex, o.weapon) as unit (unit)}
        <button type="button" data-order="facility-repair" data-weapon={o.weapon} data-unit={unit} title="Repaired points go back to this unit's wreck first (61.38)" onclick={() => onorder({ "facility-repair": { hex: o.hex, weapon: o.weapon, unit } })}>Repair {o.weapon} at the {o.kind} facility, {ownerName(unit)}'s first</button>
      {/each}
    {/each}
    {#each destroyedRepairs as o (o.weapon)}
      <button type="button" data-order="repair-destroyed-tanks" data-weapon={o.weapon} onclick={() => onorder({ "repair-destroyed-tanks": { hex: o.hex, weapon: o.weapon, points: o.points } })}>Try {o.points} destroyed {o.weapon} at the {o.kind} facility</button>
    {/each}
    {#each repairs as weapon (weapon)}
      <button type="button" data-order="repair" data-weapon={weapon} onclick={() => hex && onorder({ repair: { hex, weapon } })}>Repair {weapon} at {hex}</button>
      {#each hex ? wreckOwners(view, hex, weapon) : [] as unit (unit)}
        <button type="button" data-order="repair" data-weapon={weapon} data-unit={unit} title="Repaired points go back to this unit's wreck first (61.38)" onclick={() => hex && onorder({ repair: { hex, weapon, unit } })}>Repair {weapon} at {hex}, {ownerName(unit)}'s first</button>
      {/each}
    {/each}
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
