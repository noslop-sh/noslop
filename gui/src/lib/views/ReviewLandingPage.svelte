<script lang="ts">
  import type { Review, StructuredDiff, DismissReason, Severity } from '$lib/types';
  import {
    blockingFindings,
    findingCountsByFile,
    sortFindingsBySeverity,
    changeTypeLabel,
    changeTypeColor,
  } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import { ShieldAlert, AlertTriangle, CheckCircle, Check } from '@lucide/svelte';

  interface Props {
    review: Review;
    diff: StructuredDiff | null;
    viewedFiles: Set<string>;
    onClose: () => void;
    onScrollToBlocker: () => void;
    onFileClick: (path: string) => void;
    onFindingClick: (id: string) => void;
    onResolve: (findingId: string) => void;
    onDismiss: (findingId: string, reason: DismissReason) => void;
  }

  let {
    review,
    diff,
    viewedFiles,
    onClose,
    onScrollToBlocker,
    onFileClick,
    onFindingClick,
    onResolve,
    onDismiss,
  }: Props = $props();

  // Verdict
  let blockers = $derived(blockingFindings(review.findings));
  let blockCount = $derived(blockers.length);
  let warnCount = $derived(
    review.findings.filter((f) => f.severity === 'warn' && f.status === 'open').length
  );

  type VerdictState = 'blockers' | 'warnings' | 'clean';
  let verdictState: VerdictState = $derived(
    blockCount > 0 ? 'blockers' : warnCount > 0 ? 'warnings' : 'clean'
  );

  let verdictLabel = $derived(
    verdictState === 'blockers'
      ? `${blockCount} Blocker${blockCount > 1 ? 's' : ''}`
      : verdictState === 'warnings'
        ? `${warnCount} Warning${warnCount > 1 ? 's' : ''}`
        : 'Clean'
  );

  let verdictTextClass = $derived(
    verdictState === 'blockers'
      ? 'text-finding-block'
      : verdictState === 'warnings'
        ? 'text-finding-warn'
        : 'text-success'
  );

  // Stats
  let filesChanged = $derived(diff?.stats.files_changed ?? 0);
  let additions = $derived(diff?.stats.additions ?? 0);
  let deletions = $derived(diff?.stats.deletions ?? 0);

  // Progress
  let resolvedCount = $derived(review.findings.filter((f) => f.status !== 'open').length);
  let totalFindings = $derived(review.findings.length);
  let viewedCount = $derived(diff ? diff.files.filter((f) => viewedFiles.has(f.path)).length : 0);
  let viewedPercent = $derived(filesChanged > 0 ? Math.round((viewedCount / filesChanged) * 100) : 0);
  let resolvedPercent = $derived(
    totalFindings > 0 ? Math.round((resolvedCount / totalFindings) * 100) : 0
  );

  // File rows with findings
  let fileRows = $derived.by(() => {
    if (!diff) return [];
    return diff.files
      .map((f) => ({
        path: f.path,
        change_type: f.change_type,
        additions: f.additions,
        deletions: f.deletions,
        openFindings: sortFindingsBySeverity(
          review.findings.filter((fn) => fn.target.path === f.path && fn.status === 'open')
        ),
        counts: findingCountsByFile(review.findings, f.path),
        viewed: viewedFiles.has(f.path),
      }))
      .sort((a, b) => {
        if (a.counts.block !== b.counts.block) return b.counts.block - a.counts.block;
        if (a.counts.warn !== b.counts.warn) return b.counts.warn - a.counts.warn;
        if (a.viewed !== b.viewed) return a.viewed ? 1 : -1;
        return a.path.localeCompare(b.path);
      });
  });

  function severityIcon(severity: Severity, sourceKind: string): string {
    if (sourceKind === 'human') return '\u25C6';
    switch (severity) {
      case 'block':
        return '\u25CF';
      case 'warn':
        return '\u25B2';
      case 'info':
        return '\u25CB';
    }
  }

  function severityColor(severity: Severity, sourceKind: string): string {
    if (sourceKind === 'human') return 'text-[var(--finding-human)]';
    switch (severity) {
      case 'block':
        return 'text-[var(--finding-block)]';
      case 'warn':
        return 'text-[var(--finding-warn)]';
      case 'info':
        return 'text-[var(--finding-info)]';
    }
  }
</script>

