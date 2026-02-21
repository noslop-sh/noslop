<script lang="ts">
  import type { Review, ReviewView } from '$lib/types';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Popover from '$lib/components/ui/popover';
  import * as Command from '$lib/components/ui/command';
  import { blockingFindings } from '$lib/helpers';
  import { Sun, Moon, ChevronsLeftRight, Check } from '@lucide/svelte';

  interface Props {
    review: Review | null;
    isCompact: boolean;
    baseBranch: string;
    compareBranch: string;
    branches: string[];
    activeView: ReviewView;
    onBaseChange: (branch: string) => void;
    onCompareChange: (branch: string) => void;
    onViewChange: (view: ReviewView) => void;
    onToggleTheme: () => void;
  }

  let {
    review,
    isCompact,
    baseBranch,
    compareBranch,
    branches,
    activeView,
    onBaseChange,
    onCompareChange,
    onViewChange,
    onToggleTheme,
  }: Props = $props();

  let blockCount = $derived(review ? blockingFindings(review.findings).length : 0);
  let warnCount = $derived(
    review ? review.findings.filter((f) => f.severity === 'warn' && f.status === 'open').length : 0
  );
  let viewedCount = $derived(review ? review.viewed_files.length : 0);

  let baseOpen = $state(false);
  let compareOpen = $state(false);
</script>

<header
  class="sticky top-0 z-30 flex h-10 shrink-0 items-center border-b border-border bg-background px-4"
>
  <!-- Branch selectors -->
  <div class="flex items-center gap-1.5">
    <Popover.Root bind:open={baseOpen}>
      <Popover.Trigger>
        <button
          type="button"
          class="flex h-6 items-center gap-1 rounded border border-border bg-muted/50 px-2 font-mono text-xs text-muted-foreground hover:text-foreground hover:border-foreground/25 transition-colors"
        >
          <span class="max-w-24 truncate">{baseBranch || '...'}</span>
          <ChevronsLeftRight class="size-2.5 shrink-0 opacity-50" />
        </button>
      </Popover.Trigger>
      <Popover.Content class="w-64 p-0" align="start">
        <Command.Root>
          <Command.Input placeholder="Filter branches..." />
          <Command.List>
            <Command.Empty>No branches found.</Command.Empty>
            <Command.Group>
              {#each branches as branch (branch)}
                <Command.Item
                  value={branch}
                  onSelect={() => {
                    onBaseChange(branch);
                    baseOpen = false;
                  }}
                >
                  <Check class="size-3 {baseBranch === branch ? 'opacity-100' : 'opacity-0'}" />
                  <span class="truncate font-mono text-xs">{branch}</span>
                </Command.Item>
              {/each}
            </Command.Group>
          </Command.List>
        </Command.Root>
      </Popover.Content>
    </Popover.Root>

    <span class="text-[10px] text-muted-foreground">...</span>

    <Popover.Root bind:open={compareOpen}>
      <Popover.Trigger>
        <button
          type="button"
          class="flex h-6 items-center gap-1 rounded border border-border bg-muted/50 px-2 font-mono text-xs text-foreground hover:border-foreground/25 transition-colors"
        >
          <span class="max-w-36 truncate">{compareBranch || '...'}</span>
          <ChevronsLeftRight class="size-2.5 shrink-0 opacity-50" />
        </button>
      </Popover.Trigger>
      <Popover.Content class="w-64 p-0" align="start">
        <Command.Root>
          <Command.Input placeholder="Filter branches..." />
          <Command.List>
            <Command.Empty>No branches found.</Command.Empty>
            <Command.Group>
              {#each branches as branch (branch)}
                <Command.Item
                  value={branch}
                  onSelect={() => {
                    onCompareChange(branch);
                    compareOpen = false;
                  }}
                >
                  <Check
                    class="size-3 {compareBranch === branch ? 'opacity-100' : 'opacity-0'}"
                  />
                  <span class="truncate font-mono text-xs">{branch}</span>
                </Command.Item>
              {/each}
            </Command.Group>
          </Command.List>
        </Command.Root>
      </Popover.Content>
    </Popover.Root>
  </div>

  {#if review}
    <!-- View tabs (centered) -->
    <div class="flex flex-1 items-center justify-center gap-1">
      <button
        type="button"
        class="h-6 rounded px-2.5 text-xs font-medium transition-colors {activeView === 'summary'
          ? 'bg-muted text-foreground'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => onViewChange('summary')}
      >
        Summary
      </button>
      <button
        type="button"
        class="flex h-6 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors {activeView === 'files'
          ? 'bg-muted text-foreground'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => onViewChange('files')}
      >
        Files
        {#if blockCount > 0}
          <Badge variant="destructive" class="h-4 px-1 text-[9px] leading-none">
            {blockCount}
          </Badge>
        {:else if warnCount > 0}
          <Badge
            variant="secondary"
            class="h-4 px-1 text-[9px] leading-none bg-finding-warn/15 text-finding-warn border-finding-warn/25"
          >
            {warnCount}
          </Badge>
        {/if}
      </button>
    </div>

    <!-- Right info -->
    <div class="flex items-center gap-2">
      <span class="text-xs text-muted-foreground">
        {viewedCount} viewed
      </span>
    </div>
  {:else}
    <div class="flex-1"></div>
  {/if}

  <!-- Right side actions -->
  <div class="ml-2 flex items-center gap-1">
    <Button variant="ghost" size="icon-sm" onclick={onToggleTheme} aria-label="Toggle theme">
      <Sun class="size-4 rotate-0 scale-100 transition-transform dark:-rotate-90 dark:scale-0" />
      <Moon
        class="absolute size-4 rotate-90 scale-0 transition-transform dark:rotate-0 dark:scale-100"
      />
    </Button>
  </div>
</header>
