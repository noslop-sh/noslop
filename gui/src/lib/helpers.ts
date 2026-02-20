import type {
  ActiveFilters,
  FileChangeType,
  FileDiff,
  FileTreeEntry,
  Finding,
  Severity,
  SortMode,
} from './types';

// --- Finding helpers ---

export function openFindingCount(findings: Finding[]): number {
  return findings.filter((f) => f.status === 'open').length;
}

export function blockingFindings(findings: Finding[]): Finding[] {
  return findings.filter((f) => f.severity === 'block' && f.status === 'open');
}

export function findingsForFile(findings: Finding[], path: string): Finding[] {
  return findings.filter((f) => f.target.path === path);
}

export function findingCountsByFile(
  findings: Finding[],
  path: string
): { block: number; warn: number; info: number } {
  const fileFn = findings.filter((f) => f.target.path === path && f.status === 'open');
  return {
    block: fileFn.filter((f) => f.severity === 'block').length,
    warn: fileFn.filter((f) => f.severity === 'warn').length,
    info: fileFn.filter((f) => f.severity === 'info').length,
  };
}

export function applyFindingFilters(findings: Finding[], filters: ActiveFilters): Finding[] {
  return findings.filter((f) => {
    if (filters.status !== 'all' && f.status !== filters.status) return false;
    if (filters.severity !== 'all' && f.severity !== filters.severity) return false;
    if (filters.source !== 'all' && f.source.kind !== filters.source) return false;
    return true;
  });
}

export function sortFindingsBySeverity(findings: Finding[]): Finding[] {
  const order: Record<Severity, number> = { block: 0, warn: 1, info: 2 };
  return [...findings].sort((a, b) => order[a.severity] - order[b.severity]);
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

export function formatRelativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

// --- File tree construction ---

export function buildFileTree(
  files: FileDiff[],
  findings: Finding[],
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
      findings: findingCountsByFile(findings, f.path),
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
          findings: { block: 0, warn: 0, info: 0 },
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

  // Aggregate finding counts up to directories
  function aggregateDir(node: FileTreeEntry): void {
    if (node.kind === 'directory') {
      node.findings = { block: 0, warn: 0, info: 0 };
      node.additions = 0;
      node.deletions = 0;
      let allViewed = true;

      for (const child of node.children) {
        aggregateDir(child);
        node.findings.block += child.findings.block;
        node.findings.warn += child.findings.warn;
        node.findings.info += child.findings.info;
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
  if (sortMode === 'findings') {
    return sortByFindings(compacted);
  }
  return sortAlphabetically(compacted);
}

function sortByFindings(nodes: FileTreeEntry[]): FileTreeEntry[] {
  return [...nodes]
    .sort((a, b) => {
      // Directories first
      if (a.kind !== b.kind) return a.kind === 'directory' ? -1 : 1;
      // Block findings first
      if (a.findings.block !== b.findings.block) return b.findings.block - a.findings.block;
      // Then warn
      if (a.findings.warn !== b.findings.warn) return b.findings.warn - a.findings.warn;
      // Then info
      if (a.findings.info !== b.findings.info) return b.findings.info - a.findings.info;
      // Unviewed first
      if (a.viewed !== b.viewed) return a.viewed ? 1 : -1;
      // Alphabetical tiebreak
      return a.name.localeCompare(b.name);
    })
    .map((n) => ({
      ...n,
      children: n.children.length > 0 ? sortByFindings(n.children) : n.children,
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
