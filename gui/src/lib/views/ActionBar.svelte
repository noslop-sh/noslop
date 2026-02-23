<script lang="ts">
  import type { Review } from '$lib/types';
  import { Button } from '$lib/components/ui/button';
  import { blockingFeedbacks, openFeedbackCount } from '$lib/helpers';
  import { Check, AlertTriangle, ShieldCheck } from '@lucide/svelte';

  interface Props {
    review: Review | null;
    onClose: () => void;
    onScrollToBlocker: () => void;
  }

  let { review, onClose, onScrollToBlocker }: Props = $props();

  let blockers = $derived(review ? blockingFeedbacks(review.feedbacks) : []);
  let blockerCount = $derived(blockers.length);
  let remainingCount = $derived(review ? openFeedbackCount(review.feedbacks) : 0);
  let isClosed = $derived(review?.status === 'closed');
</script>

<footer
  class="sticky bottom-0 z-30 flex h-12 shrink-0 items-center justify-between border-t border-border bg-background px-4"
>
  {#if review}
    <div class="flex items-center gap-3">
      {#if isClosed}
        <Button variant="outline" size="sm" disabled>
          <Check class="size-4" />
          Review Closed
        </Button>
      {:else if blockerCount > 0}
        <Button variant="destructive" size="sm" onclick={onScrollToBlocker}>
          <AlertTriangle class="size-4" />
          Resolve {blockerCount} Blocker{blockerCount === 1 ? '' : 's'}
        </Button>
      {:else}
        <Button
          size="sm"
          class="bg-success text-success-foreground hover:bg-success/90 shadow-xs"
          onclick={onClose}
        >
          <ShieldCheck class="size-4" />
          Close Review & Allow Push
        </Button>
      {/if}
    </div>

    <div class="flex items-center">
      {#if !isClosed && remainingCount > 0}
        <span class="text-xs text-muted-foreground">
          {remainingCount} feedback item{remainingCount === 1 ? '' : 's'} remaining
        </span>
      {:else if isClosed}
        <span class="text-xs text-muted-foreground">
          Closed {review.closed_at ? new Date(review.closed_at).toLocaleString() : ''}
        </span>
      {:else}
        <span class="text-xs text-muted-foreground"> No open feedback </span>
      {/if}
    </div>
  {:else}
    <div class="flex-1"></div>
  {/if}
</footer>
