<script lang="ts">
  // The engine's refusal of an order, shown over the map for a moment
  // (spec, *Phase controls*: an illegal click is a toast).
  let { text, ondone }: { text: string; ondone: () => void } = $props();

  const SHOWN_MS = 6000;
  $effect(() => {
    void text;
    const t = setTimeout(ondone, SHOWN_MS);
    return () => clearTimeout(t);
  });
</script>

<div class="toast" role="alert" data-testid="toast">
  <span>{text}</span>
  <button type="button" aria-label="Dismiss" onclick={ondone}>×</button>
</div>

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 1.2rem;
    transform: translateX(-50%);
    z-index: 50;
    display: flex;
    align-items: center;
    gap: 0.8rem;
    max-width: min(90vw, 640px);
    padding: 0.6rem 0.9rem;
    background: #a4161a;
    color: #fff;
    font: 14px/1.35 system-ui, sans-serif;
    border-radius: 6px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }
  button {
    background: none;
    border: none;
    color: inherit;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 0;
  }
</style>
