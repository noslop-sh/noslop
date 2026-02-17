export interface Review {
  id: string;
  base: string;
  head: string;
  status: 'open' | 'closed';
  findings: Finding[];
  created_at: string;
}

export interface Finding {
  id: string;
  target: string;
  severity: string;
  message: string;
  source: string;
  status: 'open' | 'resolved' | 'dismissed';
  suggestion: string | null;
}

export interface DiffFile {
  path: string;
  status: 'added' | 'modified' | 'deleted' | 'renamed';
  additions: number;
  deletions: number;
}
