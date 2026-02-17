import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { api } from './api';

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

export function useDiff(base: string, head: string) {
  return createQuery({
    queryKey: ['diff', base, head],
    queryFn: () => api.getDiff(base, head),
    enabled: !!base && !!head,
  });
}

export function useStartReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { base: string; head: string }) =>
      api.startReview(params.base, params.head),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}

export function useAddFinding() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; target: string; message: string; severity?: string }) =>
      api.addFinding(params.reviewId, params.target, params.message, params.severity),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ['review', reviewId] });
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

export function useCloseReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (id: string) => api.closeReview(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['reviews'] });
    },
  });
}
