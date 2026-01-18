<script lang="ts">
	import '../app.css';
	import { onMount, onDestroy } from 'svelte';
	import { loadAll, startPolling, stopPolling } from '$lib/stores';

	let { children } = $props();

	onMount(() => {
		loadAll();
		startPolling();
	});

	onDestroy(() => {
		stopPolling();
	});
</script>

<svelte:head>
	<title>noslop</title>
</svelte:head>

{@render children()}
