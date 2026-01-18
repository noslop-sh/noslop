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
	class:border-l-2={task.current || task.blocked}
	class:border-l-primary={task.current}
	class:border-l-destructive={task.blocked && !task.current}
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

		{#if task.blocked && task.status !== 'done'}
			<span class="text-destructive">Blocked</span>
		{/if}

		{#if task.check_count > 0}
			{@const progress = task.checks_verified / task.check_count}
			{@const circumference = 2 * Math.PI * 5}
			{@const strokeColor = progress === 1 ? '#46a758' : progress >= 0.5 ? '#f5a623' : '#e5484d'}
			<span
				class="flex items-center gap-1"
				title="{task.checks_verified}/{task.check_count} checks verified"
			>
				<svg width="14" height="14" viewBox="0 0 14 14">
					<!-- Background circle (gray) -->
					<circle
						cx="7"
						cy="7"
						r="5"
						fill="var(--color-muted)"
						stroke="var(--color-border)"
						stroke-width="1.5"
					/>
					<!-- Progress arc -->
					<circle
						cx="7"
						cy="7"
						r="5"
						fill="none"
						stroke={strokeColor}
						stroke-width="1.5"
						stroke-dasharray={circumference}
						stroke-dashoffset={circumference * (1 - progress)}
						transform="rotate(-90 7 7)"
						stroke-linecap="round"
					/>
				</svg>
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
