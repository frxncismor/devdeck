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
</script>

<div
  class="virtual"
  style:height="{height}px"
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
