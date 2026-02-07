<script lang="ts">
  import { useResolveComment } from '$lib/queries';
  import { Button } from '$lib/components/ui/button';
  import { Card } from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import type { Comment } from '$lib/types';

  interface Props {
    comment: Comment;
    reviewId: string;
  }

  let { comment, reviewId: _reviewId }: Props = $props();

  let resolveMessage = $state('');
  let isResolving = $state(false);

  const resolveComment = useResolveComment();

  async function handleResolve() {
    isResolving = true;
    try {
      await $resolveComment.mutateAsync({
        commentId: comment.id,
        message: resolveMessage || undefined,
      });
      resolveMessage = '';
    } finally {
      isResolving = false;
    }
  }
</script>

<Card class="p-4">
  <div class="flex items-start justify-between gap-4">
    <div class="flex-1">
      <div class="flex items-center gap-2">
        <span class="font-mono text-sm text-muted-foreground">{comment.target}</span>
        {#if comment.line}
          <span class="text-xs text-muted-foreground">L{comment.line}</span>
        {/if}
        <Badge variant={comment.status === 'open' ? 'destructive' : 'secondary'}>
          {comment.status}
        </Badge>
      </div>
      <p class="mt-2">{comment.message}</p>

      {#if comment.status === 'resolved' && comment.resolution_message}
        <p class="mt-2 text-sm text-muted-foreground">
          Resolved: {comment.resolution_message}
        </p>
      {/if}
    </div>
  </div>

  {#if comment.status === 'open'}
    <div class="mt-4 flex gap-2">
      <input
        type="text"
        bind:value={resolveMessage}
        placeholder="Resolution message (optional)"
        class="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm"
      />
      <Button onclick={handleResolve} disabled={isResolving || $resolveComment.isPending}>
        {isResolving ? 'Resolving...' : 'Resolve'}
      </Button>
    </div>
  {/if}
</Card>
