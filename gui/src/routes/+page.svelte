<script lang="ts">
  import type {
    Review,
    ReviewView,
    StructuredDiff,
    SidebarCollapseState,
    DiffViewMode,
    ActiveFilters,
    DismissReason,
    PaletteCommand,
    Severity,
  } from '$lib/types';
  import { blockingFeedbacks } from '$lib/helpers';
  import { getSession, updateSession } from '$lib/session';
  import { createReviewNavigation } from '$lib/stores/review-navigation.svelte';
  import { createKeyboardManager, type KeyboardActions } from '$lib/stores/keyboard.svelte';
  import { api } from '$lib/api';
  import {
    useStartReview,
    useCloseReview,
    useResolveFeedback,
    useDismissFeedback,
    useMarkFileViewed,
    useAddFeedback,
    useAddFeedbackNote,
    useCurrentBranch,
    useBranches,
    useDefaultBranch,
  } from '$lib/queries';

  import AppShell from '$lib/views/AppShell.svelte';
  import BranchCompare from '$lib/views/BranchCompare.svelte';
  import FileTree from '$lib/views/FileTree.svelte';
  import FeedbackPanel from '$lib/views/FeedbackPanel.svelte';
  import ReviewLandingPage from '$lib/views/ReviewLandingPage.svelte';
  import DiffView from '$lib/views/DiffView.svelte';
  import NewReviewDialog from '$lib/views/NewReviewDialog.svelte';
  import CloseReviewDialog from '$lib/views/CloseReviewDialog.svelte';
  import BlockedCloseDialog from '$lib/views/BlockedCloseDialog.svelte';
  import CommandPalette from '$lib/views/CommandPalette.svelte';
  import ShortcutOverlay from '$lib/views/ShortcutOverlay.svelte';
  import { ChevronDown, ChevronRight } from '@lucide/svelte';

  // ---------------------------------------------------------------------------
  // Session & UI state
  // ---------------------------------------------------------------------------

  const session = getSession();

  let selectedReviewId = $state<string | null>(session.selected_review_id);
  let baseBranch = $state<string>(session.base_branch ?? '');
  let compareBranch = $state<string>(session.compare_branch ?? '');
  let sidebarWidth = $state(session.sidebar_width);
  let sidebarCollapseState = $state<SidebarCollapseState>(session.sidebar_collapse_state);
  let diffViewMode = $state<DiffViewMode>(session.diff_view_mode);
  let activeFilters = $state<ActiveFilters>(session.active_filters);
  let feedbackPanelCollapsed = $state(session.feedback_panel_collapsed);
  let expandedFeedbackIds = $state<string[]>(session.expanded_feedback_ids);
  let activeView = $state<ReviewView>(session.active_view);

  // Dialog state
  let showNewReviewDialog = $state(false);
  let showCloseDialog = $state(false);
  let showBlockedDialog = $state(false);
  let showCommandPalette = $state(false);
  let showShortcutOverlay = $state(false);

  // Branch resolution state
  let branchesLoading = $state(true);
  let branchError = $state<string | null>(null);
  let resolvingReview = $state(false);

  // Persist session changes
  $effect(() => {
    updateSession({
      selected_review_id: selectedReviewId,
      base_branch: baseBranch || null,
      compare_branch: compareBranch || null,
      sidebar_width: sidebarWidth,
      sidebar_collapse_state: sidebarCollapseState,
      diff_view_mode: diffViewMode,
      active_filters: activeFilters,
      feedback_panel_collapsed: feedbackPanelCollapsed,
      expanded_feedback_ids: expandedFeedbackIds,
      active_view: activeView,
    });
  });

  // ---------------------------------------------------------------------------
  // Data queries
  // ---------------------------------------------------------------------------

  const currentBranchQuery = useCurrentBranch();
  const defaultBranchQuery = useDefaultBranch();
  const branchesQuery = useBranches();
  const startReview = useStartReview();
  const closeReview = useCloseReview();
  const resolveFeedback = useResolveFeedback();
  const dismissFeedback = useDismissFeedback();
  const markFileViewed = useMarkFileViewed();
  const addFeedback = useAddFeedback();
  const addFeedbackNote = useAddFeedbackNote();

  const allBranches = $derived($branchesQuery.data ?? []);

  // Initialize branch defaults from query data
  $effect(() => {
    if (!baseBranch && $defaultBranchQuery.data) {
      baseBranch = $defaultBranchQuery.data;
    }
  });

  $effect(() => {
    if (!compareBranch && $currentBranchQuery.data) {
      compareBranch = $currentBranchQuery.data;
    }
  });

  // Track loading state
  $effect(() => {
    branchesLoading =
      $currentBranchQuery.isLoading || $defaultBranchQuery.isLoading || $branchesQuery.isLoading;
  });

  // Find or create review when branches change
  let _branchGen = 0;

  $effect(() => {
    const base = baseBranch;
    const compare = compareBranch;
    if (!base || !compare || base === compare) {
      selectedReviewId = null;
      branchError = null;
      resolvingReview = false;
      return;
    }

    const gen = ++_branchGen;
    branchError = null;
    resolvingReview = true;
    resolveReviewForBranches(base, compare, gen);
  });

  async function resolveReviewForBranches(
    base: string,
    compare: string,
    gen: number
  ): Promise<void> {
    try {
      const openReviews = await api.listReviews(true);
      if (gen !== _branchGen) return;

      const existing = openReviews.find((r) => r.base === base && r.head === compare);
      if (existing) {
        selectedReviewId = existing.id;
      } else {
        const newReview = await $startReview.mutateAsync({
          base,
          head: compare,
          branch: compare,
        });
        if (gen !== _branchGen) return;
        selectedReviewId = newReview.id;
      }
      resolvingReview = false;
    } catch (e) {
      if (gen !== _branchGen) return;
      branchError = e instanceof Error ? e.message : String(e);
      resolvingReview = false;
    }
  }

  // Review and diff data - loaded via direct API calls for reactive parameter support
  let review = $state<Review | null>(null);
  let diff = $state<StructuredDiff | null>(null);
  let rawPatch = $state<string | null>(null);

  // Load review when selectedReviewId changes
  $effect(() => {
    const id = selectedReviewId;
    if (!id) {
      review = null;
      diff = null;
      rawPatch = null;
      return;
    }
    api.getReview(id).then((r) => {
      if (selectedReviewId === id) review = r;
    });
  });

  // Load diff when review changes
  $effect(() => {
    const r = review;
    if (!r) {
      diff = null;
      rawPatch = null;
      return;
    }
    api.getStructuredDiff(r.base, r.head).then((d) => {
      if (review?.id === r.id) diff = d;
    });
    api.getDiff(r.base, r.head).then((patch) => {
      if (review?.id === r.id) rawPatch = patch;
    });
  });

  // Refetch review data (called after mutations)
  async function refetchReview(): Promise<void> {
    if (selectedReviewId) {
      review = await api.getReview(selectedReviewId);
    }
  }

  // ---------------------------------------------------------------------------
  // Navigation store
  // ---------------------------------------------------------------------------

  const nav = createReviewNavigation();

  // Sync viewed_files from review data into nav store
  $effect(() => {
    if (review?.viewed_files) {
      for (const path of review.viewed_files) {
        if (!nav.viewedFiles.has(path)) {
          nav.toggleViewed(path);
        }
      }
    }
  });

  // ---------------------------------------------------------------------------
  // Action handlers
  // ---------------------------------------------------------------------------

  async function handleStartReview(base: string, head: string): Promise<void> {
    const newReview = await $startReview.mutateAsync({ base, head });
    selectedReviewId = newReview.id;
    showNewReviewDialog = false;
  }

  function handleBaseChange(branch: string): void {
    baseBranch = branch;
  }

  function handleCompareChange(branch: string): void {
    compareBranch = branch;
  }

  function handleCloseAttempt(): void {
    if (!review) return;
    const blockers = blockingFeedbacks(review.feedbacks);
    if (blockers.length > 0) {
      showBlockedDialog = true;
    } else {
      showCloseDialog = true;
    }
  }

  async function handleConfirmClose(): Promise<void> {
    if (!selectedReviewId) return;
    await $closeReview.mutateAsync(selectedReviewId);
    showCloseDialog = false;
    await refetchReview();
  }

  async function handleResolve(feedbackId: string): Promise<void> {
    if (!selectedReviewId) return;
    await $resolveFeedback.mutateAsync({ reviewId: selectedReviewId, feedbackId });
    await refetchReview();
  }

  async function handleDismiss(feedbackId: string, reason: DismissReason): Promise<void> {
    if (!selectedReviewId) return;
    await $dismissFeedback.mutateAsync({ reviewId: selectedReviewId, feedbackId, reason });
    await refetchReview();
  }

  async function handleToggleViewed(path: string): Promise<void> {
    if (!selectedReviewId) return;
    nav.toggleViewed(path);
    await $markFileViewed.mutateAsync({ reviewId: selectedReviewId, path });
    await refetchReview();
  }

  function handleFeedbackClick(id: string): void {
    nav.selectFeedback(id);
    // Toggle expanded state
    if (expandedFeedbackIds.includes(id)) {
      expandedFeedbackIds = expandedFeedbackIds.filter((fid) => fid !== id);
    } else {
      expandedFeedbackIds = [...expandedFeedbackIds, id];
    }
    activeView = 'files';
  }

  async function handleSubmitFeedback(
    filePath: string,
    startLine: number,
    endLine: number,
    message: string,
    severity: Severity
  ): Promise<void> {
    if (!selectedReviewId) return;
    await $addFeedback.mutateAsync({
      reviewId: selectedReviewId,
      target: filePath,
      message,
      severity,
      startLine,
      endLine,
    });
    await refetchReview();
  }

  async function handleAddNote(feedbackId: string, content: string): Promise<void> {
    if (!selectedReviewId) return;
    await $addFeedbackNote.mutateAsync({ reviewId: selectedReviewId, feedbackId, content });
    await refetchReview();
  }

  function handleScrollToBlocker(): void {
    if (!review) return;
    const blockers = blockingFeedbacks(review.feedbacks);
    if (blockers.length > 0) {
      nav.selectFeedback(blockers[0].id);
      nav.selectFile(blockers[0].target.path);
      if (!expandedFeedbackIds.includes(blockers[0].id)) {
        expandedFeedbackIds = [...expandedFeedbackIds, blockers[0].id];
      }
      showBlockedDialog = false;
      activeView = 'files';
    }
  }

  function switchToFilesView(): void {
    activeView = 'files';
  }

  function cycleSidebar(): void {
    const states: SidebarCollapseState[] = ['full', 'mini', 'hidden'];
    const idx = states.indexOf(sidebarCollapseState);
    sidebarCollapseState = states[(idx + 1) % states.length];
  }

  function toggleDiffMode(): void {
    diffViewMode = diffViewMode === 'split' ? 'unified' : 'split';
  }

  // ---------------------------------------------------------------------------
  // Keyboard manager
  // ---------------------------------------------------------------------------

  const feedbacks = $derived(review?.feedbacks ?? []);
  const files = $derived(diff?.files ?? []);

  const keyboardActions: KeyboardActions = {
    nextFile: () => {
      nav.nextFile(files);
      switchToFilesView();
    },
    prevFile: () => {
      nav.prevFile(files);
      switchToFilesView();
    },
    nextFeedback: () => {
      nav.nextFeedback(feedbacks);
      switchToFilesView();
    },
    prevFeedback: () => {
      nav.prevFeedback(feedbacks);
      switchToFilesView();
    },
    nextUnresolved: () => {
      nav.nextUnresolved(feedbacks);
      switchToFilesView();
    },
    prevUnresolved: () => {
      nav.prevUnresolved(feedbacks);
      switchToFilesView();
    },
    resolveFocused: () => {
      if (nav.currentFeedbackId) handleResolve(nav.currentFeedbackId);
    },
    dismissFocused: () => {
      // For keyboard shortcut, we can't show dropdown - use 'false_positive' as default
      // Users can dismiss with reason via the card UI
    },
    addFeedbackOnLine: () => {
      // Line selection and feedback creation is handled inline by DiffView
    },
    toggleViewed: () => {
      if (nav.currentFilePath) handleToggleViewed(nav.currentFilePath);
    },
    cycleSidebar,
    toggleDiffMode,
    toggleWhitespace: () => nav.toggleWhitespaceVisibility(),
    expandFocused: () => {
      if (nav.currentFeedbackId) {
        handleFeedbackClick(nav.currentFeedbackId);
      }
    },
    collapseFocused: () => {
      if (nav.currentFeedbackId && expandedFeedbackIds.includes(nav.currentFeedbackId)) {
        expandedFeedbackIds = expandedFeedbackIds.filter((id) => id !== nav.currentFeedbackId);
      }
    },
    openCommandPalette: () => {
      showCommandPalette = true;
    },
    openFileJump: () => {
      showCommandPalette = true;
    },
    showShortcuts: () => {
      showShortcutOverlay = !showShortcutOverlay;
    },
    switchToSummary: () => {
      activeView = 'summary';
    },
    switchToFiles: () => {
      activeView = 'files';
    },
  };

  const keyboard = createKeyboardManager(keyboardActions);

  // ---------------------------------------------------------------------------
  // Command palette commands
  // ---------------------------------------------------------------------------

  let paletteCommands = $derived<PaletteCommand[]>([
    {
      id: 'close-review',
      label: 'Close Review',
      group: 'actions',
      action: handleCloseAttempt,
      available: () => !!review && review.status === 'open',
    },
    {
      id: 'new-review',
      label: 'New Review',
      group: 'actions',
      action: () => (showNewReviewDialog = true),
      available: () => true,
    },
    {
      id: 'toggle-dark-mode',
      label: 'Toggle Dark Mode',
      group: 'actions',
      action: () => document.documentElement.classList.toggle('dark'),
      available: () => true,
    },
    {
      id: 'toggle-split-view',
      label: 'Toggle Split/Unified',
      group: 'actions',
      shortcut: 's',
      action: toggleDiffMode,
      available: () => !!review,
    },
    {
      id: 'toggle-sidebar',
      label: 'Toggle Sidebar',
      group: 'actions',
      shortcut: 'f',
      action: cycleSidebar,
      available: () => true,
    },
    ...files.map((f) => ({
      id: `file-${f.path}`,
      label: f.path,
      group: 'files' as const,
      action: () => nav.selectFile(f.path),
      available: () => true,
    })),
    ...feedbacks
      .filter((f) => f.status === 'open')
      .slice(0, 20)
      .map((f) => ({
        id: `feedback-${f.id}`,
        label: `${f.severity}: ${f.message.slice(0, 60)}`,
        group: 'feedbacks' as const,
        action: () => handleFeedbackClick(f.id),
        available: () => true,
      })),
  ]);
