<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import type { TaskDetailData } from '$lib/api/types';
	import * as api from '$lib/api/client';
	import { concepts, tasks, loadTasks, loadConcepts } from '$lib/stores';

	interface Props {
		task: TaskDetailData | null;
		open: boolean;
		onClose: () => void;
	}

	let { task: initialTask, open, onClose }: Props = $props();

	// Local mutable copy of task that can be updated after mutations
	let task = $state<TaskDetailData | null>(null);

	// Sync local task with prop when it changes (e.g., modal opens with new task)
	$effect(() => {
		task = initialTask;
	});

	// Editing states
	let editingField = $state<'title' | 'description' | null>(null);
	let editingTitle = $state('');
	let editingDescription = $state('');

	// Reset editing state when task changes
	$effect(() => {
		if (task) {
			editingField = null;
			editingTitle = task.title;
			editingDescription = task.description ?? '';
		}
	});

	function getConceptName(conceptId: string): string {
		return $concepts.find((c) => c.id === conceptId)?.name ?? conceptId;
	}

	function startEditTitle() {
		if (!task) return;
		editingTitle = task.title;
		editingField = 'title';
	}

	function startEditDescription() {
		if (!task) return;
		editingDescription = task.description ?? '';
		editingField = 'description';
	}

	function cancelEdit() {
		editingField = null;
		if (task) {
			editingTitle = task.title;
			editingDescription = task.description ?? '';
		}
	}

	async function saveTitle() {
		if (!task || !editingTitle.trim()) {
			cancelEdit();
			return;
		}
		try {
			const updated = await api.updateTask(task.id, { title: editingTitle.trim() });
			task = updated;
			editingField = null;
			await loadTasks();
		} catch (error) {
			console.error('Failed to save title:', error);
		}
	}

	async function saveDescription() {
		if (!task) return;
		try {
			const updated = await api.updateTask(task.id, {
				description: editingDescription.trim() || null
			});
			task = updated;
			editingField = null;
			await loadTasks();
		} catch (error) {
			console.error('Failed to save description:', error);
		}
	}

	function handleTitleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			saveTitle();
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}

	function handleDescKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			saveDescription();
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}

	async function removeConcept(conceptId: string) {
		if (!task) return;
		const newConcepts = (task.concepts || []).filter((c) => c !== conceptId);
		try {
			const updated = await api.updateTask(task.id, { concepts: newConcepts });
			task = updated;
			await Promise.all([loadTasks(), loadConcepts()]);
		} catch (error) {
			console.error('Failed to remove concept:', error);
		}
	}

	async function addConcept(conceptId: string) {
		if (!task || (task.concepts || []).includes(conceptId)) return;
		const newConcepts = [...(task.concepts || []), conceptId];
		try {
			const updated = await api.updateTask(task.id, { concepts: newConcepts });
			task = updated;
			await Promise.all([loadTasks(), loadConcepts()]);
		} catch (error) {
			console.error('Failed to add concept:', error);
		}
	}

	async function removeBlocker(blockerId: string) {
		if (!task) return;
		try {
			await api.removeBlocker(task.id, { blocker_id: blockerId });
			// Re-fetch task detail to get updated blockers
			const updated = await api.getTask(task.id);
			task = updated;
			await loadTasks();
		} catch (error) {
			console.error('Failed to remove blocker:', error);
		}
	}

	async function addBlocker(blockerId: string) {
		if (!task) return;
		try {
			await api.addBlocker(task.id, { blocker_id: blockerId });
			// Re-fetch task detail to get updated blockers
			const updated = await api.getTask(task.id);
			task = updated;
			await loadTasks();
		} catch (error) {
			console.error('Failed to add blocker:', error);
		}
	}

	const availableConcepts = $derived(
		$concepts.filter((c) => !(task?.concepts || []).includes(c.id))
	);

	const availableBlockers = $derived(
		$tasks.filter(
			(t) => t.id !== task?.id && t.status !== 'done' && !(task?.blocked_by || []).includes(t.id)
		)
	);
</script>

