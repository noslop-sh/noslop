import type { Finding, DismissReason, Severity } from '$lib/types';
import { formatSource } from '$lib/helpers';

/**
 * Mutable ref object for callbacks. Captured by closure, properties
 * updated by $effect in FileDiffRenderer so we never read stale values.
 */
export interface AnnotationCallbacksRef {
  onResolve: (findingId: string) => void;
  onDismiss: (findingId: string, reason: DismissReason) => void;
  onFindingClick: (findingId: string) => void;
}

const DISMISS_OPTIONS: { label: string; reason: DismissReason }[] = [
  { label: 'False positive', reason: 'false_positive' },
  { label: "Won't fix", reason: 'wont_fix' },
  { label: 'Not applicable', reason: 'not_applicable' },
];

function severityVar(severity: Severity, sourceKind: string): string {
  if (sourceKind === 'human') return 'var(--finding-human)';
  switch (severity) {
    case 'block':
      return 'var(--finding-block)';
    case 'warn':
      return 'var(--finding-warn)';
    case 'info':
      return 'var(--finding-info)';
  }
}

/**
 * Renders a finding annotation as plain DOM with inline styles.
 * CSS custom properties (var()) inherit through Shadow DOM boundaries,
 * unlike Tailwind utility classes which do not cascade into Shadow DOM.
 */
export function renderFindingAnnotation(
  wrapper: HTMLElement,
  finding: Finding,
  callbacksRef: AnnotationCallbacksRef
): void {
  const isOpen = finding.status === 'open';
  const isResolved = finding.status === 'resolved';
  const isDismissed = finding.status === 'dismissed';
  const color = severityVar(finding.severity, finding.source.kind);

  // Outer container
  const container = document.createElement('div');
  Object.assign(container.style, {
    borderLeft: `3px solid ${color}`,
    borderRadius: '2px',
    background: 'var(--card)',
    color: 'var(--card-foreground)',
    padding: '6px 12px',
    fontSize: '13px',
    lineHeight: '1.4',
    opacity: isResolved || isDismissed ? '0.5' : '1',
    cursor: 'pointer',
  });
  container.addEventListener('click', () => callbacksRef.onFindingClick(finding.id));

  // Row: severity badge + message + source + actions
  const row = document.createElement('div');
  Object.assign(row.style, {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  });

  // Severity badge
  const badge = document.createElement('span');
  badge.textContent = finding.severity.toUpperCase();
  Object.assign(badge.style, {
    fontSize: '11px',
    fontWeight: '700',
    textTransform: 'uppercase',
    color,
    flexShrink: '0',
  });
  row.appendChild(badge);

  // Message
  const msg = document.createElement('span');
  msg.textContent = finding.message;
  Object.assign(msg.style, {
    flex: '1',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    textDecoration: isDismissed ? 'line-through' : 'none',
  });
  row.appendChild(msg);

  // Source label
  const src = document.createElement('span');
  src.textContent = formatSource(finding.source);
  Object.assign(src.style, {
    fontSize: '11px',
    color: 'var(--muted-foreground)',
    flexShrink: '0',
  });
  row.appendChild(src);

  // Resolved check
  if (isResolved) {
    const check = document.createElement('span');
    check.textContent = '\u2713';
    Object.assign(check.style, {
      color: 'var(--success)',
      fontSize: '14px',
      flexShrink: '0',
    });
    row.appendChild(check);
  }

  // Action buttons (only for open findings)
  if (isOpen) {
    const resolveBtn = createActionButton('Resolve', (e) => {
      e.stopPropagation();
      callbacksRef.onResolve(finding.id);
    });
    row.appendChild(resolveBtn);

    const dismissContainer = createDismissDropdown(finding.id, callbacksRef);
    row.appendChild(dismissContainer);
  }

  container.appendChild(row);
  wrapper.appendChild(container);
}

function createActionButton(label: string, onClick: (e: MouseEvent) => void): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.textContent = label;
  Object.assign(btn.style, {
    background: 'transparent',
    border: 'none',
    color: 'var(--muted-foreground)',
    fontSize: '12px',
    padding: '1px 6px',
    borderRadius: '2px',
    cursor: 'pointer',
    flexShrink: '0',
  });
  btn.addEventListener('mouseenter', () => {
    btn.style.color = 'var(--foreground)';
    btn.style.background = 'var(--accent)';
  });
  btn.addEventListener('mouseleave', () => {
    btn.style.color = 'var(--muted-foreground)';
    btn.style.background = 'transparent';
  });
  btn.addEventListener('click', onClick);
  return btn;
}

function createDismissDropdown(
  findingId: string,
  callbacksRef: AnnotationCallbacksRef
): HTMLElement {
  const container = document.createElement('div');
  container.style.position = 'relative';
  container.style.flexShrink = '0';
  container.addEventListener('click', (e) => e.stopPropagation());

  const trigger = createActionButton('Dismiss \u25BE', (e) => {
    e.stopPropagation();
    const menu = container.querySelector('[data-dismiss-menu]') as HTMLElement | null;
    if (menu) {
      menu.style.display = menu.style.display === 'none' ? 'block' : 'none';
    }
  });
  container.appendChild(trigger);

  const menu = document.createElement('div');
  menu.setAttribute('data-dismiss-menu', '');
  Object.assign(menu.style, {
    display: 'none',
    position: 'absolute',
    right: '0',
    top: '100%',
    zIndex: '50',
    background: 'var(--popover)',
    color: 'var(--popover-foreground)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
    padding: '4px 0',
    minWidth: '140px',
    boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
  });

  for (const opt of DISMISS_OPTIONS) {
    const item = document.createElement('div');
    item.textContent = opt.label;
    Object.assign(item.style, {
      padding: '4px 12px',
      fontSize: '12px',
      cursor: 'pointer',
    });
    item.addEventListener('mouseenter', () => {
      item.style.background = 'var(--accent)';
    });
    item.addEventListener('mouseleave', () => {
      item.style.background = 'transparent';
    });
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      callbacksRef.onDismiss(findingId, opt.reason);
      menu.style.display = 'none';
    });
    menu.appendChild(item);
  }

  container.appendChild(menu);

  // Close on outside click
  const closeHandler = (e: MouseEvent) => {
    if (!container.contains(e.target as Node)) {
      menu.style.display = 'none';
    }
  };
  document.addEventListener('click', closeHandler);
  // Cleanup: use a MutationObserver to detect removal
  const observer = new MutationObserver(() => {
    if (!container.isConnected) {
      document.removeEventListener('click', closeHandler);
      observer.disconnect();
    }
  });
  observer.observe(container.ownerDocument.body, { childList: true, subtree: true });

  return container;
}
