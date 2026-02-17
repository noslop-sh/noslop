# GUI/UX Design Specification

Complete redesign specification for the noslop Tauri desktop app. Transforms the current sidebar + finding card layout into a full code review workstation with diff rendering, file tree navigation, inline findings, and keyboard-first interaction.

**Tech stack**: Svelte 5 (runes) + Tailwind CSS v4 (oklch) + shadcn-svelte (bits-ui v2) + @tanstack/svelte-query v5 + Tauri v2 + Lucide icons

**References**: `00-core-domain-model.md` (domain types), `01-review-pipeline.md` (pipeline architecture), `06-gui-ux-research.md` (UX research)

---

## 1. Design Philosophy

noslop is a **trust checkpoint** -- the human verifies AI-generated code before pushing. It is not a social code review tool. There is no "other person" to approve. The developer is triaging findings from automated checks, AI agents, and their own observations on a local branch.

**Mental model**: Airport security checkpoint, not GitHub pull request.

### Three Principles

| Principle          | Meaning                                                      | Example                                                                  |
| ------------------ | ------------------------------------------------------------ | ------------------------------------------------------------------------ |
| **Transparency**   | Every finding shows its source and reasoning. No black boxes | Finding card shows `source: agent:security` and expandable AI reasoning  |
| **Control**        | The human decides what blocks push, not the tool             | Dismiss with reason IS the escape hatch (no `--force` flag)              |
| **Predictability** | Same input produces same UI state. No surprises              | Deterministic sort order, session persistence, no "helpful" auto-actions |

### Interaction Model

- **Keyboard-first**: Every action has a keyboard shortcut. Mouse is supported but never required
- **Progressive disclosure**: Gutter icon -> one-line summary -> full card -> AI reasoning -> suggestion diff
- **Continuous flow**: Landing page scrolls into diff. No page transitions within a review
- **Local-first**: All state lives on disk. No accounts, no network, no sync

---

## 2. Application Layout

### Full Shell

```
┌─────────────────────────────────────────────────────────────────┐
│ StatusBar                                                       │
│ Review: feature/add-auth  |  3 block  5 warn  |  Tests: pass   │
├────────────┬────────────────────────────────────────────────────┤
│            │                                                    │
│  Review    │  Main Content Area                                 │
│  List      │                                                    │
│  (collaps) │  ┌──────────────────────────────────────────────┐  │
│            │  │  Landing Page (scrolls away)                 │  │
│ ────────── │  │  Verdict + Stats + Risk Areas + CTA          │  │
│            │  └──────────────────────────────────────────────┘  │
│  File      │                                                    │
│  Tree      │  ┌──────────────────────────────────────────────┐  │
│  (primary) │  │  Continuous Diff Scroll                      │  │
│            │  │  ┌──sticky file header──────────────────────┐│  │
│ ────────── │  │  │ src/auth.rs  +42 -15  [block] [viewed]  ││  │
│            │  │  ├──────────────────────────────────────────┤│  │
│  Findings  │  │  │  ... diff hunks with gutter icons ...   ││  │
│  Panel     │  │  │  ... inline finding cards ...           ││  │
│  (collaps) │  │  │                                         ││  │
│            │  │  └─────────────────────────────────────────┘│  │
│            │  │  ┌──sticky file header──────────────────────┐│  │
│            │  │  │ src/main.rs  +8 -2  [warn]              ││  │
│            │  │  │  ...                                    ││  │
│            │  └──────────────────────────────────────────────┘  │
│            │                                                    │
├────────────┴────────────────────────────────────────────────────┤
│ ActionBar                                                       │
│ [Close Review & Allow Push]          2 findings remaining       │
└─────────────────────────────────────────────────────────────────┘
```

### Panel Dimensions

| Panel                  | Default Width | Min           | Max   | Collapse                  |
| ---------------------- | ------------- | ------------- | ----- | ------------------------- |
| Sidebar (total)        | 280px         | 200px         | 400px | `f` toggles 3 states      |
| Review list section    | auto-height   | 0 (collapsed) | 200px | Chevron toggle            |
| File tree section      | flex-1        | 100px         | --    | Always visible in sidebar |
| Findings panel section | 180px         | 0 (collapsed) | 300px | Chevron toggle            |
| Main content           | flex-1        | 600px         | --    | Never collapses           |

### Sidebar Collapse States

Cycled by pressing `f`:

```
State 1: Full (280px)        State 2: Mini (44px)       State 3: Hidden (0px)
┌──────────┐                 ┌──┐                       ┌┐
│ Reviews  │                 │A │                       ││
│ ──────── │                 │M │                       ││
│ src/     │                 │D │                       ││
│  auth.rs │                 │M │                       ││
│  main.rs │                 │  │                       ││
│ ──────── │                 └──┘                       └┘
│ Findings │
└──────────┘
```

Mini-tree shows: change type icon (colored A/M/D/R), finding severity dot, no text.

### Resizing

Drag handle between sidebar and main content. Uses `ResizablePanel` component (shadcn-svelte). Persists width to session state.

---

## 3. Review Landing Page

The landing page is the top of the continuous scroll. It is NOT a separate route. When the user scrolls past it, the `StatusBar` collapses the summary into a single-line sticky header.

### Information Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  VERDICT BADGE                                              │
│  ┌─────────────────────────────┐                            │
│  │  3 Blockers Must Be Resolved│  (red, when blocked)       │
│  └─────────────────────────────┘                            │
│  ┌─────────────────────────────┐                            │
│  │  Ready to Close             │  (green, when clean)       │
│  └─────────────────────────────┘                            │
│                                                             │
│  STAT BAR (5 items max)                                     │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐              │
│  │8     │ │+342  │ │3     │ │5     │ │pass  │              │
│  │files │ │-28   │ │block │ │warn  │ │tests │              │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘              │
│                                                             │
│  RISK AREAS                                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ ! Authentication   src/auth.rs, src/middleware.rs   │    │
│  │ ! Database         migrations/003_add_users.sql     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  ADAPTIVE CTA                                               │
│  ┌───────────────────────────────┐                          │
│  │  Resolve 3 Blockers           │  (when blocked)          │
│  └───────────────────────────────┘                          │
│  ┌───────────────────────────────┐                          │
│  │  Close Review & Allow Push    │  (when clean)            │
│  └───────────────────────────────┘                          │
│                                                             │
│  ─── continuous scroll into diff below ───                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Verdict Badge Rules

| State                  | Badge                         | Color                | CTA                                     |
| ---------------------- | ----------------------------- | -------------------- | --------------------------------------- |
| Open blocking findings | "N Blockers Must Be Resolved" | `destructive` (red)  | "Resolve N Blockers" (scrolls to first) |
| Open warnings only     | "N Warnings to Review"        | `secondary` (amber)  | "Close Review & Allow Push"             |
| All clear              | "Ready to Close"              | green                | "Close Review & Allow Push"             |
| Pipeline running       | "Analysis Running..."         | muted                | Skeleton loading                        |
| Closed review          | "Review Closed"               | green with checkmark | "Reopen Review"                         |

### Risk Areas

Configured in `.noslop.toml`:

```toml
[risk]
patterns = [
  { glob = "src/**/auth*", label = "Authentication" },
  { glob = "**/migrations/**", label = "Database migrations" },
  { glob = ".github/workflows/**", label = "CI/CD" },
]
```

Displayed only when at least one matched file is in the diff. Each risk area links to its first matching file in the diff.

### Edge Cases

| Scenario                  | Behavior                                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Single file in diff       | Skip file tree section, show file directly                                                                                      |
| Closed review             | Read-only. Green banner at top. All actions disabled except "Reopen Review"                                                     |
| Stale review (HEAD moved) | Yellow warning banner: "Branch has new commits since this review. Re-run analysis or start new review"                          |
| 100+ findings             | Group by severity in landing page stats. Default filter: Status=Open. Truncate finding list in sidebar to 50, "Show all N" link |
| No findings               | "All Clear" verdict. One-click close                                                                                            |
| Pipeline still running    | Skeleton screens for stats/findings. Findings appear progressively as computed. Stat counters animate up                        |

