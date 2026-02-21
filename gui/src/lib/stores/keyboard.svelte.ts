import type { FocusZone } from '$lib/types';

export interface KeyboardActions {
  nextFile: () => void;
  prevFile: () => void;
  nextFinding: () => void;
  prevFinding: () => void;
  nextUnresolved: () => void;
  prevUnresolved: () => void;
  resolveFocused: () => void;
  dismissFocused: () => void;
  addFindingOnLine: () => void;
  toggleViewed: () => void;
  cycleSidebar: () => void;
  toggleDiffMode: () => void;
  toggleWhitespace: () => void;
  expandFocused: () => void;
  collapseFocused: () => void;
  openCommandPalette: () => void;
  openFileJump: () => void;
  showShortcuts: () => void;
  switchToSummary: () => void;
  switchToFiles: () => void;
}

function isTextInput(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || (el as HTMLElement).isContentEditable;
}

export function createKeyboardManager(actions: KeyboardActions) {
  let focusZone = $state<FocusZone>('tree');

  function setFocusZone(zone: FocusZone): void {
    focusZone = zone;
  }

  function handleKeydown(e: KeyboardEvent): void {
    // Modifier shortcuts always work
    if (e.metaKey || e.ctrlKey) {
      if (e.key === 'k') {
        actions.openCommandPalette();
        e.preventDefault();
        return;
      }
      if (e.key === 'p') {
        actions.openFileJump();
        e.preventDefault();
        return;
      }
      return;
    }

    // Single-key shortcuts disabled in text inputs
    if (isTextInput()) return;

    const handlers: Record<string, () => void> = {
      ']': actions.nextFile,
      '[': actions.prevFile,
      j: actions.nextFinding,
      k: actions.prevFinding,
      n: actions.nextUnresolved,
      p: actions.prevUnresolved,
      r: actions.resolveFocused,
      d: actions.dismissFocused,
      c: actions.addFindingOnLine,
      v: actions.toggleViewed,
      f: actions.cycleSidebar,
      s: actions.toggleDiffMode,
      w: actions.toggleWhitespace,
      Enter: actions.expandFocused,
      Escape: actions.collapseFocused,
      '?': actions.showShortcuts,
      '1': actions.switchToSummary,
      '2': actions.switchToFiles,
    };

    const handler = handlers[e.key];
    if (handler) {
      handler();
      e.preventDefault();
    }
  }

  return {
    get focusZone() {
      return focusZone;
    },
    setFocusZone,
    handleKeydown,
  };
}
