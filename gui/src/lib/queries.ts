import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { api } from './api';

// --- Query hooks ---

export function useReviews(openOnly = true) {
  return createQuery({
    queryKey: ['reviews', openOnly],
    queryFn: () => api.listReviews(openOnly),
  });
}

export function useReview(id: string) {
  return createQuery({
    queryKey: ['review', id],
    queryFn: () => api.getReview(id),
    enabled: !!id,
  });
}

export function useStructuredDiff(base: string, head: string) {
  return createQuery({
    queryKey: ['structured-diff', base, head],
    queryFn: () => api.getStructuredDiff(base, head),
    enabled: !!base && !!head,
    staleTime: 1000 * 60 * 5,
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

export function useReopenReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (id: string) => api.reopenReview(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}

export function useResolveFinding() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; findingId: string }) =>
      api.resolveFinding(params.reviewId, params.findingId),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['reviews'] });
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useDismissFinding() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; findingId: string; reason: string }) =>
      api.dismissFinding(params.reviewId, params.findingId, params.reason),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
    },
  });
}

export function useApplySuggestion() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; findingId: string }) =>
      api.applySuggestion(params.reviewId, params.findingId),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
      client.invalidateQueries({ queryKey: ['structured-diff'] });
    },
  });
}

export function useAddFinding() {
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
      api.addFinding(
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

export function useAddFindingNote() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; findingId: string; content: string }) =>
      api.addFindingNote(params.reviewId, params.findingId, params.content),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
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
