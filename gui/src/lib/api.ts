import { invoke } from '@tauri-apps/api/core';
import type { Review, Comment } from './types';

export const api = {
  listReviews: (openOnly: boolean) => invoke<Review[]>('list_reviews', { openOnly }),

  getReview: (id: string) => invoke<Review>('get_review', { id }),

  getDiff: (base: string, head: string) => invoke<string>('get_diff', { base, head }),

  startReview: (base: string, head: string) => invoke<Review>('start_review', { base, head }),

  addComment: (reviewId: string, target: string, message: string, line?: number) =>
    invoke<Comment>('add_comment', { reviewId, target, message, line }),

  resolveComment: (commentId: string, message?: string) =>
    invoke<void>('resolve_comment', { commentId, message }),

  closeReview: (id: string) => invoke<void>('close_review', { id }),
};
