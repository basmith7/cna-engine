<script lang="ts">
  import { clockLine, sideName, toActLine, weatherName } from "./clock";
  import type { View } from "./types";

  let { view, hex }: { view: View; hex: string | null } = $props();
</script>

<header class="topbar">
  <span class="title">Graziani's Offensive</span>
  <span class="clock" data-testid="clock">{clockLine(view)}</span>
  <span class="to-act" data-testid="to-act">{toActLine(view)}</span>
  <span class="detail">weather {weatherName(view.weather)}</span>
  <span class="detail">initiative {sideName(view.initiative)}</span>
  <span class="detail">Player A {sideName(view.player_a)}</span>
  <span class="detail hex" data-testid="hex">{hex ? `hex ${hex}` : ""}</span>
</header>

<style>
  .topbar {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 1.2rem;
    padding: 0.5rem 1rem;
    background: #1f2a33;
    color: #f2efe6;
    font: 15px/1.3 system-ui, sans-serif;
  }
  .title {
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .clock {
    font-variant-numeric: tabular-nums;
  }
  .to-act {
    font-weight: 600;
    color: #ffd166;
  }
  .detail {
    color: #b9c2c9;
    font-size: 13px;
  }
  .hex {
    margin-left: auto;
    font-variant-numeric: tabular-nums;
  }
</style>
