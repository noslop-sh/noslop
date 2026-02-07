<script lang="ts">
  import ReviewList from '$lib/views/ReviewList.svelte';
  import DiffView from '$lib/views/DiffView.svelte';
  import { Button } from '$lib/components/ui/button';
  import { useStartReview, useCloseReview } from '$lib/queries';
  import * as Dialog from '$lib/components/ui/dialog';

  let selectedReview = $state<string | null>(null);
  let showNewReviewDialog = $state(false);
  let baseSha = $state('HEAD~1');
  let headSha = $state('HEAD');

  const startReview = useStartReview();
  const closeReview = useCloseReview();

  async function handleSelect(id: string) {
    selectedReview = id;
  }

  async function handleStartReview() {
    const review = await $startReview.mutateAsync({ base: baseSha, head: headSha });
    selectedReview = review.id;
    showNewReviewDialog = false;
    baseSha = 'HEAD~1';
    headSha = 'HEAD';
  }

  async function handleCloseReview() {
    if (selectedReview) {
      await $closeReview.mutateAsync(selectedReview);
      selectedReview = null;
    }
  }
</script>

<div class="flex h-screen bg-background text-foreground">
  <!-- Sidebar -->
  <aside class="flex w-80 flex-col border-r border-border">
    <div class="flex items-center justify-between border-b border-border p-4">
      <h1 class="text-lg font-semibold">noslop Reviews</h1>
      <Button size="sm" onclick={() => (showNewReviewDialog = true)}>New</Button>
    </div>
    <div class="flex-1 overflow-hidden">
      <ReviewList selected={selectedReview} onSelect={handleSelect} />
    </div>
  </aside>

  <!-- Main content -->
  <main class="flex-1 overflow-hidden">
    {#if selectedReview}
      <div class="flex h-full flex-col">
        <div class="flex items-center justify-between border-b border-border p-4">
          <span class="font-mono text-sm">{selectedReview}</span>
          <Button variant="outline" size="sm" onclick={handleCloseReview}>Close Review</Button>
        </div>
        <div class="flex-1 overflow-hidden">
          <DiffView reviewId={selectedReview} />
        </div>
      </div>
    {:else}
      <div class="flex h-full flex-col items-center justify-center text-muted-foreground">
        <p>Select a review or create a new one</p>
      </div>
    {/if}
  </main>
</div>

<!-- New Review Dialog -->
<Dialog.Root bind:open={showNewReviewDialog}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Start New Review</Dialog.Title>
    </Dialog.Header>
    <div class="space-y-4 py-4">
      <div>
        <label for="base" class="text-sm font-medium">Base Commit</label>
        <input
          id="base"
          type="text"
          bind:value={baseSha}
          class="mt-1 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
          placeholder="HEAD~1"
        />
      </div>
      <div>
        <label for="head" class="text-sm font-medium">Head Commit</label>
        <input
          id="head"
          type="text"
          bind:value={headSha}
          class="mt-1 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
          placeholder="HEAD"
        />
      </div>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showNewReviewDialog = false)}>Cancel</Button>
      <Button onclick={handleStartReview} disabled={$startReview.isPending}>
        {$startReview.isPending ? 'Starting...' : 'Start Review'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
