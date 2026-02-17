<script lang="ts">
  import { useResolveFinding } from '$lib/queries';
  import { Button } from '$lib/components/ui/button';
  import { Card } from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import type { Finding } from '$lib/types';

  interface Props {
    finding: Finding;
    reviewId: string;
  }

  let { finding, reviewId }: Props = $props();

  const resolveFinding = useResolveFinding();

  async function handleResolve() {
    await $resolveFinding.mutateAsync({ reviewId, findingId: finding.id });
  }

  function severityVariant(severity: string): 'destructive' | 'secondary' | 'outline' {
    if (severity === 'block') return 'destructive';
    if (severity === 'warn') return 'secondary';
    return 'outline';
  }

  function statusVariant(status: string): 'destructive' | 'secondary' | 'outline' {
    if (status === 'open') return 'destructive';
    if (status === 'resolved') return 'secondary';
    return 'outline';
  }
</script>

<Card class="p-4">
  <div class="flex items-start justify-between gap-4">
    <div class="flex-1">
      <div class="flex items-center gap-2">
        <span class="font-mono text-sm text-muted-foreground">{finding.target}</span>
        <Badge variant={severityVariant(finding.severity)}>{finding.severity}</Badge>
        <Badge variant={statusVariant(finding.status)}>{finding.status}</Badge>
      </div>
      <p class="mt-2">{finding.message}</p>
      <p class="mt-1 text-xs text-muted-foreground">source: {finding.source}</p>

      {#if finding.suggestion}
        <p class="mt-2 rounded bg-muted p-2 text-sm">
          Suggestion: {finding.suggestion}
        </p>
      {/if}
    </div>
  </div>

  {#if finding.status === 'open'}
    <div class="mt-4">
      <Button onclick={handleResolve} disabled={$resolveFinding.isPending}>
        {$resolveFinding.isPending ? 'Resolving...' : 'Resolve'}
      </Button>
    </div>
  {/if}
</Card>
