<script lang="ts">
  import "./map.css";
  import { baseSvg, THEATRE_BOUNDS } from "./map";

  let { onhex }: { onhex?: (hex: string) => void } = $props();

  // Rendered once: a plain string, not reactive state.
  const base = baseSvg();
  const b = THEATRE_BOUNDS;
  const viewBox = `${b.x} ${b.y} ${b.width} ${b.height}`;

  function clicked(e: MouseEvent) {
    const hex = (e.target as Element).closest("[data-hex]")?.getAttribute("data-hex");
    if (hex) onhex?.(hex);
  }
</script>

<!-- One delegated handler reads data-hex; the polygons themselves hold no listeners. -->
<!-- Keyboard access is not a goal of this mission (spec, Non-goals). -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<svg class="map far" role="application" aria-label="The theatre map" {viewBox} preserveAspectRatio="xMidYMid meet" onclick={clicked}>
  <g class="base">{@html base}</g>
  <g class="overlays"></g>
</svg>

<style>
  .map {
    display: block;
    width: 100%;
    height: 100%;
    background: #fbf8f0;
  }
</style>
