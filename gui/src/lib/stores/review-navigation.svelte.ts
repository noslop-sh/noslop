import type { FileDiff, Feedback, SortMode } from '$lib/types';

export function createReviewNavigation() {
  let currentFilePath = $state<string | null>(null);
  let currentFeedbackId = $state<string | null>(null);
  let viewedFiles = $state<Set<string>>(new Set());
  let sortMode = $state<SortMode>('feedbacks');
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

  function nextFeedback(feedbacks: Feedback[]): void {
    if (feedbacks.length === 0) return;
    if (!currentFeedbackId) {
      currentFeedbackId = feedbacks[0].id;
      return;
    }
    const idx = feedbacks.findIndex((f) => f.id === currentFeedbackId);
    if (idx < feedbacks.length - 1) {
      currentFeedbackId = feedbacks[idx + 1].id;
    }
  }

  function prevFeedback(feedbacks: Feedback[]): void {
    if (feedbacks.length === 0) return;
    if (!currentFeedbackId) {
      currentFeedbackId = feedbacks[feedbacks.length - 1].id;
      return;
    }
    const idx = feedbacks.findIndex((f) => f.id === currentFeedbackId);
    if (idx > 0) {
      currentFeedbackId = feedbacks[idx - 1].id;
    }
  }

  function nextUnresolved(feedbacks: Feedback[]): void {
    const open = feedbacks.filter((f) => f.status === 'open');
    if (open.length === 0) return;
    if (!currentFeedbackId) {
      currentFeedbackId = open[0].id;
      return;
    }
    const idx = open.findIndex((f) => f.id === currentFeedbackId);
    const next = idx < open.length - 1 ? open[idx + 1] : open[0];
    currentFeedbackId = next.id;
  }

  function prevUnresolved(feedbacks: Feedback[]): void {
    const open = feedbacks.filter((f) => f.status === 'open');
    if (open.length === 0) return;
    if (!currentFeedbackId) {
      currentFeedbackId = open[open.length - 1].id;
      return;
    }
    const idx = open.findIndex((f) => f.id === currentFeedbackId);
    const prev = idx > 0 ? open[idx - 1] : open[open.length - 1];
    currentFeedbackId = prev.id;
  }

  function selectFile(path: string): void {
    currentFilePath = path;
  }

  function selectFeedback(id: string): void {
    currentFeedbackId = id;
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
    get currentFeedbackId() {
      return currentFeedbackId;
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
    nextFeedback,
    prevFeedback,
    nextUnresolved,
    prevUnresolved,
    selectFile,
    selectFeedback,
    toggleViewed,
    setSortMode,
    setFilterText,
    toggleWhitespaceVisibility,
  };
}
