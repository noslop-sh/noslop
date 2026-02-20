<script lang="ts">
  import type { Finding, DismissReason, FindingStatus, Severity, ActiveFilters } from '$lib/types';
  import { applyFindingFilters, sortFindingsBySeverity } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import FindingCard from './FindingCard.svelte';

  interface Props {
    findings: Finding[];
    reviewId: string;
    activeFilters: ActiveFilters;
    focusedFindingId: string | null;
    onFindingClick: (id: string) => void;
    onFilterChange: (filters: ActiveFilters) => void;
    onResolve: (findingId: string) => void;
    onDismiss: (findingId: string, reason: DismissReason) => void;
  }

  let {
    findings,
    reviewId,
    activeFilters,
    focusedFindingId,
    onFindingClick,
    onFilterChange,
    onResolve,
    onDismiss,
  }: Props = $props();

  let expandedId = $state<string | null>(null);

  let statusFilter = $derived(activeFilters.status);
  let severityFilter = $derived(activeFilters.severity);

  let filteredFindings = $derived(
    sortFindingsBySeverity(applyFindingFilters(findings, activeFilters))
  );

  let showResolveAll = $derived(
    activeFilters.status === 'open' && filteredFindings.some((f) => f.status === 'open')
  );

  let openCount = $derived(filteredFindings.filter((f) => f.status === 'open').length);

  function handleStatusChange(value: string): void {
    onFilterChange({
      ...activeFilters,
      status: value as FindingStatus | 'all',
    });
  }

  function handleSeverityChange(value: string): void {
    onFilterChange({
      ...activeFilters,
      severity: value as Severity | 'all',
    });
  }

  function handleResolveAll(): void {
    for (const f of filteredFindings) {
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
            <span class="text-[var(--finding-block)]">{'\u25CF'}</span>
            Block
          </Tabs.Trigger>
          <Tabs.Trigger value="warn">
            <span class="text-[var(--finding-warn)]">{'\u25B2'}</span>
            Warn
          </Tabs.Trigger>
          <Tabs.Trigger value="info">
            <span class="text-[var(--finding-info)]">{'\u25CB'}</span>
            Info
          </Tabs.Trigger>
          <Tabs.Trigger value="all">All</Tabs.Trigger>
        </Tabs.List>
      </Tabs.Root>
    </div>

    <div class="text-xs text-muted-foreground">
      {filteredFindings.length} finding{filteredFindings.length !== 1 ? 's' : ''}
      {#if activeFilters.status === 'all' || activeFilters.status === 'open'}
        ({openCount} open)
      {/if}
    </div>
  </div>

  <!-- Scrollable findings list -->
  <ScrollArea class="flex-1">
    <div class="space-y-1 p-2">
      {#if filteredFindings.length === 0}
        <p class="px-2 py-8 text-center text-sm text-muted-foreground">
          No findings match the current filters.
        </p>
      {:else}
        {#each filteredFindings as finding (finding.id)}
          <div
            onclick={() => onFindingClick(finding.id)}
            role="button"
            tabindex={0}
            onkeydown={(e: KeyboardEvent) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onFindingClick(finding.id);
              }
            }}
          >
            <FindingCard
              {finding}
              {reviewId}
              expanded={expandedId === finding.id}
              focused={focusedFindingId === finding.id}
              onToggleExpand={() => {
                expandedId = expandedId === finding.id ? null : finding.id;
              }}
              onResolve={() => onResolve(finding.id)}
              onDismiss={(reason) => onDismiss(finding.id, reason)}
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
