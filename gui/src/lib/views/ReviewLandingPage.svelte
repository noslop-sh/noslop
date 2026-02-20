<script lang="ts">
  import type { Review, StructuredDiff } from '$lib/types';
  import { blockingFindings, openFindingCount } from '$lib/helpers';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import {
    ShieldAlert,
    AlertTriangle,
    CheckCircle,
    FileText,
    Plus,
    Minus,
    CircleAlert,
    FlaskConical,
    X,
  } from '@lucide/svelte';

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
  let openCount = $derived(openFindingCount(review.findings));
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

  let verdictVariant = $derived(
    verdictState === 'blockers'
      ? 'destructive'
      : verdictState === 'warnings'
        ? 'secondary'
        : 'outline'
  ) as 'destructive' | 'secondary' | 'outline';

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
    { pattern: '*.lock', label: 'Lock files changed', icon: FileText },
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

<div class="mx-auto max-w-3xl space-y-6 px-6 py-8">
  <!-- Close button -->
  <div class="flex justify-end">
    <Button variant="ghost" size="icon-sm" onclick={onClose} aria-label="Close landing page">
      <X class="size-4" />
    </Button>
  </div>

  <!-- Verdict badge -->
  <div class="flex flex-col items-center gap-3 text-center">
    <div class="flex items-center gap-2 rounded-lg border px-4 py-3 {verdictBgClass}">
      {#if verdictState === 'blockers'}
        <ShieldAlert class="size-6 {verdictTextClass}" />
      {:else if verdictState === 'warnings'}
        <AlertTriangle class="size-6 {verdictTextClass}" />
      {:else}
        <CheckCircle class="size-6 {verdictTextClass}" />
      {/if}
      <span class="text-lg font-semibold {verdictTextClass}">{verdictLabel}</span>
    </div>
    <p class="text-sm text-muted-foreground">
      {review.branch ? `Branch: ${review.branch}` : ''}
      {review.base.slice(0, 7)}..{review.head.slice(0, 7)}
    </p>
  </div>

  <!-- Stat bar -->
  <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
    <Card.Root>
      <Card.Content class="flex flex-col items-center gap-1 p-3">
        <FileText class="size-4 text-muted-foreground" />
        <span class="text-lg font-semibold">{filesChanged}</span>
        <span class="text-[10px] text-muted-foreground">Files changed</span>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="flex flex-col items-center gap-1 p-3">
        <Plus class="size-4 text-green-600 dark:text-green-400" />
        <span class="text-lg font-semibold text-green-600 dark:text-green-400">+{additions}</span>
        <span class="text-[10px] text-muted-foreground">Additions</span>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="flex flex-col items-center gap-1 p-3">
        <Minus class="size-4 text-red-600 dark:text-red-400" />
        <span class="text-lg font-semibold text-red-600 dark:text-red-400">-{deletions}</span>
        <span class="text-[10px] text-muted-foreground">Deletions</span>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="flex flex-col items-center gap-1 p-3">
        <ShieldAlert class="size-4 text-finding-block" />
        <span class="text-lg font-semibold">{blockCount}</span>
        <span class="text-[10px] text-muted-foreground">Blockers</span>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="flex flex-col items-center gap-1 p-3">
        <CircleAlert class="size-4 text-finding-warn" />
        <span class="text-lg font-semibold">{warnCount}</span>
        <span class="text-[10px] text-muted-foreground">Warnings</span>
      </Card.Content>
    </Card.Root>
  </div>

  <!-- Risk areas -->
  {#if matchedRisks.length > 0}
    <div>
      <h3 class="mb-2 text-sm font-medium">Risk areas detected</h3>
      <div class="space-y-1.5">
        {#each matchedRisks as risk (risk.pattern)}
          <div
            class="flex items-center gap-2 rounded-md border border-finding-warn/25 bg-finding-warn/5 px-3 py-2"
          >
            <risk.icon class="size-4 text-finding-warn" />
            <span class="text-sm">{risk.label}</span>
            <Badge variant="outline" class="ml-auto text-[10px]">{risk.pattern}</Badge>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Adaptive CTA -->
  <div class="flex justify-center pt-2">
    <Button
      size="lg"
      variant={verdictState === 'clean' ? 'default' : 'destructive'}
      onclick={handleCta}
    >
      {ctaLabel}
    </Button>
  </div>
</div>
