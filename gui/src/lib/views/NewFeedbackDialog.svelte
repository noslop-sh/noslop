<script lang="ts">
  import type { Severity } from '$lib/types';
  import { SEVERITY_OPTIONS } from '$lib/helpers';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { MessageSquarePlus } from '@lucide/svelte';

  interface Props {
    open: boolean;
    filePath: string;
    startLine: number;
    endLine: number;
    onSubmit: (message: string, severity: Severity) => void;
    onCancel: () => void;
  }

  let { open = $bindable(), filePath, startLine, endLine, onSubmit, onCancel }: Props = $props();

  let severity = $state<Severity>('warn');
  let message = $state('');

  let lineLabel = $derived(
    startLine === endLine ? `L${startLine}` : `L${startLine}-L${endLine}`
  );

  function handleSubmit(): void {
    if (!message.trim()) return;
    onSubmit(message.trim(), severity);
    message = '';
    severity = 'warn';
  }

  function handleCancel(): void {
    message = '';
    severity = 'warn';
    onCancel();
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) handleCancel();
  }}
>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <MessageSquarePlus class="size-5" />
        Add Feedback
      </Dialog.Title>
      <Dialog.Description>
        <span class="font-mono">{filePath}</span> {lineLabel}
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-4">
      <!-- Severity selector -->
      <div>
        <label class="mb-1.5 block text-sm font-medium">Severity</label>
        <div class="flex gap-1 rounded-md border border-border p-0.5">
          {#each SEVERITY_OPTIONS as opt (opt.value)}
            <button
              class="flex-1 rounded px-3 py-1.5 text-sm font-medium transition-colors
                {severity === opt.value
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => (severity = opt.value)}
            >
              {opt.label}
            </button>
          {/each}
        </div>
      </div>

      <!-- Message textarea -->
      <div>
        <label for="feedback-message" class="mb-1.5 block text-sm font-medium">Message</label>
        <textarea
          id="feedback-message"
          bind:value={message}
          onkeydown={handleKeydown}
          rows="3"
          placeholder="Describe the issue..."
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm
            placeholder:text-muted-foreground focus:border-ring focus:ring-1 focus:ring-ring
            focus:outline-none resize-none"
        ></textarea>
        <p class="mt-1 text-xs text-muted-foreground">Cmd+Enter to submit</p>
      </div>
    </div>

    <Dialog.Footer class="mt-4">
      <Button variant="outline" onclick={handleCancel}>Cancel</Button>
      <Button onclick={handleSubmit} disabled={!message.trim()}>
        Add Feedback
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
