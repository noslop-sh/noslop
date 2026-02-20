<script lang="ts" module>
  /** Parse a key string into individual key segments for rendering as separate kbd elements. */
  function parseKeys(key: string): string[] {
    // Handle modifier combinations like "⌘K" -> ["⌘", "K"]
    if (key.startsWith('\u2318')) {
      return ['\u2318', key.slice(1)];
    }
    // Single keys render as one badge
    return [key];
  }
</script>

<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Keyboard } from '@lucide/svelte';

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open = $bindable(), onClose }: Props = $props();

  interface ShortcutEntry {
    key: string;
    description: string;
  }

  interface ShortcutGroup {
    label: string;
    shortcuts: ShortcutEntry[];
  }

  const groups: ShortcutGroup[] = [
    {
      label: 'Navigation',
      shortcuts: [
        { key: ']', description: 'Next file' },
        { key: '[', description: 'Previous file' },
        { key: 'j', description: 'Next finding' },
        { key: 'k', description: 'Previous finding' },
        { key: 'n', description: 'Next unresolved finding' },
        { key: 'p', description: 'Previous unresolved finding' },
        { key: '\u2318P', description: 'Go to file' },
      ],
    },
    {
      label: 'Findings',
      shortcuts: [
        { key: 'r', description: 'Resolve finding' },
        { key: 'd', description: 'Dismiss finding' },
        { key: 'c', description: 'Add comment' },
        { key: 'Enter', description: 'Expand/collapse finding' },
      ],
    },
    {
      label: 'View',
      shortcuts: [
        { key: 'f', description: 'Toggle file tree' },
        { key: 's', description: 'Toggle split/unified diff' },
        { key: 'w', description: 'Toggle whitespace' },
        { key: 'v', description: 'Mark file as viewed' },
      ],
    },
    {
      label: 'Review',
      shortcuts: [
        { key: '\u2318K', description: 'Command palette' },
        { key: '?', description: 'Toggle shortcut overlay' },
        { key: 'Esc', description: 'Close dialog / deselect' },
      ],
    },
  ];

  // Split into two columns for layout
  let leftGroups = $derived(groups.slice(0, 2));
  let rightGroups = $derived(groups.slice(2));
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onClose();
  }}
>
  <Dialog.Content class="sm:max-w-2xl">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <Keyboard class="size-5" />
        Keyboard Shortcuts
      </Dialog.Title>
      <Dialog.Description>
        Available keyboard shortcuts for navigating the review.
      </Dialog.Description>
    </Dialog.Header>

    <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
      <div class="space-y-5">
        {#each leftGroups as group (group.label)}
          <div>
            <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              {group.label}
            </h3>
            <div class="space-y-1.5">
              {#each group.shortcuts as shortcut (shortcut.key)}
                <div class="flex items-center justify-between gap-3">
                  <span class="text-sm text-foreground">{shortcut.description}</span>
                  <div class="flex shrink-0 items-center gap-1">
                    {#each parseKeys(shortcut.key) as keyPart (keyPart)}
                      <kbd
                        class="inline-flex h-6 min-w-6 items-center justify-center rounded border
                          border-border bg-muted px-1.5 font-mono text-xs font-medium text-foreground
                          shadow-sm"
                      >
                        {keyPart}
                      </kbd>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>

      <div class="space-y-5">
        {#each rightGroups as group (group.label)}
          <div>
            <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              {group.label}
            </h3>
            <div class="space-y-1.5">
              {#each group.shortcuts as shortcut (shortcut.key)}
                <div class="flex items-center justify-between gap-3">
                  <span class="text-sm text-foreground">{shortcut.description}</span>
                  <div class="flex shrink-0 items-center gap-1">
                    {#each parseKeys(shortcut.key) as keyPart (keyPart)}
                      <kbd
                        class="inline-flex h-6 min-w-6 items-center justify-center rounded border
                          border-border bg-muted px-1.5 font-mono text-xs font-medium text-foreground
                          shadow-sm"
                      >
                        {keyPart}
                      </kbd>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
  </Dialog.Content>
</Dialog.Root>
