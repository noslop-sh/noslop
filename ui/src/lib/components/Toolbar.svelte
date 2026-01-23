<script lang="ts">
	import * as Select from '$lib/components/ui/select';
	import { topics, currentTopic } from '$lib/stores';
	import * as api from '$lib/api/client';

	async function handleTopicChange(value: string | undefined) {
		const topicId = value === 'all' ? null : (value ?? null);
		try {
			await api.selectTopic({ id: topicId });
			currentTopic.set(topicId);
		} catch (error) {
			console.error('Failed to select topic:', error);
		}
	}

	const selectedValue = $derived($currentTopic ?? 'all');
</script>

<header class="flex h-12 shrink-0 items-center border-b border-border bg-card px-3">
	<!-- Left: Logo + nav -->
	<div class="flex items-center gap-1">
		<a href="/" class="mr-3 text-sm font-semibold text-foreground hover:text-primary">noslop</a>
		<a
			href="/"
			class="rounded px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
		>
			Tasks
		</a>
		<a
			href="/checks"
			class="rounded px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
		>
			Checks
		</a>
		<a
			href="/topics"
			class="rounded px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
		>
			Topics
		</a>
	</div>

	<!-- Center spacer -->
	<div class="flex-1"></div>

	<!-- Right: Topic filter only -->
	<Select.Root type="single" value={selectedValue} onValueChange={handleTopicChange}>
		<Select.Trigger
			class="h-7 w-32 border-0 bg-transparent text-xs text-muted-foreground hover:text-foreground"
		>
			{#if selectedValue === 'all'}
				All tasks
			{:else}
				{$topics.find((c) => c.id === selectedValue)?.name ?? '?'}
			{/if}
		</Select.Trigger>
		<Select.Content>
			<Select.Item value="all">All tasks</Select.Item>
			{#each $topics as topic}
				<Select.Item value={topic.id}>{topic.name}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
</header>
