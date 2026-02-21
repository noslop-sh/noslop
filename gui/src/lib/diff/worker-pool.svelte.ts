import { getContext, setContext, onDestroy } from 'svelte';
import {
  getOrCreateWorkerPoolSingleton,
  terminateWorkerPoolSingleton,
  type WorkerPoolManager,
} from '@pierre/diffs/worker';

const WORKER_POOL_KEY = Symbol('worker-pool');

/**
 * Call in the top-level component (DiffView) to create the shared worker pool
 * and provide it via Svelte context to all child components.
 */
export function provideWorkerPool(): WorkerPoolManager {
  const pool = getOrCreateWorkerPoolSingleton({
    poolOptions: {
      workerFactory: () =>
        new Worker(new URL('@pierre/diffs/worker/worker.js', import.meta.url), {
          type: 'module',
        }),
      poolSize: 8,
    },
    highlighterOptions: {
      theme: { dark: 'github-dark', light: 'github-light' },
    },
  });
  pool.initialize();

  setContext(WORKER_POOL_KEY, pool);

  onDestroy(() => {
    terminateWorkerPoolSingleton();
  });

  return pool;
}

/**
 * Call in child components (FileDiffRenderer) to access the shared worker pool.
 */
export function useWorkerPool(): WorkerPoolManager {
  return getContext<WorkerPoolManager>(WORKER_POOL_KEY);
}