### StatusBar (Sticky After Scroll)

When the landing page scrolls out of view, the StatusBar transitions from detailed to compact:

```
Compact StatusBar (sticky, 40px):
┌─────────────────────────────────────────────────────────────┐
│ feature/add-auth  │  3 block  5 warn  │  8/12 viewed  │  [Close Review]  │
└─────────────────────────────────────────────────────────────┘
```

Implementation: `IntersectionObserver` on the landing page container. When it leaves the viewport, StatusBar switches to compact mode via a `$state` boolean.

---

## 4. Diff Rendering

### Library Choice

**`@git-diff-view/svelte`** with **`@git-diff-view/shiki`** for syntax highlighting.

Rationale:

- Purpose-built Svelte component with split/unified toggle
- Shiki integration provides VS Code-quality syntax highlighting
- Supports custom gutter widgets (for finding icons)
- Active maintenance, ~55KB gzipped

### Backend Diff Computation

Diffs are computed in Rust and sent as structured JSON, NOT raw unified diff strings. This enables character-level highlighting and structured navigation.

#### Rust Types

```rust
/// Structured diff output for the frontend.
/// Replaces the current `get_diff` command that returns raw unified diff text.
#[derive(Debug, Clone, Serialize)]
pub struct StructuredDiff {
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,           // for renames
    pub change_type: FileChangeType,         // Added, Modified, Deleted, Renamed
    pub hunks: Vec<Hunk>,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
    pub language: Option<String>,            // detected from extension
}

#[derive(Debug, Clone, Serialize)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed { similarity: u8 },
}

#[derive(Debug, Clone, Serialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: String,                      // @@ line
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,                  // Add, Delete, Context
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    pub content: String,
    pub char_changes: Option<Vec<CharChange>>, // character-level diff
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum DiffLineKind {
    Add,
    Delete,
    Context,
}

/// Character-level change within a line.
/// Computed by the `similar` crate in Rust.
#[derive(Debug, Clone, Serialize)]
pub struct CharChange {
    pub start: usize,                        // byte offset
    pub end: usize,                          // byte offset
    pub kind: CharChangeKind,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum CharChangeKind {
    Insert,
    Delete,
    Equal,
}
```

#### Frontend TypeScript Types

```typescript
// gui/src/lib/types.ts (additions)

export type FileChangeType =
  | "added"
  | "modified"
  | "deleted"
  | { renamed: { similarity: number } };

export interface DiffStats {
  files_changed: number;
  additions: number;
  deletions: number;
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

export interface Hunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  header: string;
  lines: DiffLine[];
}

export interface DiffLine {
  kind: "add" | "delete" | "context";
  old_line_no: number | null;
  new_line_no: number | null;
  content: string;
  char_changes: CharChange[] | null;
}

export interface CharChange {
  start: number;
  end: number;
  kind: "insert" | "delete" | "equal";
}

export interface StructuredDiff {
  files: FileDiff[];
  stats: DiffStats;
}
```

### View Modes

| Mode                 | When                   | Description                 |
| -------------------- | ---------------------- | --------------------------- |
| **Split** (default)  | Window width >= 1200px | Side-by-side old/new        |
| **Unified** (auto)   | Window width < 1200px  | Single column, +/- prefixed |
| **Unified** (manual) | User presses `s`       | Toggle regardless of width  |

### Character-Level Highlighting

Two-tier visual hierarchy:

```
Tier 1 (line background):
  Added line:    bg-green-50 / dark:bg-green-950
  Deleted line:  bg-red-50   / dark:bg-red-950

Tier 2 (character foreground, overlaid on tier 1):
  Inserted chars:  bg-green-200 / dark:bg-green-800
  Deleted chars:   bg-red-200   / dark:bg-red-800
```

Computed by the `similar` crate in Rust. The frontend receives `CharChange[]` per line and renders highlighted `<span>` elements.

### Context Lines

| Setting          | Value                                                               |
| ---------------- | ------------------------------------------------------------------- |
| Default context  | 5 lines above/below each hunk                                       |
| Expand increment | 20 lines per click                                                  |
| Expand trigger   | "Show 20 more lines" button between hunks                           |
| Full file        | "Show entire file" in file header dropdown                          |
| Data source      | `get_file_content` Tauri command (path, commit, startLine, endLine) |

### Continuous Scroll with Sticky File Headers

Files render sequentially in a single scrollable container. Each file has a sticky header that pins to the top of the viewport when scrolling through that file's diff.

```
┌──────────────────────────────────────────────────────┐
│ ■ src/auth.rs  +42 -15  [M]  ●3  ▲2  ☑ viewed     │  <- sticky
├──────────────────────────────────────────────────────┤
│  10  │  10  │  fn validate_session(token: &str) {   │
│  11  │      │- if token.is_empty() { return false; } │
│      │  11  │+ if token.is_empty() || expired(token) │
│      │  12  │+     return Err(SessionError::Expired)  │
│  ... │  ... │  ...                                   │
└──────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────┐
│ ■ src/main.rs  +8 -2  [M]  ▲1                      │  <- sticky
├──────────────────────────────────────────────────────┤
│  ...                                                 │
```

File header contents:

- Change type icon: colored letter (green A, yellow M, red D, blue R)
- File path (monospace, clickable)
- `+N -N` addition/deletion counts
- Finding badges: red circle with count (block), yellow triangle with count (warn)
- Viewed checkmark toggle (click or `v` when file is focused)

### Large Diff Handling

| Threshold                | Behavior                                                                                                         |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| File > 500 changed lines | Auto-collapsed. Shows header + "Large file (N changes). Click to expand"                                         |
| Total > 50 files         | File diffs lazy-loaded via `IntersectionObserver`. Only files within 2 viewports of scroll position are rendered |
| Binary files             | Show "Binary file changed" with file size delta. No diff                                                         |

File-level virtualization, NOT line-level. Each file is either rendered or not. Within a rendered file, all hunks are in the DOM.

---

## 5. File Tree Sidebar

### Structure

```
┌──────────────────────────────────┐
│ Files (8)  ▼ Sort: Findings ▼   │  <- header with sort dropdown
│ ┌──────────────────────────────┐ │
│ │ [filter files...]           │ │  <- inline search (P0)
│ └──────────────────────────────┘ │
│                                  │
│ ▾ src/                    ●3 ▲2 │  <- dir with aggregated badges
│   │ ● auth.rs    M +42 -15  ☑  │  <- block finding, viewed
│   │ ▲ main.rs    M  +8  -2     │  <- warn finding, not viewed
│   │   lib.rs     M  +3  -1  ☑  │  <- no findings, viewed
│   ▾ middleware/          ●1     │
│     │ ● jwt.rs   A +88  -0     │  <- new file with block finding
│ ▾ tests/                        │
│   │   auth.rs    M +22  -5  ☑  │
│ ▾ migrations/            ●1     │
│   │ ● 003.sql   A +15  -0     │
│                                  │
│ ─── progress ──────────────── │
│ ████████░░░░ 5/8 viewed         │
│ ██████░░░░░░ 7/10 resolved      │
└──────────────────────────────────┘
```

### Default Sort Order

**By finding priority** (not alphabetical):

1. Files with open `block` findings (sorted by count, descending)
2. Files with open `warn` findings (sorted by count, descending)
3. Files with open `info` findings
4. Files with no findings (unviewed first, then viewed)

Within each tier, alphabetical by path. User can switch to alphabetical via sort dropdown.

### Compact Folders

Single-child directories are collapsed into one line:

```
Before:                          After:
▾ src/                           ▾ src/middleware/
  ▾ middleware/                    │ jwt.rs
    │ jwt.rs
```

### Per-File Metadata

| Element          | Position      | Description                                              |
| ---------------- | ------------- | -------------------------------------------------------- |
| Change type icon | Left gutter   | Colored letter: green `A`, yellow `M`, red `D`, blue `R` |
| File name        | Main text     | Monospace, color-coded by git status                     |
| `+N -N`          | Right-aligned | Addition/deletion counts, green/red                      |
| Finding badges   | After +/-     | Red circle (block count), yellow triangle (warn count)   |
| Viewed checkmark | Far right     | Green checkmark when marked viewed                       |

