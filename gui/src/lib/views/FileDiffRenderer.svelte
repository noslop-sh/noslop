<script lang="ts">
  import type { Feedback, DiffViewMode, DismissReason } from '$lib/types';
  import type { AnnotationMeta } from '$lib/diff/types';
  import type {
    FileDiffMetadata,
    DiffLineAnnotation,
    SelectedLineRange,
    FileDiffOptions,
  } from '@pierre/diffs';
  import { FileDiff } from '@pierre/diffs';
  import { onDestroy, untrack } from 'svelte';
  import { useWorkerPool } from '$lib/diff/worker-pool.svelte';
  import { buildAnnotationsForFile } from '$lib/diff/annotations';
  import { renderFeedbackAnnotation, type AnnotationCallbacksRef } from '$lib/diff/annotation-dom';

  interface Props {
    fileDiffMeta: FileDiffMetadata;
    feedbacks: Feedback[];
    reviewOpen: boolean;
    diffViewMode: DiffViewMode;
    onFeedbackClick: (id: string) => void;
    onResolve: (feedbackId: string) => void;
    onDismiss: (feedbackId: string, reason: DismissReason) => void;
    onLineSelected: (range: SelectedLineRange | null) => void;
  }

  let {
    fileDiffMeta,
    feedbacks,
    reviewOpen,
    diffViewMode,
    onFeedbackClick,
    onResolve,
    onDismiss,
    onLineSelected,
  }: Props = $props();

  const pool = useWorkerPool();
  const fileName = fileDiffMeta.name;

  let instance = $state<FileDiff<AnnotationMeta> | null>(null);

  let callbacksRef: AnnotationCallbacksRef = {
    onResolve: (id) => onResolve(id),
    onDismiss: (id, reason) => onDismiss(id, reason),
    onFeedbackClick: (id) => onFeedbackClick(id),
  };

  $effect(() => {
    callbacksRef.onResolve = (id) => onResolve(id);
    callbacksRef.onDismiss = (id, reason) => onDismiss(id, reason);
    callbacksRef.onFeedbackClick = (id) => onFeedbackClick(id);
  });

  function renderAnnotation(
    annotation: DiffLineAnnotation<AnnotationMeta>
  ): HTMLElement | undefined {
    if (!annotation.metadata) return undefined;
    const wrapper = document.createElement('div');
    wrapper.style.padding = '4px 8px';
    renderFeedbackAnnotation(wrapper, annotation.metadata.feedback, callbacksRef);
    return wrapper;
  }

  // ---------------------------------------------------------------------------
  // Synchronous init via Svelte action
  // Pool is guaranteed initialized by DiffView before children mount.
  // ---------------------------------------------------------------------------

  function initDiff(node: HTMLElement) {
    const annotations = buildAnnotationsForFile(feedbacks, fileDiffMeta.name, fileDiffMeta.type);

    const options: FileDiffOptions<AnnotationMeta> = {
      diffStyle: diffViewMode === 'split' ? 'split' : 'unified',
      overflow: 'scroll',
      themeType: 'dark',
      enableLineSelection: reviewOpen,
      disableFileHeader: true,
      renderAnnotation,
      onLineSelected: (range: SelectedLineRange | null) => {
        onLineSelected(range);
      },
    };

    const inst = new FileDiff<AnnotationMeta>(options, pool);
    inst.render({
      fileDiff: fileDiffMeta,
      lineAnnotations: annotations,
      containerWrapper: node,
    });
    instance = inst;

    return {
      destroy() {
        if (instance) {
          instance.cleanUp();
          instance = null;
        }
      },
    };
  }

  // ---------------------------------------------------------------------------
  // React to feedbacks changes
  // ---------------------------------------------------------------------------

  $effect(() => {
    const annots = buildAnnotationsForFile(feedbacks, fileDiffMeta.name, fileDiffMeta.type);
    const inst = untrack(() => instance);
    if (!inst) return;
    inst.setLineAnnotations(annots);
    inst.rerender();
  });

  // ---------------------------------------------------------------------------
  // React to diffViewMode/reviewOpen changes
  // ---------------------------------------------------------------------------

  $effect(() => {
    const style = diffViewMode === 'split' ? 'split' : 'unified';
    const selectable = reviewOpen;
    const inst = untrack(() => instance);
    if (!inst) return;
    inst.setOptions({
      ...inst.options,
      diffStyle: style,
      enableLineSelection: selectable,
    });
    inst.rerender();
  });

  // ---------------------------------------------------------------------------
  // Cleanup
  // ---------------------------------------------------------------------------

  onDestroy(() => {
    if (instance) {
      instance.cleanUp();
      instance = null;
    }
  });
</script>

<div use:initDiff></div>
