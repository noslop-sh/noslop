<script lang="ts">
  import type {
    StructuredDiff,
    Finding,
    DiffViewMode,
    DismissReason,
    Severity,
    FileDiff as OurFileDiff,
  } from '$lib/types';
  import type { FileDiffMetadata, SelectedLineRange } from '@pierre/diffs';
  import { findingCountsByFile } from '$lib/helpers';
  import DiffFileHeader from './DiffFileHeader.svelte';
  import FileDiffRenderer from './FileDiffRenderer.svelte';
  import InlineFindingForm from './InlineFindingForm.svelte';

  interface Props {
    fileDiffMeta: FileDiffMetadata;
    diff: StructuredDiff | null;
    findings: Finding[];
    reviewOpen: boolean;
    diffViewMode: DiffViewMode;
    isCurrentFile: boolean;
    viewed: boolean;
    activeForm: { filePath: string; startLine: number; endLine: number } | null;
    onFileSelect: (path: string) => void;
    onToggleViewed: (path: string) => void;
    onFindingClick: (id: string) => void;
    onResolve: (findingId: string) => void;
    onDismiss: (findingId: string, reason: DismissReason) => void;
    onToggleDiffMode: () => void;
    onLineSelected: (filePath: string, range: SelectedLineRange | null) => void;
    onFormSubmit: (message: string, severity: Severity) => Promise<void>;
    onFormCancel: () => void;
  }

  let {
    fileDiffMeta,
    diff,
    findings,
    reviewOpen,
    diffViewMode,
    isCurrentFile,
    viewed,
    activeForm,
    onFileSelect,
    onToggleViewed,
    onFindingClick,
    onResolve,
    onDismiss,
    onToggleDiffMode,
    onLineSelected,
    onFormSubmit,
    onFormCancel,
  }: Props = $props();

  let headerRef = $state<HTMLElement | null>(null);

  let fileName = $derived(fileDiffMeta.name);

  let ourFileDiff = $derived<OurFileDiff | undefined>(diff?.files.find((f) => f.path === fileName));

  let fileFindingCounts = $derived(findingCountsByFile(findings, fileName));

  let showForm = $derived(activeForm !== null && activeForm.filePath === fileName);

  // Scroll to file when it becomes the current file
  $effect(() => {
    if (isCurrentFile && headerRef) {
      headerRef.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  });

  function handleLineSelected(range: SelectedLineRange | null): void {
    onLineSelected(fileName, range);
  }
</script>

<div bind:this={headerRef}>
  {#if ourFileDiff}
    <DiffFileHeader
      fileDiff={ourFileDiff}
      findingCounts={fileFindingCounts}
      {viewed}
      {diffViewMode}
      onToggleViewed={() => onToggleViewed(fileName)}
      {onToggleDiffMode}
    />
  {/if}
</div>

<div style="content-visibility: auto; contain-intrinsic-block-size: auto 300px;">
  <FileDiffRenderer
    {fileDiffMeta}
    {findings}
    {reviewOpen}
    {diffViewMode}
    {onFindingClick}
    {onResolve}
    {onDismiss}
    onLineSelected={handleLineSelected}
  />

  {#if showForm && activeForm}
    <div class="px-4 py-2">
      <InlineFindingForm
        filePath={activeForm.filePath}
        startLine={activeForm.startLine}
        endLine={activeForm.endLine}
        onSubmit={onFormSubmit}
        onCancel={onFormCancel}
      />
    </div>
  {/if}
</div>