### Directory Aggregation

Parent directories show the sum of finding badges from all descendant files. Only unresolved findings count.

### Tree Node Component Props

```typescript
// gui/src/lib/types.ts

export interface FileTreeEntry {
  path: string; // full relative path
  name: string; // display name (last segment)
  kind: "file" | "directory";
  change_type: FileChangeType | null; // null for directories
  additions: number;
  deletions: number;
  findings: {
    block: number;
    warn: number;
    info: number;
  };
  viewed: boolean;
  children: FileTreeEntry[]; // empty for files
  collapsed_prefix: string | null; // compact folder prefix
}
```

### Inline Filter

Text input in tree header. Filters visible nodes by substring match on file path. Case-insensitive. Empty string shows all. Does NOT change sort order.

### Progress Tracking

Dual-axis progress bar at bottom of file tree:

```
Files viewed:    ████████░░░░ 5/8
Findings open:   ██████░░░░░░ 3/10 resolved
```

Implementation: `$derived` from review state. Files viewed = count of files in `viewed_files` set. Findings resolved = count of findings with status `resolved` or `dismissed`.

---

## 6. Findings Display

### Three-Layer Rendering

Findings render at three levels of detail, each progressively more visible:

```
Layer 1: Gutter Icon (always visible in diff)
    ● ▲ ● ◆                    (colored dots in line number gutter)

Layer 2: Hover Preview (on gutter icon hover)
    ┌──────────────────────────────────┐
    │ ● block | JWT secret validation  │
    │   agent:security                 │
    └──────────────────────────────────┘

Layer 3: Inline Card (on click or Enter)
    ┌──────────────────────────────────────────────────────┐
    │ ● block  JWT secret read without validation          │
    │   agent:security                              [v][x] │
    │ ─────────────────────────────────────────────────── │
    │ The JWT_SECRET env var is read on line 14 without    │
    │ checking if it's set. In production this would...    │
    │                                                      │
    │ Suggestion:                                          │
    │ - let secret = env::var("JWT_SECRET")?;              │
    │ + let secret = env::var("JWT_SECRET")                │
    │ +     .expect("JWT_SECRET must be set");             │
    │                                                      │
    │ [Resolve]  [Dismiss v]  [Apply Suggestion]           │
    │                                                      │
    │ Notes:                                               │
    │ [Add a note...]                                      │
    └──────────────────────────────────────────────────────┘
```

### Gutter Icons

| Severity | Icon              | Color (light)                     | Color (dark)             |
| -------- | ----------------- | --------------------------------- | ------------------------ |
| Block    | Filled circle `●` | `oklch(0.577 0.245 27)` (red)     | `oklch(0.704 0.191 22)`  |
| Warn     | Triangle `▲`      | `oklch(0.769 0.188 70)` (amber)   | `oklch(0.828 0.189 84)`  |
| Info     | Circle `○`        | `oklch(0.488 0.243 264)` (blue)   | `oklch(0.696 0.17 162)`  |
| Human    | Diamond `◆`       | `oklch(0.627 0.265 303)` (purple) | `oklch(0.627 0.265 303)` |

Multiple findings on the same line: show highest severity icon. Badge with count if > 1.

Gutter icon opacity varies with confidence (P2): confidence 1.0 = full opacity, confidence 0.4 = 40% opacity.

### Finding Card States

**Collapsed (one-line)**:

```
● block  JWT secret read without validation  agent:security  [Resolve] [...]
```

**Expanded (full)**:

```
● block  JWT secret read without validation                    [Resolve] [Dismiss v]
  source: agent:security
  ────────────────────────────────────────────────────────
  Full message with AI reasoning chain...

  Suggestion:
  ┌──────────────────────────────────────────────────┐
  │ - let secret = env::var("JWT_SECRET")?;          │
  │ + let secret = env::var("JWT_SECRET")            │
  │ +     .expect("JWT_SECRET must be set");         │
  │                                    [Apply] [Edit]│
  └──────────────────────────────────────────────────┘

  Notes:
  [Add a note...]
```

### Finding TypeScript Types

```typescript
// gui/src/lib/types.ts (extensions to existing Finding)

export type Severity = "block" | "warn" | "info";

export type FindingStatus = "open" | "resolved" | "dismissed";

export type FindingSourceKind = "check" | "script" | "agent" | "human";

export interface FindingSource {
  kind: FindingSourceKind;
  name: string | null; // null for human
}

export interface Suggestion {
  original: string | null; // original code (for mini-diff)
  replacement: string; // suggested replacement
  edited: boolean; // user edited the suggestion
}

export type DismissReason =
  | "false_positive"
  | "wont_fix"
  | "not_applicable"
  | "investigate_later";

export type ResolutionReason =
  | "manual"
  | "suggestion_applied"
  | "code_changed"
  | "file_removed";

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
  confidence: number | null; // 0.0-1.0, internal sorting only
  notes: FindingNote[];
  created_at: string;
}

export interface Target {
  path: string;
  span: Span | null;
  commit: string | null;
}

export interface Span {
  start: number; // 1-indexed, inclusive
  end: number; // 1-indexed, inclusive
}
```

### Resolve Action

- **Trigger**: Click "Resolve" button or press `r`
- **Friction**: Zero. One click/keypress
- **Animation**: Green checkmark scales in (Svelte `scale` transition, 200ms). Card background fades to 50% opacity
- **Persistence**: Calls `resolve_finding` Tauri command. Invalidates review query

### Dismiss Action

- **Trigger**: Click "Dismiss" button or press `d`
- **Friction**: Two steps. Click opens reason picker dropdown
- **Reason options**: "False positive" | "Won't fix" | "Not applicable" | "Investigate later"
- **Animation**: Card background fades to gray (distinct from green resolve)
- **Persistence**: Calls `dismiss_finding` Tauri command with reason

Dismiss reason picker rendered as a dropdown menu (shadcn-svelte `DropdownMenu`):

```
┌──────────────────────┐
│ False positive       │
│ Won't fix            │
│ Not applicable       │
│ Investigate later    │
└──────────────────────┘
```

### Human Finding Creation

**Trigger**: Hover line number in diff -> `+` icon appears. Click to open form. Or press `c` when a line is focused.

**Multi-line**: Shift+click on second line number after clicking first. Or click-drag across gutter line numbers.

```
┌─────────────────────────────────────────────────────┐
│ Severity:  (x) info   ( ) warn   ( ) block         │
│                                                     │
│ Message:                                            │
│ ┌─────────────────────────────────────────────────┐ │
│ │                                                 │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ [ ] Include suggestion                              │
│ ┌─────────────────────────────────────────────────┐ │
│ │ replacement code                                │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│                         [Cancel]  [Add Finding]     │
└─────────────────────────────────────────────────────┘
```

Default severity: `info` (human findings are observations, not automated checks).

Source is always `FindingSource::Human`.

### Finding Filters

Located in the Findings Panel section of the sidebar. Segmented control buttons (shadcn-svelte `Tabs`):

```
Status:    [Open] [Resolved] [Dismissed] [All]
Severity:  [Block] [Warn] [Info] [All]
Source:    [All v]  (dropdown: check, agent, script, human)
```

Defaults: Status=Open, Severity=All, Source=All.

Filters affect both the sidebar findings list AND the gutter icon visibility in the diff. Resolved/dismissed findings are hidden from the gutter by default.

### Batch Actions

**P0**: "Resolve All" button on filtered view. If filtered to `Status=Open, Severity=Warn`, clicking "Resolve All" resolves all open warnings.

**P1**: Checkbox selection on individual findings + floating batch action bar:

```
┌──────────────────────────────────────────────────────┐
│ 5 selected  │  [Resolve All]  [Dismiss All v]  [x]  │
└──────────────────────────────────────────────────────┘
```

### Animations

All finding state transitions use Svelte transitions:

| Action                    | Transition           | Duration |
| ------------------------- | -------------------- | -------- |
| Card expand               | `slide`              | 200ms    |
| Card collapse             | `slide`              | 150ms    |
| Resolve                   | `fade` + green flash | 300ms    |
| Dismiss                   | `fade` to gray       | 200ms    |
| New finding appears       | `fly` from left      | 250ms    |
| Finding removed by filter | `fade` out           | 150ms    |

