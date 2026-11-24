<script lang="ts">
  import type { Snippet } from "svelte";
  import { topbarSlots } from "../lib/shell.svelte";

  /**
   * Render once inside a module screen to feed the app TopBar (the shell owns the TopBar,
   * the scope selector and the module title). Slots are cleared when the module unmounts.
   */
  interface Props {
    meta?: Snippet;
    actions?: Snippet;
  }
  let { meta, actions }: Props = $props();

  $effect(() => {
    topbarSlots.meta = meta;
    topbarSlots.actions = actions;
    return () => {
      topbarSlots.meta = undefined;
      topbarSlots.actions = undefined;
    };
  });
</script>
