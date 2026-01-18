<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Button } from '$lib/components/ui/button';
	import type { TaskItem } from '$lib/api/types';
	import { selectedTaskId, concepts } from '$lib/stores';
	import * as api from '$lib/api/client';
	import { loadTasks, loadStatus } from '$lib/stores';

	interface Props {
		task: TaskItem;
		onSelect?: (id: string) => void;
		onDetail?: (id: string) => void;
	}

	let { task, onSelect, onDetail }: Props = $props();

	const isSelected = $derived($selectedTaskId === task.id);
	let confirmingDelete = $state(false);

	function handleClick() {
		selectedTaskId.set(task.id);
		onSelect?.(task.id);
	}

	function handleDoubleClick() {
		onDetail?.(task.id);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			onDetail?.(task.id);
		} else if (e.key === ' ') {
			e.preventDefault();
			handleClick();
		} else if (e.key === 'Escape' && confirmingDelete) {
			confirmingDelete = false;
		}
	}

	async function handleDelete() {
		if (!confirmingDelete) {
			confirmingDelete = true;
			return;
		}
		try {
			await api.deleteTask(task.id);
			if ($selectedTaskId === task.id) {
				selectedTaskId.set(null);
			}
			await Promise.all([loadTasks(), loadStatus()]);
		} catch (error) {
			console.error('Failed to delete task:', error);
		}
		confirmingDelete = false;
	}

	function cancelDelete() {
		confirmingDelete = false;
	}

	function getConceptName(conceptId: string): string {
		return $concepts.find((c) => c.id === conceptId)?.name ?? conceptId;
	}
</script>

<div
	class="group cursor-pointer rounded-lg border border-border bg-card p-3 transition-all hover:-translate-y-0.5 hover:shadow-lg
		{isSelected ? 'ring-2 ring-primary' : ''}
		{task.current ? 'border-l-4 border-l-success' : ''}
		{task.blocked ? 'opacity-70' : ''}
		{task.status === 'done' ? 'opacity-75' : ''}"
	onclick={handleClick}
	ondblclick={handleDoubleClick}
	onkeydown={handleKeydown}
	tabindex="0"
	role="button"
>
	<!-- Header: ID and menu -->
	<div class="mb-2 flex items-center justify-between">
		<span class="text-sm text-muted-foreground {task.current ? 'text-success' : ''}">
			{task.id}
		</span>
		{#if confirmingDelete}
			<div class="flex items-center gap-1">
				<Button variant="destructive" size="sm" class="h-6 px-2 text-xs" onclick={handleDelete}>
					Delete
				</Button>
				<Button variant="ghost" size="sm" class="h-6 px-2 text-xs" onclick={cancelDelete}>
					Cancel
				</Button>
			</div>
		{:else}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					<Button variant="ghost" size="sm" class="h-6 w-6 p-0 opacity-0 group-hover:opacity-100">
						⋮
					</Button>
				</DropdownMenu.Trigger>
				<DropdownMenu.Content>
					<DropdownMenu.Item onclick={() => onDetail?.(task.id)}>View Details</DropdownMenu.Item>
					<DropdownMenu.Separator />
					<DropdownMenu.Item onclick={handleDelete} class="text-destructive">
						Delete
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
	</div>

	<!-- Title -->
	<h3 class="mb-2 text-sm font-medium leading-tight">{task.title}</h3>

	<!-- Footer: Tags and metadata -->
	<div class="flex flex-wrap items-center gap-1.5">
		<!-- Concept tags -->
		{#each task.concepts || [] as conceptId}
			<Badge variant="secondary" class="text-xs">{getConceptName(conceptId)}</Badge>
		{/each}

		<!-- Check counter -->
		{#if task.check_count > 0}
			<Badge
				variant={task.checks_verified === task.check_count ? 'default' : 'outline'}
				class="text-xs"
			>
				checks: {task.checks_verified}/{task.check_count}
				{task.checks_verified === task.check_count ? '✓' : ''}
			</Badge>
		{/if}

		<!-- Priority (for pending tasks) -->
		{#if task.status === 'pending' && task.priority !== 'p1'}
			<Badge variant="outline" class="text-xs">{task.priority}</Badge>
		{/if}

		<!-- Blocked indicator -->
		{#if task.blocked && task.status !== 'done'}
			<Badge variant="destructive" class="text-xs">blocked</Badge>
		{/if}

		<!-- Branch -->
		{#if task.branch}
			<span class="ml-auto max-w-24 truncate text-xs text-muted-foreground">
				{task.branch}
			</span>
		{/if}
	</div>
</div>