<div class="mx-auto max-w-4xl space-y-5 px-6 py-5">
  <!-- Header: verdict + branch + progress -->
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        {#if verdictState === 'blockers'}
          <ShieldAlert class="size-4 {verdictTextClass}" />
        {:else if verdictState === 'warnings'}
          <AlertTriangle class="size-4 {verdictTextClass}" />
        {:else}
          <CheckCircle class="size-4 {verdictTextClass}" />
        {/if}
        <span class="text-sm font-semibold {verdictTextClass}">{verdictLabel}</span>
      </div>
      <span class="font-mono text-xs text-muted-foreground">
        {review.branch ?? ''}
        <span class="opacity-50">{review.base.slice(0, 7)}..{review.head.slice(0, 7)}</span>
      </span>
    </div>

    <div class="flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-muted-foreground">
      <span>
        {filesChanged} file{filesChanged !== 1 ? 's' : ''}
        <span class="opacity-40 mx-1">&middot;</span>
        <span class="text-green-600 dark:text-green-400">+{additions}</span>
        <span class="opacity-40 mx-0.5">/</span>
        <span class="text-red-600 dark:text-red-400">-{deletions}</span>
      </span>

      {#if filesChanged > 0}
        <div class="flex items-center gap-2">
          <span>{viewedCount}/{filesChanged} viewed</span>
          <div class="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-foreground/30 transition-all"
              style="width: {viewedPercent}%"
            ></div>
          </div>
        </div>
      {/if}

      {#if totalFindings > 0}
        <div class="flex items-center gap-2">
          <span>{resolvedCount}/{totalFindings} resolved</span>
          <div class="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-foreground/30 transition-all"
              style="width: {resolvedPercent}%"
            ></div>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- File list with nested findings -->
  {#if fileRows.length > 0}
    <div class="space-y-0.5">
      {#each fileRows as file (file.path)}
        <!-- File row -->
        <button
          type="button"
          class="flex w-full items-center gap-2 rounded px-3 py-1.5 text-left transition-colors hover:bg-accent/50"
          onclick={() => onFileClick(file.path)}
        >
          <span
            class="w-4 shrink-0 text-center font-mono text-xs font-bold {changeTypeColor(file.change_type)}"
          >
            {changeTypeLabel(file.change_type)}
          </span>
          <span class="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
            {file.path}
          </span>
          {#if file.additions > 0}
            <span class="shrink-0 text-xs text-green-600 dark:text-green-400">+{file.additions}</span>
          {/if}
          {#if file.deletions > 0}
            <span class="shrink-0 text-xs text-red-600 dark:text-red-400">-{file.deletions}</span>
          {/if}
          {#if file.counts.block > 0}
            <span
              class="flex size-4 shrink-0 items-center justify-center rounded-full bg-finding-block/15 text-[9px] font-bold text-finding-block"
            >
              {file.counts.block}
            </span>
          {/if}
          {#if file.counts.warn > 0}
            <span
              class="flex size-4 shrink-0 items-center justify-center rounded-full bg-finding-warn/15 text-[9px] font-bold text-finding-warn"
            >
              {file.counts.warn}
            </span>
          {/if}
          {#if file.viewed}
            <Check class="size-3.5 shrink-0 text-muted-foreground/50" />
          {/if}
        </button>

        <!-- Finding sub-rows -->
        {#each file.openFindings as finding (finding.id)}
          <div class="flex items-center gap-2 py-1 pl-9 pr-3">
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1.5 py-0.5 text-left transition-colors hover:bg-accent/50"
              onclick={() => onFindingClick(finding.id)}
            >
              <span
                class="shrink-0 text-xs font-bold leading-none {severityColor(finding.severity, finding.source.kind)}"
              >
                {severityIcon(finding.severity, finding.source.kind)}
              </span>
              <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                {finding.message}
              </span>
              {#if finding.target.span}
                <span class="shrink-0 font-mono text-[10px] text-muted-foreground/50">
                  :{finding.target.span.start}
                </span>
              {/if}
            </button>
            <Button
              variant="ghost"
              size="sm"
              class="h-5 shrink-0 px-1.5 text-[10px]"
              onclick={(e: MouseEvent) => {
                e.stopPropagation();
                onResolve(finding.id);
              }}
            >
              Resolve
            </Button>
          </div>
        {/each}
      {/each}
    </div>
  {/if}
</div>
