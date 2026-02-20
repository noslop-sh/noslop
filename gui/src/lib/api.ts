import { invoke } from '@tauri-apps/api/core';
import type { Review, Finding, FindingNote, StructuredDiff } from './types';

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

  // Findings
  addFinding: (
    reviewId: string,
    target: string,
    message: string,
    severity?: string,
    startLine?: number,
    endLine?: number
  ) =>
    invoke<Finding>('add_finding', {
      reviewId,
      target,
      message,
      severity,
      startLine,
      endLine,
    }),
  resolveFinding: (reviewId: string, findingId: string) =>
    invoke<void>('resolve_finding', { reviewId, findingId }),
  dismissFinding: (reviewId: string, findingId: string, reason: string) =>
    invoke<void>('dismiss_finding', { reviewId, findingId, reason }),
  applySuggestion: (reviewId: string, findingId: string) =>
    invoke<void>('apply_suggestion', { reviewId, findingId }),
  addFindingNote: (reviewId: string, findingId: string, content: string) =>
    invoke<FindingNote>('add_finding_note', { reviewId, findingId, content }),

  // Files
  markFileViewed: (reviewId: string, path: string) =>
    invoke<void>('mark_file_viewed', { reviewId, path }),

  // Git
  getCurrentBranch: () => invoke<string>('get_current_branch'),
  getBranches: () => invoke<string[]>('get_branches'),
  getDefaultBranch: () => invoke<string>('get_default_branch'),
  getMergeBase: (branch: string) => invoke<string>('get_merge_base', { branch }),
};
