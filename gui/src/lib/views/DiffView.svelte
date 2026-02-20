<script lang="ts">
  import type {
    StructuredDiff,
    Finding,
    DiffViewMode,
    DismissReason,
    Severity,
    FileDiff as OurFileDiff,
  } from '$lib/types';
  import { findingCountsByFile } from '$lib/helpers';
  import DiffFileHeader from './DiffFileHeader.svelte';
  import InlineFindingComment from './InlineFindingComment.svelte';
  import InlineFindingForm from './InlineFindingForm.svelte';
  import { onDestroy, mount, unmount } from 'svelte';
  import {
    FileDiff,
    parsePatchFiles,
    type FileDiffMetadata,
    type DiffLineAnnotation,
    type SelectedLineRange,
    type FileDiffOptions,
  } from '@pierre/diffs';
  import { getOrCreateWorkerPoolSingleton, type WorkerPoolManager } from '@pierre/diffs/worker';

  interface AnnotationMeta {
    finding: Finding;
  }

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

  // ---------------------------------------------------------------------------
  // Worker pool — eagerly initialized, shared across all FileDiff instances
  // ---------------------------------------------------------------------------

  let workerPool: WorkerPoolManager | null = null;

  function getWorkerPool(): WorkerPoolManager {
    if (!workerPool) {
      workerPool = getOrCreateWorkerPoolSingleton({
        poolOptions: {
          workerFactory: () =>
            new Worker(new URL('@pierre/diffs/worker/worker.js', import.meta.url), {
              type: 'module',
            }),
          poolSize: 4,
        },
        highlighterOptions: {
          theme: { dark: 'github-dark', light: 'github-light' },
        },
      });
      // Eagerly initialize so workers are warm before files scroll into view
      workerPool.initialize();
    }
    return workerPool;
  }

  // Warm up the pool immediately
  getWorkerPool();

  // ---------------------------------------------------------------------------
  // Parse patch into FileDiffMetadata[]
  // ---------------------------------------------------------------------------

  let parsedFiles = $derived.by(() => {
    if (!rawPatch) return [];
    const patches = parsePatchFiles(rawPatch, 'diff');
    return patches.flatMap((p) => p.files);
  });

  // Look up our FileDiff type from StructuredDiff for file headers
  function findOurFileDiff(fileName: string): OurFileDiff | undefined {
    return diff?.files.find((f) => f.path === fileName);
  }

  // ---------------------------------------------------------------------------
  // Active inline form state (line selection for finding creation)
  // ---------------------------------------------------------------------------

  let activeForm = $state<{
    filePath: string;
    startLine: number;
    endLine: number;
  } | null>(null);

  // ---------------------------------------------------------------------------
  // FileDiff instance tracking
  // ---------------------------------------------------------------------------

  let instances = new Map<string, FileDiff<AnnotationMeta>>();
  let instanceOptions = new Map<string, FileDiffOptions<AnnotationMeta>>();
  let mountedComponents: Array<Record<string, unknown>> = [];

  // File header refs for scroll-to-file
  let fileHeaderRefs = $state<Record<string, HTMLElement>>({});

  // Scroll to file when currentFilePath changes
  $effect(() => {
    if (currentFilePath && fileHeaderRefs[currentFilePath]) {
      fileHeaderRefs[currentFilePath].scrollIntoView({
        behavior: 'smooth',
        block: 'start',
      });
    }
  });

  // ---------------------------------------------------------------------------
  // Build annotations from findings for a given file
  // ---------------------------------------------------------------------------

  function buildAnnotations(filePath: string): DiffLineAnnotation<AnnotationMeta>[] {
    return findings
      .filter((f) => f.target.path === filePath && f.target.span !== null)
      .map((f) => ({
        side: 'additions' as const,
        lineNumber: f.target.span!.start,
        metadata: { finding: f },
      }));
  }

  // ---------------------------------------------------------------------------
  // renderAnnotation callback — imperatively mounts Svelte components into DOM
  // ---------------------------------------------------------------------------

  function renderAnnotation(
    annotation: DiffLineAnnotation<AnnotationMeta>
  ): HTMLElement | undefined {
    if (!annotation.metadata) return undefined;

    const { finding } = annotation.metadata;
    const wrapper = document.createElement('div');
    wrapper.style.padding = '4px 8px';

    const component = mount(InlineFindingComment, {
      target: wrapper,
      props: {
        finding,
        onResolve: () => onResolve(finding.id),
        onDismiss: (reason: DismissReason) => onDismiss(finding.id, reason),
        onclick: () => onFindingClick(finding.id),
      },
    });
    mountedComponents.push(component as unknown as Record<string, unknown>);

    return wrapper;
  }

  // ---------------------------------------------------------------------------
  // Deferred rendering queue — renders files near the viewport first via
  // IntersectionObserver, then pre-renders remaining files in idle time so
  // scrolling through many files never hits an unrendered placeholder.
  // ---------------------------------------------------------------------------

  type PendingFile = { node: HTMLElement; fileDiffMeta: FileDiffMetadata; fileName: string };
  let pendingQueue: PendingFile[] = [];
  let idleCallbackId: number | null = null;

  function drainQueue(): void {
    if (pendingQueue.length === 0) {
      idleCallbackId = null;
      return;
    }
    const item = pendingQueue.shift()!;
    createFileDiffInstance(item.node, item.fileDiffMeta, item.fileName);
    // Yield between files so we don't block the main thread
    idleCallbackId = requestIdleCallback(drainQueue, { timeout: 200 });
  }

  function startIdleDrain(): void {
    if (idleCallbackId != null) return;
    idleCallbackId = requestIdleCallback(drainQueue, { timeout: 200 });
  }

  function createFileDiffInstance(
    node: HTMLElement,
    fileDiffMeta: FileDiffMetadata,
    fileName: string
  ): FileDiff<AnnotationMeta> | null {
    if (instances.has(fileName)) return instances.get(fileName)!;

    const pool = getWorkerPool();
    const annotations = buildAnnotations(fileName);

    const options: FileDiffOptions<AnnotationMeta> = {
      diffStyle: diffViewMode === 'split' ? 'split' : 'unified',
      overflow: 'scroll',
      themeType: 'dark',
      enableLineSelection: reviewOpen,
      disableFileHeader: true,
      renderAnnotation,
      onLineSelected: (range: SelectedLineRange | null) => {
        if (!range) {
          activeForm = null;
          return;
        }
        activeForm = {
          filePath: fileName,
          startLine: range.start,
          endLine: range.end,
        };
      },
    };

    const instance = new FileDiff<AnnotationMeta>(options, pool);
    instances.set(fileName, instance);
    instanceOptions.set(fileName, options);

    instance.render({
      fileDiff: fileDiffMeta,
      lineAnnotations: annotations,
      containerWrapper: node,
    });

    node.style.minHeight = '';
    return instance;
  }

  // Svelte action: files near the viewport render immediately via
  // IntersectionObserver; files far away get queued for idle pre-rendering.
  function initFileDiff(
    node: HTMLElement,
    params: { fileDiffMeta: FileDiffMetadata; fileName: string }
  ) {
    const { fileDiffMeta, fileName } = params;

    // Estimate height so scroll position is roughly correct before render
    const estimatedLines = fileDiffMeta.unifiedLineCount || fileDiffMeta.splitLineCount || 20;
    node.style.minHeight = `${Math.min(estimatedLines * 20, 600)}px`;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          // Remove from idle queue if it was queued — we need it now
          pendingQueue = pendingQueue.filter((p) => p.fileName !== fileName);
          createFileDiffInstance(node, fileDiffMeta, fileName);
          observer.disconnect();
        }
      },
      { rootMargin: '2000px' }
    );

    observer.observe(node);

    // Also enqueue for idle pre-rendering (fires if user doesn't scroll here first)
    pendingQueue.push({ node, fileDiffMeta, fileName });
    startIdleDrain();

    return {
      destroy() {
        observer.disconnect();
        pendingQueue = pendingQueue.filter((p) => p.fileName !== fileName);
        const instance = instances.get(fileName);
        if (instance) {
          instance.cleanUp();
          instances.delete(fileName);
          instanceOptions.delete(fileName);
        }
      },
    };
  }

  // ---------------------------------------------------------------------------
  // React to diffViewMode changes
  // ---------------------------------------------------------------------------

  $effect(() => {
    const style = diffViewMode === 'split' ? 'split' : 'unified';
    for (const [fileName, instance] of instances) {
      const opts = instanceOptions.get(fileName);
      if (!opts) continue;
      const newOpts = { ...opts, diffStyle: style } as FileDiffOptions<AnnotationMeta>;
      instanceOptions.set(fileName, newOpts);
      instance.setOptions(newOpts);
      instance.rerender();
    }
  });

  // ---------------------------------------------------------------------------
  // React to findings changes — update annotations
  // ---------------------------------------------------------------------------

  $effect(() => {
    const _f = findings;

    for (const comp of mountedComponents) {
      try {
        unmount(comp as any);
      } catch {
        // ignore
      }
    }
    mountedComponents = [];

    for (const [fileName, instance] of instances) {
      const annotations = buildAnnotations(fileName);
      instance.setLineAnnotations(annotations);
      instance.rerender();
    }
  });

  // ---------------------------------------------------------------------------
  // Cleanup on destroy
  // ---------------------------------------------------------------------------

  onDestroy(() => {
    if (idleCallbackId != null) cancelIdleCallback(idleCallbackId);
    pendingQueue = [];

    for (const [, instance] of instances) {
      instance.cleanUp();
    }
    instances.clear();
    instanceOptions.clear();

    for (const comp of mountedComponents) {
      try {
        unmount(comp as any);
      } catch {
        // ignore
      }
    }
    mountedComponents = [];
  });

  // ---------------------------------------------------------------------------
  // Inline form submission
  // ---------------------------------------------------------------------------

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
    for (const [, instance] of instances) {
      instance.setSelectedLines(null);
    }
  }

  function handleFormCancel(): void {
    activeForm = null;
    for (const [, instance] of instances) {
      instance.setSelectedLines(null);
    }
  }
