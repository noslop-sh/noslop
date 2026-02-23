<script lang="ts">
  import type { Severity } from '$lib/types';
  import { Button } from '$lib/components/ui/button';

  interface Props {
    filePath: string;
    startLine: number;
    endLine: number;
    onSubmit: (message: string, severity: Severity) => void;
    onCancel: () => void;
  }

  let { filePath, startLine, endLine, onSubmit, onCancel }: Props = $props();

  let severity = $state<Severity>('warn');
  let message = $state('');
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  let lineLabel = $derived(startLine === endLine ? `L${startLine}` : `L${startLine}-L${endLine}`);

  const severityOptions: { label: string; value: Severity }[] = [
    { label: 'Block', value: 'block' },
    { label: 'Warn', value: 'warn' },
    { label: 'Info', value: 'info' },
  ];

  $effect(() => {
    textareaEl?.focus();
  });

  function handleSubmit(): void {
    if (!message.trim()) return;
    onSubmit(message.trim(), severity);
    message = '';
    severity = 'warn';
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  }
</script>

<div class="border-l-3 border-l-[var(--feedback-warn)] rounded-sm border border-border bg-card p-3">
  <div class="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
    <span class="font-mono">{filePath.split('/').pop()}</span>
    <span>{lineLabel}</span>
  </div>

  <textarea
    bind:this={textareaEl}
    bind:value={message}
    onkeydown={handleKeydown}
    rows="2"
    placeholder="Describe the issue..."
    class="mb-2 w-full rounded border border-input bg-background px-2 py-1.5 font-mono text-sm
      placeholder:text-muted-foreground focus:border-ring focus:ring-1 focus:ring-ring
      focus:outline-none resize-none"
  ></textarea>

  <div class="flex items-center gap-2">
    <div class="flex gap-0.5 rounded border border-border p-0.5">
      {#each severityOptions as opt (opt.value)}
        <button
          type="button"
          class="rounded px-2 py-0.5 text-xs font-medium transition-colors
            {severity === opt.value
            ? 'bg-accent text-accent-foreground'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (severity = opt.value)}
        >
          {opt.label}
        </button>
      {/each}
    </div>

    <div class="flex-1"></div>

    <span class="text-[10px] text-muted-foreground">⌘↩</span>
    <Button variant="ghost" size="sm" class="h-6 px-2 text-xs" onclick={onCancel}>Cancel</Button>
    <Button size="sm" class="h-6 px-2 text-xs" onclick={handleSubmit} disabled={!message.trim()}>
      Add Feedback
    </Button>
  </div>
</div>
