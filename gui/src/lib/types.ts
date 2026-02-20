// Domain types matching Rust backend DTOs

export type ReviewStatus = 'open' | 'closed';
export type Severity = 'block' | 'warn' | 'info';
export type FindingStatus = 'open' | 'resolved' | 'dismissed';
export type FindingSourceKind = 'check' | 'script' | 'agent' | 'human';

export type DismissReason = 'false_positive' | 'wont_fix' | 'not_applicable' | 'investigate_later';

export type ResolutionReason = 'manual' | 'suggestion_applied' | 'code_changed' | 'file_removed';

export interface Span {
  start: number; // 1-indexed, inclusive
  end: number; // 1-indexed, inclusive
}

export interface Target {
  path: string;
  span: Span | null;
  commit: string | null;
}

export interface FindingSource {
  kind: FindingSourceKind;
  name: string | null; // null for human
}

export interface Suggestion {
  original: string | null;
  replacement: string;
  edited: boolean;
}

export interface FindingNote {
  id: string;
  content: string;
  created_at: string;
}

export interface Finding {
  id: string;
  target: Target;
  severity: Severity;
  message: string;
  source: FindingSource;
  status: FindingStatus;
  suggestion: Suggestion | null;
  dismiss_reason: DismissReason | null;
  resolution_reason: ResolutionReason | null;
  confidence: number | null;
  notes: FindingNote[];
  created_at: string;
}

export interface Review {
  id: string;
  base: string;
  head: string;
  branch: string | null;
  status: ReviewStatus;
  findings: Finding[];
  viewed_files: string[];
  created_at: string;
  closed_at: string | null;
}

// Diff types from structured diff computation

export type FileChangeType = 'added' | 'modified' | 'deleted' | { renamed: { similarity: number } };

export interface DiffStats {
  files_changed: number;
  additions: number;
  deletions: number;
}

export interface CharChange {
  start: number; // byte offset
  end: number; // byte offset
  kind: 'insert' | 'delete' | 'equal';
}

export interface DiffLine {
  kind: 'add' | 'delete' | 'context';
  old_line_no: number | null;
  new_line_no: number | null;
  content: string;
  char_changes: CharChange[] | null;
}

export interface Hunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  header: string;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  old_path: string | null;
  change_type: FileChangeType;
  hunks: Hunk[];
  additions: number;
  deletions: number;
  is_binary: boolean;
  language: string | null;
}

export interface StructuredDiff {
  files: FileDiff[];
  stats: DiffStats;
}

// File tree types

export interface FileTreeEntry {
  path: string;
  name: string;
  kind: 'file' | 'directory';
  change_type: FileChangeType | null;
  additions: number;
  deletions: number;
  findings: { block: number; warn: number; info: number };
  viewed: boolean;
  children: FileTreeEntry[];
  collapsed_prefix: string | null;
}

// UI state types

export type SidebarCollapseState = 'full' | 'mini' | 'hidden';
export type DiffViewMode = 'split' | 'unified';
export type ThemeMode = 'light' | 'dark' | 'system';
export type FocusZone = 'tree' | 'diff' | 'finding' | 'dialog';
export type SortMode = 'findings' | 'alphabetical';

export interface ActiveFilters {
  status: FindingStatus | 'all';
  severity: Severity | 'all';
  source: FindingSourceKind | 'all';
}

// Command palette types

export interface PaletteCommand {
  id: string;
  label: string;
  group: 'actions' | 'files' | 'findings' | 'navigation';
  shortcut?: string;
  action: () => void;
  available: () => boolean;
}
