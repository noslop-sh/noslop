<script lang="ts">
	import Toolbar from '$lib/components/Toolbar.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import { topics, loadTopics } from '$lib/stores';
	import * as api from '$lib/api/client';

	let showForm = $state(false);
	let name = $state('');
	let description = $state('');

	// Track which topic is being edited or deleted
	let editingId = $state<string | null>(null);
	let editingName = $state('');
	let editingDescription = $state('');
	let editingField = $state<'name' | 'description' | null>(null);
	let confirmingDeleteId = $state<string | null>(null);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!name.trim()) return;

		try {
			await api.createTopic({
				name: name.trim(),
				description: description.trim() || undefined
			});
			name = '';
			description = '';
			showForm = false;
			await loadTopics();
		} catch (error) {
			console.error('Failed to create topic:', error);
		}
	}

	function toggleForm() {
		showForm = !showForm;
	}

	function hideForm() {
		showForm = false;
	}

	function startDelete(id: string) {
		confirmingDeleteId = id;
	}

	function cancelDelete() {
		confirmingDeleteId = null;
	}

	async function handleDelete(id: string) {
		try {
			await api.deleteTopic(id);
			await loadTopics();
		} catch (error) {
			console.error('Failed to delete topic:', error);
		}
		confirmingDeleteId = null;
	}

	function startEditName(id: string, currentName: string) {
		editingId = id;
		editingName = currentName;
		editingField = 'name';
	}

	function startEditDescription(id: string, currentDesc: string | undefined) {
		editingId = id;
		editingDescription = currentDesc ?? '';
		editingField = 'description';
	}

	function cancelEdit() {
		editingId = null;
		editingName = '';
		editingDescription = '';
		editingField = null;
	}

	async function saveName(id: string) {
		if (!editingName.trim()) {
			cancelEdit();
			return;
		}
		try {
			await api.updateTopic(id, { name: editingName.trim() });
			await loadTopics();
		} catch (error) {
			console.error('Failed to update topic:', error);
		}
		cancelEdit();
	}

	async function saveDescription(id: string) {
		try {
			await api.updateTopic(id, { description: editingDescription.trim() || null });
			await loadTopics();
		} catch (error) {
			console.error('Failed to update topic:', error);
		}
		cancelEdit();
	}

	function handleNameKeydown(e: KeyboardEvent, id: string) {
		if (e.key === 'Enter') {
			e.preventDefault();
			saveName(id);
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}

	function handleDescKeydown(e: KeyboardEvent, id: string) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			saveDescription(id);
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}
</script>

<div class="flex h-screen flex-col bg-background">
	<Toolbar />

	<div class="flex h-10 shrink-0 items-center justify-between border-b border-border px-4">
		<h1 class="text-sm font-medium text-foreground">Topics</h1>
		<Button size="sm" class="h-7 text-xs" onclick={toggleForm}>+ New</Button>
	</div>

	<main class="mx-auto w-full max-w-3xl flex-1 overflow-y-auto p-4">
		<!-- New topic form -->
		{#if showForm}
			<div class="mb-6 rounded-md border border-border bg-card p-4">
				<form onsubmit={handleSubmit} class="space-y-4">
					<div>
						<label for="topic-name" class="mb-1 block text-sm text-muted-foreground">Name</label>
						<Input id="topic-name" bind:value={name} placeholder="Authentication" />
					</div>
					<div>
						<label for="topic-description" class="mb-1 block text-sm text-muted-foreground"
							>Description (for LLM context)</label
						>
						<textarea
							id="topic-description"
							class="w-full rounded-md border border-border bg-muted p-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
							rows="3"
							bind:value={description}
							placeholder="User login, session management, OAuth integration..."
						></textarea>
					</div>
					<div class="flex gap-2">
						<Button type="submit" size="sm">Create Topic</Button>
						<Button type="button" variant="outline" size="sm" onclick={hideForm}>Cancel</Button>
					</div>
				</form>
			</div>
		{/if}

		<!-- Topics list -->
		<div class="space-y-2">
			{#each $topics as topic (topic.id)}
				<div class="group rounded-md border border-border bg-card p-3">
					<div class="flex items-start justify-between">
						<div class="flex-1">
							<!-- Name row -->
							<div class="mb-1 flex items-center gap-2">
								{#if editingId === topic.id && editingField === 'name'}
									<input
										type="text"
										class="flex-1 rounded border border-border bg-muted px-2 py-1 text-sm font-medium text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
										bind:value={editingName}
										onkeydown={(e) => handleNameKeydown(e, topic.id)}
										onblur={() => saveName(topic.id)}
									/>
								{:else}
									<button
										class="text-sm font-medium text-foreground hover:text-primary"
										onclick={() => startEditName(topic.id, topic.name)}
									>
										{topic.name}
									</button>
								{/if}
								<Badge variant="secondary" class="text-xs">{topic.task_count} tasks</Badge>
							</div>

							<!-- Scope -->
							{#if (topic.scope || []).length > 0}
								<p class="mb-1 font-mono text-xs text-muted-foreground">
									{(topic.scope || []).join(', ')}
								</p>
							{/if}

							<!-- Description -->
							{#if editingId === topic.id && editingField === 'description'}
								<div class="mt-2">
									<textarea
										class="w-full rounded-md border border-border bg-muted p-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
										rows="2"
										bind:value={editingDescription}
										onkeydown={(e) => handleDescKeydown(e, topic.id)}
										placeholder="Enter description..."
									></textarea>
									<div class="mt-2 flex gap-2">
										<Button size="sm" class="h-7 text-xs" onclick={() => saveDescription(topic.id)}
											>Save</Button
										>
										<Button variant="ghost" size="sm" class="h-7 text-xs" onclick={cancelEdit}
											>Cancel</Button
										>
									</div>
								</div>
							{:else}
								<button
									class="mt-1 w-full text-left text-sm text-muted-foreground hover:text-foreground"
									onclick={() => startEditDescription(topic.id, topic.description)}
								>
									{#if topic.description}
										{topic.description}
									{:else}
										<span class="italic">Click to add description...</span>
									{/if}
								</button>
							{/if}
						</div>

						<!-- Action icons -->
						{#if confirmingDeleteId === topic.id}
							<div class="flex items-center gap-1">
								<Button
									variant="destructive"
									size="sm"
									class="h-7 px-2 text-xs"
									onclick={() => handleDelete(topic.id)}
								>
									Delete
								</Button>
								<Button variant="ghost" size="sm" class="h-7 px-2 text-xs" onclick={cancelDelete}>
									Cancel
								</Button>
							</div>
						{:else if editingId !== topic.id}
							<button
								class="p-1 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover:opacity-100"
								onclick={() => startDelete(topic.id)}
								title="Delete topic"
							>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="16"
									height="16"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<path d="M3 6h18"></path>
									<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path>
									<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path>
								</svg>
							</button>
						{/if}
					</div>
				</div>
			{:else}
				<div
					class="flex h-32 items-center justify-center rounded-lg border border-dashed border-border text-sm text-muted-foreground"
				>
					No topics defined. Create one to group related tasks and scope patterns.
				</div>
			{/each}
		</div>
	</main>
</div>
