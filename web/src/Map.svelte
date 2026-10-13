<script lang="ts">
  import "./map.css";
  import { centre } from "./geom";
  import { baseSvg, isHex, START_BOUNDS, THEATRE_BOUNDS } from "./map";
  import { centreOn, fit, isFar, panBy, scale, zoomAt, type Port, type ViewBox } from "./viewport";
  import type { Snippet } from "svelte";

  let { onhex, overlays }: { onhex?: (hex: string) => void; overlays?: Snippet<[number]> } = $props();

  // Rendered once: a plain string, not reactive state.
  const base = baseSvg();

  let port: Port = $state({ width: 1, height: 1 });
  let vb: ViewBox = $state({ ...START_BOUNDS });
  let fitted = false;
  let goto = $state("");
  let gotoError = $state(false);

  // The first real port size fits the start view; later resizes keep the zoom.
  $effect(() => {
    if (!fitted && port.width > 1 && port.height > 1) {
      vb = fit(START_BOUNDS, port);
      fitted = true;
    }
  });

  const viewBox = $derived(`${vb.x} ${vb.y} ${vb.width} ${vb.height}`);
  const far = $derived(isFar(vb, port));
  const px = $derived(scale(vb, port));

  // Drag to pan; a click that did not drag selects a hex. The hex is read
  // at pointerdown: the pointer is captured by the svg for the drag, so
  // pointerup's target is the svg, never the polygon.
  let drag: { x: number; y: number; moved: boolean; hex: string | null } | null = null;

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    const hex = (e.target as Element).closest("[data-hex]")?.getAttribute("data-hex") ?? null;
    drag = { x: e.clientX, y: e.clientY, moved: false, hex };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!drag.moved && Math.hypot(dx, dy) < 4) return;
    drag.moved = true;
    vb = panBy(vb, port, dx, dy);
    drag.x = e.clientX;
    drag.y = e.clientY;
  }
  function up() {
    if (!drag) return;
    const { moved, hex } = drag;
    drag = null;
    if (!moved && hex) onhex?.(hex);
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const r = (e.currentTarget as Element).getBoundingClientRect();
    vb = zoomAt(vb, port, e.deltaY < 0 ? 1.25 : 0.8, e.clientX - r.left, e.clientY - r.top);
  }

  /** The zoom buttons: about the centre of the port. */
  export function zoom(factor: number) {
    vb = zoomAt(vb, port, factor, port.width / 2, port.height / 2);
  }
  export function fitTheatre() {
    vb = fit(THEATRE_BOUNDS, port);
  }
  /** Centre a hex and, if the view is far out, come in to where the labels show. */
  export function goTo(hex: string): boolean {
    const id = hex.trim().toUpperCase();
    if (!isHex(id)) return false;
    let next = centreOn(vb, centre(id));
    if (isFar(next, port)) next = zoomAt(next, port, 3 / scale(next, port), port.width / 2, port.height / 2);
    vb = next;
    onhex?.(id);
    return true;
  }
  function submitGoto(e: SubmitEvent) {
    e.preventDefault();
    gotoError = !goTo(goto);
    if (!gotoError) goto = "";
  }

</script>

<div class="board" bind:clientWidth={port.width} bind:clientHeight={port.height}>
  <!-- One delegated handler reads data-hex; the polygons themselves hold no listeners. -->
  <!-- Keyboard access is not a goal of this mission (spec, Non-goals). -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <svg
    class="map"
    class:far
    role="application"
    aria-label="The theatre map"
    data-testid="map"
    data-scale={px.toFixed(3)}
    {viewBox}
    preserveAspectRatio="xMidYMid meet"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (drag = null)}
    onwheel={wheel}
  >
    <g class="base">{@html base}</g>
    <g class="overlays">{@render overlays?.(px)}</g>
  </svg>
  <form class="controls" onsubmit={submitGoto}>
    <button type="button" title="Zoom in" aria-label="Zoom in" onclick={() => zoom(1.5)}>+</button>
    <button type="button" title="Zoom out" aria-label="Zoom out" onclick={() => zoom(1 / 1.5)}>−</button>
    <button type="button" title="Fit the whole theatre" onclick={fitTheatre}>fit</button>
    <input
      bind:value={goto}
      class:bad={gotoError}
      placeholder="go to hex"
      aria-label="Go to hex"
      size="8"
      spellcheck="false"
      oninput={() => (gotoError = false)}
    />
  </form>
</div>

<style>
  .board {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .map {
    display: block;
    width: 100%;
    height: 100%;
    background: #fbf8f0;
    touch-action: none;
    cursor: grab;
    user-select: none;
  }
  .controls {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem;
    background: rgba(255, 255, 255, 0.85);
    border: 1px solid #c9c2b2;
    border-radius: 4px;
    font: 14px system-ui, sans-serif;
  }
  .controls button {
    min-width: 1.8rem;
    font: inherit;
    background: #f2efe6;
    border: 1px solid #b9b2a2;
    border-radius: 3px;
    cursor: pointer;
  }
  .controls input {
    font: inherit;
    font-variant-numeric: tabular-nums;
    text-transform: uppercase;
    border: 1px solid #b9b2a2;
    border-radius: 3px;
    padding: 0 0.3rem;
  }
  .controls input::placeholder {
    text-transform: none;
  }
  .controls input.bad {
    border-color: #a4161a;
    background: #fde8e8;
  }
</style>
