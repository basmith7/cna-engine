<script lang="ts">
  // The full-screen curtain between two players at one screen (spec,
  // *Secrecy*). Not a modal: one click, or Escape, and it is gone.
  import { sideName } from "./clock";
  import type { Side } from "./types";

  let { side, ondismiss }: { side: Side; ondismiss: () => void } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && ondismiss()} />

<button type="button" class="curtain side-{side}" data-testid="curtain" data-side={side} onclick={ondismiss}>
  <span class="card">
    <span class="who">{sideName(side)} to play</span>
    <span class="how">Hand over the keyboard, then click to continue.</span>
  </span>
</button>

<style>
  .curtain {
    border: none;
    padding: 0;
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #1f2a33;
    color: #f2efe6;
    cursor: pointer;
    font: 16px/1.4 system-ui, sans-serif;
  }
  .card {
    display: block;
    text-align: center;
    padding: 2rem 3rem;
    border: 2px solid currentColor;
    border-radius: 8px;
  }
  .side-axis .card { color: #f0b7a8; }
  .side-cw .card { color: #a9c4e8; }
  .who {
    display: block;
    margin: 0 0 0.5rem;
    font-size: 32px;
    font-weight: 700;
  }
  .how {
    display: block;
    margin: 0;
    color: #c8c2b4;
  }
</style>
