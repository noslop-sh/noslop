# noslop GUI UX Research Report

Synthesized from deep research into GitHub, Graphite, GitLab, desktop diff tools, IDE integrations, and AI code review platforms.

---

## Executive Summary

noslop's Tauri GUI occupies a unique position: a **local-first code review tool for human-AI collaboration**. No existing tool does exactly this. The closest comparisons are:

- **GitHub/GitLab/Graphite** — team review on remote PRs
- **CodeRabbit/Qodo/SonarQube** — AI findings delivery
- **VS Code/JetBrains diff editors** — desktop diff rendering
- **git-appraise** — local-first review storage in git

The GUI should steal the best patterns from each while designing for its unique workflow: a solo developer triaging findings from automated checks, AI agents, and their own observations on a local branch.

---

## 1. The Review Lifecycle (How a User Starts and Finishes)

### What the platforms do

| Platform | How review starts                                   | How it ends                                             |
| -------- | --------------------------------------------------- | ------------------------------------------------------- |
| GitHub   | Navigate to PR → Files Changed tab → leave comments | Submit review (Approve / Request Changes / Comment)     |
| GitLab   | Navigate to MR → Changes tab → "Start a review"     | Submit with summary + disposition                       |
| Graphite | PR Inbox → click PR → continuous diff scroll        | Sticky bottom bar: R+A (approve), R+N (request changes) |

### What noslop should do

The workflow is fundamentally different: there's no "other person" to approve. The human is reviewing their agent's work product.

**Proposed flow:**

```
1. START: User selects branch to review (or noslop auto-detects feature branch vs main)
   → Landing page shows: agent intent, test results, file tree with change counts,
     risk-flagged files, finding summary by severity

2. ANALYZE: Pipeline runs (convention checks, optional AI analysis)
   → Findings appear progressively as computed, not batched

3. TRIAGE: Human walks through diff file-by-file
   → Inline findings from all sources (checks, agent, human)
   → Each finding: resolve, dismiss (with reason), or leave open
   → Human can add their own findings/annotations

4. CLOSE: All blocking findings resolved or dismissed
   → Review marked complete → safe to push
```

**Key insight from Graphite**: The "PR Contract" adapted for AI agents should be the review landing page. Show:

- What task the agent was working on
- Test/build results (pass/fail)
- Which files touch risky areas (auth, security, external APIs)
- AI-generated summary of what changed and why

---

## 2. Diff Rendering

### Best practices across platforms

| Feature                | GitHub                          | GitLab                    | Graphite                   | Desktop tools  |
| ---------------------- | ------------------------------- | ------------------------- | -------------------------- | -------------- |
| Unified + split toggle | Yes                             | Yes                       | Yes                        | All            |
| Syntax highlighting    | Yes                             | Yes                       | VS Code-style              | Language-aware |
| File tree sidebar      | Resizable, hierarchical         | Tree + list modes         | Collapsible with mini-tree | N/A            |
| Whitespace toggle      | Yes (shortcut: `w`)             | Yes                       | Yes                        | Yes            |
| "Viewed" checkmarks    | Yes, per-file                   | Yes, per-file             | Via versions               | N/A            |
| Large diff handling    | Virtualization (3k files)       | Auto-collapse large files | Continuous scroll          | Native perf    |
| Context expansion      | Expand arrows above/below hunks | "Previous/Next 20 lines"  | Continuous                 | Configurable   |

### Recommendations for noslop

1. **Split + unified toggle** — table stakes. Auto-switch to unified when window is narrow (VS Code pattern).

2. **Two-tier highlighting** — line-level background color (green/red) PLUS character-level foreground color for changes within a line (Beyond Compare pattern). This is critical for AI-generated code where many lines may change but only a few characters differ.

3. **Continuous scroll with file headers** — follow Graphite's approach. One continuous diff document, not paginated. Each file has a sticky header with path, +/- counts, viewed checkbox, and finding count badge.

4. **File tree sidebar** — resizable, collapsible. Show:
   - Directory hierarchy (tree view)
   - Color-coded status (green = added, red = deleted, yellow = modified)
   - **Finding count badges** per file (red for blocking, yellow for warnings)
   - Bold text for unreviewed files (GitLab pattern)

