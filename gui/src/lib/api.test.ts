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

  // --- Reviews ---

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
    const mockReview = { id: 'REV-1234', status: 'open', feedbacks: [] };
    vi.mocked(invoke).mockResolvedValue(mockReview);
    const result = await api.getReview('REV-1234');
    expect(invoke).toHaveBeenCalledWith('get_review', { id: 'REV-1234' });
    expect(result).toEqual(mockReview);
  });

  it('startReview calls invoke with base and head', async () => {
    const mockReview = { id: 'REV-1234', base: 'abc', head: 'def' };
    vi.mocked(invoke).mockResolvedValue(mockReview);
    const result = await api.startReview('abc', 'def');
    expect(invoke).toHaveBeenCalledWith('start_review', { base: 'abc', head: 'def' });
    expect(result).toEqual(mockReview);
  });

  it('closeReview calls invoke with id', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.closeReview('REV-1234');
    expect(invoke).toHaveBeenCalledWith('close_review', { id: 'REV-1234' });
  });

  // --- Diff ---

  it('getStructuredDiff calls invoke with base and head', async () => {
    const mockDiff = { files: [], stats: { files_changed: 0, additions: 0, deletions: 0 } };
    vi.mocked(invoke).mockResolvedValue(mockDiff);
    const result = await api.getStructuredDiff('abc123', 'def456');
    expect(invoke).toHaveBeenCalledWith('get_structured_diff', {
      base: 'abc123',
      head: 'def456',
    });
    expect(result).toEqual(mockDiff);
  });

  it('getFileContent calls invoke with all params', async () => {
    vi.mocked(invoke).mockResolvedValue('line 10\nline 11\n');
    await api.getFileContent('src/main.rs', 'abc123', 10, 11);
    expect(invoke).toHaveBeenCalledWith('get_file_content', {
      path: 'src/main.rs',
      commit: 'abc123',
      startLine: 10,
      endLine: 11,
    });
  });

  // --- Feedbacks ---

  it('addFeedback calls invoke with all params including line span', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'F-12345678' });
    await api.addFeedback('REV-1234', 'src/main.rs', 'Fix this', 'block', 10, 15);
    expect(invoke).toHaveBeenCalledWith('add_feedback', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      severity: 'block',
      startLine: 10,
      endLine: 15,
    });
  });

  it('addFeedback works without optional params', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'F-12345678' });
    await api.addFeedback('REV-1234', 'src/main.rs', 'Fix this');
    expect(invoke).toHaveBeenCalledWith('add_feedback', {
      reviewId: 'REV-1234',
      target: 'src/main.rs',
      message: 'Fix this',
      severity: undefined,
      startLine: undefined,
      endLine: undefined,
    });
  });

  it('resolveFeedback calls invoke correctly', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.resolveFeedback('REV-1234', 'F-12345678');
    expect(invoke).toHaveBeenCalledWith('resolve_feedback', {
      reviewId: 'REV-1234',
      feedbackId: 'F-12345678',
    });
  });

  it('dismissFeedback calls invoke with reason', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.dismissFeedback('REV-1234', 'F-12345678', 'false_positive');
    expect(invoke).toHaveBeenCalledWith('dismiss_feedback', {
      reviewId: 'REV-1234',
      feedbackId: 'F-12345678',
      reason: 'false_positive',
    });
  });

  it('addFeedbackNote calls invoke correctly', async () => {
    const mockNote = { id: 'N-1', content: 'test note', created_at: '2026-01-01' };
    vi.mocked(invoke).mockResolvedValue(mockNote);
    const result = await api.addFeedbackNote('REV-1234', 'F-12345678', 'test note');
    expect(invoke).toHaveBeenCalledWith('add_feedback_note', {
      reviewId: 'REV-1234',
      feedbackId: 'F-12345678',
      content: 'test note',
    });
    expect(result).toEqual(mockNote);
  });

  // --- Files ---

  it('markFileViewed calls invoke correctly', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await api.markFileViewed('REV-1234', 'src/main.rs');
    expect(invoke).toHaveBeenCalledWith('mark_file_viewed', {
      reviewId: 'REV-1234',
      path: 'src/main.rs',
    });
  });

  // --- Git ---

  it('getCurrentBranch calls invoke', async () => {
    vi.mocked(invoke).mockResolvedValue('feature/auth');
    const result = await api.getCurrentBranch();
    expect(invoke).toHaveBeenCalledWith('get_current_branch');
    expect(result).toBe('feature/auth');
  });

  it('getBranches calls invoke', async () => {
    vi.mocked(invoke).mockResolvedValue(['main', 'feature/auth']);
    const result = await api.getBranches();
    expect(invoke).toHaveBeenCalledWith('get_branches');
    expect(result).toEqual(['main', 'feature/auth']);
  });
});
