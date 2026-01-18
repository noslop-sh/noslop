<script lang="ts">
	import type { TaskItem } from '$lib/api/types';
	import KanbanCard from './KanbanCard.svelte';

	interface Props {
		title: string;
		tasks: TaskItem[];
		status: string;
		color: string;
		onTaskDetail?: (id: string) => void;
		onDrop?: (taskId: string, newStatus: string) => void;
	}

	let { title, tasks, status, color, onTaskDetail, onDrop }: Props = $props();

	let isDragOver = $state(false);

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		isDragOver = true;
	}

	function handleDragLeave() {
		isDragOver = false;
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		const taskId = e.dataTransfer?.getData('text/plain');
		if (taskId && onDrop) {
			onDrop(taskId, status);
		}
	}
</script>

<div class="flex min-w-[260px] flex-1 flex-col">
	<!-- Header -->
	<div class="mb-2 flex items-center gap-2 px-1">
		<span class="h-2 w-2 rounded-full" style="background-color: {color};"></span>
		<span class="text-sm font-medium text-muted-foreground">
			{title}
		</span>
		<span class="text-xs text-muted-foreground">
			{tasks.length}
		</span>
	</div>

	<!-- Content -->
	<div
		class="flex-1 overflow-y-auto rounded-lg p-1 transition-colors"
		class:bg-accent={isDragOver}
		ondragover={handleDragOver}
		ondragleave={handleDragLeave}
		ondrop={handleDrop}
		role="list"
	>
		{#each tasks as task (task.id)}
			<KanbanCard {task} {onTaskDetail} />
		{:else}
			<div
				class="flex h-16 items-center justify-center rounded-lg border border-dashed border-border text-xs text-muted-foreground"
			>
				No tasks
			</div>
		{/each}
	</div>
</div>
