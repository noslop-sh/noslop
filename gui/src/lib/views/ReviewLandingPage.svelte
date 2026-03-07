<script lang="ts">
  import type { Review, StructuredDiff, DismissReason } from '$lib/types';
  import {
    blockingFeedbacks,
    feedbackCountsByFile,
    sortFeedbacksBySeverity,
    changeTypeLabel,
    changeTypeColor,
    getCodeSnippet,
    severityIcon,
    severityColor,
  } from '$lib/helpers';
  import { slide } from 'svelte/transition';
  import { Button } from '$lib/components/ui/button';
  import {
    ShieldAlert,
    AlertTriangle,
    AlertCircle,
    CheckCircle,
    Check,
    ChevronRight,
    ChevronDown,
    Sparkles,
    Loader2,
  } from '@lucide/svelte';

  interface Props {
    review: Review;
    diff: StructuredDiff | null;
    viewedFiles: Set<string>;
    onClose: () => void;
    onScrollToBlocker: () => void;
    onFileClick: (path: string) => void;
    onFeedbackClick: (id: string) => void;
    onResolve: (feedbackId: string) => void;
    onDismiss: (feedbackId: string, reason: DismissReason) => void;
    onAddNote: (feedbackId: string, content: string) => void;
    onRunAgent: () => void;
    agentRunning: boolean;
    agentError: string | null;
    agentResult: { feedback_count: number; duration_secs: number; agent_output: string } | null;
  }

  let {
    review,
    diff,
    viewedFiles,
    onClose,
    onScrollToBlocker,
    onFileClick,
    onFeedbackClick,
    onResolve,
    onDismiss,
    onAddNote,
    onRunAgent,
    agentRunning,
    agentError,
    agentResult,
  }: Props = $props();

  // Expanded feedback IDs (local to summary view)
  let expandedFeedbackIds = $state<Set<string>>(new Set());

  function toggleFeedbackExpanded(id: string) {
    const next = new Set(expandedFeedbackIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedFeedbackIds = next;
  }

  // Verdict
  let blockers = $derived(blockingFeedbacks(review.feedbacks));
  let blockCount = $derived(blockers.length);
  let warnCount = $derived(
    review.feedbacks.filter((f) => f.severity === 'warn' && f.status === 'open').length
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
      ? 'text-feedback-block'
      : verdictState === 'warnings'
        ? 'text-feedback-warn'
        : 'text-success'
  );

  // Stats
  let filesChanged = $derived(diff?.stats.files_changed ?? 0);
  let additions = $derived(diff?.stats.additions ?? 0);
  let deletions = $derived(diff?.stats.deletions ?? 0);

  // Progress
  let resolvedCount = $derived(review.feedbacks.filter((f) => f.status !== 'open').length);
  let totalFeedbacks = $derived(review.feedbacks.length);
  let viewedCount = $derived(diff ? diff.files.filter((f) => viewedFiles.has(f.path)).length : 0);
  let viewedPercent = $derived(
    filesChanged > 0 ? Math.round((viewedCount / filesChanged) * 100) : 0
  );
  let resolvedPercent = $derived(
    totalFeedbacks > 0 ? Math.round((resolvedCount / totalFeedbacks) * 100) : 0
  );

  // Agent output toggle
  let showAgentOutput = $state(false);

  // File rows with feedback
  let diffPaths = $derived(new Set(diff?.files.map((f) => f.path) ?? []));

  let fileRows = $derived.by(() => {
    if (!diff) return [];
    return diff.files
      .map((f) => ({
        path: f.path,
        change_type: f.change_type,
        additions: f.additions,
        deletions: f.deletions,
        openFeedbacks: sortFeedbacksBySeverity(
          review.feedbacks.filter((fn) => fn.target.path === f.path && fn.status === 'open')
        ),
        counts: feedbackCountsByFile(review.feedbacks, f.path),
        viewed: viewedFiles.has(f.path),
      }))
      .sort((a, b) => {
        if (a.counts.block !== b.counts.block) return b.counts.block - a.counts.block;
        if (a.counts.warn !== b.counts.warn) return b.counts.warn - a.counts.warn;
        if (a.viewed !== b.viewed) return a.viewed ? 1 : -1;
        return a.path.localeCompare(b.path);
      });
  });

  // Feedbacks targeting files not in the diff
  let orphanFeedbacks = $derived(
    sortFeedbacksBySeverity(
      review.feedbacks.filter((f) => f.status === 'open' && !diffPaths.has(f.target.path))
    )
  );
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
      <div class="flex items-center gap-3">
        <span class="font-mono text-xs text-muted-foreground">
          {review.branch ?? ''}
          <span class="opacity-50">{review.base.slice(0, 7)}..{review.head.slice(0, 7)}</span>
        </span>
        <Button
          variant="outline"
          size="sm"
          class="h-7 gap-1.5 px-2.5 text-xs"
          disabled={agentRunning}
          onclick={onRunAgent}
        >
          {#if agentRunning}
            <Loader2 class="size-3.5 animate-spin" />
            Analyzing...
          {:else}
            <Sparkles class="size-3.5" />
            Run Agent
          {/if}
        </Button>
      </div>
    </div>

    {#if agentError}
      <div class="space-y-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2">
        <div class="flex items-start gap-2">
          <AlertCircle class="mt-0.5 size-3.5 shrink-0 text-destructive" />
          <p class="text-xs text-destructive">{agentError}</p>
        </div>
        {#if agentResult?.agent_output}
          <button
            type="button"
            class="text-[10px] text-destructive/60 hover:text-destructive"
            onclick={() => (showAgentOutput = !showAgentOutput)}
          >
            {showAgentOutput ? 'Hide' : 'Show'} agent output
          </button>
          {#if showAgentOutput}
            <pre class="max-h-40 overflow-auto rounded bg-background/50 p-2 text-[10px] text-muted-foreground">{agentResult.agent_output}</pre>
          {/if}
        {/if}
      </div>
    {:else if agentResult}
      <div class="space-y-2 rounded-md border border-border bg-muted/30 px-3 py-2">
        <div class="flex items-center gap-2">
          <CheckCircle class="size-3.5 shrink-0 text-success" />
          <p class="text-xs text-muted-foreground">
            Agent finished in {agentResult.duration_secs.toFixed(1)}s
            — {agentResult.feedback_count} feedback{agentResult.feedback_count !== 1 ? 's' : ''} added
          </p>
        </div>
        {#if agentResult.agent_output}
          <button
            type="button"
            class="text-[10px] text-muted-foreground/50 hover:text-muted-foreground"
            onclick={() => (showAgentOutput = !showAgentOutput)}
          >
            {showAgentOutput ? 'Hide' : 'Show'} agent output
          </button>
          {#if showAgentOutput}
            <pre class="max-h-40 overflow-auto rounded bg-background/50 p-2 text-[10px] text-muted-foreground">{agentResult.agent_output}</pre>
          {/if}
        {/if}
      </div>
    {/if}

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

      {#if totalFeedbacks > 0}
        <div class="flex items-center gap-2">
          <span>{resolvedCount}/{totalFeedbacks} resolved</span>
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

  <!-- Summary (from agent) -->
  {#if review.summary}
    <div class="rounded-lg border border-border bg-muted/30 px-4 py-3">
      <p class="whitespace-pre-wrap text-sm text-foreground/80">{review.summary}</p>
    </div>
  {/if}

  <!-- File list with nested feedback -->
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
            class="w-4 shrink-0 text-center font-mono text-xs font-bold {changeTypeColor(
              file.change_type
            )}"
          >
            {changeTypeLabel(file.change_type)}
          </span>
          <span class="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
            {file.path}
          </span>
          {#if file.additions > 0}
            <span class="shrink-0 text-xs text-green-600 dark:text-green-400"
              >+{file.additions}</span
            >
          {/if}
          {#if file.deletions > 0}
            <span class="shrink-0 text-xs text-red-600 dark:text-red-400">-{file.deletions}</span>
          {/if}
          {#if file.counts.block > 0}
            <span
              class="flex size-4 shrink-0 items-center justify-center rounded-full bg-feedback-block/15 text-[9px] font-bold text-feedback-block"
            >
              {file.counts.block}
            </span>
          {/if}
          {#if file.counts.warn > 0}
            <span
              class="flex size-4 shrink-0 items-center justify-center rounded-full bg-feedback-warn/15 text-[9px] font-bold text-feedback-warn"
            >
              {file.counts.warn}
            </span>
          {/if}
          {#if file.viewed}
            <Check class="size-3.5 shrink-0 text-muted-foreground/50" />
          {/if}
        </button>

        <!-- Feedback sub-rows -->
        {#each file.openFeedbacks as feedback (feedback.id)}
          {@const isExpanded = expandedFeedbackIds.has(feedback.id)}
          <div>
            <div class="flex items-center gap-2 py-1 pl-9 pr-3">
              <button
                type="button"
                class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1.5 py-0.5 text-left transition-colors hover:bg-accent/50"
                onclick={() => toggleFeedbackExpanded(feedback.id)}
              >
                {#if isExpanded}
                  <ChevronDown class="size-3 shrink-0 text-muted-foreground/50" />
                {:else}
                  <ChevronRight class="size-3 shrink-0 text-muted-foreground/50" />
                {/if}
                <span
                  class="shrink-0 text-xs font-bold leading-none {severityColor(
                    feedback.severity,
                    feedback.source.kind
                  )}"
                >
                  {severityIcon(feedback.severity, feedback.source.kind)}
                </span>
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {feedback.message}
                </span>
                {#if feedback.target.span}
                  <span class="shrink-0 font-mono text-[10px] text-muted-foreground/50">
                    :{feedback.target.span.start}
                  </span>
                {/if}
              </button>
              <Button
                variant="ghost"
                size="sm"
                class="h-5 shrink-0 px-1.5 text-[10px]"
                onclick={(e: MouseEvent) => {
                  e.stopPropagation();
                  onResolve(feedback.id);
                }}
              >
                Resolve
              </Button>
            </div>

            {#if isExpanded}
              <div transition:slide={{ duration: 200 }} class="pb-2">
                <!-- Code snippet -->
                {#if feedback.target.span && diff}
                  {@const snippetLines = getCodeSnippet(diff, file.path, feedback.target.span)}
                  {#if snippetLines.length > 0}
                    <div class="ml-9 mr-3 mt-1 rounded border border-border overflow-hidden">
                      <div class="overflow-x-auto">
                        {#each snippetLines as line}
                          {@const lineNo = line.new_line_no ?? line.old_line_no}
                          {@const isInSpan =
                            lineNo !== null &&
                            feedback.target.span !== null &&
                            lineNo >= feedback.target.span.start &&
                            lineNo <= feedback.target.span.end}
                          {@const lineBg =
                            isInSpan && line.kind === 'add'
                              ? 'bg-green-500/20'
                              : isInSpan && line.kind === 'delete'
                                ? 'bg-red-500/20'
                                : isInSpan
                                  ? 'bg-accent/20'
                                  : line.kind === 'add'
                                    ? 'bg-green-500/10'
                                    : line.kind === 'delete'
                                      ? 'bg-red-500/10'
                                      : ''}
                          <div class="flex text-xs font-mono leading-5 {lineBg}">
                            <span
                              class="w-10 shrink-0 select-none text-right pr-2 text-muted-foreground/50 border-r border-border"
                            >
                              {lineNo ?? ''}
                            </span>
                            <span class="px-2 whitespace-pre">{line.content}</span>
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}
                {/if}

                <!-- Existing notes -->
                {#if feedback.notes.length > 0}
                  <div class="ml-9 mr-3 mt-2 space-y-1">
                    {#each feedback.notes as note (note.id)}
                      <p class="text-xs text-muted-foreground pl-2 border-l-2 border-border">
                        {note.content}
                      </p>
                    {/each}
                  </div>
                {/if}

                <!-- Add note input -->
                <div class="ml-9 mr-3 mt-2">
                  <form
                    onsubmit={(e: SubmitEvent) => {
                      e.preventDefault();
                      const form = e.currentTarget as HTMLFormElement;
                      const input = form.elements.namedItem('note') as HTMLInputElement;
                      const value = input.value.trim();
                      if (value) {
                        onAddNote(feedback.id, value);
                        input.value = '';
                      }
                    }}
                    class="flex gap-1"
                  >
                    <input
                      name="note"
                      type="text"
                      placeholder="Add a note..."
                      class="flex-1 rounded border border-border bg-transparent px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    <Button variant="ghost" size="sm" class="h-6 px-2 text-xs" type="submit"
                      >Add</Button
                    >
                  </form>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      {/each}
    </div>
  {/if}

  <!-- Orphan feedbacks (targeting files not in the diff) -->
  {#if orphanFeedbacks.length > 0}
    <div class="space-y-0.5">
      <p class="px-3 text-[10px] font-medium uppercase tracking-wider text-muted-foreground/50">
        Other feedbacks
      </p>
      {#each orphanFeedbacks as feedback (feedback.id)}
        <div class="flex items-center gap-2 py-1 pl-3 pr-3">
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1.5 py-0.5 text-left transition-colors hover:bg-accent/50"
            onclick={() => toggleFeedbackExpanded(feedback.id)}
          >
            <span
              class="shrink-0 text-xs font-bold leading-none {severityColor(
                feedback.severity,
                feedback.source.kind
              )}"
            >
              {severityIcon(feedback.severity, feedback.source.kind)}
            </span>
            <span class="shrink-0 font-mono text-[10px] text-muted-foreground/50">
              {feedback.target.path}
            </span>
            <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
              {feedback.message}
            </span>
          </button>
          <Button
            variant="ghost"
            size="sm"
            class="h-5 shrink-0 px-1.5 text-[10px]"
            onclick={(e: MouseEvent) => {
              e.stopPropagation();
              onResolve(feedback.id);
            }}
          >
            Resolve
          </Button>
        </div>
      {/each}
    </div>
  {/if}
</div>