---

## 7. Close Flow

### Close Button Behavior

The close button is **always enabled**, never disabled. Clicking it always does something meaningful.

### When Blocked (Open Block-Severity Findings)

Click "Close Review" -> Interstitial dialog listing all blockers:

```
┌─────────────────────────────────────────────────────────────┐
│  Cannot Close: 3 Blocking Findings                          │
│                                                             │
│  These findings must be resolved or dismissed before        │
│  closing this review:                                       │
│                                                             │
│  1. ● src/auth.rs:14 - JWT secret read without validation  │
│     agent:security                                          │
│                                                             │
│  2. ● src/auth.rs:42 - Session validation changed           │
│     check:NOS-1                                             │
│                                                             │
│  3. ● migrations/003.sql - NOT NULL without DEFAULT         │
│     script:check-migrations                                 │
│                                                             │
│  [Jump to First Blocker]                     [Dismiss]      │
└─────────────────────────────────────────────────────────────┘
```

"Jump to First Blocker" scrolls to the finding in the diff and focuses it. "Dismiss" closes the dialog (NOT the review).

### When Clear (No Open Block-Severity Findings)

Click "Close Review" -> Confirmation dialog with structured summary:

```
┌─────────────────────────────────────────────────────────────┐
│  Close Review: feature/add-auth                             │
│                                                             │
│  Summary:                                                   │
│  Findings resolved ........... 7/10                         │
│  Findings dismissed ........... 2/10  (1 false positive,    │
│                                        1 won't fix)         │
│  Findings remaining ........... 1    (info, non-blocking)  │
│  Files viewed ............... 6/8    (75%)                  │
│  Tests ....................... pass                          │
│                                                             │
│  [Cancel]                    [Close Review & Allow Push]     │
└─────────────────────────────────────────────────────────────┘
```

### Dynamic Dialog Content

The dialog NEVER shows identical text twice across sessions. The summary is always derived from actual review state. This prevents automated click-through.

### Action Button Labels

| State                 | Label                             |
| --------------------- | --------------------------------- |
| Clean, ready to close | "Close Review & Allow Push"       |
| Blocked               | "Jump to First Blocker" (primary) |
| Review already closed | "Review is closed" (disabled)     |

### After Close

1. Toast notification: "Review closed. Safe to push." with "Copy push command" action button
2. Review card in sidebar transitions: removes red badge, shows green checkmark
3. Diff view becomes read-only (no resolve/dismiss buttons, no add finding)
4. Landing page shows green "Review Closed" banner with "Reopen Review" button

### Blocking Policy

Configurable in `.noslop.toml`:

```toml
[review]
close_policy = "normal"    # "strict" | "normal" | "lenient"
```

| Policy             | Block on                       | Warn on                         |
| ------------------ | ------------------------------ | ------------------------------- |
| `strict`           | Block + Warn findings          | --                              |
| `normal` (default) | Block findings only            | --                              |
| `lenient`          | Nothing (close always allowed) | Block findings shown as warning |

### Dismiss as Escape Hatch

There is NO `--force` flag. The escape hatch is: Dismiss with mandatory reason. This creates an audit trail in the review JSON.

### Re-open

Button on closed review's landing page: "Reopen Review". Transitions status back to `Open`. All finding states preserved.

---

## 8. New Review Creation

### Primary Path (Zero-Config)

```
User clicks "New" button in sidebar header
  -> Auto-detect current branch name (get_current_branch Tauri command)
  -> Compute merge-base with main/master (get_merge_base Tauri command)
  -> Immediately start review with those SHAs
  -> Pipeline begins running, findings appear progressively
```

One click, zero configuration. The most common case: "review what I have on this feature branch vs main."

### Advanced Path

Long-press "New" button (500ms) OR `Cmd+K` -> "New Review (Custom Range)":

```
┌─────────────────────────────────────────────────────────────┐
│  New Review (Custom Range)                                  │
│                                                             │
│  Base branch:  [main           v]                           │
│  Head branch:  [feature/auth   v]                           │
│                                                             │
│  Preview:                                                   │
│  3 commits, 8 files changed, +342 -28                       │
│                                                             │
│  Pipeline:                                                  │
│  [x] Convention checks                                      │
│  [x] Formatting                                             │
│  [ ] Agent: security                                        │
│  [ ] Agent: quality                                         │
│                                                             │
│  [Cancel]                          [Start Review]           │
└─────────────────────────────────────────────────────────────┘
```

