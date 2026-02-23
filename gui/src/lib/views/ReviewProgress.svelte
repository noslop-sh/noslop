<script lang="ts">
  import { Progress } from '$lib/components/ui/progress';

  interface Props {
    totalFiles: number;
    viewedFiles: number;
    totalFeedback: number;
    resolvedFeedback: number;
  }

  let { totalFiles, viewedFiles, totalFeedback, resolvedFeedback }: Props = $props();

  let filesPercent = $derived(totalFiles > 0 ? Math.round((viewedFiles / totalFiles) * 100) : 0);
  let feedbackPercent = $derived(
    totalFeedback > 0 ? Math.round((resolvedFeedback / totalFeedback) * 100) : 0
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

  <!-- Feedback resolved progress -->
  <div>
    <div class="mb-1.5 flex items-center justify-between">
      <span class="text-xs text-muted-foreground">Feedback resolved</span>
      <span class="text-xs font-medium">
        {resolvedFeedback}/{totalFeedback}
        <span class="text-muted-foreground">({feedbackPercent}%)</span>
      </span>
    </div>
    <Progress value={feedbackPercent} max={100} class="h-2" />
  </div>
</div>
