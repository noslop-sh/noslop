<script lang="ts">
  import type { FileTreeEntry } from '$lib/types';
  import { changeTypeLabel, changeTypeColor } from '$lib/helpers';
  import { ChevronRight, ChevronDown, Check, Folder } from '@lucide/svelte';
  import FileTreeNode from './FileTreeNode.svelte';

  interface Props {
    entry: FileTreeEntry;
    depth: number;
    selectedPath: string | null;
    onSelect: (path: string) => void;
  }

  let { entry, depth, selectedPath, onSelect }: Props = $props();

  let expanded = $state(true);

  let isSelected = $derived(entry.kind === 'file' && entry.path === selectedPath);

  let hasBlockFindings = $derived(entry.findings.block > 0);
  let hasWarnFindings = $derived(entry.findings.warn > 0);
  let hasInfoFindings = $derived(entry.findings.info > 0);
  let hasFindings = $derived(hasBlockFindings || hasWarnFindings || hasInfoFindings);

  function handleClick() {
    if (entry.kind === 'directory') {
      expanded = !expanded;
    } else {
      onSelect(entry.path);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleClick();
    }
  }
</script>

<div>
  <button
    type="button"
    class="flex w-full items-center gap-1 rounded-sm px-1 py-0.5 text-left text-sm transition-colors hover:bg-accent {isSelected
      ? 'bg-accent text-accent-foreground'
      : ''}"
    style="padding-left: {depth * 16}px"
    onclick={handleClick}
    onkeydown={handleKeyDown}
    aria-expanded={entry.kind === 'directory' ? expanded : undefined}
    data-selected={isSelected || undefined}
  >
    {#if entry.kind === 'directory'}
      <!-- Directory node -->
      {#if expanded}
        <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
      {:else}
        <ChevronRight class="size-3.5 shrink-0 text-muted-foreground" />
      {/if}
      <Folder class="size-3.5 shrink-0 text-muted-foreground" />
      <span class="truncate font-medium">{entry.name}</span>

      <!-- Aggregated finding badges for directories -->
      {#if hasFindings}
        <div class="ml-auto flex shrink-0 items-center gap-1">
          {#if hasBlockFindings}
            <span
              class="flex size-4 items-center justify-center rounded-full bg-finding-block/15 text-[10px] font-medium text-finding-block"
            >
              {entry.findings.block}
            </span>
          {/if}
          {#if hasWarnFindings}
            <span
              class="flex size-4 items-center justify-center rounded-full bg-finding-warn/15 text-[10px] font-medium text-finding-warn"
            >
              {entry.findings.warn}
            </span>
          {/if}
        </div>
      {/if}
    {:else}
      <!-- File node -->
      <span
        class="inline-flex size-3.5 shrink-0 items-center justify-center text-[10px] font-bold {changeTypeColor(
          entry.change_type!
        )}"
      >
        {changeTypeLabel(entry.change_type!)}
      </span>
      <span class="truncate">{entry.name}</span>

      <div class="ml-auto flex shrink-0 items-center gap-1.5">
        <!-- Line count badges -->
        {#if entry.additions > 0}
          <span class="text-[10px] text-green-600 dark:text-green-400">+{entry.additions}</span>
        {/if}
        {#if entry.deletions > 0}
          <span class="text-[10px] text-red-600 dark:text-red-400">-{entry.deletions}</span>
        {/if}

        <!-- Finding badges -->
        {#if hasBlockFindings}
          <span
            class="flex size-4 items-center justify-center rounded-full bg-finding-block text-[10px] font-bold text-white"
            title="{entry.findings.block} blocking finding{entry.findings.block > 1 ? 's' : ''}"
          >
            {entry.findings.block}
          </span>
        {/if}
        {#if hasWarnFindings}
          <span
            class="flex size-4 items-center justify-center rounded-full bg-finding-warn/80 text-[10px] font-bold text-white"
            title="{entry.findings.warn} warning{entry.findings.warn > 1 ? 's' : ''}"
          >
            {entry.findings.warn}
          </span>
        {/if}

        <!-- Viewed checkmark -->
        {#if entry.viewed}
          <Check class="size-3.5 text-success" />
        {/if}
      </div>
    {/if}
  </button>

  <!-- Recursive children for expanded directories -->
  {#if entry.kind === 'directory' && expanded}
    {#each entry.children as child (child.path)}
      <FileTreeNode entry={child} depth={depth + 1} {selectedPath} {onSelect} />
    {/each}
  {/if}
</div>
