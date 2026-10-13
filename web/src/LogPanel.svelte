<script lang="ts">
  // The game's history as the engine told it (spec, *Log*), newest at
  // the bottom and kept in view.
  import type { LogEntry } from "./log";

  let { entries }: { entries: LogEntry[] } = $props();

  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void entries.length;
    if (list) list.scrollTop = list.scrollHeight;
  });
</script>

<section class="log" data-testid="log">
  <h3>Log</h3>
  <ol bind:this={list} data-testid="log-entries">
    {#each entries as e, i (i)}
      <li class={e.kind}>{e.text}</li>
    {:else}
      <li class="empty">Nothing yet: every accepted order is told here.</li>
    {/each}
  </ol>
</section>

<style>
  .log {
    display: flex;
    flex-direction: column;
    min-height: 0;
    font: 13px/1.35 system-ui, sans-serif;
    color: #1b1b1b;
  }
  h3 {
    margin: 0 0 0.3rem;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #333;
  }
  ol {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0.3rem 0.5rem;
    list-style: none;
    background: #fbf8f0;
    border: 1px solid #d8d1c0;
    border-radius: 4px;
  }
  li {
    padding: 0.1rem 0;
  }
  .header {
    margin-top: 0.4rem;
    font-weight: 700;
    color: #1f2a33;
    border-bottom: 1px solid #d8d1c0;
  }
  .header:first-child {
    margin-top: 0;
  }
  .illegal {
    color: #a4161a;
  }
  .empty {
    color: #777;
    font-style: italic;
  }
</style>
