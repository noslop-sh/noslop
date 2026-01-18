<script lang="ts">
	import Toolbar from '$lib/components/Toolbar.svelte';
	import KanbanColumn from '$lib/components/KanbanColumn.svelte';
	import TaskDetailModal from '$lib/components/TaskDetailModal.svelte';
	import { filteredTasks } from '$lib/stores';
	import { loadTasks, loadStatus } from '$lib/stores';
	import * as api from '$lib/api/client';
	import type { TaskDetailData } from '$lib/api/types';

	let detailTask = $state<TaskDetailData | null>(null);
	let detailOpen = $state(false);

	// Compute columns from filtered tasks
	// Statuses: backlog -> pending -> in_progress -> done
	// Blocked is an overlay (task has unfinished blockers), not a separate status
	const columns = $derived({
		backlog: $filteredTasks.filter((t) => t.status === 'backlog'),
		pending: $filteredTasks.filter((t) => t.status === 'pending'),
		inProgress: $filteredTasks.filter((t) => t.status === 'in_progress'),
		done: $filteredTasks
			.filter((t) => t.status === 'done')
			.sort((a, b) => {
				if (!a.completed_at && !b.completed_at) return 0;
				if (!a.completed_at) return 1;
				if (!b.completed_at) return -1;
				return b.completed_at.localeCompare(a.completed_at);
			})
			.slice(0, 20)
	});

	async function handleTaskDetail(id: string) {
		try {
			detailTask = await api.getTask(id);
			detailOpen = true;
		} catch (error) {
			console.error('Failed to load task detail:', error);
		}
	}

	function handleCloseDetail() {
		detailOpen = false;
		detailTask = null;
	}

	async function handleDrop(taskId: string, newStatus: string) {
		try {
			if (newStatus === 'in_progress') {
				await api.startTask(taskId);
			} else if (newStatus === 'pending') {
				await api.resetTask(taskId);
			} else if (newStatus === 'backlog') {
				await api.backlogTask(taskId);
			} else if (newStatus === 'done') {
				await api.completeTask(taskId);
			}
			await Promise.all([loadTasks(), loadStatus()]);
		} catch (error) {
			console.error('Failed to update task status:', error);
		}
	}
</script>

<div class="flex h-screen flex-col bg-background">
	<Toolbar />

	<main class="flex flex-1 gap-3 overflow-x-auto p-4">
		<KanbanColumn
			title="Backlog"
			tasks={columns.backlog}
			status="backlog"
			color="#8b8b8b"
			onTaskDetail={handleTaskDetail}
			onDrop={handleDrop}
		/>

		<KanbanColumn
			title="Pending"
			tasks={columns.pending}
			status="pending"
			color="#0091ff"
			onTaskDetail={handleTaskDetail}
			onDrop={handleDrop}
		/>

		<KanbanColumn
			title="In Progress"
			tasks={columns.inProgress}
			status="in_progress"
			color="#5e6ad2"
			onTaskDetail={handleTaskDetail}
			onDrop={handleDrop}
		/>

		<KanbanColumn
			title="Done"
			tasks={columns.done}
			status="done"
			color="#46a758"
			onTaskDetail={handleTaskDetail}
			onDrop={handleDrop}
		/>
	</main>
</div>

<TaskDetailModal task={detailTask} open={detailOpen} onClose={handleCloseDetail} />
