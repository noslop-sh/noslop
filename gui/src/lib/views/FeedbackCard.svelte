<script lang="ts">
  import type { Feedback, DismissReason } from '$lib/types';
  import {
    formatSource,
    severityIcon,
    severityColor,
    formatReason,
    DISMISS_OPTIONS,
  } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { Check, ChevronDown } from '@lucide/svelte';
  import { slide } from 'svelte/transition';
  import { cn } from '$lib/utils';

  interface Props {
    feedback: Feedback;
    reviewId: string;
    expanded: boolean;
    focused: boolean;
    onToggleExpand: () => void;
    onResolve: () => void;
    onDismiss: (reason: DismissReason) => void;
  }

  let { feedback, reviewId, expanded, focused, onToggleExpand, onResolve, onDismiss }: Props =
    $props();

  let dismissOpen = $state(false);

  let icon = $derived(severityIcon(feedback.severity, feedback.source.kind));
  let color = $derived(severityColor(feedback.severity, feedback.source.kind));
  let sourceDisplay = $derived(formatSource(feedback.source));

  let isOpen = $derived(feedback.status === 'open');
  let isResolved = $derived(feedback.status === 'resolved');
  let isDismissed = $derived(feedback.status === 'dismissed');

  let cardClasses = $derived(
    cn(
      'group relative rounded-lg border px-3 py-2 transition-all',
      isOpen && 'border-border bg-card',
      isResolved && 'border-green-500/30 bg-green-50/50 opacity-50 dark:bg-green-950/20',
      isDismissed && 'border-muted bg-muted/50 opacity-60',
      focused && 'ring-2 ring-ring',
      isOpen && 'cursor-pointer hover:bg-accent/50'
    )
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class={cardClasses} onclick={onToggleExpand}>
  <!-- Header row (always visible) -->
  <div class="flex items-center gap-2">
    <span class={cn('text-sm font-bold leading-none', color)} aria-label={feedback.severity}>
      {icon}
    </span>
    <span class="text-xs font-semibold uppercase tracking-wide text-foreground">
      {feedback.severity}
    </span>
    <span class={cn('flex-1 truncate text-sm', isDismissed && 'line-through')}>
      {feedback.message}
    </span>
    <span class="shrink-0 text-xs text-muted-foreground">
      {sourceDisplay}
    </span>

    {#if isResolved}
      <span class="shrink-0 text-green-600 dark:text-green-400">
        <Check class="size-4" />
      </span>
    {/if}

    {#if isOpen}
      <Button
        variant="ghost"
        size="sm"
        class="h-6 shrink-0 px-2 text-xs"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          onResolve();
        }}
      >
        Resolve
      </Button>
    {/if}

    {#if expanded && isOpen}
      <div onclick={(e: MouseEvent) => e.stopPropagation()} role="presentation">
        <DropdownMenu.Root bind:open={dismissOpen}>
          <DropdownMenu.Trigger>
            {#snippet children()}
              <Button variant="outline" size="sm" class="h-6 shrink-0 gap-1 px-2 text-xs">
                Dismiss
                <ChevronDown class="size-3" />
              </Button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end">
            {#each DISMISS_OPTIONS as opt (opt.reason)}
              <DropdownMenu.Item
                onclick={() => {
                  onDismiss(opt.reason);
                  dismissOpen = false;
                }}
              >
                {opt.label}
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    {/if}
  </div>

  <!-- Expanded detail -->
  {#if expanded}
    <div transition:slide={{ duration: 200 }} class="mt-3">
      <div class="mb-2 text-xs text-muted-foreground">
        source: {sourceDisplay}
      </div>

      <hr class="mb-3 border-border" />

      <p class="whitespace-pre-wrap text-sm text-foreground">
        {feedback.message}
      </p>

      {#if feedback.suggestion}
        <div class="mt-3">
          <p class="mb-1 text-xs font-medium text-muted-foreground">Suggestion:</p>
          <div class="rounded border border-border bg-muted/50 p-3">
            <pre class="overflow-x-auto whitespace-pre text-xs font-mono text-foreground">{feedback
                .suggestion.replacement}</pre>
          </div>
        </div>
      {/if}

      {#if feedback.notes.length > 0}
        <div class="mt-3">
          <p class="mb-1 text-xs font-medium text-muted-foreground">Notes:</p>
          <div class="space-y-1">
            {#each feedback.notes as note (note.id)}
              <p class="text-xs text-muted-foreground">{note.content}</p>
            {/each}
          </div>
        </div>
      {/if}

      {#if isDismissed && feedback.dismiss_reason}
        <div class="mt-2 text-xs italic text-muted-foreground">
          Dismissed: {formatReason(feedback.dismiss_reason)}
        </div>
      {/if}

      {#if isResolved && feedback.resolution_reason}
        <div class="mt-2 text-xs italic text-green-600 dark:text-green-400">
          Resolved: {formatReason(feedback.resolution_reason)}
        </div>
      {/if}
    </div>
  {/if}
</div>
