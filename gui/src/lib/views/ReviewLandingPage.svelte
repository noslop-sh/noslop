<script lang="ts">
  import type { Review, StructuredDiff } from '$lib/types';
  import { blockingFindings } from '$lib/helpers';
  import { Button } from '$lib/components/ui/button';
  import { ShieldAlert, AlertTriangle, CheckCircle, FlaskConical } from '@lucide/svelte';

  interface Props {
    review: Review;
    diff: StructuredDiff | null;
    onClose: () => void;
    onScrollToBlocker: () => void;
  }

  let { review, diff, onClose, onScrollToBlocker }: Props = $props();

  // Derived counts
  let blockers = $derived(blockingFindings(review.findings));
  let blockCount = $derived(blockers.length);
  let warnCount = $derived(
    review.findings.filter((f) => f.severity === 'warn' && f.status === 'open').length
  );
  let filesChanged = $derived(diff?.stats.files_changed ?? 0);
  let additions = $derived(diff?.stats.additions ?? 0);
  let deletions = $derived(diff?.stats.deletions ?? 0);

  // Verdict state
  type VerdictState = 'blockers' | 'warnings' | 'clean';
  let verdictState: VerdictState = $derived(
    blockCount > 0 ? 'blockers' : warnCount > 0 ? 'warnings' : 'clean'
  );

  let verdictLabel = $derived(
    verdictState === 'blockers'
      ? `${blockCount} Blocker${blockCount > 1 ? 's' : ''} Found`
      : verdictState === 'warnings'
        ? `${warnCount} Warning${warnCount > 1 ? 's' : ''}`
        : 'Clean'
  );

  let verdictBgClass = $derived(
    verdictState === 'blockers'
      ? 'bg-finding-block/10 border-finding-block/25'
      : verdictState === 'warnings'
        ? 'bg-finding-warn/10 border-finding-warn/25'
        : 'bg-success/10 border-success/25'
  );

  let verdictTextClass = $derived(
    verdictState === 'blockers'
      ? 'text-finding-block'
      : verdictState === 'warnings'
        ? 'text-finding-warn'
        : 'text-success'
  );

  // CTA label adapts based on state
  let ctaLabel = $derived(
    verdictState === 'blockers'
      ? 'Jump to First Blocker'
      : verdictState === 'warnings'
        ? 'Review Warnings'
        : 'Close Review'
  );

  function handleCta() {
    if (verdictState === 'clean') {
      onClose();
    } else {
      onScrollToBlocker();
    }
  }

  // Hardcoded risk patterns for now (config not loaded)
  const riskPatterns = [
    { pattern: '*.lock', label: 'Lock files changed', icon: ShieldAlert },
    { pattern: 'migration', label: 'Database migrations', icon: FlaskConical },
    { pattern: '.env', label: 'Environment config', icon: ShieldAlert },
    { pattern: 'auth', label: 'Authentication code', icon: ShieldAlert },
  ];

  let matchedRisks = $derived(
    diff
      ? riskPatterns.filter((rp) =>
          diff.files.some((f) => f.path.toLowerCase().includes(rp.pattern.toLowerCase()))
        )
      : []
  );
</script>

<div class="mx-auto max-w-3xl space-y-4 px-6 py-6">
  <!-- Verdict badge -->
  <div class="flex flex-col items-center gap-2 text-center">
    <div class="flex items-center gap-2 rounded-lg border px-4 py-2.5 {verdictBgClass}">
      {#if verdictState === 'blockers'}
        <ShieldAlert class="size-5 {verdictTextClass}" />
      {:else if verdictState === 'warnings'}
        <AlertTriangle class="size-5 {verdictTextClass}" />
      {:else}
        <CheckCircle class="size-5 {verdictTextClass}" />
      {/if}
      <span class="text-base font-semibold {verdictTextClass}">{verdictLabel}</span>
    </div>
    <p class="text-xs text-muted-foreground">
      {review.branch ? review.branch : ''}
      <span class="font-mono">{review.base.slice(0, 7)}..{review.head.slice(0, 7)}</span>
    </p>
  </div>

  <!-- Compact stat line -->
  <div class="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
    <span>{filesChanged} file{filesChanged !== 1 ? 's' : ''}</span>
    <span class="opacity-40">&middot;</span>
    <span>
      <span class="text-green-600 dark:text-green-400">+{additions}</span>
      {' / '}
      <span class="text-red-600 dark:text-red-400">-{deletions}</span>
    </span>
    {#if blockCount > 0}
      <span class="opacity-40">&middot;</span>
      <span class="text-finding-block">{blockCount} block</span>
    {/if}
    {#if warnCount > 0}
      <span class="opacity-40">&middot;</span>
      <span class="text-finding-warn">{warnCount} warn</span>
    {/if}
  </div>

  <!-- Risk areas -->
  {#if matchedRisks.length > 0}
    <div class="flex flex-wrap items-center justify-center gap-2">
      {#each matchedRisks as risk (risk.pattern)}
        <div
          class="flex items-center gap-1.5 rounded-md border border-finding-warn/25 bg-finding-warn/5 px-2.5 py-1"
        >
          <risk.icon class="size-3.5 text-finding-warn" />
          <span class="text-xs">{risk.label}</span>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Adaptive CTA -->
  <div class="flex justify-center">
    <Button
      size="sm"
      variant={verdictState === 'clean' ? 'default' : 'destructive'}
      onclick={handleCta}
    >
      {ctaLabel}
    </Button>
  </div>
</div>
