<script lang="ts">
  // Save and Load in play (spec, *Save and load*): Save downloads the
  // state JSON; Load hands a file to the engine, whose refusal stays
  // here while the current game goes on untouched.
  let {
    error,
    onsave,
    onload,
  }: {
    /** The engine's refusal of the last file loaded, or null. */
    error: string | null;
    onsave: () => void;
    onload: (file: File) => void;
  } = $props();

  function onfile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) onload(file);
  }
</script>

<section class="game" data-testid="game-panel">
  <div class="buttons">
    <button type="button" data-testid="save" onclick={onsave}>Save</button>
    <label class="file"><span>Load…</span><input type="file" accept="application/json,.json" aria-label="Load a save" onchange={onfile} /></label>
    <span class="hint">Autosaved after every order.</span>
  </div>
  {#if error}
    <p class="error" data-testid="load-error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .game {
    font: 14px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
    margin-bottom: 0.6rem;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }
  button,
  .file {
    font: inherit;
    padding: 0.25rem 0.6rem;
    background: #f2efe6;
    color: #1b1b1b;
    border: 1px solid #b9b2a2;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover,
  .file:hover {
    background: #e4dfd2;
  }
  .file input {
    display: none;
  }
  .hint {
    color: #666;
    font-size: 12px;
  }
  .error {
    margin: 0.4rem 0 0;
    padding: 0.5rem 0.6rem;
    background: #fbe3e1;
    border: 1px solid #d9847d;
    border-radius: 4px;
    color: #7a1a1d;
    overflow-wrap: anywhere;
  }
</style>