5. **Syntax highlighting** — use a library like `syntect` (Rust) for the backend or `highlight.js`/Shiki for the webview. VS Code-quality highlighting is the expectation.

6. **Context expansion** — show 3-5 lines of context by default. Expand buttons above/below each hunk to load more. "Show full file" option per file.

7. **Syntax-aware diffing** — investigate `difftastic` (Rust, tree-sitter based) for computing diffs. This reduces noise from AI reformatting, understanding structural changes rather than line-by-line text diff.

---

## 3. Findings Display (The Core Innovation)

### How AI review tools present findings

| Tool       | Approach                        | Pros                         | Cons                            |
| ---------- | ------------------------------- | ---------------------------- | ------------------------------- |
| CodeRabbit | Inline comments on PR lines     | Feels like a real reviewer   | High volume, noise              |
| Qodo       | Structured groups by category   | Great for triage             | Less spatial connection to code |
| SonarQube  | Dashboard + inline markers      | Comprehensive lifecycle      | Heavy, enterprise feel          |
| Graphite   | Comments float to right of diff | Doesn't disrupt reading flow | Less direct line association    |

### Recommendations for noslop

**Dual-panel approach**: structured overview + inline markers.

**Left panel (or tab): Finding Overview**

- Grouped by file (matching diff navigation)
- Within each file: sorted by severity (block → warn → info)
- Each finding shows: severity badge, one-line message, source icon (check/agent/human), status
- Click a finding → jumps to it in the diff
- Filter controls: by severity, by source, by status
- Count summary at top: "3 blocking, 5 warnings, 2 info"

**In the diff: Inline markers**

- **Gutter icons** — colored dots/icons in the line number gutter indicating findings on that line
  - Red filled circle = blocking finding
  - Yellow triangle = warning
  - Blue circle = info
  - Purple diamond = human annotation
- Click gutter icon → finding card expands inline (below the line, like GitHub)
- Finding card shows: severity, message, source, suggestion (if any), and action buttons

**Finding card actions:**

- **Resolve** — "I fixed this" (one click)
- **Dismiss** — requires selecting a reason: "False positive" / "Won't fix" / "Not applicable"
- **Reply** — add a human comment to an AI finding (conversation thread)
- **Edit severity** — override the AI's severity assessment

**Progressive disclosure** (from CodeRabbit research):

- Default: severity icon + one-line summary
- Expand: full message + AI reasoning chain
- Further expand: suggested fix (if available, rendered as a mini-diff)

---

## 4. Adding Human Findings/Annotations

### What the platforms do

- **GitHub**: Hover line → blue "+" icon → comment form with "Start a review" / "Add single comment"
- **GitLab**: Hover line number → comment icon → "Add comment now" / "Start a review"
- **Graphite**: Hover line → comment icon. Also: overflow menu with "Add comment", "Suggest change", "Add to chat"

### Recommendations for noslop

**Hover a line number → "+" icon appears.** Click to open finding form:

```
┌─────────────────────────────────────────────┐
│ ● block  ○ warn  ○ info                     │
│                                             │
│ [Message                                  ] │
│                                             │
│ [ ] Include suggestion                      │
│ ┌─────────────────────────────────────────┐ │
│ │ suggested code replacement              │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│              [Cancel]  [Add Finding]        │
└─────────────────────────────────────────────┘
```

- **Multi-line selection**: click-and-drag across line numbers, then click "+"
- **Severity selector** — default to "block" (safer default)
- **Optional suggestion** — a code block the author can apply with one click
- **Source is always "human"** — distinguished from check/agent findings by icon

**Commenting on existing findings:**

- Click an existing finding → expands → reply textarea at bottom
- Threaded conversation model (like GitHub)
- Each reply can include "Resolve" or "Dismiss" action

---

## 5. Navigation

### Keyboard shortcuts (synthesized from all platforms)

**Essential shortcuts for noslop:**

