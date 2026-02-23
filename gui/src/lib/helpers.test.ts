import { describe, it, expect } from 'vitest';
import {
  openFeedbackCount,
  blockingFeedbacks,
  feedbacksForFile,
  feedbackCountsByFile,
  applyFeedbackFilters,
  sortFeedbacksBySeverity,
  formatSource,
  changeTypeLabel,
  changeTypeColor,
  formatDate,
  formatRelativeDate,
  buildFileTree,
} from './helpers';
import type { Feedback, FileDiff } from './types';

function makeFeedback(overrides: Partial<Feedback> = {}): Feedback {
  return {
    id: 'F-001',
    target: { path: 'src/main.rs', span: null, commit: null },
    severity: 'block',
    message: 'Test feedback',
    source: { kind: 'check', name: 'NOS-1' },
    status: 'open',
    suggestion: null,
    dismiss_reason: null,
    resolution_reason: null,
    confidence: null,
    notes: [],
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

function makeFileDiff(overrides: Partial<FileDiff> = {}): FileDiff {
  return {
    path: 'src/main.rs',
    old_path: null,
    change_type: 'modified',
    hunks: [],
    additions: 10,
    deletions: 5,
    is_binary: false,
    language: 'rust',
    ...overrides,
  };
}

describe('openFeedbackCount', () => {
  it('counts only open feedbacks', () => {
    const feedbacks = [
      makeFeedback({ id: 'F-1', status: 'open' }),
      makeFeedback({ id: 'F-2', status: 'resolved' }),
      makeFeedback({ id: 'F-3', status: 'open' }),
      makeFeedback({ id: 'F-4', status: 'dismissed' }),
    ];
    expect(openFeedbackCount(feedbacks)).toBe(2);
  });

  it('returns 0 for empty array', () => {
    expect(openFeedbackCount([])).toBe(0);
  });
});

describe('blockingFeedbacks', () => {
  it('returns only open block-severity feedbacks', () => {
    const feedbacks = [
      makeFeedback({ id: 'F-1', severity: 'block', status: 'open' }),
      makeFeedback({ id: 'F-2', severity: 'block', status: 'resolved' }),
      makeFeedback({ id: 'F-3', severity: 'warn', status: 'open' }),
      makeFeedback({ id: 'F-4', severity: 'block', status: 'open' }),
    ];
    const result = blockingFeedbacks(feedbacks);
    expect(result).toHaveLength(2);
    expect(result.map((f) => f.id)).toEqual(['F-1', 'F-4']);
  });
});

describe('feedbacksForFile', () => {
  it('filters feedbacks by file path', () => {
    const feedbacks = [
      makeFeedback({ id: 'F-1', target: { path: 'src/auth.rs', span: null, commit: null } }),
      makeFeedback({ id: 'F-2', target: { path: 'src/main.rs', span: null, commit: null } }),
      makeFeedback({ id: 'F-3', target: { path: 'src/auth.rs', span: null, commit: null } }),
    ];
    const result = feedbacksForFile(feedbacks, 'src/auth.rs');
    expect(result).toHaveLength(2);
    expect(result.map((f) => f.id)).toEqual(['F-1', 'F-3']);
  });
});

describe('feedbackCountsByFile', () => {
  it('counts open feedbacks by severity for a file', () => {
    const feedbacks = [
      makeFeedback({ id: 'F-1', severity: 'block', status: 'open' }),
      makeFeedback({ id: 'F-2', severity: 'warn', status: 'open' }),
      makeFeedback({ id: 'F-3', severity: 'block', status: 'resolved' }),
      makeFeedback({ id: 'F-4', severity: 'info', status: 'open' }),
    ];
    const result = feedbackCountsByFile(feedbacks, 'src/main.rs');
    expect(result).toEqual({ block: 1, warn: 1, info: 1 });
  });
});

describe('applyFeedbackFilters', () => {
  const feedbacks = [
    makeFeedback({
      id: 'F-1',
      status: 'open',
      severity: 'block',
      source: { kind: 'check', name: 'x' },
    }),
    makeFeedback({
      id: 'F-2',
      status: 'resolved',
      severity: 'warn',
      source: { kind: 'agent', name: 'y' },
    }),
    makeFeedback({
      id: 'F-3',
      status: 'open',
      severity: 'info',
      source: { kind: 'human', name: null },
    }),
  ];

  it('filters by status', () => {
    const result = applyFeedbackFilters(feedbacks, {
      status: 'open',
      severity: 'all',
      source: 'all',
    });
    expect(result).toHaveLength(2);
  });

  it('filters by severity', () => {
    const result = applyFeedbackFilters(feedbacks, {
      status: 'all',
      severity: 'block',
      source: 'all',
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe('F-1');
  });

  it('filters by source kind', () => {
    const result = applyFeedbackFilters(feedbacks, {
      status: 'all',
      severity: 'all',
      source: 'human',
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe('F-3');
  });

  it('all filters pass everything', () => {
    const result = applyFeedbackFilters(feedbacks, { status: 'all', severity: 'all', source: 'all' });
    expect(result).toHaveLength(3);
  });
});

describe('sortFeedbacksBySeverity', () => {
  it('sorts block first, then warn, then info', () => {
    const feedbacks = [
      makeFeedback({ id: 'F-info', severity: 'info' }),
      makeFeedback({ id: 'F-block', severity: 'block' }),
      makeFeedback({ id: 'F-warn', severity: 'warn' }),
    ];
    const sorted = sortFeedbacksBySeverity(feedbacks);
    expect(sorted.map((f) => f.id)).toEqual(['F-block', 'F-warn', 'F-info']);
  });

  it('does not mutate original array', () => {
    const feedbacks = [makeFeedback({ severity: 'info' }), makeFeedback({ severity: 'block' })];
    const sorted = sortFeedbacksBySeverity(feedbacks);
    expect(sorted).not.toBe(feedbacks);
    expect(feedbacks[0].severity).toBe('info');
  });
});

describe('formatSource', () => {
  it('formats check source', () => {
    expect(formatSource({ kind: 'check', name: 'NOS-1' })).toBe('check:NOS-1');
  });

  it('formats human source', () => {
    expect(formatSource({ kind: 'human', name: null })).toBe('human');
  });

  it('formats agent source', () => {
    expect(formatSource({ kind: 'agent', name: 'security' })).toBe('agent:security');
  });
});

describe('changeTypeLabel', () => {
  it('returns first letter for string types', () => {
    expect(changeTypeLabel('added')).toBe('A');
    expect(changeTypeLabel('modified')).toBe('M');
    expect(changeTypeLabel('deleted')).toBe('D');
  });

  it('returns R for renamed', () => {
    expect(changeTypeLabel({ renamed: { similarity: 90 } })).toBe('R');
  });
});

describe('changeTypeColor', () => {
  it('returns green for added', () => {
    expect(changeTypeColor('added')).toContain('green');
  });

  it('returns yellow for modified', () => {
    expect(changeTypeColor('modified')).toContain('yellow');
  });

  it('returns red for deleted', () => {
    expect(changeTypeColor('deleted')).toContain('red');
  });

  it('returns blue for renamed', () => {
    expect(changeTypeColor({ renamed: { similarity: 90 } })).toContain('blue');
  });
});

describe('formatDate', () => {
  it('formats ISO date string', () => {
    const result = formatDate('2026-01-15T10:30:00Z');
    expect(result).toBeTruthy();
    expect(typeof result).toBe('string');
  });
});

describe('formatRelativeDate', () => {
  it('returns "just now" for recent dates', () => {
    const now = new Date().toISOString();
    expect(formatRelativeDate(now)).toBe('just now');
  });
});

describe('buildFileTree', () => {
  it('builds flat tree for files in root', () => {
    const files = [makeFileDiff({ path: 'README.md' }), makeFileDiff({ path: 'Cargo.toml' })];
    const tree = buildFileTree(files, [], new Set(), 'alphabetical', '');
    expect(tree).toHaveLength(2);
    expect(tree[0].name).toBe('Cargo.toml');
    expect(tree[1].name).toBe('README.md');
  });

  it('builds nested tree with directories', () => {
    const files = [
      makeFileDiff({ path: 'src/main.rs' }),
      makeFileDiff({ path: 'src/lib.rs' }),
      makeFileDiff({ path: 'tests/test.rs' }),
    ];
    const tree = buildFileTree(files, [], new Set(), 'alphabetical', '');
    expect(tree).toHaveLength(2); // src/ and tests/
    const srcDir = tree.find((n) => n.name === 'src');
    expect(srcDir).toBeTruthy();
    expect(srcDir!.kind).toBe('directory');
    expect(srcDir!.children).toHaveLength(2);
  });

  it('compacts single-child directories', () => {
    const files = [makeFileDiff({ path: 'src/middleware/jwt.rs' })];
    const tree = buildFileTree(files, [], new Set(), 'alphabetical', '');
    // src/middleware should be compacted into one node
    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe('src/middleware');
    expect(tree[0].children).toHaveLength(1);
    expect(tree[0].children[0].name).toBe('jwt.rs');
  });

  it('aggregates feedback counts to directories', () => {
    const files = [makeFileDiff({ path: 'src/auth.rs' }), makeFileDiff({ path: 'src/main.rs' })];
    const feedbacks = [
      makeFeedback({
        id: 'F-1',
        severity: 'block',
        status: 'open',
        target: { path: 'src/auth.rs', span: null, commit: null },
      }),
      makeFeedback({
        id: 'F-2',
        severity: 'warn',
        status: 'open',
        target: { path: 'src/main.rs', span: null, commit: null },
      }),
    ];
    const tree = buildFileTree(files, feedbacks, new Set(), 'feedbacks', '');
    const srcDir = tree[0]; // should be the src directory
    expect(srcDir.feedbacks.block).toBe(1);
    expect(srcDir.feedbacks.warn).toBe(1);
  });

  it('sorts by feedbacks priority', () => {
    const files = [
      makeFileDiff({ path: 'no-feedbacks.rs' }),
      makeFileDiff({ path: 'has-block.rs' }),
    ];
    const feedbacks = [
      makeFeedback({
        id: 'F-1',
        severity: 'block',
        status: 'open',
        target: { path: 'has-block.rs', span: null, commit: null },
      }),
    ];
    const tree = buildFileTree(files, feedbacks, new Set(), 'feedbacks', '');
    expect(tree[0].name).toBe('has-block.rs');
    expect(tree[1].name).toBe('no-feedbacks.rs');
  });

  it('filters by text', () => {
    const files = [
      makeFileDiff({ path: 'src/auth.rs' }),
      makeFileDiff({ path: 'src/main.rs' }),
      makeFileDiff({ path: 'tests/test.rs' }),
    ];
    const tree = buildFileTree(files, [], new Set(), 'alphabetical', 'auth');
    // Only auth.rs should match
    const allFiles = flattenFiles(tree);
    expect(allFiles).toHaveLength(1);
    expect(allFiles[0].path).toBe('src/auth.rs');
  });

  it('marks viewed files', () => {
    const files = [makeFileDiff({ path: 'src/main.rs' })];
    const viewed = new Set(['src/main.rs']);
    const tree = buildFileTree(files, [], viewed, 'alphabetical', '');
    const file = flattenFiles(tree)[0];
    expect(file.viewed).toBe(true);
  });
});

// Helper to flatten tree into file nodes only
function flattenFiles(nodes: ReturnType<typeof buildFileTree>): ReturnType<typeof buildFileTree> {
  const result: ReturnType<typeof buildFileTree> = [];
  for (const node of nodes) {
    if (node.kind === 'file') result.push(node);
    else result.push(...flattenFiles(node.children));
  }
  return result;
}
