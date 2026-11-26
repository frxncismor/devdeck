<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  interface Props {
    items: T[];
    rowHeight: number;
    height?: number;
    row: Snippet<[T, number]>;
  }
  let { items, rowHeight, height = 400, row }: Props = $props();

  let scrollTop = $state(0);
  const overscan = 4;
  const start = $derived(
    Math.max(0, Math.floor(scrollTop / rowHeight) - overscan),
  );
  const end = $derived(
    Math.min(
      items.length,
      Math.ceil((scrollTop + height) / rowHeight) + overscan,
    ),
  );

  let viewport: HTMLDivElement;

  /** Brings row `index` into view (centred); does nothing if it is already visible. */
  export function scrollToIndex(index: number) {
    const top = index * rowHeight;
    const shown =
      top >= viewport.scrollTop &&
      top + rowHeight <= viewport.scrollTop + height;
    if (shown) return;
    viewport.scrollTop = Math.max(0, top - (height - rowHeight) / 2);
    scrollTop = viewport.scrollTop;
  }
</script>

<div
  bind:this={viewport}
  class="virtual"
  style:height="{height}px"
  style:overflow-x="hidden"
  style:overflow-y="auto"
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  <div style:height="{items.length * rowHeight}px" style:position="relative">
    {#each items.slice(start, end) as item, i (start + i)}
      <div
        style:position="absolute"
        style:top="{(start + i) * rowHeight}px"
        style:height="{rowHeight}px"
        style:width="100%"
      >
        {@render row(item, start + i)}
      </div>
    {/each}
  </div>
</div>
