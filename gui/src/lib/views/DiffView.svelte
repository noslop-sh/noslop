<script lang="ts">
  import type { StructuredDiff, Finding, DiffViewMode, DismissReason, Severity } from '$lib/types';
  import type { SelectedLineRange } from '@pierre/diffs';
  import { parsePatchFiles } from '@pierre/diffs';
  import { provideWorkerPool } from '$lib/diff/worker-pool.svelte';
  import FileDiffSection from './FileDiffSection.svelte';

  interface Props {
    rawPatch: string | null;
    diff: StructuredDiff | null;
    findings: Finding[];
    reviewId: string;
    reviewOpen: boolean;
    currentFilePath: string | null;
    viewedFiles: Set<string>;
    diffViewMode: DiffViewMode;
    focusedFindingId: string | null;
    onFileSelect: (path: string) => void;
    onToggleViewed: (path: string) => void;
    onFindingClick: (id: string) => void;
    onResolve: (findingId: string) => void;
    onDismiss: (findingId: string, reason: DismissReason) => void;
    onToggleDiffMode: () => void;
    onSubmitFinding: (
      filePath: string,
      startLine: number,
      endLine: number,
      message: string,
      severity: Severity
    ) => Promise<void>;
  }

  let {
    rawPatch,
    diff,
    findings,
    reviewId,
    reviewOpen,
    currentFilePath,
    viewedFiles,
    diffViewMode,
    focusedFindingId,
    onFileSelect,
    onToggleViewed,
    onFindingClick,
    onResolve,
    onDismiss,
    onToggleDiffMode,
    onSubmitFinding,
  }: Props = $props();

  // Provide shared worker pool to all FileDiffRenderer children.
  // Wait for pool initialization before rendering any diffs so that
  // FileDiffRenderer can render synchronously (no async gaps = no grey flashes).
  const pool = provideWorkerPool();
  let poolReady = $state(false);

  $effect(() => {
    pool.initialize().then(() => {
      poolReady = true;
    });
  });

  // Parse raw patch into FileDiffMetadata[]
  let parsedFiles = $derived.by(() => {
    if (!rawPatch) return [];
    const patches = parsePatchFiles(rawPatch, 'diff');
    return patches.flatMap((p) => p.files);
  });

  // Active inline form state (line selection for finding creation)
  let activeForm = $state<{
    filePath: string;
    startLine: number;
    endLine: number;
  } | null>(null);

  function handleLineSelected(filePath: string, range: SelectedLineRange | null): void {
    if (!range) {
      activeForm = null;
      return;
    }
    activeForm = {
      filePath,
      startLine: range.start,
      endLine: range.end,
    };
  }

  async function handleFormSubmit(message: string, severity: Severity): Promise<void> {
    if (!activeForm) return;
    await onSubmitFinding(
      activeForm.filePath,
      activeForm.startLine,
      activeForm.endLine,
      message,
      severity
    );
    activeForm = null;
  }

  function handleFormCancel(): void {
    activeForm = null;
  }
</script>

<div class="h-full overflow-y-auto" data-diff-scroll>
  {#if !rawPatch}
    <div class="flex items-center justify-center p-8 text-muted-foreground">
      No diff data available
    </div>
  {:else if parsedFiles.length === 0}
    <div class="flex items-center justify-center p-8 text-muted-foreground">No changed files</div>
  {:else if !poolReady}
    <div class="flex items-center justify-center p-8 text-muted-foreground">Loading diffs...</div>
  {:else}
    {#each parsedFiles as fileDiffMeta (fileDiffMeta.name)}
      <FileDiffSection
        {fileDiffMeta}
        {diff}
        {findings}
        {reviewOpen}
        {diffViewMode}
        isCurrentFile={currentFilePath === fileDiffMeta.name}
        viewed={viewedFiles.has(fileDiffMeta.name)}
        {activeForm}
        {onFileSelect}
        {onToggleViewed}
        {onFindingClick}
        {onResolve}
        {onDismiss}
        {onToggleDiffMode}
        onLineSelected={handleLineSelected}
        onFormSubmit={handleFormSubmit}
        onFormCancel={handleFormCancel}
      />
    {/each}
  {/if}
</div>
