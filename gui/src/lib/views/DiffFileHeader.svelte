<script lang="ts">
  import type { FileDiff, DiffViewMode } from '$lib/types';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import { changeTypeLabel, changeTypeColor } from '$lib/helpers';
  import { Check, Columns2, AlignJustify } from '@lucide/svelte';

  interface Props {
    fileDiff: FileDiff;
    findingCounts: { block: number; warn: number; info: number };
    viewed: boolean;
    diffViewMode: DiffViewMode;
    onToggleViewed: () => void;
    onToggleDiffMode: () => void;
  }

  let { fileDiff, findingCounts, viewed, diffViewMode, onToggleViewed, onToggleDiffMode }: Props =
    $props();

  let typeLabel = $derived(changeTypeLabel(fileDiff.change_type));
  let typeColor = $derived(changeTypeColor(fileDiff.change_type));

  let displayPath = $derived(
    fileDiff.old_path && fileDiff.old_path !== fileDiff.path
      ? `${fileDiff.old_path} → ${fileDiff.path}`
      : fileDiff.path
  );
</script>

<div
  class="sticky top-10 z-10 flex items-center gap-2 border-b border-border bg-background px-4 py-2"
>
  <!-- Change type icon -->
  <span
    class="flex size-5 shrink-0 items-center justify-center rounded text-xs font-bold {typeColor}"
  >
    {typeLabel}
  </span>

  <!-- File path -->
  <span class="min-w-0 truncate font-mono text-sm">{displayPath}</span>

  <!-- +N / -N counts -->
  {#if fileDiff.additions > 0}
    <span class="shrink-0 text-xs font-medium text-green-600 dark:text-green-400">
      +{fileDiff.additions}
    </span>
  {/if}
  {#if fileDiff.deletions > 0}
    <span class="shrink-0 text-xs font-medium text-red-600 dark:text-red-400">
      -{fileDiff.deletions}
    </span>
  {/if}

  <!-- Spacer -->
  <div class="flex-1"></div>

  <!-- Split/unified toggle -->
  <Button
    variant="ghost"
    size="icon-sm"
    onclick={onToggleDiffMode}
    title={diffViewMode === 'split' ? 'Switch to unified (s)' : 'Switch to split (s)'}
    aria-label="Toggle diff mode"
  >
    {#if diffViewMode === 'split'}
      <Columns2 class="size-3.5" />
    {:else}
      <AlignJustify class="size-3.5" />
    {/if}
  </Button>

  <!-- Finding badges -->
  {#if findingCounts.block > 0}
    <Badge variant="destructive" class="h-5 gap-1 px-1.5 text-[10px]">
      {findingCounts.block} block
    </Badge>
  {/if}
  {#if findingCounts.warn > 0}
    <Badge
      variant="secondary"
      class="h-5 gap-1 px-1.5 text-[10px] bg-finding-warn/15 text-finding-warn border-finding-warn/25"
    >
      {findingCounts.warn} warn
    </Badge>
  {/if}

  <!-- Viewed toggle -->
  <Button
    variant={viewed ? 'secondary' : 'ghost'}
    size="icon-sm"
    onclick={onToggleViewed}
    aria-label={viewed ? 'Mark as unviewed' : 'Mark as viewed'}
    class="shrink-0"
  >
    <Check
      class="size-4 {viewed ? 'text-green-600 dark:text-green-400' : 'text-muted-foreground'}"
    />
  </Button>
</div>