| Shortcut  | Action                              | Inspired by                      |
| --------- | ----------------------------------- | -------------------------------- |
| `j` / `k` | Next / previous finding             | GitHub (hunks), GitLab (threads) |
| `]` / `[` | Next / previous file                | GitLab                           |
| `f`       | Toggle file tree                    | Graphite, GitLab                 |
| `v`       | Mark file as viewed                 | GitHub, GitLab                   |
| `r`       | Resolve current finding             | —                                |
| `d`       | Dismiss current finding             | —                                |
| `c`       | Add comment/finding on current line | —                                |
| `s`       | Toggle split/unified diff           | —                                |
| `w`       | Toggle whitespace                   | GitHub                           |
| `Cmd+K`   | Command palette                     | VS Code, Superhuman              |
| `Cmd+P`   | Quick file jump                     | GitLab, VS Code                  |
| `n` / `p` | Next / previous unresolved finding  | GitLab                           |
| `Enter`   | Expand focused finding              | —                                |
| `Escape`  | Close/collapse current panel        | —                                |
| `?`       | Show keyboard shortcut reference    | GitHub, GitLab                   |

**Command palette** (Cmd+K):

- Fuzzy search over all actions
- Recent commands at top
- Show keyboard shortcuts next to each command
- Context-aware (available commands change based on current view)
- This is the single most important UX investment for a keyboard-driven desktop tool

### Navigation structure

