<script lang="ts">
  import type { FileDiff, Finding, SortMode } from '$lib/types';
  import { buildFileTree, openFindingCount } from '$lib/helpers';
  import { Progress } from '$lib/components/ui/progress';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { Search, ArrowUpDown } from '@lucide/svelte';
  import FileTreeNode from './FileTreeNode.svelte';

  interface Props {
    files: FileDiff[];
    findings: Finding[];
    viewedFiles: Set<string>;
    selectedPath: string | null;
    sortMode: SortMode;
    filterText: string;
    onFileSelect: (path: string) => void;
    onSortChange: (mode: SortMode) => void;
    onFilterChange: (text: string) => void;
  }

  let {
    files,
    findings,
    viewedFiles,
    selectedPath,
    sortMode,
    filterText,
    onFileSelect,
    onSortChange,
    onFilterChange,
  }: Props = $props();

  let tree = $derived(buildFileTree(files, findings, viewedFiles, sortMode, filterText));

  let fileCount = $derived(files.length);
  let viewedCount = $derived(viewedFiles.size);
  let viewedPercent = $derived(fileCount > 0 ? Math.round((viewedCount / fileCount) * 100) : 0);

  let totalFindings = $derived(findings.filter((f) => f.status === 'open').length);
  let resolvedFindings = $derived(findings.filter((f) => f.status === 'resolved').length);
  let totalFindingCount = $derived(findings.length);
  let resolvedPercent = $derived(
    totalFindingCount > 0 ? Math.round((resolvedFindings / totalFindingCount) * 100) : 0
  );

  function handleFilterInput(event: Event) {
    const target = event.target as HTMLInputElement;
    onFilterChange(target.value);
  }
</script>

<div class="flex h-full flex-col">
  <!-- Header: file count + sort dropdown -->
  <div class="flex items-center justify-between border-b border-border px-3 py-2">
    <span class="text-xs font-medium text-muted-foreground">
      {fileCount} file{fileCount !== 1 ? 's' : ''}
    </span>

    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            type="button"
            class="flex items-center gap-1 rounded-sm px-1.5 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
            aria-label="Sort files"
          >
            <ArrowUpDown class="size-3" />
            <span class="capitalize">{sortMode}</span>
          </button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content align="end" class="w-40">
        <DropdownMenu.Item
          onclick={() => onSortChange('findings')}
          class={sortMode === 'findings' ? 'bg-accent' : ''}
        >
          By findings
        </DropdownMenu.Item>
        <DropdownMenu.Item
          onclick={() => onSortChange('alphabetical')}
          class={sortMode === 'alphabetical' ? 'bg-accent' : ''}
        >
          Alphabetical
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>

  <!-- Filter input -->
  <div class="relative border-b border-border px-3 py-1.5">
    <Search class="absolute left-4.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
    <input
      type="text"
      placeholder="Filter files..."
      value={filterText}
      oninput={handleFilterInput}
      class="h-7 w-full rounded-sm bg-transparent pl-6 pr-2 text-xs outline-none placeholder:text-muted-foreground"
    />
  </div>

  <!-- Tree content -->
  <div class="flex-1 overflow-y-auto px-1 py-1">
    {#if tree.length === 0}
      <p class="px-3 py-4 text-center text-xs text-muted-foreground">
        {filterText ? 'No matching files' : 'No files'}
      </p>
    {:else}
      {#each tree as entry (entry.path)}
        <FileTreeNode {entry} depth={0} {selectedPath} onSelect={onFileSelect} />
      {/each}
    {/if}
  </div>

  <!-- Progress bars at bottom -->
  <div class="border-t border-border px-3 py-2 space-y-2">
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class="text-[10px] text-muted-foreground">Files viewed</span>
        <span class="text-[10px] font-medium text-muted-foreground">
          {viewedCount}/{fileCount}
        </span>
      </div>
      <Progress value={viewedPercent} max={100} class="h-1.5" />
    </div>
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class="text-[10px] text-muted-foreground">Findings resolved</span>
        <span class="text-[10px] font-medium text-muted-foreground">
          {resolvedFindings}/{totalFindingCount}
        </span>
      </div>
      <Progress value={resolvedPercent} max={100} class="h-1.5" />
    </div>
  </div>
</div>
