import { browser } from '$app/environment';
import type {
  ActiveFilters,
  DiffViewMode,
  FindingStatus,
  ReviewView,
  Severity,
  SidebarCollapseState,
  ThemeMode,
} from './types';

export interface SessionState {
  selected_review_id: string | null;
  base_branch: string | null;
  compare_branch: string | null;
  scroll_positions: Record<string, number>;
  expanded_finding_ids: string[];
  sidebar_width: number;
  sidebar_collapse_state: SidebarCollapseState;
  file_tree_collapsed_dirs: string[];
  active_filters: ActiveFilters;
  diff_view_mode: DiffViewMode;
  theme: ThemeMode;
  findings_panel_collapsed: boolean;
  active_view: ReviewView;
}

const STORAGE_KEY = 'noslop-session';

function defaultSession(): SessionState {
  return {
    selected_review_id: null,
    base_branch: null,
    compare_branch: null,
    scroll_positions: {},
    expanded_finding_ids: [],
    sidebar_width: 280,
    sidebar_collapse_state: 'full',
    file_tree_collapsed_dirs: [],
    active_filters: {
      status: 'all' as FindingStatus | 'all',
      severity: 'all' as Severity | 'all',
      source: 'all',
    },
    diff_view_mode: 'split',
    theme: 'dark',
    findings_panel_collapsed: false,
    active_view: 'summary',
  };
}

function loadSession(): SessionState {
  if (!browser) return defaultSession();
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return defaultSession();
  try {
    return { ...defaultSession(), ...JSON.parse(raw) };
  } catch {
    return defaultSession();
  }
}

function saveSession(state: SessionState): void {
  if (!browser) return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function debouncedSave(state: SessionState): void {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => saveSession(state), 500);
}

// Singleton session store
let _session: SessionState = loadSession();

export function getSession(): SessionState {
  return _session;
}

export function updateSession(patch: Partial<SessionState>): void {
  _session = { ..._session, ...patch };
  debouncedSave(_session);
}
