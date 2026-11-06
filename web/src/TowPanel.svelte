<script lang="ts">
  // Phase K (21.6x): the phasing side's broken-down vehicles that may be
  // towed toward a repair facility. Arm a hex's tow, click the path hex
  // by hex on the map, then send; the engine judges the path.
  import type { HexId, View } from "./types";

  export interface Tow {
    hex: HexId;
    weapon: string;
    points: number;
    path: HexId[];
  }

  let {
    view,
    tow = null,
    onarm,
    onsend,
    oncancel,
  }: {
    view: View;
    tow?: Tow | null;
    onarm: (t: Tow) => void;
    onsend: () => void;
    oncancel: () => void;
  } = $props();

  const groups = $derived(towable(view));
  let counts: Record<string, number> = $state({});
  const key = (g: { hex: HexId; weapon: string }) => `${g.hex} ${g.weapon}`;
</script>

<script lang="ts" module>
  /** The side to act's broken-down points by hex and weapon not yet towed this phase K. */
  export function towable(view: View): { hex: HexId; weapon: string; points: number }[] {
    const side = view.to_act;
    if (!side || view.clock?.phase !== "repair") return [];
    const free = new Map<string, { hex: HexId; weapon: string; points: number }>();
    for (const [hex, wrecks] of Object.entries(view.broken_down ?? {})) {
      for (const w of wrecks) {
        const points = w.points - (w.towed ?? 0);
        if (w.side !== side || points <= 0) continue;
        const g = free.get(`${hex} ${w.weapon}`) ?? { hex, weapon: w.weapon, points: 0 };
        g.points += points;
        free.set(`${hex} ${w.weapon}`, g);
      }
    }
    return [...free.values()];
  }
</script>

{#if groups.length > 0 || tow}
  <section class="panel" data-testid="tow-panel">
    <h2>Broken down · tow (21.6x)</h2>
    {#if tow}
      <p>
        Towing {tow.points} {tow.weapon} from {tow.hex}{#if tow.path.length > 0}: {tow.path.join(" → ")}{:else}: click the hexes of the path, 10 CP as a medium truck, ending nearer a repair facility{/if}.
      </p>
      <button type="button" data-testid="tow-send" disabled={tow.path.length === 0} onclick={onsend}>Tow</button>
      <button type="button" onclick={oncancel}>Cancel</button>
    {:else}
      <ul>
        {#each groups as g (key(g))}
          <li>
            <label>{g.hex}, {g.weapon}: <input type="number" min="1" max={g.points} value={counts[key(g)] ?? g.points} oninput={(e) => (counts[key(g)] = Number((e.target as HTMLInputElement).value))} aria-label="Points of {g.weapon} to tow from {g.hex}" /> of {g.points}</label>
            <button type="button" data-testid="tow-arm" data-hex={g.hex} data-weapon={g.weapon} onclick={() => onarm({ hex: g.hex, weapon: g.weapon, points: counts[key(g)] ?? g.points, path: [] })}>Tow…</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .panel {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.6rem;
    background: #f3ece2;
    border: 1px solid #c7ad8a;
    border-radius: 4px;
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 15px;
  }
  p {
    margin: 0 0 0.3rem;
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
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  input {
    width: 3.5rem;
  }
</style>
