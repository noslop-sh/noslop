<script lang="ts">
	import Toolbar from '$lib/components/Toolbar.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { checks, loadChecks } from '$lib/stores';
	import * as api from '$lib/api/client';

	let showForm = $state(false);
	let scope = $state('');
	let message = $state('');
	let severity = $state<string>('block');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!scope.trim() || !message.trim()) return;

		try {
			await api.createCheck({
				scope: scope.trim(),
				message: message.trim(),
				severity
			});
			scope = '';
			message = '';
			severity = 'block';
			showForm = false;
			await loadChecks();
		} catch (error) {
			console.error('Failed to create check:', error);
		}
	}

	function toggleForm() {
		showForm = !showForm;
	}

	function hideForm() {
		showForm = false;
	}

	function getSeverityColor(sev: string): string {
		switch (sev) {
			case 'block':
				return 'border-l-destructive';
			case 'warn':
				return 'border-l-warning';
			case 'info':
				return 'border-l-info';
			default:
				return '';
		}
	}
</script>

<div class="flex h-screen flex-col bg-background">
	<Toolbar />

	<div class="flex h-10 shrink-0 items-center justify-between border-b border-border px-4">
		<h1 class="text-sm font-medium text-foreground">Checks</h1>
		<Button size="sm" class="h-7 text-xs" onclick={toggleForm}>+ New</Button>
	</div>

	<main class="mx-auto w-full max-w-3xl flex-1 overflow-y-auto p-4">
		<!-- New check form -->
		{#if showForm}
			<div class="mb-6 rounded-md border border-border bg-card p-4">
				<form onsubmit={handleSubmit} class="space-y-4">
					<div>
						<label for="check-scope" class="mb-1 block text-sm text-muted-foreground"
							>Scope (file or pattern)</label
						>
						<Input id="check-scope" bind:value={scope} placeholder="src/**/*.rs" />
					</div>
					<div>
						<label for="check-message" class="mb-1 block text-sm text-muted-foreground"
							>Message</label
						>
						<Input id="check-message" bind:value={message} placeholder="What must be verified..." />
					</div>
					<div>
						<label for="check-severity" class="mb-1 block text-sm text-muted-foreground"
							>Severity</label
						>
						<Select.Root type="single" value={severity} onValueChange={(v) => v && (severity = v)}>
							<Select.Trigger id="check-severity" class="w-full">
								{severity}
							</Select.Trigger>
							<Select.Content>
								<Select.Item value="block">Block</Select.Item>
								<Select.Item value="warn">Warn</Select.Item>
								<Select.Item value="info">Info</Select.Item>
							</Select.Content>
						</Select.Root>
					</div>
					<div class="flex gap-2">
						<Button type="submit" size="sm">Create Check</Button>
						<Button type="button" variant="outline" size="sm" onclick={hideForm}>Cancel</Button>
					</div>
				</form>
			</div>
		{/if}

		<!-- Checks list -->
		<div class="space-y-2">
			{#each $checks as check (check.id)}
				<div
					class="rounded-md border border-border border-l-4 bg-card p-3 {getSeverityColor(
						check.severity
					)}"
				>
					<div class="flex items-start justify-between">
						<div class="flex-1">
							<div class="mb-1 flex items-center gap-2">
								<span class="text-sm font-medium text-foreground">{check.id}</span>
								<Badge variant="outline" class="text-xs">{check.severity}</Badge>
							</div>
							<p class="mb-1 font-mono text-xs text-muted-foreground">{check.scope}</p>
							<p class="text-sm text-foreground">{check.message}</p>
						</div>
					</div>
				</div>
			{:else}
				<div
					class="flex h-32 items-center justify-center rounded-lg border border-dashed border-border text-sm text-muted-foreground"
				>
					No checks defined. Create one to require verification when specific files change.
				</div>
			{/each}
		</div>
	</main>
</div>