</script>

<svelte:window onkeydown={keyboard.handleKeydown} />

{#if selectedReviewId && review}
  <AppShell
    {review}
    {diff}
    onClose={handleCloseAttempt}
    onScrollToBlocker={handleScrollToBlocker}
    {sidebarWidth}
    {sidebarCollapseState}
    onSidebarWidthChange={(w) => (sidebarWidth = w)}
    onCycleSidebar={cycleSidebar}
    {activeView}
    onViewChange={(v) => (activeView = v)}
    {baseBranch}
    {compareBranch}
    branches={allBranches}
    onBaseChange={handleBaseChange}
    onCompareChange={handleCompareChange}
  >
    {#snippet sidebarContent()}
      <!-- File tree (primary) -->
      {#if diff}
        <div class="flex-1 overflow-hidden">
          <FileTree
            files={diff.files}
            feedbacks={review!.feedbacks}
            viewedFiles={nav.viewedFiles}
            selectedPath={nav.currentFilePath}
            sortMode={nav.sortMode}
            filterText={nav.filterText}
            onFileSelect={(path) => {
              nav.selectFile(path);
              switchToFilesView();
            }}
            onSortChange={(mode) => nav.setSortMode(mode)}
            onFilterChange={(text) => nav.setFilterText(text)}
          />
        </div>
      {/if}

      <!-- Feedback panel (collapsible) -->
      <div class="border-t border-border">
        <button
          type="button"
          class="flex w-full items-center gap-1 px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground"
          onclick={() => (feedbackPanelCollapsed = !feedbackPanelCollapsed)}
        >
          {#if feedbackPanelCollapsed}
            <ChevronRight class="size-3" />
          {:else}
            <ChevronDown class="size-3" />
          {/if}
          Feedback
        </button>
        {#if !feedbackPanelCollapsed}
          <div class="max-h-72 overflow-hidden">
            <FeedbackPanel
              feedbacks={review!.feedbacks}
              reviewId={review!.id}
              {activeFilters}
              focusedFeedbackId={nav.currentFeedbackId}
              onFeedbackClick={handleFeedbackClick}
              onFilterChange={(filters) => (activeFilters = filters)}
              onResolve={handleResolve}
              onDismiss={handleDismiss}
            />
          </div>
        {/if}
      </div>
    {/snippet}

    <!-- Main content: conditional view rendering -->
    {#if activeView === 'summary'}
      <ReviewLandingPage
        {review}
        {diff}
        viewedFiles={nav.viewedFiles}
        onClose={handleCloseAttempt}
        onScrollToBlocker={handleScrollToBlocker}
        onFileClick={(path) => {
          nav.selectFile(path);
          switchToFilesView();
        }}
        onFeedbackClick={(id) => {
          const feedback = review!.feedbacks.find((f) => f.id === id);
          if (feedback) nav.selectFile(feedback.target.path);
          handleFeedbackClick(id);
          switchToFilesView();
        }}
        onResolve={handleResolve}
        onDismiss={handleDismiss}
        onAddNote={handleAddNote}
      />
    {:else if diff}
      <DiffView
        {rawPatch}
        {diff}
        feedbacks={review.feedbacks}
        reviewId={review.id}
        reviewOpen={review.status === 'open'}
        currentFilePath={nav.currentFilePath}
        viewedFiles={nav.viewedFiles}
        {diffViewMode}
        focusedFeedbackId={nav.currentFeedbackId}
        onFileSelect={(path) => nav.selectFile(path)}
        onToggleViewed={handleToggleViewed}
        onFeedbackClick={handleFeedbackClick}
        onResolve={handleResolve}
        onDismiss={handleDismiss}
        onToggleDiffMode={toggleDiffMode}
        onSubmitFeedback={handleSubmitFeedback}
      />
    {/if}
  </AppShell>
{:else}
  <!-- No review: show branch comparison selector -->
  <div class="flex h-screen flex-col items-center justify-center bg-background text-foreground">
    <div class="w-full max-w-sm space-y-4">
      <h1 class="text-center text-lg font-semibold">noslop</h1>
      <p class="text-center text-sm text-muted-foreground">
        Select branches to compare and start reviewing.
      </p>
      <BranchCompare
        {baseBranch}
        {compareBranch}
        branches={allBranches}
        loading={branchesLoading}
        onBaseChange={handleBaseChange}
        onCompareChange={handleCompareChange}
      />
      {#if branchError}
        <p class="text-center text-xs text-destructive">{branchError}</p>
      {:else if branchesLoading}
        <p class="text-center text-xs text-muted-foreground">Loading branches...</p>
      {:else if resolvingReview}
        <p class="text-center text-xs text-muted-foreground">Loading review...</p>
      {/if}
    </div>
  </div>
{/if}

<!-- Dialogs -->
<NewReviewDialog
  bind:open={showNewReviewDialog}
  onStart={handleStartReview}
  onCancel={() => (showNewReviewDialog = false)}
/>

{#if review}
  <CloseReviewDialog
    bind:open={showCloseDialog}
    {review}
    onClose={handleConfirmClose}
    onCancel={() => (showCloseDialog = false)}
  />

  <BlockedCloseDialog
    bind:open={showBlockedDialog}
    {review}
    onJumpToBlocker={handleScrollToBlocker}
    onCancel={() => (showBlockedDialog = false)}
  />
{/if}

<CommandPalette
  bind:open={showCommandPalette}
  commands={paletteCommands}
  onClose={() => (showCommandPalette = false)}
/>

<ShortcutOverlay bind:open={showShortcutOverlay} onClose={() => (showShortcutOverlay = false)} />