```
┌──────────────────────────────────────────────────────────┐
│  [Review: feature-x]    main..HEAD    3🔴 5🟡 2🔵      │
│  ──────────────────────────────────────────────────────  │
│  │ File Tree    │ Diff + Findings                      │ │
│  │              │                                      │ │
│  │ ▾ src/       │  src/auth.rs  +42 -15  🔴2 🟡1     │ │
│  │   🔴 auth.rs │  ─────────────────────────────────   │ │
│  │   🟡 main.rs │   ... diff content ...              │ │
│  │   ✓ lib.rs   │     [finding card inline]           │ │
│  │ ▾ tests/     │   ... more diff ...                 │ │
│  │   ○ test.rs  │                                      │ │
│  │              │  src/main.rs  +8 -2  🟡1             │ │
│  │              │  ─────────────────────────────────   │ │
│  │              │   ... diff content ...              │ │
│  │──────────────│──────────────────────────────────────│ │
│  │ Findings (10)│                [Resolve All Warnings]│ │
│  │ Filter: All  │                                      │ │
│  └──────────────┴──────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ [Comment only]  [Close Review]          Cmd+K ⌨️   │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

---

## 6. Review Landing Page (The "PR Contract")

When a user opens a review, before diving into the diff, show a summary page:

```
┌──────────────────────────────────────────────┐
│  Review: feature/add-auth                     │
│  Branch: feature/add-auth ← main              │
│  Commits: 3 (by claude)                       │
│  ──────────────────────────────────────────── │
│                                               │
│  📋 Summary                                   │
│  Added JWT authentication middleware with      │
│  login/logout endpoints and token refresh.     │
│                                               │
│  📊 Stats                                     │
│  8 files changed, +342 -28                    │
│  ██████████░░░░░ 67% reviewed                 │
│                                               │
│  🔍 Findings                                  │
│  🔴 3 blocking  🟡 5 warnings  🔵 2 info     │
│                                               │
│  ⚠️  Risk Areas                               │
│  • src/auth.rs — touches authentication       │
│  • src/middleware.rs — new file, untested      │
│                                               │
│  ✅ Tests: 45 passing, 0 failing              │
│  🔨 Build: clean                              │
│                                               │
│  [Open Diff View]        [Run Analysis Again] │
└──────────────────────────────────────────────┘
```

This is inspired by:

- GitHub's PR conversation tab (context before code)
- Graphite's PR page redesign (stack + metadata + description at top)
- Addy Osmani's "PR Contract" for AI code (intent, proof, risk, focus areas)
- SonarQube's dashboard (aggregate metrics)

---

## 7. Review Close Flow

### What platforms do

- **GitHub**: "Review changes" button → dropdown with Approve/Request Changes/Comment
- **GitLab**: Same three options, plus blocking "Request changes" that prevents merge
- **Graphite**: Sticky bottom bar, keyboard chords (R+A, R+N, R+C, R+Y for quick-approve)

### What noslop should do

Since there's no "other person," the close flow is simpler:

1. **Close Review** — all blocking findings must be resolved/dismissed. Shows a confirmation:

   ```
   Close review for feature/add-auth?

   ✅ 3 blocking findings resolved
   ⚠️  2 warnings remaining (non-blocking)
   ℹ️  1 info finding dismissed

   [Cancel]  [Close Review]
   ```

2. **Cannot close with open blocking findings** — the button is disabled with explanation:

   ```
   Cannot close: 2 blocking findings unresolved
   [Jump to first blocking finding]
   ```

3. **After close** — review is marked as closed. `noslop review --check` returns 0 (safe to push). The pre-push hook passes.

---

## 8. Unique Patterns Worth Stealing

### From Graphite

- **Floating comments** — findings appear to the right of the diff, not expanding inline. This preserves the reading flow of the code. The diff is an uninterrupted document.
- **Version-based re-review** — when the branch is updated, show only what changed since last review. "Hide reviewed changes" banner.
- **Sticky review bar** at bottom — never lose access to the submit/close action.
- **Quick-approve shortcut** (R+Y) — for when everything looks good.

### From GitLab

- **Thread vs comment distinction** — findings (threads) can block close. Human annotations (comments) are informational.
- **"Create issue to resolve thread"** — promote a finding to a tracked issue without blocking the review.
- **Auto-resolve outdated threads** — when a push changes the lines a finding references, auto-resolve it.
- **Conventional Comments format** — structured labels (suggestion:, issue:, question:, praise:) for human annotations.

### From GitHub

- **Suggestion feature** — inline code suggestions rendered as mini-diffs with one-click apply. Creates a commit with co-authorship attribution.
- **Viewed checkmarks with progress bar** — "3 of 12 files viewed" with auto-uncheck when file changes.
- **Conversations dropdown** — jump to any unresolved/resolved thread from a single menu.
- **Pending/draft comments** — batch comments and submit together (single notification).

### From AI review tools

- **Progressive disclosure** (CodeRabbit) — severity icon → one-line summary → full reasoning → suggested fix.
- **Structured grouping** (Qodo) — group findings by category, not just scatter inline.
- **Judge/filter layer** (Qodo) — filter low-confidence AI findings before showing to user.
- **Learning from dismissals** (Sourcery) — if users keep dismissing a type of finding, suppress it.
- **Confidence indicators** — show how confident the AI is in each finding.

### From desktop tools

- **Multi-mode gutter** (GitLens) — toggle between: diff markers, findings, blame, heatmap.
- **Three diff layouts** (Kaleidoscope) — blocks (scanning), fluid (flow), unified (deep comparison).
- **Syntax-aware diffing** (difftastic) — tree-sitter based, ignores reformatting noise.
- **Command palette** (VS Code/Superhuman) — the backbone of a keyboard-driven desktop app.

---

## 9. Technical Implementation Notes

### Diff rendering

- Consider **diff2html** (JavaScript) for the webview rendering layer — supports unified/split, syntax highlighting, word-level diff, synchronized scrolling.
- Compute diffs in Rust using `git2` (already in the codebase) for performance.
- Investigate **difftastic** for syntax-aware diff as an option.

### Storage

- Reviews already stored as JSON files in `.noslop/reviews/` — this is fine for now.
- Future: consider `git notes` for findings that should travel with the repo.

### Component library

- Already using **shadcn/ui** components (badge, button, card, dialog, scroll-area, separator, tabs).
- Add: resizable panels (for file tree + diff split), command palette, tooltip, popover.

### Dark mode

- System preference detection with manual override. Non-negotiable for developer tools.

---

## 10. Priority Roadmap

### P0 — Must have for usable review experience

1. Review landing page with summary, stats, findings count
2. Proper diff rendering (unified + split toggle, syntax highlighting, context expansion)
3. File tree sidebar with finding count badges
4. Inline findings display with resolve/dismiss actions
5. Keyboard navigation (j/k, n/p, ]/[, f, v)
6. Close review flow with blocking check

### P1 — Makes it great

7. Command palette (Cmd+K)
8. Human finding creation (click line → add finding form)
9. Threaded replies on findings
10. "Viewed" checkmarks per file with progress bar
11. Filter/sort findings by severity, source, status
12. Dark mode

### P2 — Differentiators

13. Floating comments mode (Graphite-style, findings to the right of diff)
14. Suggestion feature with one-click apply
15. Version-based re-review ("show only what changed since last review")
16. AI confidence indicators on findings
17. Auto-resolve findings when underlying code changes
18. Learning from dismissal patterns
