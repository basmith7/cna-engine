<script lang="ts">
  // The first screen (spec, *Save and load*): Continue the autosave if
  // there is one, else a new game of the scenario picked with its seed shown, or a save file.
  // The engine judges the file; its refusal is shown here and nothing
  // starts.
  import { DEFAULT_SCENARIO, type Scenario } from "./engine";
  import { parseSeed, randomSeed } from "./saves";

  let {
    autosave,
    error,
    scenarios,
    oncontinue,
    onnew,
    onload,
  }: {
    /** The autosave's clock line, or null when there is none. */
    autosave: string | null;
    /** The engine's refusal of the last file, or null. */
    error: string | null;
    /** The scenarios a new game can start. */
    scenarios: Scenario[];
    oncontinue: () => void;
    onnew: (scenario: string, seed: number) => void;
    onload: (file: File) => void;
  } = $props();

  let seedText = $state(String(randomSeed()));
  const seed = $derived(parseSeed(seedText));
  let scenario = $state(DEFAULT_SCENARIO);

  function onfile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) onload(file);
  }
</script>

<div class="start" data-testid="start">
  <section class="card">
    <h1>The Campaign for North Africa</h1>
    <p class="sub">Two players at one screen; the engine enforces the rules.</p>
    {#if autosave}
      <p class="autosave">
        <button type="button" class="primary" data-testid="continue" onclick={oncontinue}>Continue</button>
        <span class="where">Autosave at {autosave}</span>
      </p>
    {/if}
    <p class="scenario">
      <label>Scenario
        <select data-testid="scenario" aria-label="Scenario" bind:value={scenario}>
          {#each scenarios as s (s.slug)}<option value={s.slug}>{s.name}</option>{/each}
        </select>
      </label>
    </p>
    <p class="new">
      <button type="button" class:primary={!autosave} data-testid="new-game" disabled={seed === null} onclick={() => seed !== null && onnew(scenario, seed)}>New game</button>
      <label>with seed <input type="text" inputmode="numeric" size="12" data-testid="seed" aria-label="Seed" bind:value={seedText} /></label>
      {#if seed === null}<span class="bad">a whole number, please</span>{/if}
    </p>
    <p class="load">
      <label class="file"><span>Load a save…</span><input type="file" accept="application/json,.json" aria-label="Load a save" onchange={onfile} /></label>
    </p>
    {#if error}
      <p class="error" data-testid="load-error" role="alert">{error}</p>
    {/if}
  </section>
</div>

<style>
  .start {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    font: 15px/1.4 system-ui, sans-serif;
    color: #1b1b1b;
  }
  .card {
    width: min(100%, 440px);
    min-width: 0;
    padding: 1.6rem 1.8rem;
    background: #f4f0e6;
    border: 1px solid #d8d1c0;
    border-radius: 8px;
  }
  h1 {
    margin: 0 0 0.2rem;
    font-size: 24px;
  }
  .sub {
    margin: 0 0 1.2rem;
    color: #555;
  }
  p {
    margin: 0 0 0.8rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
  }
  button,
  .file {
    font: inherit;
    padding: 0.35rem 0.8rem;
    background: #f2efe6;
    color: #1b1b1b;
    border: 1px solid #b9b2a2;
    border-radius: 4px;
    cursor: pointer;
  }
  button.primary {
    background: #1f2a33;
    color: #f2efe6;
    border-color: #0f161c;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .where {
    color: #555;
  }
  .scenario label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    max-width: 100%;
  }
  select {
    min-width: 0;
    max-width: 100%;
  }
  select,
  input[type="text"] {
    font: inherit;
    padding: 0.25rem 0.4rem;
    font-variant-numeric: tabular-nums;
  }
  .file input {
    display: none;
  }
  .bad {
    color: #a4161a;
  }
  .error {
    display: block;
    margin: 0.4rem 0 0;
    padding: 0.5rem 0.6rem;
    background: #fbe3e1;
    border: 1px solid #d9847d;
    border-radius: 4px;
    color: #7a1a1d;
    overflow-wrap: anywhere;
  }
</style>
