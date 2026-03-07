import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { api } from './api';

// --- Query hooks ---

export function useReviews(openOnly = true) {
  return createQuery({
    queryKey: ['reviews', openOnly],
    queryFn: () => api.listReviews(openOnly),
  });
}

export function useCurrentBranch() {
  return createQuery({
    queryKey: ['current-branch'],
    queryFn: () => api.getCurrentBranch(),
    staleTime: 1000 * 10,
  });
}

export function useBranches() {
  return createQuery({
    queryKey: ['branches'],
    queryFn: () => api.getBranches(),
    staleTime: 1000 * 30,
  });
}

export function useDefaultBranch() {
  return createQuery({
    queryKey: ['default-branch'],
    queryFn: () => api.getDefaultBranch(),
    staleTime: 1000 * 60 * 5,
  });
}

// --- Mutation hooks ---

export function useStartReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { base: string; head: string; branch?: string }) =>
      api.startReview(params.base, params.head, params.branch),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}

export function useCloseReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (id: string) => api.closeReview(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}

export function useResolveFeedback() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; feedbackId: string }) =>
      api.resolveFeedback(params.reviewId, params.feedbackId),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['reviews'] });
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useDismissFeedback() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; feedbackId: string; reason: string }) =>
      api.dismissFeedback(params.reviewId, params.feedbackId, params.reason),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useAddFeedback() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: {
      reviewId: string;
      target: string;
      message: string;
      severity?: string;
      startLine?: number;
      endLine?: number;
    }) =>
      api.addFeedback(
        params.reviewId,
        params.target,
        params.message,
        params.severity,
        params.startLine,
        params.endLine
      ),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useAddFeedbackNote() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; feedbackId: string; content: string }) =>
      api.addFeedbackNote(params.reviewId, params.feedbackId, params.content),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useRunAgentReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (reviewId: string) => api.runAgentReview(reviewId),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}

export function useMarkFileViewed() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; path: string }) =>
      api.markFileViewed(params.reviewId, params.path),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}