Branch pickers are dropdown selects populated from `get_branches` Tauri command. NOT raw SHA text inputs (the current UI's approach).

---

## 9. Keyboard Shortcuts

### Global Shortcuts

These work regardless of focus zone, UNLESS focus is in a textarea (see "Text Input Safety" below).

| Key      | Action                                   | Mnemonic                |
| -------- | ---------------------------------------- | ----------------------- |
| `]`      | Next file                                | Right bracket = forward |
| `[`      | Previous file                            | Left bracket = back     |
| `j`      | Next finding                             | Vim down                |
| `k`      | Previous finding                         | Vim up                  |
| `n`      | Next unresolved finding                  | Next                    |
| `p`      | Previous unresolved finding              | Previous                |
| `r`      | Resolve focused finding                  | Resolve                 |
| `d`      | Dismiss focused finding                  | Dismiss                 |
| `c`      | Add finding on current line              | Comment                 |
| `v`      | Toggle file as viewed                    | Viewed                  |
| `f`      | Cycle sidebar collapse state             | File tree               |
| `s`      | Toggle split/unified diff                | Split                   |
| `w`      | Toggle whitespace visibility             | Whitespace              |
| `m`      | Toggle floating/inline comment mode (P2) | Mode                    |
| `Cmd+K`  | Open command palette                     | --                      |
| `Cmd+P`  | Quick file jump                          | --                      |
| `Enter`  | Expand focused finding                   | --                      |
| `Escape` | Close/collapse current panel or dialog   | --                      |
| `?`      | Show keyboard shortcut overlay           | --                      |

### Focus Zones

The UI has four focus zones. Focus determines which element `j`/`k`/`Enter`/`Escape` act on.

```
Zone 1: File Tree          Zone 2: Diff Content
  j/k = move selection      j/k = next/prev finding
  Enter = open file,         Enter = expand finding
    move focus to Zone 2     Escape = collapse finding,
  Escape = nothing             return focus to Zone 1

Zone 3: Finding Card       Zone 4: Dialog/Modal
  r = resolve                Enter = confirm
  d = dismiss                Escape = cancel/close
  Enter = toggle expand
  Escape = collapse,
    return focus to Zone 2
```

`f` toggles sidebar visibility but does not change focus zone.

### Text Input Safety

When focus is in a `<textarea>` or `<input>`:

- Single-key shortcuts (`r`, `d`, `j`, `k`, etc.) are **disabled**
- Modifier shortcuts (`Cmd+K`, `Cmd+P`) remain **active**
- `Escape` exits the text input and returns to the previous focus zone

Implementation: `keyboard.svelte.ts` store checks `document.activeElement.tagName` before dispatching single-key shortcuts.

### Shortcut Overlay

Pressing `?` shows a full-screen overlay with all shortcuts grouped by context:

```
┌─────────────────────────────────────────────────────────────┐
│  Keyboard Shortcuts                                    [x]  │
│                                                             │
│  Navigation                   Findings                      │
│  ] / [    Next/prev file      r    Resolve                  │
│  j / k    Next/prev finding   d    Dismiss                  │
│  n / p    Next/prev open      c    Add finding              │
│  Cmd+P    Quick file jump     Enter Expand                  │
│                                                             │
│  View                         Review                        │
│  f    Toggle file tree        Cmd+K  Command palette        │
│  s    Split/unified           ?      This overlay           │
│  w    Whitespace              Esc    Close/collapse          │
│  v    Mark file viewed                                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 10. Command Palette (Cmd+K)

### Implementation

Built with the shadcn-svelte `Command` component, which wraps bits-ui's command primitive following cmdk patterns.

### Structure

```
┌─────────────────────────────────────────────────────────────┐
│  > search commands...                                       │
│ ─────────────────────────────────────────────────────────── │
│  Recent                                                     │
│    Resolve finding                                     r    │
│    Toggle split view                                   s    │
│                                                             │
│  Actions                                                    │
│    Close Review                                             │
│    Resolve All Filtered                                     │
│    Toggle Dark Mode                                         │
│    Rerun Analysis                                           │
│                                                             │
│  Files                                                      │
│    src/auth.rs                                     ]  /  [  │
│    src/main.rs                                              │
│    migrations/003.sql                                       │
│                                                             │
│  Findings                                                   │
│    JWT secret read without validation              j  /  k  │
│    Session validation changed                               │
│                                                             │
│  Navigation                                                 │
│    Go to landing page                                       │
│    Go to first blocker                                      │
└─────────────────────────────────────────────────────────────┘
```

### Behavior

| Feature          | Detail                                                       |
| ---------------- | ------------------------------------------------------------ |
| Activation       | `Cmd+K` from anywhere                                        |
| Search           | Fuzzy match over command names, file paths, finding messages |
| Groups           | "Recent", "Actions", "Files", "Findings", "Navigation"       |
| Keyboard nav     | Arrow keys to move, `Enter` to execute                       |
| Shortcut display | Keyboard shortcut shown right-aligned for each command       |
| Context-aware    | Landing page shows different commands than diff view         |
| Dismiss          | `Escape` or click outside                                    |

### Command Registry

```typescript
// gui/src/lib/stores/command-registry.ts

export interface PaletteCommand {
  id: string;
  label: string;
  group: "actions" | "files" | "findings" | "navigation";
  shortcut?: string;
  action: () => void;
  available: () => boolean; // context-sensitive visibility
  icon?: Component; // Lucide icon component
}
```

---

## 11. Dark Mode

### Implementation

Three-way toggle: Light / Dark / System (follow OS preference).

Already have oklch color tokens for both themes in `gui/src/app.css` (`:root` for light, `.dark` for dark).

### Toggle Location

- StatusBar: Sun/Moon icon button (always visible)
- Command palette: "Toggle Dark Mode" action

### Diff-Specific Colors

Diff highlighting needs separate color values for dark mode:

```css
/* gui/src/app.css additions */

:root {
  --diff-add-bg: oklch(0.95 0.05 145);
  --diff-add-char: oklch(0.85 0.12 145);
  --diff-del-bg: oklch(0.95 0.05 25);
  --diff-del-char: oklch(0.85 0.12 25);
  --diff-hunk-header: oklch(0.95 0.03 250);
  --finding-block: oklch(0.577 0.245 27);
  --finding-warn: oklch(0.769 0.188 70);
  --finding-info: oklch(0.488 0.243 264);
  --finding-human: oklch(0.627 0.265 303);
}

.dark {
  --diff-add-bg: oklch(0.22 0.05 145);
  --diff-add-char: oklch(0.35 0.12 145);
  --diff-del-bg: oklch(0.22 0.05 25);
  --diff-del-char: oklch(0.35 0.12 25);
  --diff-hunk-header: oklch(0.22 0.03 250);
  --finding-block: oklch(0.704 0.191 22);
  --finding-warn: oklch(0.828 0.189 84);
  --finding-info: oklch(0.696 0.17 162);
  --finding-human: oklch(0.627 0.265 303);
}
```

### Persistence

Store preference in Tauri app data directory via `@tauri-apps/api/path` + `@tauri-apps/plugin-store`. Persists across sessions.

```typescript
// theme: 'light' | 'dark' | 'system'
// Stored at: $APPDATA/noslop/settings.json
```

---

## 12. Session Persistence

### Two Categories of State

| Category             | Storage                          | Examples                                                           |
| -------------------- | -------------------------------- | ------------------------------------------------------------------ |
| **Domain state**     | Review JSON (`.noslop/reviews/`) | Finding statuses, viewed files, review status, dismiss reasons     |
| **UI session state** | `localStorage` / Tauri app data  | Selected review, scroll positions, expanded findings, panel widths |

### Domain State (Persisted via Tauri Commands)

These are part of the `Review` domain model and saved to the review JSON file:

- `finding.status` (open / resolved / dismissed)
- `finding.dismiss_reason`
- `finding.resolution_reason`
- `finding.notes`
- `review.viewed_files: string[]`
- `review.status` (open / closed)

### UI Session State

```typescript
// gui/src/lib/session.ts

export interface SessionState {
  selected_review_id: string | null;
  scroll_positions: Record<string, number>; // review_id -> scrollTop
  expanded_finding_ids: Set<string>;
  sidebar_width: number;
  sidebar_collapse_state: "full" | "mini" | "hidden";
  file_tree_collapsed_dirs: Set<string>;
  active_filters: {
    status: FindingStatus | "all";
    severity: Severity | "all";
    source: FindingSourceKind | "all";
  };
  diff_view_mode: "split" | "unified";
  theme: "light" | "dark" | "system";
  review_list_collapsed: boolean;
  findings_panel_collapsed: boolean;
}
```

### Persistence Strategy

- **Write**: Debounced (500ms) write to `localStorage` on any session state change
- **Read**: On app launch, restore from `localStorage` silently (VS Code pattern)
- **No "restore?" dialog**: State is restored without asking. The app looks exactly as the user left it
- **Stale review handling**: If the selected review no longer exists (deleted), clear selection and show review list

### Svelte Store Implementation

```typescript
// gui/src/lib/session.ts

import { browser } from "$app/environment";

const STORAGE_KEY = "noslop-session";

function loadSession(): SessionState {
  if (!browser) return defaultSession();
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return defaultSession();
  try {
    return JSON.parse(raw);
  } catch {
    return defaultSession();
  }
}

// Reactive state with auto-persist
let session = $state<SessionState>(loadSession());

$effect(() => {
  if (browser) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
  }
});
```

---

## 13. P2 Differentiator Features

### 13.1 Floating Comments Mode

Toggle between inline expansion (default, for triage) and floating margin comments (for reading flow).

```
Inline Mode (default):                    Floating Mode (toggle with m):
┌────────────────────────┐                ┌──────────────────┬─────────────┐
│ 10  fn validate() {    │                │ 10  fn validate() │ ● JWT secre │
│ 11    if token.empty() │                │ 11    if token.em │   agent:sec │
│ ┌──────────────────┐   │                │ 12    ...         │             │
│ │ ● JWT secret ... │   │                │ 13    ...        ─┤─ ▲ Consider │
│ │ [Resolve] [...]  │   │                │ 14    ...         │   agent:qua │
│ └──────────────────┘   │                │                   │             │
│ 12    ...              │                └──────────────────┴─────────────┘
└────────────────────────┘
```

Floating margin: 280px wide. Comments anchored to code lines with thin connector lines. Displacement algorithm prevents overlap (stack vertically with 4px gap).

Keyboard toggle: `m`.

### 13.2 Suggestion Apply

Finding suggestions with `original` + `replacement` render as mini-diffs in the finding card.

```
Suggestion:
┌────────────────────────────────────────────────────┐
│ - let secret = env::var("JWT_SECRET")?;            │
│ + let secret = env::var("JWT_SECRET")              │
│ +     .expect("JWT_SECRET must be set");           │
│                                                    │
│ [Apply]  [Edit]                                    │
└────────────────────────────────────────────────────┘
```

| Action       | Behavior                                                                                                                                                                                      |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Apply**    | Calls `apply_suggestion` Tauri command. Validates span against current file content. Writes replacement. Re-computes diff. Auto-resolves finding with `resolution_reason: suggestion_applied` |
| **Edit**     | Opens `replacement` in editable textarea. User modifies, then clicks "Apply Edited"                                                                                                           |
| **Conflict** | If file changed since review started (span doesn't match), show error: "File has changed. Suggestion may not apply cleanly."                                                                  |

Batch: "Fix all auto-fixable" button applies all suggestions that have no conflicts.

Conflict detection via `Span::overlaps()` in Rust backend.

### 13.3 Version Re-Review

When the branch HEAD moves beyond the review's recorded head SHA:

```rust
// Rust domain model extension
pub struct ReviewVersion {
    pub head: String,                      // SHA at this version
    pub created_at: String,
    pub finding_snapshot: Vec<String>,     // finding IDs at this version
}

// Review extension
pub struct Review {
    // ... existing fields
    pub versions: Vec<ReviewVersion>,
}
```

UI behavior:

- Yellow banner: "Branch updated. 2 new commits since last review."
- "Show only new changes" button filters diff to hunks not present in previous version
- Files unchanged since last version: collapsed with "Previously reviewed" badge
- Stale findings (on lines that changed): dimmed, not auto-resolved

### 13.4 Auto-Resolve

Trigger: When code changes and a finding's span overlaps with a changed hunk, the finding is auto-resolved.

Rules:

- Never auto-resolve `FindingSource::Human` findings
- Show as "Auto-resolved (code changed)" with dimmed styling
- "Re-open" button available
- No confirmation dialog
- `resolution_reason: code_changed`

### 13.5 Confidence Indicators

AI findings include a `confidence: Option<f32>` field (0.0 to 1.0).

| Range      | Behavior                                                              |
| ---------- | --------------------------------------------------------------------- |
| 0.8 - 1.0  | Full gutter icon opacity. Sorted first                                |
| 0.4 - 0.79 | Reduced gutter icon opacity (proportional). Normal position           |
| Below 0.4  | Hidden from main view. Moved to collapsed "Other suggestions" section |

Confidence is NEVER displayed as a number. It affects sort order and visual emphasis only.

### 13.6 Learning from Dismissals

Rule-based suppression:

```
IF same FindingSource + same message pattern dismissed >= 3 times
THEN suppress future matches from that source
```

Storage: `.noslop/preferences.toml`

```toml
[[suppress]]
source = "agent:quality"
pattern = "Consider adding.*documentation"
count = 5
last_dismissed = "2026-02-15T10:30:00Z"
```

Constraints:

- Never suppress `severity = Block`
- Transparency: "N findings suppressed" indicator at bottom of findings panel
- Management UI: Dialog listing all suppression rules with "Remove" buttons

---

## 14. Component Architecture

### Full Component Tree

```
gui/src/lib/
  types.ts                                 -- All TypeScript types (extended)
  api.ts                                   -- Tauri IPC functions (extended)
  queries.ts                               -- @tanstack/svelte-query hooks (extended)
  session.ts                               -- Session persistence (localStorage)

  stores/
    review-navigation.svelte.ts            -- Current file, viewed set, sort, filter
    keyboard.svelte.ts                     -- Shortcut registration and focus zones
    command-registry.ts                    -- Command palette command definitions

  views/
    AppShell.svelte                        -- Top-level layout: StatusBar + Sidebar + Main + ActionBar
    StatusBar.svelte                       -- Sticky top bar, compact/expanded modes
    ActionBar.svelte                       -- Sticky bottom bar: close button + finding counts

    ReviewLandingPage.svelte               -- Verdict, stats, risk areas, adaptive CTA
    ReviewProgress.svelte                  -- Dual-axis progress bar (viewed + resolved)

    DiffView.svelte                        -- @git-diff-view/svelte wrapper, continuous scroll
    DiffFileHeader.svelte                  -- Sticky file header (path, +/-, badges, viewed)
    DiffContextExpander.svelte             -- "Show N more lines" between hunks

    FileTree.svelte                        -- Recursive tree container with search + sort
    FileTreeNode.svelte                    -- Single node (file or directory) with badges

    FindingsPanel.svelte                   -- Filterable findings list (sidebar section)
    FindingCard.svelte                     -- Two-state card (collapsed/expanded)
    FindingGutterIcon.svelte               -- Colored severity dot for diff gutter
    InlineFindingCard.svelte               -- Finding card rendered inline below diff line
    FindingHoverPreview.svelte             -- Popover on gutter icon hover
    AddFindingForm.svelte                  -- Severity selector + message + suggestion
    FindingNotes.svelte                    -- Notes textarea per finding

    CloseReviewDialog.svelte               -- Confirmation dialog with structured summary
    BlockedCloseDialog.svelte              -- Interstitial listing all blockers
    NewReviewDialog.svelte                 -- Advanced review creation (branch pickers)

    CommandPalette.svelte                  -- Cmd+K fuzzy search overlay
    ShortcutOverlay.svelte                 -- ? keyboard reference sheet

  components/ui/
    (existing shadcn components: badge, button, card, dialog, scroll-area, separator, tabs)
    command/                               -- Command palette primitive (shadcn-svelte)
    dropdown-menu/                         -- Dismiss reason picker, sort dropdown
    resizable-panel/                       -- Drag-to-resize panel splitter
    tooltip/                               -- Tooltips for icon buttons
    popover/                               -- Finding hover preview
    toggle-group/                          -- Filter segmented controls
    progress/                              -- Progress bars
```

### Component Responsibilities

| Component           | Props                                 | Key State                           | Emits                            |
| ------------------- | ------------------------------------- | ----------------------------------- | -------------------------------- |
| `AppShell`          | --                                    | `selectedReviewId`, sidebar state   | --                               |
| `StatusBar`         | `review`, `isCompact`                 | --                                  | --                               |
| `ActionBar`         | `review`                              | --                                  | `onClose`                        |
| `ReviewLandingPage` | `review`, `diff`                      | --                                  | `onClose`, `onScrollToBlocker`   |
| `DiffView`          | `reviewId`, `diff`, `findings`        | `scrollPosition`, `focusedLine`     | `onFindingClick`, `onAddFinding` |
| `DiffFileHeader`    | `fileDiff`, `findingCounts`, `viewed` | `sticky`                            | `onToggleViewed`                 |
| `FileTree`          | `files`, `findings`, `viewedFiles`    | `filterText`, `sortMode`            | `onFileSelect`                   |
| `FileTreeNode`      | `entry`, `depth`, `selected`          | `expanded`                          | `onSelect`                       |
| `FindingsPanel`     | `findings`                            | `filters`                           | `onFindingClick`                 |
| `FindingCard`       | `finding`, `reviewId`                 | `expanded`                          | `onResolve`, `onDismiss`         |
| `FindingGutterIcon` | `severity`, `count`, `confidence`     | --                                  | `onClick`, `onHover`             |
| `AddFindingForm`    | `filePath`, `span`                    | `severity`, `message`, `suggestion` | `onSubmit`, `onCancel`           |
| `CommandPalette`    | `commands`                            | `query`, `selectedIndex`            | `onExecute`                      |

### Store Definitions

```typescript
// gui/src/lib/stores/review-navigation.svelte.ts

export function createReviewNavigation(reviewId: string) {
  let currentFilePath = $state<string | null>(null);
  let currentFindingId = $state<string | null>(null);
  let viewedFiles = $state<Set<string>>(new Set());
  let sortMode = $state<"findings" | "alphabetical">("findings");
  let filterText = $state("");
  let focusZone = $state<"tree" | "diff" | "finding" | "dialog">("tree");

  // Navigation actions
  function nextFile(files: FileDiff[]) {
    /* ... */
  }
  function prevFile(files: FileDiff[]) {
    /* ... */
  }
  function nextFinding(findings: Finding[]) {
    /* ... */
  }
  function prevFinding(findings: Finding[]) {
    /* ... */
  }
  function nextUnresolved(findings: Finding[]) {
    /* ... */
  }
  function toggleViewed(path: string) {
    /* ... */
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
    // ... etc
  };
}
```

```typescript
// gui/src/lib/stores/keyboard.svelte.ts

export function createKeyboardManager() {
  let focusZone = $state<"tree" | "diff" | "finding" | "dialog">("tree");

  function isTextInput(): boolean {
    const tag = document.activeElement?.tagName;
    return tag === "INPUT" || tag === "TEXTAREA";
  }

  function handleKeydown(e: KeyboardEvent) {
    // Modifier shortcuts always work
    if (e.metaKey || e.ctrlKey) {
      if (e.key === "k") {
        openCommandPalette();
        e.preventDefault();
      }
      if (e.key === "p") {
        openFileJump();
        e.preventDefault();
      }
      return;
    }

    // Single-key shortcuts disabled in text inputs
    if (isTextInput()) return;

    switch (e.key) {
      case "]":
        nextFile();
        break;
      case "[":
        prevFile();
        break;
      case "j":
        nextFinding();
        break;
      case "k":
        prevFinding();
        break;
      case "n":
        nextUnresolved();
        break;
      case "p":
        prevUnresolved();
        break;
      case "r":
        resolveFocused();
        break;
      case "d":
        dismissFocused();
        break;
      case "c":
        addFindingOnLine();
        break;
      case "v":
        toggleViewed();
        break;
      case "f":
        cycleSidebar();
        break;
      case "s":
        toggleDiffMode();
        break;
      case "w":
        toggleWhitespace();
        break;
      case "m":
        toggleCommentMode();
        break;
      case "Enter":
        expandFocused();
        break;
      case "Escape":
        collapseFocused();
        break;
      case "?":
        showShortcuts();
        break;
    }
  }

  return { handleKeydown, focusZone };
}
```

---

## 15. Backend Changes Required

### New Tauri Commands

| Command               | Signature                                                                 | Description                                                    |
| --------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `get_structured_diff` | `(base: String, head: String) -> StructuredDiff`                          | Parsed diff with hunks, char-level changes, language detection |
| `dismiss_finding`     | `(review_id: String, finding_id: String, reason: String) -> ()`           | Dismiss with mandatory reason                                  |
| `apply_suggestion`    | `(review_id: String, finding_id: String) -> ()`                           | Validate span, write file, re-compute diff                     |
| `reopen_review`       | `(id: String) -> ()`                                                      | Transition closed review back to open                          |
| `get_current_branch`  | `() -> String`                                                            | Current git branch name                                        |
| `get_branches`        | `() -> Vec<String>`                                                       | List of local branch names                                     |
| `get_merge_base`      | `(branch: String) -> String`                                              | Merge-base SHA of branch with main                             |
| `get_file_content`    | `(path: String, commit: String, start: u32, end: u32) -> String`          | File content range for context expansion                       |
| `add_finding_note`    | `(review_id: String, finding_id: String, content: String) -> FindingNote` | Add note to finding                                            |
| `mark_file_viewed`    | `(review_id: String, path: String) -> ()`                                 | Toggle file viewed status                                      |

### Updated Tauri Command Registrations

```rust
// gui/src-tauri/src/lib.rs

.invoke_handler(tauri::generate_handler![
    // Existing
    commands::list_reviews,
    commands::get_review,
    commands::start_review,
    commands::add_finding,
    commands::resolve_finding,
    commands::close_review,
    // New
    commands::get_structured_diff,       // replaces get_diff
    commands::dismiss_finding,
    commands::apply_suggestion,
    commands::reopen_review,
    commands::get_current_branch,
    commands::get_branches,
    commands::get_merge_base,
    commands::get_file_content,
    commands::add_finding_note,
    commands::mark_file_viewed,
])
```

### New Cargo Dependencies

```toml
# gui/src-tauri/Cargo.toml additions

[dependencies]
similar = "2"                            # Character-level diff computation
syntect = { version = "5", default-features = false, features = ["default-syntaxes", "default-themes", "parsing"] }
                                         # Syntax highlighting (server-side)
tauri-plugin-store = "2"                 # Persistent key-value storage (theme, settings)
glob-match = "0.2"                       # Glob pattern matching for risk areas
```

### Domain Model Extensions

```rust
// src/core/models/finding.rs additions

/// Why a finding was dismissed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissReason {
    FalsePositive,
    WontFix,
    NotApplicable,
    InvestigateLater,
}

