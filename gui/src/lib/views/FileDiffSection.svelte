<script lang="ts">
  import type {
    StructuredDiff,
    Feedback,
    DiffViewMode,
    DismissReason,
    Severity,
    FileDiff as OurFileDiff,
  } from '$lib/types';
  import type { FileDiffMetadata, SelectedLineRange } from '@pierre/diffs';
  import { feedbackCountsByFile } from '$lib/helpers';
  import DiffFileHeader from './DiffFileHeader.svelte';
  import FileDiffRenderer from './FileDiffRenderer.svelte';
  import InlineFeedbackForm from './InlineFeedbackForm.svelte';

  interface Props {
    fileDiffMeta: FileDiffMetadata;
    diff: StructuredDiff | null;
    feedbacks: Feedback[];
    reviewOpen: boolean;
    diffViewMode: DiffViewMode;
    isCurrentFile: boolean;
    viewed: boolean;
    activeForm: { filePath: string; startLine: number; endLine: number } | null;
    onFileSelect: (path: string) => void;
    onToggleViewed: (path: string) => void;
    onFeedbackClick: (id: string) => void;
    onResolve: (feedbackId: string) => void;
    onDismiss: (feedbackId: string, reason: DismissReason) => void;
    onToggleDiffMode: () => void;
    onLineSelected: (filePath: string, range: SelectedLineRange | null) => void;
    onFormSubmit: (message: string, severity: Severity) => Promise<void>;
    onFormCancel: () => void;
  }

  let {
    fileDiffMeta,
    diff,
    feedbacks,
    reviewOpen,
    diffViewMode,
    isCurrentFile,
    viewed,
    activeForm,
    onFileSelect,
    onToggleViewed,
    onFeedbackClick,
    onResolve,
    onDismiss,
    onToggleDiffMode,
    onLineSelected,
    onFormSubmit,
    onFormCancel,
  }: Props = $props();

  let headerRef = $state<HTMLElement | null>(null);
  let wrapperRef = $state<HTMLElement | null>(null);
  let diffContainerRef = $state<HTMLElement | null>(null);
  let formRef = $state<HTMLElement | null>(null);
  let formTop = $state<number | null>(null);

  let fileName = $derived(fileDiffMeta.name);

  let ourFileDiff = $derived<OurFileDiff | undefined>(diff?.files.find((f) => f.path === fileName));

  let fileFeedbackCounts = $derived(feedbackCountsByFile(feedbacks, fileName));

  let showForm = $derived(activeForm !== null && activeForm.filePath === fileName);

  // Scroll to file when it becomes the current file
  $effect(() => {
    if (isCurrentFile && headerRef) {
      headerRef.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  });

  // Calculate form position from selected lines in Shadow DOM
  $effect(() => {
    if (!showForm || !diffContainerRef || !wrapperRef) {
      formTop = null;
      return;
    }

    const diffsContainer = diffContainerRef.querySelector('diffs-container');
    const shadowRoot = diffsContainer?.shadowRoot;
    if (!shadowRoot) {
      formTop = null;
      return;
    }

    const selectedLines = shadowRoot.querySelectorAll('[data-selected-line]');
    if (selectedLines.length === 0) {
      formTop = null;
      return;
    }

    const lastLine = selectedLines[selectedLines.length - 1];
    const lineRect = lastLine.getBoundingClientRect();
    const wrapperRect = wrapperRef.getBoundingClientRect();
    formTop = lineRect.bottom - wrapperRect.top;
  });

  // Scroll form into view when it appears
  $effect(() => {
    if (showForm && formRef) {
      formRef.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
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
      feedbackCounts={fileFeedbackCounts}
      {viewed}
      {diffViewMode}
      onToggleViewed={() => onToggleViewed(fileName)}
      {onToggleDiffMode}
    />
  {/if}
</div>

<div bind:this={wrapperRef} class="relative">
  <div style="content-visibility: auto; contain-intrinsic-block-size: auto 300px;">
    <div bind:this={diffContainerRef}>
      <FileDiffRenderer
        {fileDiffMeta}
        {feedbacks}
        {reviewOpen}
        {diffViewMode}
        {onFeedbackClick}
        {onResolve}
        {onDismiss}
        onLineSelected={handleLineSelected}
      />
    </div>
  </div>

  {#if showForm && activeForm}
    <div
      bind:this={formRef}
      class="absolute left-0 right-0 z-20 px-4 py-2"
      style={formTop !== null ? `top: ${formTop}px` : ''}
    >
      <InlineFeedbackForm
        filePath={activeForm.filePath}
        startLine={activeForm.startLine}
        endLine={activeForm.endLine}
        onSubmit={onFormSubmit}
        onCancel={onFormCancel}
      />
    </div>
  {/if}
</div>
