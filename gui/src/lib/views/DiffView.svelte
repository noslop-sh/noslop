<script lang="ts">
  import { useReview } from '$lib/queries';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import FindingCard from './FindingCard.svelte';

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
        {$review.data.base.slice(0, 7)}..{$review.data.head.slice(0, 7)}
      </p>
      <p class="mt-1 text-sm">
        {$review.data.findings.length} finding(s),
        {$review.data.findings.filter((f) => f.status === 'open').length} open
      </p>
    </div>

    <ScrollArea class="flex-1">
      <div class="p-4">
        {#if $review.data.findings.length === 0}
          <p class="text-muted-foreground">No findings</p>
        {:else}
          <div class="space-y-4">
            {#each $review.data.findings as finding (finding.id)}
              <FindingCard {finding} {reviewId} />
            {/each}
          </div>
        {/if}
      </div>
    </ScrollArea>
  {/if}
</div>
