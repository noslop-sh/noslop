import type {
  ActiveFilters,
  DiffLine,
  DismissReason,
  FileChangeType,
  FileDiff,
  FileTreeEntry,
  Feedback,
  Severity,
  SortMode,
  Span,
  StructuredDiff,
} from './types';

// --- Feedback helpers ---

export function openFeedbackCount(feedbacks: Feedback[]): number {
  return feedbacks.filter((f) => f.status === 'open').length;
}

export function blockingFeedbacks(feedbacks: Feedback[]): Feedback[] {
  return feedbacks.filter((f) => f.severity === 'block' && f.status === 'open');
}

export function feedbackCountsByFile(
  feedbacks: Feedback[],
  path: string
): { block: number; warn: number; info: number } {
  const fileFn = feedbacks.filter((f) => f.target.path === path && f.status === 'open');
  return {
    block: fileFn.filter((f) => f.severity === 'block').length,
    warn: fileFn.filter((f) => f.severity === 'warn').length,
    info: fileFn.filter((f) => f.severity === 'info').length,
  };
}

export function applyFeedbackFilters(feedbacks: Feedback[], filters: ActiveFilters): Feedback[] {
  return feedbacks.filter((f) => {
    if (filters.status !== 'all' && f.status !== filters.status) return false;
    if (filters.severity !== 'all' && f.severity !== filters.severity) return false;
    if (filters.source !== 'all' && f.source.kind !== filters.source) return false;
    return true;
  });
}

export function sortFeedbacksBySeverity(feedbacks: Feedback[]): Feedback[] {
  const order: Record<Severity, number> = { block: 0, warn: 1, info: 2 };
  return [...feedbacks].sort((a, b) => order[a.severity] - order[b.severity]);
}

// --- Code snippet extraction ---

export function getCodeSnippet(
  diff: StructuredDiff,
  path: string,
  span: Span,
  contextLines: number = 3
): DiffLine[] {
  const file = diff.files.find((f) => f.path === path);
  if (!file) return [];

  const startLine = Math.max(1, span.start - contextLines);
  const endLine = span.end + contextLines;

  const lines: DiffLine[] = [];
  for (const hunk of file.hunks) {
    for (const line of hunk.lines) {
      const lineNo = line.new_line_no ?? line.old_line_no;
      if (lineNo !== null && lineNo >= startLine && lineNo <= endLine) {
        lines.push(line);
      }
    }
  }

  return lines;
}

// --- Severity display helpers ---

export function severityIcon(severity: Severity, sourceKind: string): string {
  if (sourceKind === 'human') return '\u25C6'; // diamond
  switch (severity) {
    case 'block':
      return '\u25CF'; // filled circle
    case 'warn':
      return '\u25B2'; // triangle
    case 'info':
      return '\u25CB'; // circle outline
  }
}

export function severityColor(severity: Severity, sourceKind: string): string {
  if (sourceKind === 'human') return 'text-[var(--feedback-human)]';
  switch (severity) {
    case 'block':
      return 'text-[var(--feedback-block)]';
    case 'warn':
      return 'text-[var(--feedback-warn)]';
    case 'info':
      return 'text-[var(--feedback-info)]';
  }
}

export function severityBorderColor(severity: Severity, sourceKind: string): string {
  if (sourceKind === 'human') return 'border-l-[var(--feedback-human)]';
  switch (severity) {
    case 'block':
      return 'border-l-[var(--feedback-block)]';
    case 'warn':
      return 'border-l-[var(--feedback-warn)]';
    case 'info':
      return 'border-l-[var(--feedback-info)]';
  }
}

export function severityVar(severity: Severity, sourceKind: string): string {
  if (sourceKind === 'human') return 'var(--feedback-human)';
  switch (severity) {
    case 'block':
      return 'var(--feedback-block)';
    case 'warn':
      return 'var(--feedback-warn)';
    case 'info':
      return 'var(--feedback-info)';
  }
}

// --- Shared constants ---

export const DISMISS_OPTIONS: { label: string; reason: DismissReason }[] = [
  { label: 'False positive', reason: 'false_positive' },
  { label: "Won't fix", reason: 'wont_fix' },
  { label: 'Not applicable', reason: 'not_applicable' },
  { label: 'Investigate later', reason: 'investigate_later' },
];

export const SEVERITY_OPTIONS: { label: string; value: Severity }[] = [
  { label: 'Block', value: 'block' },
  { label: 'Warn', value: 'warn' },
  { label: 'Info', value: 'info' },
];

export function formatReason(reason: string): string {
  return reason.replace(/_/g, ' ');
}

// --- Source display ---

export function formatSource(source: { kind: string; name: string | null }): string {
  if (source.kind === 'human') return 'human';
  return `${source.kind}:${source.name ?? 'unknown'}`;
}

// --- File change type helpers ---

export function changeTypeLabel(ct: FileChangeType): string {
  if (typeof ct === 'string') return ct.charAt(0).toUpperCase();
  return 'R';
}

