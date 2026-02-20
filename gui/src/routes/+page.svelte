<script lang="ts">
  import type {
    Review,
    StructuredDiff,
    SidebarCollapseState,
    DiffViewMode,
    ActiveFilters,
    DismissReason,
    PaletteCommand,
    Severity,
  } from '$lib/types';
  import { blockingFindings } from '$lib/helpers';
  import { getSession, updateSession } from '$lib/session';
  import { createReviewNavigation } from '$lib/stores/review-navigation.svelte';
  import { createKeyboardManager, type KeyboardActions } from '$lib/stores/keyboard.svelte';
  import { api } from '$lib/api';
  import {
    useStartReview,
    useCloseReview,
    useReopenReview,
    useResolveFinding,
    useDismissFinding,
    useMarkFileViewed,
    useAddFinding,
    useCurrentBranch,
    useBranches,
    useDefaultBranch,
  } from '$lib/queries';

  import AppShell from '$lib/views/AppShell.svelte';
  import BranchCompare from '$lib/views/BranchCompare.svelte';
  import FileTree from '$lib/views/FileTree.svelte';
  import FindingsPanel from '$lib/views/FindingsPanel.svelte';
  import ReviewLandingPage from '$lib/views/ReviewLandingPage.svelte';
  import DiffView from '$lib/views/DiffView.svelte';
  import NewReviewDialog from '$lib/views/NewReviewDialog.svelte';
  import CloseReviewDialog from '$lib/views/CloseReviewDialog.svelte';
  import BlockedCloseDialog from '$lib/views/BlockedCloseDialog.svelte';
  import CommandPalette from '$lib/views/CommandPalette.svelte';
  import ShortcutOverlay from '$lib/views/ShortcutOverlay.svelte';
  import { Separator } from '$lib/components/ui/separator';
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
  let findingsPanelCollapsed = $state(session.findings_panel_collapsed);
  let expandedFindingIds = $state<string[]>(session.expanded_finding_ids);

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
      findings_panel_collapsed: findingsPanelCollapsed,
      expanded_finding_ids: expandedFindingIds,
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
  const reopenReview = useReopenReview();
  const resolveFinding = useResolveFinding();
  const dismissFinding = useDismissFinding();
  const markFileViewed = useMarkFileViewed();
  const addFinding = useAddFinding();

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
    const blockers = blockingFindings(review.findings);
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

  async function handleResolve(findingId: string): Promise<void> {
    if (!selectedReviewId) return;
    await $resolveFinding.mutateAsync({ reviewId: selectedReviewId, findingId });
    await refetchReview();
  }

  async function handleDismiss(findingId: string, reason: DismissReason): Promise<void> {
    if (!selectedReviewId) return;
    await $dismissFinding.mutateAsync({ reviewId: selectedReviewId, findingId, reason });
    await refetchReview();
  }

  async function handleToggleViewed(path: string): Promise<void> {
    if (!selectedReviewId) return;
    nav.toggleViewed(path);
    await $markFileViewed.mutateAsync({ reviewId: selectedReviewId, path });
    await refetchReview();
  }

  function handleFindingClick(id: string): void {
    nav.selectFinding(id);
    // Toggle expanded state
    if (expandedFindingIds.includes(id)) {
      expandedFindingIds = expandedFindingIds.filter((fid) => fid !== id);
    } else {
      expandedFindingIds = [...expandedFindingIds, id];
    }
  }

  async function handleSubmitFinding(
    filePath: string,
    startLine: number,
    endLine: number,
    message: string,
    severity: Severity
  ): Promise<void> {
    if (!selectedReviewId) return;
    await $addFinding.mutateAsync({
      reviewId: selectedReviewId,
      target: filePath,
      message,
      severity,
      startLine,
      endLine,
    });
    await refetchReview();
  }

  function handleScrollToBlocker(): void {
    if (!review) return;
    const blockers = blockingFindings(review.findings);
    if (blockers.length > 0) {
      nav.selectFinding(blockers[0].id);
      nav.selectFile(blockers[0].target.path);
      if (!expandedFindingIds.includes(blockers[0].id)) {
        expandedFindingIds = [...expandedFindingIds, blockers[0].id];
      }
      showBlockedDialog = false;
    }
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

  const findings = $derived(review?.findings ?? []);
  const files = $derived(diff?.files ?? []);

  const keyboardActions: KeyboardActions = {
    nextFile: () => nav.nextFile(files),
    prevFile: () => nav.prevFile(files),
    nextFinding: () => nav.nextFinding(findings),
    prevFinding: () => nav.prevFinding(findings),
    nextUnresolved: () => nav.nextUnresolved(findings),
    prevUnresolved: () => nav.prevUnresolved(findings),
    resolveFocused: () => {
      if (nav.currentFindingId) handleResolve(nav.currentFindingId);
    },
    dismissFocused: () => {
      // For keyboard shortcut, we can't show dropdown - use 'false_positive' as default
      // Users can dismiss with reason via the card UI
    },
    addFindingOnLine: () => {
      // Line selection and finding creation is handled inline by DiffView
    },
    toggleViewed: () => {
      if (nav.currentFilePath) handleToggleViewed(nav.currentFilePath);
    },
    cycleSidebar,
    toggleDiffMode,
    toggleWhitespace: () => nav.toggleWhitespaceVisibility(),
    expandFocused: () => {
      if (nav.currentFindingId) {
        handleFindingClick(nav.currentFindingId);
      }
    },
    collapseFocused: () => {
      if (nav.currentFindingId && expandedFindingIds.includes(nav.currentFindingId)) {
        expandedFindingIds = expandedFindingIds.filter((id) => id !== nav.currentFindingId);
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
    ...findings
      .filter((f) => f.status === 'open')
      .slice(0, 20)
      .map((f) => ({
        id: `finding-${f.id}`,
        label: `${f.severity}: ${f.message.slice(0, 60)}`,
        group: 'findings' as const,
        action: () => handleFindingClick(f.id),
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
            findings={review!.findings}
            viewedFiles={nav.viewedFiles}
            selectedPath={nav.currentFilePath}
            sortMode={nav.sortMode}
            filterText={nav.filterText}
            onFileSelect={(path) => nav.selectFile(path)}
            onSortChange={(mode) => nav.setSortMode(mode)}
            onFilterChange={(text) => nav.setFilterText(text)}
          />
        </div>
      {/if}

      <!-- Findings panel (collapsible) -->
      <div class="border-t border-border">
        <button
          type="button"
          class="flex w-full items-center gap-1 px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground"
          onclick={() => (findingsPanelCollapsed = !findingsPanelCollapsed)}
        >
          {#if findingsPanelCollapsed}
            <ChevronRight class="size-3" />
          {:else}
            <ChevronDown class="size-3" />
          {/if}
          Findings
        </button>
        {#if !findingsPanelCollapsed}
          <div class="max-h-72 overflow-hidden">
            <FindingsPanel
              findings={review!.findings}
              reviewId={review!.id}
              {activeFilters}
              focusedFindingId={nav.currentFindingId}
              onFindingClick={handleFindingClick}
              onFilterChange={(filters) => (activeFilters = filters)}
              onResolve={handleResolve}
              onDismiss={handleDismiss}
            />
          </div>
        {/if}
      </div>
    {/snippet}

    <!-- Main content: landing page + continuous diff -->
    <ReviewLandingPage
      {review}
      {diff}
      onClose={handleCloseAttempt}
      onScrollToBlocker={handleScrollToBlocker}
    />

    {#if diff}
      <Separator />
      <DiffView
        {rawPatch}
        {diff}
        findings={review.findings}
        reviewId={review.id}
        reviewOpen={review.status === 'open'}
        currentFilePath={nav.currentFilePath}
        viewedFiles={nav.viewedFiles}
        {diffViewMode}
        focusedFindingId={nav.currentFindingId}
        onFileSelect={(path) => nav.selectFile(path)}
        onToggleViewed={handleToggleViewed}
        onFindingClick={handleFindingClick}
        onResolve={handleResolve}
        onDismiss={handleDismiss}
        onToggleDiffMode={toggleDiffMode}
        onSubmitFinding={handleSubmitFinding}
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