/// Why a finding was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionReason {
    Manual,
    SuggestionApplied,
    CodeChanged,
    FileRemoved,
}

/// A structured suggestion with original + replacement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub replacement: String,
    pub original: Option<String>,
    pub edited: bool,
}

/// A note on a finding (lightweight, no threads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingNote {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

// Finding struct extensions (add fields):
pub struct Finding {
    // ... existing fields
    pub dismiss_reason: Option<DismissReason>,
    pub resolution_reason: Option<ResolutionReason>,
    pub confidence: Option<f32>,
    pub notes: Vec<FindingNote>,
}
```

```rust
// src/core/models/review.rs additions

/// A snapshot of the review at a specific branch HEAD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewVersion {
    pub head: String,
    pub created_at: String,
    pub finding_ids: Vec<String>,
}

// Review struct extensions (add fields):
pub struct Review {
    // ... existing fields
    pub branch: Option<String>,
    pub viewed_files: Vec<String>,
    pub versions: Vec<ReviewVersion>,
}
```

### Updated DTO Layer

```rust
// gui/src-tauri/src/dto.rs extensions

#[derive(Debug, Serialize)]
pub struct FindingDto {
    // ... existing fields
    pub dismiss_reason: Option<String>,
    pub resolution_reason: Option<String>,
    pub confidence: Option<f32>,
    pub notes: Vec<FindingNoteDto>,
    pub suggestion: Option<SuggestionDto>,
    pub target: TargetDto,                   // structured, not flattened string
}