export function changeTypeColor(ct: FileChangeType): string {
  if (ct === 'added') return 'text-green-600 dark:text-green-400';
  if (ct === 'modified') return 'text-yellow-600 dark:text-yellow-400';
  if (ct === 'deleted') return 'text-red-600 dark:text-red-400';
  return 'text-blue-600 dark:text-blue-400'; // renamed
}

// --- Date formatting ---

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
}

// --- File tree construction ---

export function buildFileTree(
  files: FileDiff[],
  feedbacks: Feedback[],
  viewedFiles: Set<string>,
  sortMode: SortMode,
  filterText: string
): FileTreeEntry[] {
  // Build flat entries for each file
  const entries: FileTreeEntry[] = files
    .filter((f) => !filterText || f.path.toLowerCase().includes(filterText.toLowerCase()))
    .map((f) => ({
      path: f.path,
      name: f.path.split('/').pop() ?? f.path,
      kind: 'file' as const,
      change_type: f.change_type,
      additions: f.additions,
      deletions: f.deletions,
      feedbacks: feedbackCountsByFile(feedbacks, f.path),
      viewed: viewedFiles.has(f.path),
      children: [],
      collapsed_prefix: null,
    }));

  // Build directory tree
  const root: FileTreeEntry[] = [];
  const dirMap = new Map<string, FileTreeEntry>();

  for (const entry of entries) {
    const parts = entry.path.split('/');
    if (parts.length === 1) {
      root.push(entry);
      continue;
    }

    let currentPath = '';
    let parent: FileTreeEntry[] = root;

    for (let i = 0; i < parts.length - 1; i++) {
      currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];

      let dir = dirMap.get(currentPath);
      if (!dir) {
        dir = {
          path: currentPath,
          name: parts[i],
          kind: 'directory',
          change_type: null,
          additions: 0,
          deletions: 0,
          feedbacks: { block: 0, warn: 0, info: 0 },
          viewed: false,
          children: [],
          collapsed_prefix: null,
        };
        dirMap.set(currentPath, dir);
        parent.push(dir);
      }
      parent = dir.children;
    }

    parent.push(entry);
  }

  // Aggregate feedback counts up to directories
  function aggregateDir(node: FileTreeEntry): void {
    if (node.kind === 'directory') {
      node.feedbacks = { block: 0, warn: 0, info: 0 };
      node.additions = 0;
      node.deletions = 0;
      let allViewed = true;

      for (const child of node.children) {
        aggregateDir(child);
        node.feedbacks.block += child.feedbacks.block;
        node.feedbacks.warn += child.feedbacks.warn;
        node.feedbacks.info += child.feedbacks.info;
        node.additions += child.additions;
        node.deletions += child.deletions;
        if (!child.viewed) allViewed = false;
      }

      node.viewed = node.children.length > 0 && allViewed;
    }
  }

  // Compact single-child directories
  function compact(nodes: FileTreeEntry[]): FileTreeEntry[] {
    return nodes.map((node) => {
      if (node.kind === 'directory') {
        node.children = compact(node.children);

        if (node.children.length === 1 && node.children[0].kind === 'directory') {
          const child = node.children[0];
          child.collapsed_prefix = node.collapsed_prefix
            ? `${node.collapsed_prefix}/${node.name}`
            : node.name;
          child.name = `${node.name}/${child.name}`;
          return child;
        }
      }
      return node;
    });
  }

  for (const node of root) aggregateDir(node);
  const compacted = compact(root);

  // Sort
  if (sortMode === 'feedbacks') {
    return sortByFeedbacks(compacted);
  }
  return sortAlphabetically(compacted);
}

function sortByFeedbacks(nodes: FileTreeEntry[]): FileTreeEntry[] {
  return [...nodes]
    .sort((a, b) => {
      // Directories first
      if (a.kind !== b.kind) return a.kind === 'directory' ? -1 : 1;
      // Block feedbacks first
      if (a.feedbacks.block !== b.feedbacks.block) return b.feedbacks.block - a.feedbacks.block;
      // Then warn
      if (a.feedbacks.warn !== b.feedbacks.warn) return b.feedbacks.warn - a.feedbacks.warn;
      // Then info
      if (a.feedbacks.info !== b.feedbacks.info) return b.feedbacks.info - a.feedbacks.info;
      // Unviewed first
      if (a.viewed !== b.viewed) return a.viewed ? 1 : -1;
      // Alphabetical tiebreak
      return a.name.localeCompare(b.name);
    })
    .map((n) => ({
      ...n,
      children: n.children.length > 0 ? sortByFeedbacks(n.children) : n.children,
    }));
}

function sortAlphabetically(nodes: FileTreeEntry[]): FileTreeEntry[] {
  return [...nodes]
    .sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === 'directory' ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((n) => ({
      ...n,
      children: n.children.length > 0 ? sortAlphabetically(n.children) : n.children,
    }));
}
