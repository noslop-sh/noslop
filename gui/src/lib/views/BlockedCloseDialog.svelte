<script lang="ts">
  import type { Review } from '$lib/types';
  import { blockingFeedbacks, formatSource } from '$lib/helpers';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { AlertTriangle } from '@lucide/svelte';

  interface Props {
    open: boolean;
    review: Review;
    onJumpToBlocker: () => void;
    onCancel: () => void;
  }

  let { open = $bindable(), review, onJumpToBlocker, onCancel }: Props = $props();

  let blockers = $derived(blockingFeedbacks(review.feedbacks));
  let blockerCount = $derived(blockers.length);

  function formatTarget(target: {
    path: string;
    span: { start: number; end: number } | null;
  }): string {
    if (target.span) {
      return `${target.path}:${target.span.start}`;
    }
    return target.path;
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onCancel();
  }}
>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2 text-destructive">
        <AlertTriangle class="size-5" />
        Cannot Close: {blockerCount} Blocking Feedback{blockerCount === 1 ? '' : 's'}
      </Dialog.Title>
      <Dialog.Description>
        This feedback must be resolved or dismissed before closing:
      </Dialog.Description>
    </Dialog.Header>

    <div class="max-h-64 space-y-3 overflow-y-auto">
      {#each blockers as blocker, i (blocker.id)}
        <div class="flex gap-3 rounded-md border border-destructive/20 bg-destructive/5 px-3 py-2">
          <span class="mt-0.5 shrink-0 text-sm font-bold text-[var(--feedback-block)]">
            {i + 1}.
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-baseline gap-2">
              <span class="font-bold text-[var(--feedback-block)]">{'\u25CF'}</span>
              <span class="truncate font-mono text-sm text-foreground">
                {formatTarget(blocker.target)}
              </span>
            </div>
            <p class="mt-0.5 text-sm text-foreground">
              {blocker.message}
            </p>
            <p class="mt-0.5 text-xs text-muted-foreground">
              {formatSource(blocker.source)}
            </p>
          </div>
        </div>
      {/each}
    </div>

    <Dialog.Footer class="mt-4">
      <Button variant="outline" onclick={onCancel}>Dismiss</Button>
      <Button variant="destructive" onclick={onJumpToBlocker}>Jump to First Blocker</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