<Dialog.Root {open} onOpenChange={(v) => !v && onClose()}>
	<Dialog.Content class="max-h-[85vh] max-w-lg overflow-y-auto border-border bg-card p-0">
		{#if task}
			<div class="p-5">
				<!-- Title -->
				{#if editingField === 'title'}
					<input
						type="text"
						class="-ml-1 mb-1 w-[calc(100%+0.25rem)] rounded-sm border-none bg-transparent px-1 text-lg font-semibold text-foreground outline-none ring-1 ring-primary"
						bind:value={editingTitle}
						onkeydown={handleTitleKeydown}
						onblur={saveTitle}
					/>
				{:else}
					<button
						class="-ml-1 mb-1 rounded-sm px-1 text-left text-lg font-semibold text-foreground hover:bg-muted"
						onclick={startEditTitle}
					>
						{task.title}
					</button>
				{/if}

				<!-- Inline status/meta on same line as title area -->
				<div class="mb-4 flex items-center gap-2 text-xs text-muted-foreground">
					<span class="font-mono">{task.id}</span>
					<span class="text-muted-foreground/50">·</span>
					<span>{task.status}{task.current ? ' (active)' : ''}</span>
					{#if task.priority && task.priority !== 'p2'}
						<span class="text-muted-foreground/50">·</span>
						<span
							class={task.priority === 'p0'
								? 'text-destructive'
								: task.priority === 'p1'
									? 'text-warning'
									: ''}>{task.priority}</span
						>
					{/if}
				</div>

				<!-- Description -->
				{#if editingField === 'description'}
					<div class="mb-4">
						<textarea
							class="w-full rounded-md border border-border bg-muted p-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
							rows="3"
							bind:value={editingDescription}
							onkeydown={handleDescKeydown}
							placeholder="Add a description..."
						></textarea>
						<div class="mt-2 flex gap-2">
							<Button size="sm" class="h-7 text-xs" onclick={saveDescription}>Save</Button>
							<Button variant="ghost" size="sm" class="h-7 text-xs" onclick={cancelEdit}
								>Cancel</Button
							>
						</div>
					</div>
				{:else}
					<button
						class="-ml-1 mb-4 w-full rounded-sm p-1 text-left text-sm leading-relaxed text-muted-foreground hover:bg-muted hover:text-foreground"
						onclick={startEditDescription}
					>
						{#if task.description}
							<span class="whitespace-pre-wrap">{task.description}</span>
						{:else}
							<span class="italic">Add description...</span>
						{/if}
					</button>
				{/if}

				<!-- Concepts (inline tags) -->
				<div class="mb-3 flex flex-wrap items-center gap-1.5">
					{#each task.concepts || [] as conceptId}
						<Badge variant="secondary" class="gap-1 pr-1 text-xs">
							{getConceptName(conceptId)}
							<button
								class="ml-0.5 rounded-full p-0.5 hover:bg-muted-foreground/20"
								onclick={() => removeConcept(conceptId)}
								aria-label="Remove concept"
							>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="10"
									height="10"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.5"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<path d="M18 6 6 18"></path>
									<path d="m6 6 12 12"></path>
								</svg>
							</button>
						</Badge>
					{/each}
					{#if availableConcepts.length > 0}
						<select
							class="h-5 rounded border-none bg-transparent px-1 text-xs text-muted-foreground hover:text-foreground focus:outline-none"
							onchange={(e) => {
								const target = e.target as HTMLSelectElement;
								if (target.value) {
									addConcept(target.value);
									target.value = '';
								}
							}}
						>
							<option value="">+ concept</option>
							{#each availableConcepts as concept}
								<option value={concept.id}>{concept.name}</option>
							{/each}
						</select>
					{/if}
				</div>

				<!-- Blockers (only show if there are any or available to add) -->
				{#if (task.blocked_by || []).length > 0 || availableBlockers.length > 0}
					<div class="mb-4 flex flex-wrap items-center gap-1.5">
						{#if (task.blocked_by || []).length > 0}
							<span class="text-xs text-muted-foreground">Blocked by:</span>
						{/if}
						{#each task.blocked_by || [] as blockerId}
							<Badge variant="destructive" class="gap-1 pr-1 text-xs">
								{blockerId}
								<button
									class="ml-0.5 rounded-full p-0.5 hover:bg-white/20"
									onclick={() => removeBlocker(blockerId)}
									aria-label="Remove blocker"
								>
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="10"
										height="10"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										stroke-linecap="round"
										stroke-linejoin="round"
									>
										<path d="M18 6 6 18"></path>
										<path d="m6 6 12 12"></path>
									</svg>
								</button>
							</Badge>
						{/each}
						{#if availableBlockers.length > 0}
							<select
								class="h-5 rounded border-none bg-transparent px-1 text-xs text-muted-foreground hover:text-foreground focus:outline-none"
								onchange={(e) => {
									const target = e.target as HTMLSelectElement;
									if (target.value) {
										addBlocker(target.value);
										target.value = '';
									}
								}}
							>
								<option value="">+ blocker</option>
								{#each availableBlockers as t}
									<option value={t.id}>{t.title} ({t.id})</option>
								{/each}
							</select>
						{/if}
					</div>
				{/if}

				<!-- Checks (contextual, only if any exist) -->
				{#if (task.checks || []).length > 0}
					<div class="mb-4 rounded-md border border-border p-3">
						<div class="mb-2 flex items-center justify-between">
							<span class="text-xs font-medium text-foreground">Checks</span>
							<span
								class="text-xs {task.checks_verified === task.check_count
									? 'text-success'
									: 'text-muted-foreground'}"
							>
								{task.checks_verified}/{task.check_count} verified
							</span>
						</div>
						<div class="space-y-1.5">
							{#each task.checks || [] as check}
								<div
									class="flex items-center justify-between text-xs
										{check.severity === 'block' ? 'text-destructive' : ''}
										{check.severity === 'warn' ? 'text-warning' : ''}
										{check.severity === 'info' ? 'text-muted-foreground' : ''}"
								>
									<span>{check.message}</span>
									<span class={check.verified ? 'text-success' : 'opacity-40'}>
										{check.verified ? '✓' : '○'}
									</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Footer: branch + scope (subtle, only if relevant) -->
				{#if task.branch || (task.scope || []).length > 0}
					<div class="border-t border-border pt-3 text-xs text-muted-foreground">
						{#if task.branch}
							<p class="font-mono">{task.branch}</p>
						{/if}
						{#if (task.scope || []).length > 0}
							<div class="mt-1 space-y-0.5">
								{#each task.scope || [] as pattern}
									<p class="font-mono opacity-60">{pattern}</p>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</Dialog.Content>
</Dialog.Root>
