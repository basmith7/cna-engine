<script lang="ts">
  // The end of the game (spec, *End of game*): the engine's victory
  // report, each side's level, every condition it checked and the prose
  // ones it could not. Dismissable, so the final position stays
  // viewable; the app offers it again from a button.
  import { clockLine, sideName } from "./clock";
  import { label } from "./counters";
  import { placeName } from "./map";
  import type { PieceId, View } from "./types";

  let { view, onclose }: { view: View; onclose: () => void } = $props();

  const report = $derived(view.over!);
  const name = (id: PieceId) => (view.pieces[id] ? label(view.pieces[id]).long : id);
  const level = (l: string | null) => (l ? l[0].toUpperCase() + l.slice(1) : "No level");
  /** The hexes in brackets only where the objective's name does not already spell them out. */
  const hexNote = (c: { objective: string; hexes: string[]; any_hex: boolean }) => {
    const spelled = c.hexes.length === 1 ? c.hexes[0] : c.hexes.join(" ");
    if (c.objective === spelled) return c.any_hex ? "(any one)" : "";
    return `(${c.any_hex ? "any of " : ""}${c.hexes.join(", ")})`;
  };
</script>

<div class="backdrop" data-testid="end-dialog" role="dialog" aria-labelledby="end-title">
  <section class="dialog">
    <h2 id="end-title">Game over</h2>
    <p class="clock">The clock stopped at {clockLine(view)}.</p>
    <div class="levels">
      <div class="level side-axis" data-level="axis"><span class="who">Axis</span><span class="what">{level(report.axis)}</span></div>
      <div class="level side-cw" data-level="cw"><span class="who">Commonwealth</span><span class="what">{level(report.cw)}</span></div>
    </div>
    {#if report.automatic}
      <p class="automatic" data-testid="automatic">Won at once: {report.automatic}.</p>
    {/if}
    {#if report.points}
      <p class="clock" data-testid="points">Points: Axis {report.points.axis}, Commonwealth {report.points.cw}.</p>
      <table data-testid="scored">
        <thead>
          <tr><th>Objective</th><th>Held by</th><th>Points</th><th>Out of supply</th></tr>
        </thead>
        <tbody>
          {#each report.scored ?? [] as s, i (i)}
            <tr>
              <td class="obj">{placeName(s.objective)}</td>
              <td class="held" data-label="Held by">{s.holder ? sideName(s.holder) : "–"}</td>
              <td class="met" data-label="Points">{s.points}</td>
              <td class="unsup" class:none={!s.unsupplied?.length} data-label="Out of supply">{(s.unsupplied ?? []).map(name).join(", ") || "–"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if report.checked.length > 0}
      <table data-testid="conditions">
        <thead>
          <tr><th>Side</th><th>Level</th><th>Objective</th><th>Held</th><th>Unsupplied</th><th>Met</th></tr>
        </thead>
        <tbody>
          {#each report.checked as c, i (i)}
            <tr data-met={c.met}>
              <td class="side">{sideName(c.side)}</td>
              <td class="lvl">{c.level}</td>
              <td class="obj">{placeName(c.objective)} <span class="hexes">{hexNote(c)}</span></td>
              <td class="held hexes" class:none={c.held.length === 0} data-label="Held">{c.held.join(", ") || "–"}</td>
              <td class="unsup" class:none={c.unsupplied.length === 0} data-label="Unsupplied">{c.unsupplied.map(name).join(", ") || "–"}</td>
              <td class="met">{c.met ? "yes" : "no"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if report.unchecked.length > 0}
      <p class="unchecked-title">Stated in prose only, not checked by the engine:</p>
      <ul class="unchecked">
        {#each report.unchecked as u, i (i)}
          <li>{u}</li>
        {/each}
      </ul>
    {/if}
    <p class="buttons"><button type="button" data-testid="end-close" onclick={onclose}>Close and look at the map</button></p>
  </section>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(31, 42, 51, 0.55);
    font: 14px/1.4 system-ui, sans-serif;
    color: #1b1b1b;
  }
  .dialog {
    width: min(92vw, 760px);
    max-height: 90vh;
    overflow: auto;
    padding: 1.2rem 1.4rem;
    background: #fbf8f0;
    border: 1px solid #d8d1c0;
    border-radius: 8px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.35);
  }
  h2 {
    margin: 0 0 0.3rem;
    font-size: 22px;
  }
  .clock {
    margin: 0 0 0.8rem;
    color: #555;
  }
  .levels {
    display: flex;
    gap: 0.8rem;
    margin-bottom: 0.9rem;
  }
  .level {
    flex: 1;
    padding: 0.5rem 0.7rem;
    border-left: 5px solid;
    background: #fff;
  }
  .level.side-axis { border-color: #7a2e1f; }
  .level.side-cw { border-color: #1f3a5f; }
  .who {
    display: block;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
  }
  .what {
    display: block;
    font-size: 18px;
    font-weight: 700;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th, td {
    text-align: left;
    vertical-align: top;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid #e4dfd2;
  }
  th {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #555;
  }
  .hexes {
    font-variant-numeric: tabular-nums;
    color: #555;
  }
  tr[data-met="true"] .met { color: #2e7d32; font-weight: 700; }
  /* A phone's width has no room for the columns: each row folds into a
     few short lines (side · level, met at the right; the objective; held
     and unsupplied only when there are any), so nothing scrolls sideways. */
  @media (max-width: 560px) {
    thead {
      display: none;
    }
    table, tbody {
      display: block;
    }
    tr {
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
      gap: 0 0.35rem;
      padding: 0.3rem 0;
      border-bottom: 1px solid #e4dfd2;
    }
    td {
      padding: 0;
      border: none;
    }
    /* Line one: side · level and met, or the objective and its points. */
    .side { order: 1; font-weight: 600; }
    .lvl { order: 2; }
    .lvl::before {
      content: "· ";
    }
    .met { order: 3; margin-left: auto; }
    .obj { order: 4; flex-basis: 100%; }
    [data-testid="scored"] .obj { order: 0; flex: 1 1 0; }
    .held { order: 5; }
    .unsup { order: 6; }
    .held, .unsup {
      flex-basis: 100%;
      font-size: 12px;
    }
    .held::before, .unsup::before {
      content: attr(data-label) " ";
      color: #777;
    }
    .none {
      display: none;
    }
  }
  .automatic {
    margin: 0 0 0.8rem;
    font-weight: 700;
  }
  .unchecked-title {
    margin: 0.8rem 0 0.2rem;
    color: #555;
  }
  .unchecked {
    margin: 0;
    padding-left: 1.2rem;
    color: #444;
  }
  .buttons {
    margin: 1rem 0 0;
    text-align: right;
  }
  button {
    font: inherit;
    padding: 0.3rem 0.8rem;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
