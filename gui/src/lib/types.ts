// Domain types matching Rust backend DTOs

export type ReviewStatus = 'open' | 'closed';
export type Severity = 'block' | 'warn' | 'info';
export type FeedbackStatus = 'open' | 'resolved' | 'dismissed';
export type FeedbackSourceKind = 'check' | 'script' | 'agent' | 'human';

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

export interface FeedbackSource {
  kind: FeedbackSourceKind;
  name: string | null; // null for human
}

export interface Suggestion {
  replacement: string;
}

export interface FeedbackNote {
  id: string;
  content: string;
  created_at: string;
}

export interface Feedback {
  id: string;
  target: Target;
  severity: Severity;
  message: string;
  source: FeedbackSource;
  status: FeedbackStatus;
  suggestion: Suggestion | null;
  dismiss_reason: DismissReason | null;
  resolution_reason: ResolutionReason | null;
  confidence: number | null;
  notes: FeedbackNote[];
  created_at: string;
}

export interface Review {
  id: string;
  base: string;
  head: string;
  branch: string | null;
  status: ReviewStatus;
  feedbacks: Feedback[];
  viewed_files: string[];
  summary: string | null;
  created_at: string;
  closed_at: string | null;
}

// Agent review result
export interface AgentReviewResult {
  feedback_count: number;
  exit_code: number;
  duration_secs: number;
  errors: string[];
  agent_output: string;
}

// Diff types from structured diff computation

export type FileChangeType = 'added' | 'modified' | 'deleted' | { renamed: { similarity: number } };

export interface DiffStats {
  files_changed: number;
  additions: number;
  deletions: number;
}

export interface DiffLine {
  kind: 'add' | 'delete' | 'context';
  old_line_no: number | null;
  new_line_no: number | null;
  content: string;
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
  feedbacks: { block: number; warn: number; info: number };
  viewed: boolean;
  children: FileTreeEntry[];
  collapsed_prefix: string | null;
}

// UI state types

export type SidebarCollapseState = 'full' | 'mini' | 'hidden';
export type DiffViewMode = 'split' | 'unified';
export type ThemeMode = 'light' | 'dark' | 'system';
export type ReviewView = 'summary' | 'files';
export type SortMode = 'feedbacks' | 'alphabetical';

export interface ActiveFilters {
  status: FeedbackStatus | 'all';
  severity: Severity | 'all';
  source: FeedbackSourceKind | 'all';
}

// Command palette types

export interface PaletteCommand {
  id: string;
  label: string;
  group: 'actions' | 'files' | 'feedbacks' | 'navigation';
  shortcut?: string;
  action: () => void;
  available: () => boolean;
}
