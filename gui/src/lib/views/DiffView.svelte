<script lang="ts">
  import { useReview } from '$lib/queries';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import CommentThread from './CommentThread.svelte';

  interface Props {
    reviewId: string;
  }

  let { reviewId }: Props = $props();

  const review = useReview(reviewId);
</script>

<div class="flex h-full flex-col">
  {#if $review.isLoading}
    <div class="flex items-center justify-center p-8 text-muted-foreground">Loading review...</div>
  {:else if $review.error}
    <div class="p-4 text-destructive">Error: {$review.error.message}</div>
  {:else if $review.data}
    <div class="border-b border-border p-4">
      <h2 class="text-lg font-semibold">{$review.data.id}</h2>
      <p class="text-sm text-muted-foreground">
        {$review.data.base_sha.slice(0, 7)}..{$review.data.head_sha.slice(0, 7)}
      </p>
      <p class="mt-1 text-sm">
        {$review.data.comments.length} comment(s),
        {$review.data.comments.filter((c) => c.status === 'open').length} open
      </p>
    </div>

    <ScrollArea class="flex-1">
      <div class="p-4">
        {#if $review.data.comments.length === 0}
          <p class="text-muted-foreground">No comments yet</p>
        {:else}
          <div class="space-y-4">
            {#each $review.data.comments as comment (comment.id)}
              <CommentThread {comment} {reviewId} />
            {/each}
          </div>
        {/if}
      </div>
    </ScrollArea>
  {/if}
</div>
