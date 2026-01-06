# Story 05: Local Web UI

## Summary

A locally-hosted web UI (`noslop ui`) that provides human visibility into agent work. View branch hierarchy, manage tasks, see check status, and monitor progress.

## Motivation

**Current problem:**
- CLI is great for agents, harder for human oversight
- No visual representation of work structure
- Hard to see progress at a glance

**Solution:**
- Local web server with simple UI
- Tree view of branches and tasks
- Real-time updates as agent works
- Human can manage tasks visually

## User Stories

### US-05.1: Start local UI
**As a** developer
**I want** to start a local web UI
**So that** I can manage tasks visually

```bash
$ noslop ui
Starting noslop UI...
Open http://localhost:9999 in your browser

Press Ctrl+C to stop
```

### US-05.2: View branch tree
**As a** developer
**I want** to see all branches as a tree
**So that** I understand the work structure

```
┌─────────────────────────────────────────────────────────────┐
│  noslop                                    localhost:9999   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  BRANCHES                                                   │
│  ─────────                                                  │
│  ▼ main                                                     │
│    ▼ feature/oauth (3 tasks)                                │
│      ├── routes ✓                                           │
│      ├── refresh-tokens ◐                                   │
│      └── tests ⊘                                            │
│    ▷ feature/rate-limit (1 task)                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### US-05.3: View branch details
**As a** developer
**I want** to click a branch and see its details
**So that** I can understand what's being worked on

```
┌─────────────────────────────────────────────────────────────┐
│  feature/oauth                                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Notes                                                      │
│  ─────                                                      │
│  Implementing OAuth login flow.                             │
│  See RFC: docs/rfcs/oauth.md                                │
│                                                             │
│  Tasks                                                      │
│  ─────                                                      │
│  ✓ [1] Implement OAuth routes (feature/oauth/routes)        │
│  ◐ [2] Add refresh token logic (feature/oauth/refresh)      │
│  ⊘ [3] Add integration tests (blocked by: 2)                │
│                                                             │
│  Checks                                                     │
│  ──────                                                     │
│  ☑ src/auth/** → Security review required                   │
│  ☐ *.py → Types check (pyright)                             │
│                                                             │
│  [+ Add Task]  [Edit Notes]                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### US-05.4: Add task from UI
**As a** developer
**I want** to add tasks from the UI
**So that** I don't have to use the CLI

```
┌─────────────────────────────────────────────────────────────┐
│  Add Task                                              [X]  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Title: [Add OAuth error handling_________________]         │
│                                                             │
│  Blocked by:                                                │
│  ☐ [1] Implement OAuth routes                               │
│  ☑ [2] Add refresh token logic                              │
│  ☐ [3] Add integration tests                                │
│                                                             │
│  Notes:                                                     │
│  [Handle edge cases for token expiry___________]            │
│  [_____________________________________________]            │
│                                                             │
│  [Cancel]  [Add Task]                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### US-05.5: Create sub-branch from UI
**As a** developer
**I want** to create sub-branches from the UI
**So that** I can set up work for agents

```
┌─────────────────────────────────────────────────────────────┐
│  Create Sub-branch                                     [X]  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Parent: feature/oauth                                      │
│                                                             │
│  Name: [error-handling____________________________]         │
│                                                             │
│  Full branch: feature/oauth/error-handling                  │
│                                                             │
│  Link to task:                                              │
│  ◉ [4] Add OAuth error handling                             │
│  ○ New task                                                 │
│  ○ No task                                                  │
│                                                             │
│  [Cancel]  [Create Branch]                                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### US-05.6: See real-time updates
**As a** developer
**I want** the UI to update as agent works
**So that** I can monitor progress

- Agent commits → task status updates
- Agent creates branch → tree updates
- Agent merges → hierarchy reflects changes

### US-05.7: View commit history
**As a** developer
**I want** to see recent commits with task/check info
**So that** I understand what happened

```
┌─────────────────────────────────────────────────────────────┐
│  Recent Activity                                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  10:45  abc123  feature/oauth/routes                        │
│         "Complete OAuth routes"                             │
│         ✓ Task: Implement OAuth routes (done)               │
│         ✓ Check: Security review                            │
│                                                             │
│  10:30  def456  feature/oauth/routes                        │
│         "Add OAuth middleware"                              │
│         ◐ Task: Implement OAuth routes (partial)            │
│                                                             │
│  10:15  ghi789  feature/oauth/routes                        │
│         "Add OAuth handlers"                                │
│         ◐ Task: Implement OAuth routes (partial)            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Technical Design

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Browser                                │
│                    localhost:9999                            │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ HTTP
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    noslop ui server                          │
│                     (Rust, embedded)                         │
├─────────────────────────────────────────────────────────────┤
│  Static files (HTML/CSS/JS)                                 │
│  REST API                                                   │
│  WebSocket (real-time updates)                              │
│  File watcher (.noslop/tasks/, git refs)                    │
└─────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
       .noslop.toml  .noslop/tasks/   git
        (checks)      (tasks)       (branches)
```

### REST API

```
GET  /api/branches              # List all branches as tree
GET  /api/branches/:name        # Get branch details
GET  /api/branches/:name/tasks  # Get tasks for branch

POST /api/branches              # Create sub-branch
DELETE /api/branches/:name      # Delete branch

GET  /api/tasks/:branch         # Get tasks
POST /api/tasks/:branch         # Create task
PUT  /api/tasks/:branch/:id     # Update task
DELETE /api/tasks/:branch/:id   # Delete task

GET  /api/checks                # List all checks
GET  /api/history               # Recent commits with trailers
```

### WebSocket Events

```json
{"type": "branch_created", "branch": "feature/oauth/routes"}
{"type": "branch_deleted", "branch": "feature/oauth/routes"}
{"type": "task_updated", "branch": "feature/oauth", "task_id": 1}
{"type": "commit", "sha": "abc123", "branch": "feature/oauth"}
```

### File Watching

Watch for changes:
- `.noslop/tasks/*.toml` → task updates
- `.git/refs/heads/*` → branch changes
- `.git/logs/HEAD` → commits

### Technology Stack

**Minimal (recommended for v1):**
- Rust HTTP server (tiny-http or axum)
- Static HTML + vanilla JS
- htmx for dynamic updates
- Single binary, no Node/npm

**Enhanced (future):**
- WebSocket for real-time
- Optional: Tauri for native app

### CLI Integration

```bash
noslop ui                    # Start on default port 9999
noslop ui --port 8080        # Custom port
noslop ui --open             # Open browser automatically
noslop ui --readonly         # View only, no mutations
```

## Acceptance Criteria

- [ ] `noslop ui` starts local server
- [ ] Browser shows branch tree
- [ ] Click branch shows details (tasks, notes, checks)
- [ ] Add/edit/delete tasks from UI
- [ ] Create sub-branch from UI
- [ ] UI updates when files change
- [ ] View recent commits with trailers
- [ ] Works offline (local only)
- [ ] Single binary, no external dependencies

## Non-Goals (v1)

- Multi-user collaboration
- Remote/cloud hosting
- Authentication
- Mobile-responsive design
- Offline sync

## Open Questions

1. Should UI be read-only by default? (Agents shouldn't use it)
2. How to handle multiple repos? (Probably just current repo)
3. Should we support a TUI alternative? (Maybe simpler to start)
4. What's the minimum viable UI for v1?
