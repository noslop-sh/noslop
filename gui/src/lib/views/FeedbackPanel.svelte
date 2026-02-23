<script lang="ts">
  import type { Feedback, DismissReason, FeedbackStatus, Severity, ActiveFilters } from '$lib/types';
  import { applyFeedbackFilters, sortFeedbacksBySeverity } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import FeedbackCard from './FeedbackCard.svelte';

  interface Props {
    feedbacks: Feedback[];
    reviewId: string;
    activeFilters: ActiveFilters;
    focusedFeedbackId: string | null;
    onFeedbackClick: (id: string) => void;
    onFilterChange: (filters: ActiveFilters) => void;
    onResolve: (feedbackId: string) => void;
    onDismiss: (feedbackId: string, reason: DismissReason) => void;
  }

  let {
    feedbacks,
    reviewId,
    activeFilters,
    focusedFeedbackId,
    onFeedbackClick,
    onFilterChange,
    onResolve,
    onDismiss,
  }: Props = $props();

  let expandedId = $state<string | null>(null);

  let statusFilter = $derived(activeFilters.status);
  let severityFilter = $derived(activeFilters.severity);

  let filteredFeedbacks = $derived(
    sortFeedbacksBySeverity(applyFeedbackFilters(feedbacks, activeFilters))
  );

  let showResolveAll = $derived(
    activeFilters.status === 'open' && filteredFeedbacks.some((f) => f.status === 'open')
  );

  let openCount = $derived(filteredFeedbacks.filter((f) => f.status === 'open').length);

  function handleStatusChange(value: string): void {
    onFilterChange({
      ...activeFilters,
      status: value as FeedbackStatus | 'all',
    });
  }

  function handleSeverityChange(value: string): void {
    onFilterChange({
      ...activeFilters,
      severity: value as Severity | 'all',
    });
  }

  function handleResolveAll(): void {
    for (const f of filteredFeedbacks) {
      if (f.status === 'open') {
        onResolve(f.id);
      }
    }
  }
</script>

<div class="flex h-full flex-col">
  <!-- Filter controls -->
  <div class="space-y-2 border-b border-border p-3">
    <!-- Status filter -->
    <div>
      <span class="mb-1 block text-xs font-medium text-muted-foreground">Status</span>
      <Tabs.Root value={statusFilter} onValueChange={handleStatusChange}>
        <Tabs.List class="w-full">
          <Tabs.Trigger value="open">Open</Tabs.Trigger>
          <Tabs.Trigger value="resolved">Resolved</Tabs.Trigger>
          <Tabs.Trigger value="dismissed">Dismissed</Tabs.Trigger>
          <Tabs.Trigger value="all">All</Tabs.Trigger>
        </Tabs.List>
      </Tabs.Root>
    </div>

    <!-- Severity filter -->
    <div>
      <span class="mb-1 block text-xs font-medium text-muted-foreground">Severity</span>
      <Tabs.Root value={severityFilter} onValueChange={handleSeverityChange}>
        <Tabs.List class="w-full">
          <Tabs.Trigger value="block">
            <span class="text-[var(--feedback-block)]">{'\u25CF'}</span>
            Block
          </Tabs.Trigger>
          <Tabs.Trigger value="warn">
            <span class="text-[var(--feedback-warn)]">{'\u25B2'}</span>
            Warn
          </Tabs.Trigger>
          <Tabs.Trigger value="info">
            <span class="text-[var(--feedback-info)]">{'\u25CB'}</span>
            Info
          </Tabs.Trigger>
          <Tabs.Trigger value="all">All</Tabs.Trigger>
        </Tabs.List>
      </Tabs.Root>
    </div>

    <div class="text-xs text-muted-foreground">
      {filteredFeedbacks.length} feedback item{filteredFeedbacks.length !== 1 ? 's' : ''}
      {#if activeFilters.status === 'all' || activeFilters.status === 'open'}
        ({openCount} open)
      {/if}
    </div>
  </div>

  <!-- Scrollable feedback list -->
  <ScrollArea class="flex-1">
    <div class="space-y-1 p-2">
      {#if filteredFeedbacks.length === 0}
        <p class="px-2 py-8 text-center text-sm text-muted-foreground">
          No feedback matches the current filters.
        </p>
      {:else}
        {#each filteredFeedbacks as feedback (feedback.id)}
          <div
            onclick={() => onFeedbackClick(feedback.id)}
            role="button"
            tabindex={0}
            onkeydown={(e: KeyboardEvent) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onFeedbackClick(feedback.id);
              }
            }}
          >
            <FeedbackCard
              {feedback}
              {reviewId}
              expanded={expandedId === feedback.id}
              focused={focusedFeedbackId === feedback.id}
              onToggleExpand={() => {
                expandedId = expandedId === feedback.id ? null : feedback.id;
              }}
              onResolve={() => onResolve(feedback.id)}
              onDismiss={(reason) => onDismiss(feedback.id, reason)}
            />
          </div>
        {/each}
      {/if}
    </div>
  </ScrollArea>

  <!-- Resolve All button -->
  {#if showResolveAll}
    <div class="border-t border-border p-3">
      <Button variant="default" size="sm" class="w-full" onclick={handleResolveAll}>
        Resolve All ({openCount})
      </Button>
    </div>
  {/if}
</div>
