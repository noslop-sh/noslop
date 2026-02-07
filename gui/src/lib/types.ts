export interface Review {
  id: string;
  base_sha: string;
  head_sha: string;
  status: 'open' | 'closed';
  comments: Comment[];
  created_at: string;
}

export interface Comment {
  id: string;
  target: string;
  message: string;
  line: number | null;
  status: 'open' | 'resolved';
  resolution_message: string | null;
}

export interface DiffFile {
  path: string;
  status: 'added' | 'modified' | 'deleted' | 'renamed';
  additions: number;
  deletions: number;
}
