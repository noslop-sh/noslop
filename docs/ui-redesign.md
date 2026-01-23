# UI Redesign Plan: Work Management Dashboard

## Overview

This document captures the complete UI redesign plan for noslop. The goal is to transform the generic kanban board into a purpose-built work management dashboard that reflects noslop's core value proposition: verification-aware task tracking with scope-based check associations.

## Core Insight

The UI is the **human command center** for directing agent work and monitoring progress:
- **Agent** → uses CLI for execution
- **Human** → uses UI to plan, organize, and monitor

---

## Design Decisions

### 1. Check-Task Association
Checks are derived from **aggregated scope overlap**:
- Task's effective scope = `task.scope ∪ topics[*].scope`
- `task.scope`: Files the task directly touches (auto-enriched from commits)
- `topics[*].scope`: Scope patterns from all attached topics
- A check applies to a task if `check.scope` overlaps with the task's effective scope

### 2. Git Visibility
Minimal - branch level only. Task detail shows branch name, no commit history in UI.

### 3. No Sidebar
Cleaner layout with:
- Topic filter in toolbar dropdown
- Checks/Topics accessed via dedicated views

---

## New Layout

### Main View: Task List (Grouped by Status)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ noslop                              branch: feature-xyz         [live]   │
├──────────────────────────────────────────────────────────────────────────┤
│ [Topic: All ▾]   [+ New Task]              [Checks]  [Topics]        │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│ ═══ IN PROGRESS ═════════════════════════════════════════════════════   │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TSK-3  Implement auth flow                              ● active   │  │
│ │        [Auth] [API]                           checks: 1/2 ✓        │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ═══ PENDING ═════════════════════════════════════════════════════════   │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TSK-5  Add rate limiting                    p1    checks: 0/1      │  │
│ │        [API]                                                       │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TSK-7  Write API tests                      p2    (no checks)      │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ═══ BLOCKED ═════════════════════════════════════════════════════════   │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TSK-6  Deploy to staging                    blocked by: TSK-3      │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ═══ DONE ════════════════════════════════════════════════════════════   │
│ │ TSK-1  Setup project                        ✓                      │  │
│ │ TSK-2  Initial schema                       ✓                      │  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Toolbar Elements

| Element | Description |
|---------|-------------|
| Topic filter | Dropdown to filter tasks by topic (default: "All") |
| + New Task | Create task button |
| [Checks] | Navigate to Checks view (CRUD) |
| [Topics] | Navigate to Topics view (CRUD) |

### Task Card Elements

| Element | Description |
|---------|-------------|
| ID + Title | e.g., "TSK-3 Implement auth flow" |
| Topic tags | Badges showing attached topics: [Auth] [API] |
| Check counter | "checks: 1/2 ✓" (1 verified, 2 total) or "(no checks)" |
| Priority | p1, p2, etc. (for pending tasks) |
| Blocked by | Shows blocker if blocked |

---

## Task Detail Modal

Click a task to open detail modal with full info:

```
┌─ TSK-3: Implement auth flow ─────────────────────────────────┐
│                                                               │
│ Status: in_progress (active)                                  │
│ Priority: p1                                                  │
│ Branch: feature/auth                                          │
│                                                               │
│ ─── TOPICS ────────────────────────────────────────────── │
│ [Auth ×] [API ×]                           [+ Add topic]   │
│                                                               │
│ ─── SCOPE (auto-enriched from commits) ─────────────────── │
│ src/auth.rs                                                   │
│ src/login.rs                                                  │
│ [+ Add scope]                                                 │
│                                                               │
│ ─── CHECKS (from scope + topic overlap) ──────────────── │
│ CHK-1  Security review required    block   ○ pending         │
│        "Any auth changes need security sign-off"             │
│ CHK-2  Test coverage check         warn    ● verified        │
│        "Ensure >80% coverage"                                │
│                                                               │
│ ─── BLOCKERS ────────────────────────────────────────────── │
│ None                                                          │
│                                                               │
│ ─── DESCRIPTION ─────────────────────────────────────────── │
│ [Click to edit...]                                           │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

---

## Checks View (Dedicated Page)

Accessed via toolbar [Checks] button. Full CRUD:

```
┌─ CHECKS ─────────────────────────────────────────────────────────────────┐
│                                                                          │
│ [← Back to Tasks]                                    [+ New Check]       │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ CHK-1  Security review required                          [block]   │  │
│ │        scope: src/auth/**                                          │  │
│ │        "Any auth changes need security sign-off"                   │  │
│ │                                              [Edit] [Delete]       │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ CHK-2  Test coverage check                               [warn]    │  │
│ │        scope: **/*.rs                                              │  │
│ │        "Ensure >80% test coverage for all Rust code"               │  │
│ │                                              [Edit] [Delete]       │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Topics View (Dedicated Page)

Accessed via toolbar [Topics] button. Full CRUD:

