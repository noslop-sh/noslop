import type { FileDiff, Finding, SortMode } from '$lib/types';

export function createReviewNavigation() {
  let currentFilePath = $state<string | null>(null);
  let currentFindingId = $state<string | null>(null);
  let viewedFiles = $state<Set<string>>(new Set());
  let sortMode = $state<SortMode>('findings');
  let filterText = $state('');
  let showWhitespace = $state(true);

  function nextFile(files: FileDiff[]): void {
    if (files.length === 0) return;
    if (!currentFilePath) {
      currentFilePath = files[0].path;
      return;
    }
    const idx = files.findIndex((f) => f.path === currentFilePath);
    if (idx < files.length - 1) {
      currentFilePath = files[idx + 1].path;
    }
  }

  function prevFile(files: FileDiff[]): void {
    if (files.length === 0) return;
    if (!currentFilePath) {
      currentFilePath = files[files.length - 1].path;
      return;
    }
    const idx = files.findIndex((f) => f.path === currentFilePath);
    if (idx > 0) {
      currentFilePath = files[idx - 1].path;
    }
  }

  function nextFinding(findings: Finding[]): void {
    if (findings.length === 0) return;
    if (!currentFindingId) {
      currentFindingId = findings[0].id;
      return;
    }
    const idx = findings.findIndex((f) => f.id === currentFindingId);
    if (idx < findings.length - 1) {
      currentFindingId = findings[idx + 1].id;
    }
  }

  function prevFinding(findings: Finding[]): void {
    if (findings.length === 0) return;
    if (!currentFindingId) {
      currentFindingId = findings[findings.length - 1].id;
      return;
    }
    const idx = findings.findIndex((f) => f.id === currentFindingId);
    if (idx > 0) {
      currentFindingId = findings[idx - 1].id;
    }
  }

  function nextUnresolved(findings: Finding[]): void {
    const open = findings.filter((f) => f.status === 'open');
    if (open.length === 0) return;
    if (!currentFindingId) {
      currentFindingId = open[0].id;
      return;
    }
    const idx = open.findIndex((f) => f.id === currentFindingId);
    const next = idx < open.length - 1 ? open[idx + 1] : open[0];
    currentFindingId = next.id;
  }

  function prevUnresolved(findings: Finding[]): void {
    const open = findings.filter((f) => f.status === 'open');
    if (open.length === 0) return;
    if (!currentFindingId) {
      currentFindingId = open[open.length - 1].id;
      return;
    }
    const idx = open.findIndex((f) => f.id === currentFindingId);
    const prev = idx > 0 ? open[idx - 1] : open[open.length - 1];
    currentFindingId = prev.id;
  }

  function selectFile(path: string): void {
    currentFilePath = path;
  }

  function selectFinding(id: string): void {
    currentFindingId = id;
  }

  function toggleViewed(path: string): void {
    const next = new Set(viewedFiles);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    viewedFiles = next;
  }

  function setSortMode(mode: SortMode): void {
    sortMode = mode;
  }

  function setFilterText(text: string): void {
    filterText = text;
  }

  function toggleWhitespaceVisibility(): void {
    showWhitespace = !showWhitespace;
  }

  return {
    get currentFilePath() {
      return currentFilePath;
    },
    get currentFindingId() {
      return currentFindingId;
    },
    get viewedFiles() {
      return viewedFiles;
    },
    get sortMode() {
      return sortMode;
    },
    get filterText() {
      return filterText;
    },
    get showWhitespace() {
      return showWhitespace;
    },
    nextFile,
    prevFile,
    nextFinding,
    prevFinding,
    nextUnresolved,
    prevUnresolved,
    selectFile,
    selectFinding,
    toggleViewed,
    setSortMode,
    setFilterText,
    toggleWhitespaceVisibility,
  };
}
