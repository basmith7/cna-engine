<script lang="ts">
  // The three sealed combat steps (spec, *Sealed options*, *Secrecy*):
  // gun positions, barrage and force assignment, each offering only what
  // the engine's `sealed_options` computed and taking the points typed
  // in, which `apply` alone judges. The app remounts this panel whenever
  // the side to act or the step changes, so a submitted answer's inputs
  // never outlive it; a refused answer keeps them.
  import { sideName } from "./clock";
  import { label } from "./counters";
  import { assignOrder, barrageOrder, committed, fireRowFor, roleRowFor, type FireRow, type RoleRow } from "./sealed";
  import { ammoNote } from "./supply";
  import type { Committer, Firer, Illegal, Order, PieceId, SealedOptions, View } from "./types";

  let {
    view,
    options,
    onorder,
  }: { view: View; options: { ok: SealedOptions } | { ok: null; illegal: Illegal }; onorder: (o: Order) => void } = $props();

  const name = (id: PieceId) => (view.pieces[id] ? label(view.pieces[id]).long : id);
  const side = $derived(view.to_act);

  // Gun positions: the guns ticked to stand forward.
  let forward: PieceId[] = $state([]);
  function toggleGun(id: PieceId) {
    forward = forward.includes(id) ? forward.filter((x) => x !== id) : [...forward, id];
  }

  // Barrage: the fires typed, one row each.
  let fires: FireRow[] = $state([]);
  function addFire(f: Firer) {
    fires = [...fires, fireRowFor(f)];
  }
  function dropFire(i: number) {
    fires = fires.filter((_, j) => j !== i);
  }

  // Force assignment: the roles typed, one row each.
  let roles: RoleRow[] = $state([]);
  function addRole(u: Committer, role: RoleRow["role"], phasing: boolean) {
    roles = [...roles, roleRowFor(u, role, phasing)];
  }
  function dropRole(i: number) {
    roles = roles.filter((_, j) => j !== i);
  }
</script>

