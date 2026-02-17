<script lang="ts">
  import { useReviews } from '$lib/queries';
  import { Badge } from '$lib/components/ui/badge';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import type { Review } from '$lib/types';

  interface Props {
    selected: string | null;
    onSelect: (id: string) => void;
  }

  let { selected, onSelect }: Props = $props();

  const reviews = useReviews(true);

  function getOpenFindingCount(review: Review): number {
    return review.findings.filter((f) => f.status === 'open').length;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString();
  }
</script>

<ScrollArea class="h-full">
  {#if $reviews.isLoading}
    <div class="p-4 text-muted-foreground">Loading reviews...</div>
  {:else if $reviews.error}
    <div class="p-4 text-destructive">Error: {$reviews.error.message}</div>
  {:else if $reviews.data?.length === 0}
    <div class="p-4 text-muted-foreground">No open reviews</div>
  {:else}
    <div class="space-y-2 p-2">
      {#each $reviews.data ?? [] as review (review.id)}
        <button
          type="button"
          class="w-full rounded-lg border p-3 text-left transition-colors hover:bg-accent {selected ===
          review.id
            ? 'border-primary bg-accent'
            : 'border-border'}"
          data-selected={selected === review.id}
          onclick={() => onSelect(review.id)}
        >
          <div class="flex items-center justify-between">
            <span class="font-mono text-sm">{review.id}</span>
            {#if getOpenFindingCount(review) > 0}
              <Badge variant="destructive">{getOpenFindingCount(review)}</Badge>
            {/if}
          </div>
          <div class="mt-1 text-xs text-muted-foreground">
            {review.base.slice(0, 7)}..{review.head.slice(0, 7)}
          </div>
          <div class="mt-1 text-xs text-muted-foreground">
            {formatDate(review.created_at)}
          </div>
        </button>
      {/each}
    </div>
  {/if}
</ScrollArea>
