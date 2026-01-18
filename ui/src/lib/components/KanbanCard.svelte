<script lang="ts">
	import type { TaskItem } from '$lib/api/types';
	import { concepts } from '$lib/stores';

	interface Props {
		task: TaskItem;
		onTaskDetail?: (id: string) => void;
	}

	let { task, onTaskDetail }: Props = $props();

	let isDragging = $state(false);

	function handleDragStart(e: DragEvent) {
		if (e.dataTransfer) {
			e.dataTransfer.setData('text/plain', task.id);
			e.dataTransfer.effectAllowed = 'move';
		}
		isDragging = true;
	}

	function handleDragEnd() {
		isDragging = false;
	}

	function handleClick() {
		onTaskDetail?.(task.id);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onTaskDetail?.(task.id);
		}
	}

	function getConceptName(conceptId: string): string {
		return $concepts.find((c) => c.id === conceptId)?.name ?? conceptId;
	}
</script>

<div
	class="mb-1.5 cursor-pointer rounded-md border border-border bg-card p-2.5 transition-all hover:border-muted-foreground/50"
	class:opacity-40={isDragging}
	class:border-l-primary={task.current}
	class:border-l-2={task.current}
	class:opacity-50={task.blocked}
	draggable="true"
	ondragstart={handleDragStart}
	ondragend={handleDragEnd}
	onclick={handleClick}
	onkeydown={handleKeydown}
	tabindex="0"
	role="button"
>
	<!-- Title -->
	<p class="mb-1.5 text-sm leading-snug text-foreground">{task.title}</p>

	<!-- Meta row -->
	<div class="flex items-center gap-2 text-xs">
		<span class="text-muted-foreground">{task.id}</span>

		{#if task.current}
			<span class="flex items-center gap-1 text-primary">
				<span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
				Active
			</span>
		{/if}

		{#if task.blocked && task.status !== 'done'}
			<span class="text-destructive">Blocked</span>
		{/if}

		{#if task.check_count > 0}
			<span
				class={task.checks_verified === task.check_count ? 'text-success' : 'text-muted-foreground'}
			>
				{task.checks_verified}/{task.check_count} checks
			</span>
		{/if}

		<!-- Concepts -->
		{#each (task.concepts || []).slice(0, 1) as conceptId}
			<span class="rounded bg-muted px-1.5 py-0.5 text-muted-foreground"
				>{getConceptName(conceptId)}</span
			>
		{/each}
	</div>
</div>
