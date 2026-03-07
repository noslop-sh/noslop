<script lang="ts">
  import type { PaletteCommand } from '$lib/types';
  import * as Command from '$lib/components/ui/command';

  interface Props {
    open: boolean;
    commands: PaletteCommand[];
    onClose: () => void;
  }

  let { open = $bindable(), commands, onClose }: Props = $props();

  let availableCommands = $derived(commands.filter((cmd) => cmd.available()));

  let groupedCommands = $derived.by(() => {
    const groups = new Map<string, PaletteCommand[]>();
    for (const cmd of availableCommands) {
      const existing = groups.get(cmd.group);
      if (existing) {
        existing.push(cmd);
      } else {
        groups.set(cmd.group, [cmd]);
      }
    }
    return groups;
  });

  const groupLabels: Record<string, string> = {
    actions: 'Actions',
    files: 'Files',
    feedbacks: 'Feedback',
    navigation: 'Navigation',
  };

  const groupOrder = ['actions', 'files', 'feedbacks', 'navigation'];

  let orderedGroups = $derived(
    groupOrder
      .filter((g) => groupedCommands.has(g))
      .map((g) => ({ key: g, label: groupLabels[g] ?? g, commands: groupedCommands.get(g)! }))
  );

  function handleSelect(cmd: PaletteCommand): void {
    onClose();
    cmd.action();
  }
</script>

<Command.CommandDialog
  bind:open
  onOpenChange={(v) => {
    if (!v) onClose();
  }}
  title="Command Palette"
  description="Search for a command to run"
>
  <Command.CommandInput placeholder="Type a command or search..." />
  <Command.CommandList>
    <Command.CommandEmpty>No commands found.</Command.CommandEmpty>
    {#each orderedGroups as group (group.key)}
      <Command.CommandGroup heading={group.label}>
        {#each group.commands as cmd (cmd.id)}
          <Command.CommandItem value={cmd.label} onSelect={() => handleSelect(cmd)}>
            <span class="flex-1">{cmd.label}</span>
            {#if cmd.shortcut}
              <Command.CommandShortcut>{cmd.shortcut}</Command.CommandShortcut>
            {/if}
          </Command.CommandItem>
        {/each}
      </Command.CommandGroup>
    {/each}
  </Command.CommandList>
</Command.CommandDialog>