<div class="sealed" data-testid="sealed-panel">
  {#if options.ok === null}
    <p class="refusal">The engine offers nothing ({options.illegal.case}): {options.illegal.reason}</p>
  {:else if "gun-positions" in options.ok}
    {@const guns = options.ok["gun-positions"].guns}
    {@const near = guns.filter((g) => g.next_to_enemy)}
    {@const far = guns.filter((g) => !g.next_to_enemy)}
    <p>Tick the gun units to stand <strong>forward</strong> this segment; the rest stay back (12.1x). The other side will not see this until it has answered too.</p>
    {#if guns.length === 0}
      <p class="hint">None of your gun units can take a position.</p>
    {:else}
      {#if near.length === 0}
        <p class="hint">None of your guns is next to the enemy.</p>
      {/if}
      <ul class="pieces near">
        {#each near as g (g.piece)}
          <li data-unit={g.piece}>
            <label><input type="checkbox" data-gun={g.piece} checked={forward.includes(g.piece)} onchange={() => toggleGun(g.piece)} /> {name(g.piece)}</label>
          </li>
        {/each}
      </ul>
      {#if far.length > 0}
        <details class="far">
          <summary>{far.length} more gun unit{far.length === 1 ? "" : "s"} not next to the enemy</summary>
          <ul class="pieces">
            {#each far as g (g.piece)}
              <li data-unit={g.piece}>
                <label><input type="checkbox" data-gun={g.piece} checked={forward.includes(g.piece)} onchange={() => toggleGun(g.piece)} /> {name(g.piece)}</label>
              </li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}
    <p class="buttons">
      <button type="button" data-order="gun-positions" onclick={() => onorder({ "gun-positions": { forward } })}>
        Seal positions ({forward.length} forward)
      </button>
    </p>
  {:else if "barrage" in options.ok}
    {@const firers = options.ok.barrage.firers}
    <p>Each unit below may barrage an enemy unit next to it with some of its TOE points (12.2x). Add a fire per target; seal with none to fire nothing.</p>
    {#if firers.length === 0}
      <p class="hint">None of your units has a target in reach.</p>
    {/if}
    {#each firers as f (f.piece)}
      <div class="unit" data-unit={f.piece}>
        <div class="head">
          <span class="name">{name(f.piece)}</span>
          <span class="facts">TOE {f.points} · {f.forward ? "forward" : "back"}{#if ammoNote(f, "fire")} · <span class="ammo" class:short={f.ammo_short} data-ammo>{ammoNote(f, "fire")}</span>{/if}</span>
          <button type="button" class="add" data-add-fire={f.piece} onclick={() => addFire(f)}>+ fire</button>
        </div>
        {#each fires as row, i (i)}
          {#if row.piece === f.piece}
            <div class="row" data-fire={i}>
              <input type="number" min="1" max={f.points} data-points bind:value={row.points} aria-label="Points" />
              <span>points at</span>
              <select data-target bind:value={row.target} aria-label="Target">
                {#each f.targets as t (t)}
                  <option value={t}>{name(t)}</option>
                {/each}
              </select>
              <button type="button" class="drop" aria-label="Remove" onclick={() => dropFire(i)}>×</button>
            </div>
          {/if}
        {/each}
      </div>
    {/each}
    <p class="buttons">
      <button type="button" data-order="barrage" onclick={() => onorder(barrageOrder(fires))}>
        Seal barrage ({fires.length} fire{fires.length === 1 ? "" : "s"})
      </button>
    </p>
  {:else}
    {@const { phasing, units } = options.ok["assign-forces"]}
    <p>
      Split each unit's TOE points between <strong>anti-armour fire</strong> and <strong>close assault</strong> (14.1x, 15.2x); a unit with no row is withheld.
      {#if phasing}
        As the phasing player you name the enemy hex each role is against (14.12).
      {:else}
        As the non-phasing player your points defend the hex they stand in (14.12).
      {/if}
    </p>
    {#if units.length === 0}
      <p class="hint">None of your units can commit anything.</p>
    {/if}
    {#each units as u (u.piece)}
      {@const used = committed(roles, u.piece)}
      <div class="unit" data-unit={u.piece} class:over={used > u.points}>
        <div class="head">
          <span class="name">{name(u.piece)}</span>
          <span class="facts" data-committed>{used} of {u.points} committed{u.one_assault ? " · one assault at most" : ""}{#if phasing && ammoNote(u, "assault")} · <span class="ammo" class:short={u.ammo_short} data-ammo>{ammoNote(u, "assault")}</span>{/if}</span>
          <span class="adds">
            {#if u.anti_armour && (!phasing || u.anti_armour_targets.length > 0)}
              <button type="button" class="add" data-add-role="{u.piece}:anti-armour" onclick={() => addRole(u, "anti-armour", phasing)}>+ anti-armour</button>
            {/if}
            {#if !phasing || u.close_assault_targets.length > 0}
              <button type="button" class="add" data-add-role="{u.piece}:close-assault" onclick={() => addRole(u, "close-assault", phasing)}>+ assault</button>
            {/if}
          </span>
        </div>
        {#each roles as row, i (i)}
          {#if row.piece === u.piece}
            <div class="row" data-role={i} data-kind={row.role}>
              <span class="role">{row.role === "anti-armour" ? "anti-armour" : "assault"}</span>
              <input type="number" min="1" max={u.points} data-points bind:value={row.points} aria-label="Points" />
              {#if phasing}
                <span>at</span>
                <select data-target bind:value={row.target} aria-label="Target hex">
                  {#each row.role === "anti-armour" ? u.anti_armour_targets : u.close_assault_targets as h (h)}
                    <option value={h}>{h}</option>
                  {/each}
                </select>
              {/if}
              <button type="button" class="drop" aria-label="Remove" onclick={() => dropRole(i)}>×</button>
            </div>
          {/if}
        {/each}
      </div>
    {/each}
    <p class="buttons">
      <button type="button" data-order="assign-forces" onclick={() => onorder(assignOrder(roles, phasing))}>
        Seal assignment ({new Set(roles.map((r) => r.piece)).size} unit{new Set(roles.map((r) => r.piece)).size === 1 ? "" : "s"})
      </button>
    </p>
  {/if}
  <p class="who">Sealed for {sideName(side)}; revealed when both sides have answered.</p>
</div>

<style>
  .ammo.short {
    color: #a4161a;
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
  ul {
    margin: 0 0 0.3rem;
    padding: 0;
    list-style: none;
  }
  li {
    padding: 0.1rem 0;
  }
  .far {
    margin: 0 0 0.3rem;
    color: #555;
  }
  .far summary {
    cursor: pointer;
    font-size: 13px;
  }
  .unit {
    margin: 0.3rem 0;
    padding: 0.3rem 0.4rem;
    background: #fff;
    border: 1px solid #e9c46a;
    border-radius: 4px;
  }
  .unit.over {
    border-color: #a4161a;
  }
  .unit.over .facts {
    color: #a4161a;
  }
  /* The name and facts wrap in their own column; the buttons keep the right edge. */
  .head {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: start;
    gap: 0.2rem 0.5rem;
  }
  .name {
    font-weight: 600;
  }
  .facts {
    display: block;
    color: #555;
    font-size: 12px;
  }
  .head > .add,
  .head > .adds {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
  }
  .adds {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    align-items: stretch;
  }
  .add,
  .drop {
    font: 12px system-ui, sans-serif;
    padding: 0.1rem 0.4rem;
    background: #f2efe6;
    border: 1px solid #b9b2a2;
    border-radius: 3px;
    cursor: pointer;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem;
    margin-top: 0.25rem;
    font-size: 13px;
  }
  .role {
    min-width: 5.5em;
    color: #444;
  }
  input[type="number"] {
    width: 3.2em;
    font: inherit;
  }
  select {
    font: inherit;
    max-width: 13em;
  }
  .buttons {
    margin-top: 0.4rem;
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
  .who {
    color: #666;
    font-size: 12px;
    margin: 0.2rem 0 0;
  }
</style>
