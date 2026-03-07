<script lang="ts">
  import type { Review, DismissReason } from '$lib/types';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { ShieldCheck } from '@lucide/svelte';

  interface Props {
    open: boolean;
    review: Review;
    onClose: () => void;
    onCancel: () => void;
  }

  let { open = $bindable(), review, onClose, onCancel }: Props = $props();

  let totalFeedback = $derived(review.feedbacks.length);
  let resolvedCount = $derived(review.feedbacks.filter((f) => f.status === 'resolved').length);
  let dismissedFeedback = $derived(review.feedbacks.filter((f) => f.status === 'dismissed'));
  let dismissedCount = $derived(dismissedFeedback.length);
  let remainingFeedback = $derived(review.feedbacks.filter((f) => f.status === 'open'));
  let remainingCount = $derived(remainingFeedback.length);

  let dismissBreakdown = $derived.by(() => {
    const counts = new Map<DismissReason, number>();
    for (const f of dismissedFeedback) {
      if (f.dismiss_reason) {
        counts.set(f.dismiss_reason, (counts.get(f.dismiss_reason) ?? 0) + 1);
      }
    }
    return counts;
  });

  let dismissBreakdownText = $derived.by(() => {
    const parts: string[] = [];
    const labels: Record<DismissReason, string> = {
      false_positive: 'false positive',
      wont_fix: "won't fix",
      not_applicable: 'not applicable',
      investigate_later: 'investigate later',
    };
    for (const [reason, count] of dismissBreakdown) {
      parts.push(`${count} ${labels[reason]}`);
    }
    return parts.length > 0 ? `(${parts.join(', ')})` : '';
  });

  let allFilePaths = $derived.by(() => {
    const paths = new Set<string>();
    for (const f of review.feedbacks) {
      paths.add(f.target.path);
    }
    for (const p of review.viewed_files) {
      paths.add(p);
    }
    return paths;
  });

  let totalFiles = $derived(allFilePaths.size);
  let viewedCount = $derived(review.viewed_files.length);
  let viewedPercent = $derived(totalFiles > 0 ? Math.round((viewedCount / totalFiles) * 100) : 100);

  let branchName = $derived(review.branch ?? review.head);

  function dotPad(
    label: string,
    value: string,
    width: number = 32
  ): { label: string; dots: string; value: string } {
    const dotsNeeded = Math.max(2, width - label.length - value.length);
    return { label, dots: '.'.repeat(dotsNeeded), value };
  }

  let resolvedRow = $derived(dotPad('Feedback resolved', `${resolvedCount}/${totalFeedback}`));
  let dismissedRow = $derived(dotPad('Feedback dismissed', `${dismissedCount}/${totalFeedback}`));
  let remainingRow = $derived(dotPad('Feedback remaining', `${remainingCount}`));
  let viewedRow = $derived(dotPad('Files viewed', `${viewedCount}/${totalFiles}`));

  let remainingInfoCount = $derived(remainingFeedback.filter((f) => f.severity === 'info').length);
  let remainingWarnCount = $derived(remainingFeedback.filter((f) => f.severity === 'warn').length);
  let remainingBreakdownParts = $derived([
    ...(remainingInfoCount > 0 ? [`${remainingInfoCount} info`] : []),
    ...(remainingWarnCount > 0 ? [`${remainingWarnCount} warn`] : []),
  ]);
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onCancel();
  }}
>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title>Close Review: {branchName}</Dialog.Title>
      <Dialog.Description>Review summary before closing</Dialog.Description>
    </Dialog.Header>

    <div class="space-y-1 font-mono text-sm">
      <p class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        Summary
      </p>

      <div class="flex items-baseline gap-1">
        <span class="text-foreground">{resolvedRow.label}</span>
        <span class="text-muted-foreground/40">{resolvedRow.dots}</span>
        <span class="text-foreground">{resolvedRow.value}</span>
      </div>

      <div class="flex items-baseline gap-1">
        <span class="text-foreground">{dismissedRow.label}</span>
        <span class="text-muted-foreground/40">{dismissedRow.dots}</span>
        <span class="text-foreground">{dismissedRow.value}</span>
        {#if dismissBreakdownText}
          <span class="text-xs text-muted-foreground">{dismissBreakdownText}</span>
        {/if}
      </div>

      <div class="flex items-baseline gap-1">
        <span class="text-foreground">{remainingRow.label}</span>
        <span class="text-muted-foreground/40">{remainingRow.dots}</span>
        <span class="text-foreground">{remainingRow.value}</span>
        {#if remainingCount > 0 && remainingBreakdownParts.length > 0}
          <span class="text-xs text-muted-foreground"
            >({remainingBreakdownParts.join(', ')}, non-blocking)</span
          >
        {/if}
      </div>

      <div class="flex items-baseline gap-1">
        <span class="text-foreground">{viewedRow.label}</span>
        <span class="text-muted-foreground/40">{viewedRow.dots}</span>
        <span class="text-foreground">{viewedRow.value}</span>
        <span class="text-xs text-muted-foreground">({viewedPercent}%)</span>
      </div>
    </div>

    <Dialog.Footer class="mt-4">
      <Button variant="outline" onclick={onCancel}>Cancel</Button>
      <Button
        onclick={onClose}
        class="bg-success text-success-foreground hover:bg-success/90 shadow-xs"
      >
        <ShieldCheck class="size-4" />
        Close Review & Allow Push
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
