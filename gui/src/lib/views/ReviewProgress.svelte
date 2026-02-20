<script lang="ts">
  import { Progress } from '$lib/components/ui/progress';

  interface Props {
    totalFiles: number;
    viewedFiles: number;
    totalFindings: number;
    resolvedFindings: number;
  }

  let { totalFiles, viewedFiles, totalFindings, resolvedFindings }: Props = $props();

  let filesPercent = $derived(totalFiles > 0 ? Math.round((viewedFiles / totalFiles) * 100) : 0);
  let findingsPercent = $derived(
    totalFindings > 0 ? Math.round((resolvedFindings / totalFindings) * 100) : 0
  );
</script>

<div class="space-y-3">
  <!-- Files viewed progress -->
  <div>
    <div class="mb-1.5 flex items-center justify-between">
      <span class="text-xs text-muted-foreground">Files viewed</span>
      <span class="text-xs font-medium">
        {viewedFiles}/{totalFiles}
        <span class="text-muted-foreground">({filesPercent}%)</span>
      </span>
    </div>
    <Progress value={filesPercent} max={100} class="h-2" />
  </div>

  <!-- Findings resolved progress -->
  <div>
    <div class="mb-1.5 flex items-center justify-between">
      <span class="text-xs text-muted-foreground">Findings resolved</span>
      <span class="text-xs font-medium">
        {resolvedFindings}/{totalFindings}
        <span class="text-muted-foreground">({findingsPercent}%)</span>
      </span>
    </div>
    <Progress value={findingsPercent} max={100} class="h-2" />
  </div>
</div>
