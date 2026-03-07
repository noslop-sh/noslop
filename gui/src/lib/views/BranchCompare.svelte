<script lang="ts">
  import * as Popover from '$lib/components/ui/popover';
  import * as Command from '$lib/components/ui/command';
  import { Button } from '$lib/components/ui/button';
  import { GitBranch, ChevronsLeftRight, Check } from '@lucide/svelte';

  interface Props {
    baseBranch: string;
    compareBranch: string;
    branches: string[];
    loading: boolean;
    onBaseChange: (branch: string) => void;
    onCompareChange: (branch: string) => void;
  }

  let { baseBranch, compareBranch, branches, loading, onBaseChange, onCompareChange }: Props =
    $props();

  let baseOpen = $state(false);
  let compareOpen = $state(false);
</script>

<div class="space-y-2 p-3">
  <div class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
    <GitBranch class="size-3" />
    <span>Compare</span>
  </div>

  <!-- Base branch selector -->
  <div>
    <label class="mb-1 block text-[10px] uppercase tracking-wider text-muted-foreground">
      base
    </label>
    <Popover.Root bind:open={baseOpen}>
      <Popover.Trigger>
        <Button
          variant="outline"
          size="sm"
          class="w-full justify-between font-mono text-xs"
          disabled={loading}
        >
          <span class="truncate">{baseBranch || 'Select...'}</span>
          <ChevronsLeftRight class="ml-1 size-3 shrink-0 opacity-50" />
        </Button>
      </Popover.Trigger>
      <Popover.Content class="w-[--bits-popover-trigger-width] p-0" align="start">
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
  </div>

  <!-- Arrow -->
  <div class="flex justify-center text-muted-foreground">
    <span class="text-xs">...</span>
  </div>

  <!-- Compare branch selector -->
  <div>
    <label class="mb-1 block text-[10px] uppercase tracking-wider text-muted-foreground">
      compare
    </label>
    <Popover.Root bind:open={compareOpen}>
      <Popover.Trigger>
        <Button
          variant="outline"
          size="sm"
          class="w-full justify-between font-mono text-xs"
          disabled={loading}
        >
          <span class="truncate">{compareBranch || 'Select...'}</span>
          <ChevronsLeftRight class="ml-1 size-3 shrink-0 opacity-50" />
        </Button>
      </Popover.Trigger>
      <Popover.Content class="w-[--bits-popover-trigger-width] p-0" align="start">
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
                  <Check class="size-3 {compareBranch === branch ? 'opacity-100' : 'opacity-0'}" />
                  <span class="truncate font-mono text-xs">{branch}</span>
                </Command.Item>
              {/each}
            </Command.Group>
          </Command.List>
        </Command.Root>
      </Popover.Content>
    </Popover.Root>
  </div>

  {#if baseBranch && compareBranch && baseBranch === compareBranch}
    <p class="text-xs text-muted-foreground">Choose different branches to compare.</p>
  {/if}
</div>
