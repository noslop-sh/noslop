import { describe, it, expect, vi, beforeEach } from 'vitest';
import { api } from './api';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('api', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listReviews calls invoke with correct params', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await api.listReviews(true);
    expect(invoke).toHaveBeenCalledWith('list_reviews', { openOnly: true });
  });

  it('listReviews passes openOnly=false', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await api.listReviews(false);
    expect(invoke).toHaveBeenCalledWith('list_reviews', { openOnly: false });
  });

  it('getReview calls invoke with id', async () => {
    const mockReview = { id: 'REV-1234', status: 'open', findings: [] };
    vi.mocked(invoke).mockResolvedValue(mockReview);
    const result = await api.getReview('REV-1234');
    expect(invoke).toHaveBeenCalledWith('get_review', { id: 'REV-1234' });
    expect(result).toEqual(mockReview);
  });

  it('getDiff calls invoke with base and head', async () => {
    vi.mocked(invoke).mockResolvedValue('diff content');
    const result = await api.getDiff('abc123', 'def456');
    expect(invoke).toHaveBeenCalledWith('get_diff', { base: 'abc123', head: 'def456' });
    expect(result).toBe('diff content');
  });

  it('startReview calls invoke with base and head', async () => {
    const mockReview = { id: 'REV-1234', base: 'abc', head: 'def' };
    vi.mocked(invoke).mockResolvedValue(mockReview);
    const result = await api.startReview('abc', 'def');
    expect(invoke).toHaveBeenCalledWith('start_review', { base: 'abc', head: 'def' });
    expect(result).toEqual(mockReview);
  });

  it('addFinding calls invoke with all params', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'F-12345678' });
    await api.addFinding('REV-1234', 'src/main.rs', 'Fix this', 'block');
    expect(invoke).toHaveBeenCalledWith('add_finding', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      severity: 'block',
    });
  });

  it('addFinding works without severity', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'F-12345678' });
    await api.addFinding('REV-1234', 'src/main.rs', 'Fix this');
    expect(invoke).toHaveBeenCalledWith('add_finding', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      severity: undefined,
    });
  });

  it('resolveFinding calls invoke correctly', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.resolveFinding('REV-1234', 'F-12345678');
    expect(invoke).toHaveBeenCalledWith('resolve_finding', {
      reviewId: 'REV-1234',
      findingId: 'F-12345678',
    });
  });

  it('closeReview calls invoke with id', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.closeReview('REV-1234');
    expect(invoke).toHaveBeenCalledWith('close_review', { id: 'REV-1234' });
  });
});
