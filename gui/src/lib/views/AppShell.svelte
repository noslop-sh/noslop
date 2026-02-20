<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Review, StructuredDiff, SidebarCollapseState } from '$lib/types';
  import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
  import { Button } from '$lib/components/ui/button';
  import { PanelLeftClose, PanelLeft } from '@lucide/svelte';
  import StatusBar from './StatusBar.svelte';
  import ActionBar from './ActionBar.svelte';

  interface Props {
    review: Review | null;
    diff: StructuredDiff | null;
    sidebarContent: Snippet;
    children: Snippet;
    onClose: () => void;
    onScrollToBlocker: () => void;
    sidebarWidth: number;
    sidebarCollapseState: SidebarCollapseState;
    onSidebarWidthChange: (width: number) => void;
    onCycleSidebar: () => void;
    baseBranch: string;
    compareBranch: string;
    branches: string[];
    onBaseChange: (branch: string) => void;
    onCompareChange: (branch: string) => void;
  }

  let {
    review,
    diff,
    sidebarContent,
    children,
    onClose,
    onScrollToBlocker,
    sidebarWidth,
    sidebarCollapseState,
    onSidebarWidthChange,
    onCycleSidebar,
    baseBranch,
    compareBranch,
    branches,
    onBaseChange,
    onCompareChange,
  }: Props = $props();

  let isCompactStatusBar = $state(false);
  let isThemeDark = $state(false);

  let isFull = $derived(sidebarCollapseState === 'full');
  let isMini = $derived(sidebarCollapseState === 'mini');
  let isHidden = $derived(sidebarCollapseState === 'hidden');

  // Compute the sidebar size as a percentage of a reference width (assume ~1200px container).
  // PaneGroup works with percentages, so we convert pixel widths.
  let sidebarMinSize = $derived(isHidden ? 0 : isMini ? 3.5 : 15);
  let sidebarDefaultSize = $derived(isHidden ? 0 : isMini ? 3.5 : (sidebarWidth / 1200) * 100);

  function handleResize(sizes: number[]) {
    if (sizes[0] !== undefined && isFull) {
      // Convert percentage back to approximate pixel width
      const newWidth = Math.round((sizes[0] / 100) * 1200);
      onSidebarWidthChange(Math.max(200, Math.min(500, newWidth)));
    }
  }

  function toggleTheme() {
    isThemeDark = !isThemeDark;
    if (isThemeDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  function handleMainScroll(e: Event) {
    const target = e.target as HTMLElement;
    isCompactStatusBar = target.scrollTop > 120;
  }
</script>

<div class="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
  <!-- Status Bar -->
  <StatusBar
    {review}
    isCompact={isCompactStatusBar}
    {baseBranch}
    {compareBranch}
    {branches}
    {onBaseChange}
    {onCompareChange}
    onToggleTheme={toggleTheme}
  />

  <!-- Main area: sidebar + content -->
  <div class="relative flex flex-1 overflow-hidden">
    {#if isHidden}
      <!-- Hidden sidebar: just show a toggle button and the main content -->
      <div class="absolute left-2 top-2 z-20">
        <Button variant="ghost" size="icon-sm" onclick={onCycleSidebar} aria-label="Show sidebar">
          <PanelLeft class="size-4" />
        </Button>
      </div>
      <main class="flex-1 overflow-y-auto" onscroll={handleMainScroll}>
        {@render children()}
      </main>
    {:else}
      <ResizablePaneGroup direction="horizontal" onLayoutChange={handleResize}>
        <ResizablePane
          defaultSize={sidebarDefaultSize}
          minSize={sidebarMinSize}
          maxSize={isMini ? 3.5 : 40}
          collapsible={false}
        >
          <aside
            class="flex h-full flex-col border-r border-border transition-all duration-200"
            class:bg-sidebar={isFull}
            class:w-full={true}
          >
            <!-- Sidebar header with collapse toggle -->
            <div class="flex h-10 shrink-0 items-center border-b border-border px-2">
              {#if isFull}
                <span class="flex-1 truncate px-1 text-xs font-medium text-sidebar-foreground">
                  Files
                </span>
              {/if}
              <Button
                variant="ghost"
                size="icon-sm"
                onclick={onCycleSidebar}
                aria-label={isFull ? 'Collapse sidebar' : 'Expand sidebar'}
              >
                <PanelLeftClose class="size-3.5" />
              </Button>
            </div>

            <!-- Sidebar content -->
            <div class="flex-1 overflow-y-auto overflow-x-hidden">
              {#if isFull}
                {@render sidebarContent()}
              {:else if isMini}
                <!-- Mini mode: render sidebar content in a narrow strip -->
                <div class="flex flex-col items-center gap-0.5 py-1">
                  {@render sidebarContent()}
                </div>
              {/if}
            </div>
          </aside>
        </ResizablePane>

        <ResizableHandle withHandle={isFull} />

        <ResizablePane defaultSize={100 - sidebarDefaultSize} minSize={50}>
          <main class="h-full overflow-y-auto" onscroll={handleMainScroll}>
            {@render children()}
          </main>
        </ResizablePane>
      </ResizablePaneGroup>
    {/if}
  </div>

  <!-- Action Bar -->
  <ActionBar {review} {onClose} {onScrollToBlocker} />
</div>
