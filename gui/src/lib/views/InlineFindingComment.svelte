<script lang="ts">
  import type { Finding, DismissReason, Severity } from '$lib/types';
  import { formatSource } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { Check, ChevronDown } from '@lucide/svelte';

  interface Props {
    finding: Finding;
    onResolve: () => void;
    onDismiss: (reason: DismissReason) => void;
    onclick?: () => void;
  }

  let { finding, onResolve, onDismiss, onclick }: Props = $props();

  const dismissOptions: { label: string; reason: DismissReason }[] = [
    { label: 'False positive', reason: 'false_positive' },
    { label: "Won't fix", reason: 'wont_fix' },
    { label: 'Not applicable', reason: 'not_applicable' },
  ];

  let dismissOpen = $state(false);

  function borderColor(severity: Severity, sourceKind: string): string {
    if (sourceKind === 'human') return 'border-l-[var(--finding-human)]';
    switch (severity) {
      case 'block':
        return 'border-l-[var(--finding-block)]';
      case 'warn':
        return 'border-l-[var(--finding-warn)]';
      case 'info':
        return 'border-l-[var(--finding-info)]';
    }
  }

  function severityColor(severity: Severity, sourceKind: string): string {
    if (sourceKind === 'human') return 'text-[var(--finding-human)]';
    switch (severity) {
      case 'block':
        return 'text-[var(--finding-block)]';
      case 'warn':
        return 'text-[var(--finding-warn)]';
      case 'info':
        return 'text-[var(--finding-info)]';
    }
  }

  let isOpen = $derived(finding.status === 'open');
  let isResolved = $derived(finding.status === 'resolved');
  let isDismissed = $derived(finding.status === 'dismissed');
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="border-l-3 rounded-sm bg-card px-3 py-2 text-sm {borderColor(
    finding.severity,
    finding.source.kind
  )}"
  class:opacity-50={isResolved || isDismissed}
  {onclick}
>
  <div class="flex items-center gap-2">
    <span
      class="text-xs font-bold uppercase {severityColor(finding.severity, finding.source.kind)}"
    >
      {finding.severity}
    </span>
    <span class="flex-1 truncate" class:line-through={isDismissed}>
      {finding.message}
    </span>
    <span class="shrink-0 text-xs text-muted-foreground">
      {formatSource(finding.source)}
    </span>

    {#if isResolved}
      <Check class="size-3.5 shrink-0 text-green-500" />
    {/if}

    {#if isOpen}
      <Button
        variant="ghost"
        size="sm"
        class="h-5 shrink-0 px-1.5 text-xs"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          onResolve();
        }}
      >
        Resolve
      </Button>

      <div onclick={(e: MouseEvent) => e.stopPropagation()} role="presentation">
        <DropdownMenu.Root bind:open={dismissOpen}>
          <DropdownMenu.Trigger>
            {#snippet children()}
              <Button variant="ghost" size="sm" class="h-5 shrink-0 gap-0.5 px-1.5 text-xs">
                Dismiss
                <ChevronDown class="size-2.5" />
              </Button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end">
            {#each dismissOptions as opt (opt.reason)}
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
</div>