```
┌─ TOPICS ───────────────────────────────────────────────────────────────┐
│                                                                          │
│ [← Back to Tasks]                                   [+ New Topic]      │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TOP-1  Authentication                                              │  │
│ │        scope: src/auth/**, src/middleware/auth.rs                  │  │
│ │        "User login, session management, OAuth integration"         │  │
│ │                                              [Edit] [Delete]       │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐  │
│ │ TOP-2  API                                                         │  │
│ │        scope: src/api/**                                           │  │
│ │        "REST API endpoints and handlers"                           │  │
│ │                                              [Edit] [Delete]       │  │
│ └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: API Enhancement ✅ COMPLETED

- [x] Add endpoint to compute check-task association from aggregated scope
- [x] Return check status (verified/pending count) per task
- [x] Ensure all endpoints return scope fields
- [x] Add `scope` field to `ConceptInfo`
- [x] Add `check_count`, `checks_verified`, `checks` fields to `TaskItem` and `TaskDetailData`
- [x] Implement `compute_task_checks()` helper in handlers

### Phase 2: Frontend Stack Migration 🔄 IN PROGRESS

- [ ] Set up Svelte + Vite for frontend
- [ ] Install and configure shadcn-svelte
- [ ] Set up build pipeline to embed built assets in Rust binary
- [ ] Update tiny_http server to serve Svelte app

### Phase 3: Main View Implementation

- [ ] Create main layout (no sidebar)
- [ ] Build toolbar with topic filter dropdown + navigation buttons
- [ ] Build grouped task list (IN PROGRESS / PENDING / BLOCKED / DONE)
- [ ] Task cards with topic tags + check counter
- [ ] Implement drag-and-drop between status groups

### Phase 4: Task Detail Modal

- [ ] Topics section with add/remove
- [ ] Scope section (read-only, auto-enriched)
- [ ] Checks section showing actual checks with verification status
- [ ] Blockers section
- [ ] Description editing

### Phase 5: Checks View

- [ ] New page for check CRUD
- [ ] List, Create, Edit, Delete
- [ ] Navigation via toolbar button

### Phase 6: Topics View

- [ ] New page for topic CRUD
- [ ] List, Create, Edit, Delete (including scope management)
- [ ] Navigation via toolbar button

### Phase 7: Polish

- [ ] Keyboard navigation (vim-style hjkl)
- [ ] Real-time updates via long-polling
- [ ] Responsive layout
- [ ] Dark theme refinement

---

## API Endpoints (Current)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/status` | Overall status (branch, task counts, check count) |
| GET | `/api/v1/tasks` | List all tasks with check counts |
| GET | `/api/v1/tasks/:id` | Get task detail with full check list |
| POST | `/api/v1/tasks` | Create task |
| PATCH | `/api/v1/tasks/:id` | Update task (description, topics) |
| DELETE | `/api/v1/tasks/:id` | Delete task |
| POST | `/api/v1/tasks/:id/start` | Start task |
| POST | `/api/v1/tasks/:id/done` | Complete task |
| POST | `/api/v1/tasks/:id/reset` | Reset to pending |
| POST | `/api/v1/tasks/:id/backlog` | Move to backlog |
| POST | `/api/v1/tasks/:id/block` | Add blocker |
| POST | `/api/v1/tasks/:id/unblock` | Remove blocker |
| POST | `/api/v1/tasks/:id/link-branch` | Link/unlink branch |
| GET | `/api/v1/checks` | List all checks |
| POST | `/api/v1/checks` | Create check |
| GET | `/api/v1/topics` | List all topics with scope |
| POST | `/api/v1/topics` | Create topic |
| PATCH | `/api/v1/topics/:id` | Update topic |
| DELETE | `/api/v1/topics/:id` | Delete topic |
| POST | `/api/v1/topics/select` | Select current topic filter |
| GET | `/api/v1/events` | Long-polling for changes |

---

## Data Models (API Response Types)

### TaskItem (list view)
```typescript
interface TaskItem {
  id: string;
  title: string;
  description?: string;
  status: 'backlog' | 'pending' | 'in_progress' | 'done';
  priority: string;
  blocked_by: string[];
  current: boolean;
  blocked: boolean;
  branch?: string;
  started_at?: string;
  completed_at?: string;
  topics: string[];
  scope: string[];
  check_count: number;
  checks_verified: number;
}
```

### TaskDetailData (detail view)
```typescript
interface TaskDetailData extends TaskItem {
  created_at: string;
  notes?: string;
  checks: TaskCheckItem[];
}

interface TaskCheckItem {
  id: string;
  message: string;
  severity: 'block' | 'warn' | 'info';
  verified: boolean;
}
```

### TopicInfo
```typescript
interface TopicInfo {
  id: string;
  name: string;
  description?: string;
  scope: string[];
  task_count: number;
  created_at: string;
}
```

### CheckItem
```typescript
interface CheckItem {
  id: string;
  scope: string;
  message: string;
  severity: 'block' | 'warn' | 'info';
}
```

---

## Files Modified (Phase 1)

- `src/api/types.rs` - Added scope to TopicInfo, check counts to TaskItem/TaskDetailData
- `src/api/handlers.rs` - Added compute_task_checks(), updated list_tasks_filtered(), get_task(), list_topics()
- `src/storage/refs.rs` - Added scope field to TaskRef with CRUD operations
- `src/noslop_file.rs` - Added scope field to TopicEntry

---

## Tech Stack (Phase 2+)

- **Frontend**: Svelte 5 + TypeScript
- **UI Components**: shadcn-svelte
- **Styling**: Tailwind CSS
- **Build Tool**: Vite
- **Backend**: Rust (tiny_http server, embedded static files)
- **Real-time**: Long-polling via `/api/v1/events`
