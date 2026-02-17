import { invoke } from '@tauri-apps/api/core';
import type { Review, Finding } from './types';

export const api = {
  listReviews: (openOnly: boolean) => invoke<Review[]>('list_reviews', { openOnly }),

  getReview: (id: string) => invoke<Review>('get_review', { id }),

  getDiff: (base: string, head: string) => invoke<string>('get_diff', { base, head }),

  startReview: (base: string, head: string) => invoke<Review>('start_review', { base, head }),

  addFinding: (reviewId: string, target: string, message: string, severity?: string) =>
    invoke<Finding>('add_finding', { reviewId, target, message, severity }),

  resolveFinding: (reviewId: string, findingId: string) =>
    invoke<void>('resolve_finding', { reviewId, findingId }),

  closeReview: (id: string) => invoke<void>('close_review', { id }),
};
