<script lang="ts">
  import '../app.css';
  import favicon from '$lib/assets/favicon.svg';
  import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
  import { browser } from '$app/environment';
  import { getSession } from '$lib/session';

  let { children } = $props();

  // Apply dark mode before first paint
  if (browser) {
    const session = getSession();
    if (session.theme === 'dark') {
      document.documentElement.classList.add('dark');
    }
  }

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 1000 * 30, // 30 seconds
        refetchOnWindowFocus: true,
      },
    },
  });
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<QueryClientProvider client={queryClient}>
  {@render children()}
</QueryClientProvider>