</script>

<div class="h-full overflow-y-auto">
  {#if !rawPatch}
    <div class="flex items-center justify-center p-8 text-muted-foreground">
      No diff data available
    </div>
  {:else if parsedFiles.length === 0}
    <div class="flex items-center justify-center p-8 text-muted-foreground">No changed files</div>
  {:else}
    {#each parsedFiles as fileDiffMeta, idx (fileDiffMeta.name)}
      {@const fileName = fileDiffMeta.name}
      {@const ourFileDiff = findOurFileDiff(fileName)}
      {@const fileFindingCounts = findingCountsByFile(findings, fileName)}

      <!-- File header anchor for scroll-to-file -->
      <div bind:this={fileHeaderRefs[fileName]}>
        {#if ourFileDiff}
          <DiffFileHeader
            fileDiff={ourFileDiff}
            findingCounts={fileFindingCounts}
            viewed={viewedFiles.has(fileName)}
            {diffViewMode}
            onToggleViewed={() => onToggleViewed(fileName)}
            {onToggleDiffMode}
          />
        {/if}
      </div>

      <!-- Lazy-rendered: FileDiff instance created when scrolled into view -->
      <div use:initFileDiff={{ fileDiffMeta, fileName }}></div>

      <!-- Inline finding form -->
      {#if activeForm && activeForm.filePath === fileName}
        <div class="px-4 py-2">
          <InlineFindingForm
            filePath={activeForm.filePath}
            startLine={activeForm.startLine}
            endLine={activeForm.endLine}
            onSubmit={handleFormSubmit}
            onCancel={handleFormCancel}
          />
        </div>
      {/if}
    {/each}
  {/if}
</div>
