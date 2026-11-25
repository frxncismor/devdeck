<script lang="ts">
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

  interface Props {
    /** Project name. */
    project: string;
    /** CSS color, e.g. `projectColor(id, ids)` => `var(--project-2)`. */
    color?: string;
    /** Optional worktree / branch name. */
    worktree?: string;
    /** The worktree no longer exists: warning variant. */
    deleted?: boolean;
    title?: string;
  }
  let {
    project,
    color = "var(--project-none)",
    worktree,
    deleted = false,
    title,
  }: Props = $props();
</script>

<span class="chip" class:deleted {title}>
  <span class="sq" style:background={color} aria-hidden="true"></span>
  <span class="name">{project}</span>
  {#if worktree}
    <span class="wt">
      {#if deleted}
        <TriangleAlert size={11} aria-hidden="true" />
      {:else}
        <GitBranch size={11} aria-hidden="true" />
      {/if}
      {worktree}
    </span>
  {/if}
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px 0 7px;
    max-width: 100%;
    border-radius: var(--radius-full);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: var(--text-sm);
    color: var(--text);
    white-space: nowrap;
  }
  .sq {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wt {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .deleted {
    background: var(--warning-soft);
    border-color: transparent;
  }
  .deleted .wt {
    color: var(--warning);
  }
</style>
