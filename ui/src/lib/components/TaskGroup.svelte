<script lang="ts">
	import type { TaskItem } from '$lib/api/types';
	import TaskCard from './TaskCard.svelte';

	interface Props {
		title: string;
		tasks: TaskItem[];
		collapsed?: boolean;
		onTaskSelect?: (id: string) => void;
		onTaskDetail?: (id: string) => void;
	}

	let { title, tasks, collapsed = false, onTaskSelect, onTaskDetail }: Props = $props();

	let isCollapsed = $state(collapsed);
</script>

<section class="mb-6">
	<!-- Group header -->
	<button
		class="mb-3 flex w-full items-center gap-2 text-left"
		onclick={() => (isCollapsed = !isCollapsed)}
	>
		<span class="text-xs text-muted-foreground">{isCollapsed ? '▸' : '▾'}</span>
		<h2 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
			{title}
		</h2>
		<span class="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
			{tasks.length}
		</span>
		<div class="h-px flex-1 bg-border"></div>
	</button>

	<!-- Task list -->
	{#if !isCollapsed}
		<div class="space-y-2 pl-4">
			{#each tasks as task (task.id)}
				<TaskCard {task} onSelect={onTaskSelect} onDetail={onTaskDetail} />
			{:else}
				<p class="py-4 text-center text-sm italic text-muted-foreground">No tasks</p>
			{/each}
		</div>
	{/if}
</section>
