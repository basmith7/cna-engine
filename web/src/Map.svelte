<script lang="ts">
  import { outlinesSvg, THEATRE_BOUNDS } from "./map";

  let { onhex }: { onhex?: (hex: string) => void } = $props();

  // Rendered once: a plain string, not reactive state.
  const outlines = outlinesSvg();
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
<svg class="map" role="application" aria-label="The theatre map" {viewBox} preserveAspectRatio="xMidYMid meet" onclick={clicked}>
  <g class="base">{@html outlines}</g>
  <g class="overlays"></g>
</svg>

<style>
  .map {
    display: block;
    width: 100%;
    height: 100%;
    background: #fbf8f0;
  }
  .map :global(.hex) {
    fill: none;
    stroke: #8a8a8a;
    /* A hairline at any zoom: the fitted view shrinks a hex to a few pixels. */
    stroke-width: 0.6px;
    vector-effect: non-scaling-stroke;
    /* An unfilled polygon's interior takes no clicks by default. */
    pointer-events: all;
  }
</style>
