import { invoke } from '@tauri-apps/api/core';
import type { Review, Feedback, FeedbackNote, StructuredDiff, AgentReviewResult } from './types';

export const api = {
  // Reviews
  listReviews: (openOnly: boolean) => invoke<Review[]>('list_reviews', { openOnly }),
  getReview: (id: string) => invoke<Review>('get_review', { id }),
  startReview: (base: string, head: string, branch?: string) =>
    invoke<Review>('start_review', { base, head, branch }),
  closeReview: (id: string) => invoke<void>('close_review', { id }),
  reopenReview: (id: string) => invoke<void>('reopen_review', { id }),

  // Diff
  getDiff: (base: string, head: string) => invoke<string>('get_diff', { base, head }),
  getStructuredDiff: (base: string, head: string) =>
    invoke<StructuredDiff>('get_structured_diff', { base, head }),
  getFileContent: (path: string, commit: string, startLine: number, endLine: number) =>
    invoke<string>('get_file_content', { path, commit, startLine, endLine }),

  // Feedbacks
  addFeedback: (
    reviewId: string,
    target: string,
    message: string,
    severity?: string,
    startLine?: number,
    endLine?: number
  ) =>
    invoke<Feedback>('add_feedback', {
      reviewId,
      target,
      message,
      severity,
      startLine,
      endLine,
    }),
  resolveFeedback: (reviewId: string, feedbackId: string) =>
    invoke<void>('resolve_feedback', { reviewId, feedbackId }),
  dismissFeedback: (reviewId: string, feedbackId: string, reason: string) =>
    invoke<void>('dismiss_feedback', { reviewId, feedbackId, reason }),
  applySuggestion: (reviewId: string, feedbackId: string) =>
    invoke<void>('apply_suggestion', { reviewId, feedbackId }),
  addFeedbackNote: (reviewId: string, feedbackId: string, content: string) =>
    invoke<FeedbackNote>('add_feedback_note', { reviewId, feedbackId, content }),

  // Files
  markFileViewed: (reviewId: string, path: string) =>
    invoke<void>('mark_file_viewed', { reviewId, path }),

  // Agent
  runAgentReview: (reviewId: string) => invoke<AgentReviewResult>('run_agent_review', { reviewId }),

  // Git
  getCurrentBranch: () => invoke<string>('get_current_branch'),
  getBranches: () => invoke<string[]>('get_branches'),
  getDefaultBranch: () => invoke<string>('get_default_branch'),
  getMergeBase: (branch: string) => invoke<string>('get_merge_base', { branch }),
};