#[derive(Debug, Serialize)]
pub struct TargetDto {
    pub path: String,
    pub span: Option<SpanDto>,
    pub commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpanDto {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Serialize)]
pub struct SuggestionDto {
    pub replacement: String,
    pub original: Option<String>,
    pub edited: bool,
}

#[derive(Debug, Serialize)]
pub struct FindingNoteDto {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewDto {
    // ... existing fields
    pub branch: Option<String>,
    pub viewed_files: Vec<String>,
}
```

### Config Additions

```toml
# .noslop.toml additions

[review]
close_policy = "normal"           # "strict" | "normal" | "lenient"
auto_archive_days = 30            # Auto-archive closed reviews after N days

[risk]
patterns = [
  { glob = "src/**/auth*", label = "Authentication" },
  { glob = "**/migrations/**", label = "Database migrations" },
  { glob = ".github/workflows/**", label = "CI/CD" },
  { glob = "**/Cargo.toml", label = "Dependencies" },
]
```

---

## 16. New Frontend Dependencies

| Package                 | Version | Size                  | Purpose                         |
| ----------------------- | ------- | --------------------- | ------------------------------- |
| `@git-diff-view/svelte` | latest  | ~55KB gzipped         | Diff rendering component        |
| `@git-diff-view/shiki`  | latest  | ~15KB + lang grammars | Syntax highlighting integration |

No other new runtime dependencies. The existing stack (bits-ui, @tanstack/svelte-query, @tauri-apps/api, @lucide/svelte) covers all other needs.

### shadcn-svelte Components to Add

Install via `npx shadcn-svelte@latest add <component>`:

| Component          | Used For                                                  |
| ------------------ | --------------------------------------------------------- |
| `command`          | Command palette (Cmd+K)                                   |
| `dropdown-menu`    | Dismiss reason picker, sort dropdown, file header actions |
| `tooltip`          | Icon button labels, finding preview                       |
| `popover`          | Finding hover preview on gutter icon                      |
| `toggle-group`     | Filter segmented controls (status, severity)              |
| `progress`         | File viewed / findings resolved progress bars             |
| `alert`            | Warning banners (stale review, branch updated)            |
| `skeleton`         | Loading states for landing page, findings                 |
| `sonner` / `toast` | "Review closed" notification, "Push command copied"       |

---

## 17. Implementation Priority

### Phase 1: Core Review Experience (P0)

Must ship together. This is the minimum viable review workstation.

| #   | Task                      | Components                                              | Backend                                   |
| --- | ------------------------- | ------------------------------------------------------- | ----------------------------------------- |
| 1   | Application shell         | `AppShell`, `StatusBar`, `ActionBar`, `ResizablePanel`  | --                                        |
| 2   | Review landing page       | `ReviewLandingPage`, `ReviewProgress`                   | --                                        |
| 3   | Structured diff rendering | `DiffView`, `DiffFileHeader`, `DiffContextExpander`     | `get_structured_diff`, `get_file_content` |
| 4   | File tree sidebar         | `FileTree`, `FileTreeNode`                              | --                                        |
| 5   | Inline findings on diff   | `FindingGutterIcon`, `InlineFindingCard`, `FindingCard` | --                                        |
| 6   | Keyboard navigation       | `keyboard.svelte.ts`, `review-navigation.svelte.ts`     | --                                        |
| 7   | Close review flow         | `CloseReviewDialog`, `BlockedCloseDialog`               | `close_review` (updated), `reopen_review` |

### Phase 2: Power User Features (P1)

Each ships independently. No ordering dependencies.

| #   | Task                         | Components                                          | Backend                                                |
| --- | ---------------------------- | --------------------------------------------------- | ------------------------------------------------------ |
| 8   | Command palette              | `CommandPalette`, `command-registry.ts`, `command/` | --                                                     |
| 9   | Human finding creation       | `AddFindingForm`                                    | `add_finding` (updated with span)                      |
| 10  | Viewed checkmarks + progress | `ReviewProgress`, file header checkmarks            | `mark_file_viewed`                                     |
| 11  | Finding filters              | `FindingsPanel` with segmented controls             | --                                                     |
| 12  | Dark mode toggle             | Theme store, CSS variables, Sun/Moon button         | --                                                     |
| 13  | Session persistence          | `session.ts`                                        | --                                                     |
| 14  | Dismiss flow with reason     | Dismiss dropdown in `FindingCard`                   | `dismiss_finding`                                      |
| 15  | Finding notes                | `FindingNotes` textarea                             | `add_finding_note`                                     |
| 16  | New review zero-config       | `NewReviewDialog`                                   | `get_current_branch`, `get_branches`, `get_merge_base` |

### Phase 3: Differentiators (P2)

Each ships independently. Order reflects estimated value.

| #   | Task                        | Components                                            | Backend                       |
| --- | --------------------------- | ----------------------------------------------------- | ----------------------------- |
| 17  | Suggestion apply            | Mini-diff in `FindingCard`, Apply/Edit buttons        | `apply_suggestion`            |
| 18  | Auto-resolve on code change | Auto-resolve logic in review navigation store         | Span overlap detection        |
| 19  | Version re-review           | Banner, "show new changes only" filter                | `ReviewVersion` model         |
| 20  | Floating comments mode      | `FloatingFindingCard`, margin layout, connector lines | --                            |
| 21  | Batch fix                   | "Fix all auto-fixable" button                         | Batch `apply_suggestion`      |
| 22  | Shortcut overlay            | `ShortcutOverlay`                                     | --                            |
| 23  | Learning from dismissals    | Suppression rules, management dialog                  | `.noslop/preferences.toml`    |
| 24  | Confidence-based sorting    | Gutter opacity, "Other suggestions" collapse          | `confidence` field on Finding |

---

## 18. Extended API Layer

### Updated `api.ts`

```typescript
// gui/src/lib/api.ts (complete)

import { invoke } from "@tauri-apps/api/core";
import type { Review, Finding, FindingNote, StructuredDiff } from "./types";

export const api = {
  // Reviews
  listReviews: (openOnly: boolean) =>
    invoke<Review[]>("list_reviews", { openOnly }),
  getReview: (id: string) => invoke<Review>("get_review", { id }),
  startReview: (base: string, head: string) =>
    invoke<Review>("start_review", { base, head }),
  closeReview: (id: string) => invoke<void>("close_review", { id }),
  reopenReview: (id: string) => invoke<void>("reopen_review", { id }),

  // Diff
  getStructuredDiff: (base: string, head: string) =>
    invoke<StructuredDiff>("get_structured_diff", { base, head }),
  getFileContent: (
    path: string,
    commit: string,
    startLine: number,
    endLine: number,
  ) => invoke<string>("get_file_content", { path, commit, startLine, endLine }),

  // Findings
  addFinding: (
    reviewId: string,
    target: string,
    message: string,
    severity?: string,
    startLine?: number,
    endLine?: number,
  ) =>
    invoke<Finding>("add_finding", {
      reviewId,
      target,
      message,
      severity,
      startLine,
      endLine,
    }),
  resolveFinding: (reviewId: string, findingId: string) =>
    invoke<void>("resolve_finding", { reviewId, findingId }),
  dismissFinding: (reviewId: string, findingId: string, reason: string) =>
    invoke<void>("dismiss_finding", { reviewId, findingId, reason }),
  applySuggestion: (reviewId: string, findingId: string) =>
    invoke<void>("apply_suggestion", { reviewId, findingId }),
  addFindingNote: (reviewId: string, findingId: string, content: string) =>
    invoke<FindingNote>("add_finding_note", { reviewId, findingId, content }),

  // Files
  markFileViewed: (reviewId: string, path: string) =>
    invoke<void>("mark_file_viewed", { reviewId, path }),

  // Git
  getCurrentBranch: () => invoke<string>("get_current_branch"),
  getBranches: () => invoke<string[]>("get_branches"),
  getMergeBase: (branch: string) =>
    invoke<string>("get_merge_base", { branch }),
};
```

### Updated `queries.ts`

```typescript
// gui/src/lib/queries.ts (additions)

export function useStructuredDiff(base: string, head: string) {
  return createQuery({
    queryKey: ["structured-diff", base, head],
    queryFn: () => api.getStructuredDiff(base, head),
    enabled: !!base && !!head,
    staleTime: 1000 * 60 * 5, // diffs don't change often
  });
}

export function useCurrentBranch() {
  return createQuery({
    queryKey: ["current-branch"],
    queryFn: () => api.getCurrentBranch(),
    staleTime: 1000 * 10,
  });
}

export function useBranches() {
  return createQuery({
    queryKey: ["branches"],
    queryFn: () => api.getBranches(),
    staleTime: 1000 * 30,
  });
}

export function useDismissFinding() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: {
      reviewId: string;
      findingId: string;
      reason: string;
    }) => api.dismissFinding(params.reviewId, params.findingId, params.reason),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ["review", reviewId] });
    },
  });
}

