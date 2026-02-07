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
    const mockReview = { id: 'REV-1234', status: 'open', comments: [] };
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
    const mockReview = { id: 'REV-1234', base_sha: 'abc', head_sha: 'def' };
    vi.mocked(invoke).mockResolvedValue(mockReview);
    const result = await api.startReview('abc', 'def');
    expect(invoke).toHaveBeenCalledWith('start_review', { base: 'abc', head: 'def' });
    expect(result).toEqual(mockReview);
  });

  it('addComment calls invoke with all params', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'REV-1234:1' });
    await api.addComment('REV-1234', 'src/main.rs', 'Fix this', 42);
    expect(invoke).toHaveBeenCalledWith('add_comment', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      line: 42,
    });
  });

  it('addComment works without line number', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'REV-1234:1' });
    await api.addComment('REV-1234', 'src/main.rs', 'Fix this');
    expect(invoke).toHaveBeenCalledWith('add_comment', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      line: undefined,
    });
  });

  it('resolveComment calls invoke correctly', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.resolveComment('REV-1234:1', 'Fixed');
    expect(invoke).toHaveBeenCalledWith('resolve_comment', {
      commentId: 'REV-1234:1',
      message: 'Fixed',
    });
  });

  it('resolveComment works without message', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.resolveComment('REV-1234:1');
    expect(invoke).toHaveBeenCalledWith('resolve_comment', {
      commentId: 'REV-1234:1',
      message: undefined,
    });
  });

  it('closeReview calls invoke with id', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.closeReview('REV-1234');
    expect(invoke).toHaveBeenCalledWith('close_review', { id: 'REV-1234' });
  });
});
