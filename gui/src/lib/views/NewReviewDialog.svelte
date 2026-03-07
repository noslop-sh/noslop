<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { GitBranch, Play } from '@lucide/svelte';

  interface Props {
    open: boolean;
    onStart: (base: string, head: string) => void;
    onCancel: () => void;
  }

  let { open = $bindable(), onStart, onCancel }: Props = $props();

  let mode: 'quick' | 'advanced' = $state('quick');
  let base = $state('HEAD~1');
  let head = $state('HEAD');

  function handleStart(): void {
    onStart(base.trim() || 'HEAD~1', head.trim() || 'HEAD');
  }

  function handleCancel(): void {
    mode = 'quick';
    base = 'HEAD~1';
    head = 'HEAD';
    onCancel();
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleStart();
    }
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) handleCancel();
  }}
>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <GitBranch class="size-5" />
        New Review
      </Dialog.Title>
      <Dialog.Description>Create a new code review for a range of commits.</Dialog.Description>
    </Dialog.Header>

    <div class="space-y-4">
      <!-- Mode toggle -->
      <div class="flex gap-1 rounded-md border border-border p-0.5">
        <button
          class="flex-1 rounded px-3 py-1.5 text-sm font-medium transition-colors
            {mode === 'quick'
            ? 'bg-accent text-accent-foreground'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (mode = 'quick')}
        >
          Quick
        </button>
        <button
          class="flex-1 rounded px-3 py-1.5 text-sm font-medium transition-colors
            {mode === 'advanced'
            ? 'bg-accent text-accent-foreground'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (mode = 'advanced')}
        >
          Advanced
        </button>
      </div>

      {#if mode === 'quick'}
        <div class="rounded-md border border-border bg-muted/30 p-4 text-center">
          <p class="text-sm text-muted-foreground">
            Auto-detect changes since the last review point.
          </p>
          <p class="mt-1 font-mono text-xs text-muted-foreground">
            {base} .. {head}
          </p>
        </div>
      {:else}
        <!-- Advanced: base and head inputs -->
        <div class="space-y-3">
          <div>
            <label for="base-input" class="mb-1.5 block text-sm font-medium text-foreground">
              Base (from)
            </label>
            <input
              id="base-input"
              type="text"
              bind:value={base}
              onkeydown={handleKeydown}
              placeholder="HEAD~1"
              class="h-9 w-full rounded-md border border-input bg-background px-3 text-sm
                placeholder:text-muted-foreground focus:border-ring focus:ring-1 focus:ring-ring
                focus:outline-none"
            />
          </div>

          <div>
            <label for="head-input" class="mb-1.5 block text-sm font-medium text-foreground">
              Head (to)
            </label>
            <input
              id="head-input"
              type="text"
              bind:value={head}
              onkeydown={handleKeydown}
              placeholder="HEAD"
              class="h-9 w-full rounded-md border border-input bg-background px-3 text-sm
                placeholder:text-muted-foreground focus:border-ring focus:ring-1 focus:ring-ring
                focus:outline-none"
            />
          </div>

          <!-- Preview placeholder -->
          <div class="rounded-md border border-dashed border-border bg-muted/20 p-3 text-center">
            <p class="text-xs text-muted-foreground">Diff preview will appear here</p>
          </div>
        </div>
      {/if}
    </div>

    <Dialog.Footer class="mt-4">
      <Button variant="outline" onclick={handleCancel}>Cancel</Button>
      <Button onclick={handleStart}>
        <Play class="size-4" />
        Start Review
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