export function useApplySuggestion() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; findingId: string }) =>
      api.applySuggestion(params.reviewId, params.findingId),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ["review", reviewId] });
      client.invalidateQueries({ queryKey: ["structured-diff"] });
    },
  });
}

export function useReopenReview() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (id: string) => api.reopenReview(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ["reviews"] });
    },
  });
}

export function useMarkFileViewed() {
  const client = useQueryClient();
  return createMutation({
    mutationFn: (params: { reviewId: string; path: string }) =>
      api.markFileViewed(params.reviewId, params.path),
    onSuccess: (_, { reviewId }) => {
      client.invalidateQueries({ queryKey: ["review", reviewId] });
    },
  });
}
```

---

## Appendix A: Migration from Current GUI

The current GUI (`+page.svelte`) has these components that will be replaced:

| Current                                 | Replacement                                                             | Notes                                                |
| --------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------- |
| `+page.svelte` (inline layout)          | `AppShell.svelte`                                                       | Full layout with StatusBar, Sidebar, Main, ActionBar |
| `ReviewList.svelte`                     | Collapsible section in `AppShell` sidebar                               | Same data, different container                       |
| `DiffView.svelte` (shows finding cards) | `DiffView.svelte` (renders actual diff) + `FindingCard.svelte` (inline) | Complete rewrite                                     |
| `FindingCard.svelte` (basic card)       | `FindingCard.svelte` (two-state with actions)                           | Extended, not replaced                               |
| New Review Dialog (SHA text inputs)     | `NewReviewDialog.svelte` (branch pickers)                               | Rewritten                                            |
| `api.getDiff` (raw string)              | `api.getStructuredDiff` (structured JSON)                               | New command                                          |

### Breaking Changes

- `Finding.target` changes from `string` to `Target` object (structured with path, span, commit)
- `Finding.source` changes from `string` to `FindingSource` object (structured with kind, name)
- `Finding.suggestion` changes from `string | null` to `Suggestion | null` (structured with original, replacement, edited)
- `api.getDiff` removed, replaced by `api.getStructuredDiff`

The DTO layer in `gui/src-tauri/src/dto.rs` must be updated to serialize these structured types instead of flattening to strings.
