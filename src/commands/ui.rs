//! Local Web UI command
//!
//! Provides a local HTTP server for managing tasks and viewing checks.
//! Uses long-polling with file watching for real-time updates.
//!
//! ## Architecture
//!
//! - **API Layer** (`src/api/`): HTTP-agnostic types and handlers
//! - **Server Adapter** (`src/server/tiny_http.rs`): Routing and response conversion
//! - **This Module**: CLI command, static files, file watcher

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tiny_http::{Header, Method, Response, Server};

use crate::server::tiny_http::handle_api_request;
use noslop::api::{ApiResponse, EventsData};

/// Atomic counter that increments on every file change
/// Long-polling clients poll this to know when to refresh
static CHANGE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Start the local web UI server with file watching for real-time updates
pub fn ui(port: u16, open: bool) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let server = Server::http(&addr).map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;

    println!("Starting noslop UI with live updates...");
    println!("Open http://localhost:{port} in your browser");
    println!();
    println!("Press Ctrl+C to stop");

    // Start file watcher in background
    let _watcher = start_file_watcher()?;

    if open {
        open_browser(port);
    }

    for mut request in server.incoming_requests() {
        let path = request.url().to_string();
        let method = request.method().clone();

        // Long-polling endpoint runs in separate thread to not block other requests
        if path.starts_with("/api/events") || path.starts_with("/api/v1/events") {
            std::thread::spawn(move || {
                let response = handle_events(&request);
                let _ = request.respond(response);
            });
            continue;
        }

        // Static pages
        let response = match (&method, path.as_str()) {
            (&Method::Get, "/") => serve_html(INDEX_HTML),
            (&Method::Get, "/style.css") => serve_css(STYLE_CSS),
            // API routes - delegate to server adapter
            _ if path.starts_with("/api") => handle_api_request(&mut request),
            // 404 for everything else
            _ => not_found(),
        };

        let _ = request.respond(response);
    }

    Ok(())
}

// =============================================================================
// STATIC FILE SERVING
// =============================================================================

fn serve_html(content: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(content.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

fn serve_css(content: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(content.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/css; charset=utf-8").unwrap())
}

fn not_found() -> Response<Cursor<Vec<u8>>> {
    Response::from_data(b"Not Found".to_vec()).with_status_code(404)
}

// =============================================================================
// FILE WATCHER
// =============================================================================

/// Start a file watcher that monitors .noslop/ and .git/HEAD for changes
///
/// For worktree support, watches the main worktree's .noslop/ directory
/// so that changes are detected regardless of which worktree you're in.
fn start_file_watcher() -> anyhow::Result<RecommendedWatcher> {
    use noslop::git;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                // Only trigger on actual data changes
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = tx.send(());
                }
            }
        },
        Config::default(),
    )?;

    // Get main worktree for .noslop/ and .noslop.toml paths
    let main_worktree = git::get_main_worktree();

    // Watch .noslop directory for task changes (in main worktree)
    let noslop_dir = main_worktree
        .as_ref()
        .map(|root| root.join(".noslop"))
        .unwrap_or_else(|| PathBuf::from(".noslop"));
    if noslop_dir.exists() {
        watcher.watch(&noslop_dir, RecursiveMode::Recursive)?;
    }

    // Watch .git/HEAD for branch changes (local to current worktree)
    if Path::new(".git/HEAD").exists() {
        watcher.watch(Path::new(".git/HEAD"), RecursiveMode::NonRecursive)?;
    } else if Path::new(".git").exists() {
        // In a worktree, .git is a file pointing to the real git dir
        // Watch the HEAD in the current worktree's git dir
        watcher.watch(Path::new(".git"), RecursiveMode::NonRecursive)?;
    }

    // Watch .noslop.toml for check changes (in main worktree)
    let noslop_toml = main_worktree
        .as_ref()
        .map(|root| root.join(".noslop.toml"))
        .unwrap_or_else(|| PathBuf::from(".noslop.toml"));
    if noslop_toml.exists() {
        watcher.watch(&noslop_toml, RecursiveMode::NonRecursive)?;
    }

    // Background thread to debounce and increment counter
    std::thread::spawn(move || {
        loop {
            // Block until we get a file change event
            if rx.recv().is_ok() {
                // Drain any queued events (debounce by coalescing)
                while rx.try_recv().is_ok() {}
                // Increment counter to trigger refresh
                CHANGE_COUNTER.fetch_add(1, Ordering::SeqCst);
            } else {
                break; // Channel closed
            }
        }
    });

    Ok(watcher)
}

// =============================================================================
// LONG-POLLING EVENTS
// =============================================================================

/// Handle long-polling for change detection
/// Browser calls /api/events?since=N and we block until counter > N or timeout
fn handle_events(request: &tiny_http::Request) -> Response<Cursor<Vec<u8>>> {
    // Parse ?since=N from query string
    let url = request.url();
    let since: Option<u64> = url.split('?').nth(1).and_then(|qs| {
        qs.split('&')
            .find(|p| p.starts_with("since="))
            .and_then(|p| p.strip_prefix("since="))
            .and_then(|v| v.parse().ok())
    });

    let current = CHANGE_COUNTER.load(Ordering::SeqCst);

    // If no 'since' param, return current counter immediately (initial request)
    let Some(since) = since else {
        return events_response(EventsData {
            changed: false,
            counter: current,
        });
    };

    // If counter already changed, return immediately
    if current > since {
        return events_response(EventsData {
            changed: true,
            counter: current,
        });
    }

    // Wait up to 30 seconds for a change
    let deadline = std::time::Instant::now() + Duration::from_secs(30);

    loop {
        std::thread::sleep(Duration::from_millis(50));

        let current = CHANGE_COUNTER.load(Ordering::SeqCst);
        if current > since {
            return events_response(EventsData {
                changed: true,
                counter: current,
            });
        }

        if std::time::Instant::now() >= deadline {
            return events_response(EventsData {
                changed: false,
                counter: current,
            });
        }
    }
}

fn events_response(data: EventsData) -> Response<Cursor<Vec<u8>>> {
    let response = ApiResponse::success(data);
    let json =
        serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false}"#.to_string());
    Response::from_data(json.into_bytes())
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

// =============================================================================
// BROWSER OPENING
// =============================================================================

fn open_browser(port: u16) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg(format!("http://localhost:{port}"))
        .spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(format!("http://localhost:{port}"))
        .spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", &format!("http://localhost:{port}")])
        .spawn();
}

// =============================================================================
// EMBEDDED STATIC FILES
// =============================================================================

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>noslop</title>
    <link rel="stylesheet" href="/style.css">
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <script src="https://unpkg.com/htmx-ext-json-enc@2.0.1/json-enc.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/sortablejs@1.15.0/Sortable.min.js"></script>
</head>
<body hx-ext="json-enc">
    <div class="app-container">
        <!-- Sidebar -->
        <aside class="sidebar">
            <div class="sidebar-header">
                <div class="header-top">
                    <h1>noslop</h1>
                    <div id="connection-status" class="connected">live</div>
                </div>
                <div class="header-context" id="branch-context">
                    on: <span id="current-branch-name">...</span>
                </div>
            </div>

            <!-- Projects List (primary navigation) -->
            <div class="projects-section">
                <div class="projects-header">
                    <span>Projects</span>
                    <button class="btn-new-project" onclick="promptNewProject()" title="New Project">+</button>
                </div>
                <div class="projects-list" id="projects-list">
                    <!-- Dynamic project list -->
                </div>
            </div>

            <!-- Checks (collapsible, secondary) -->
            <div class="checks-section">
                <button class="checks-toggle" onclick="toggleChecksSection()">
                    <span>Checks</span>
                    <span id="checks-count" class="badge">0</span>
                    <span class="toggle-icon" id="checks-toggle-icon">▸</span>
                </button>
                <div id="checks-content" class="checks-content collapsed">
                    <div id="checks" hx-get="/api/v1/checks" hx-trigger="load, refresh from:body" hx-swap="innerHTML">
                        Loading...
                    </div>
                    <button class="btn-add-check" onclick="toggleCheckForm()">+ Add Check</button>
                    <form id="new-check-form" class="hidden" hx-post="/api/v1/checks" hx-swap="none"
                          hx-on::after-request="if(event.detail.successful) { this.reset(); this.classList.add('hidden'); htmx.trigger('#checks', 'load'); loadStatus(); }">
                        <input type="text" name="target" placeholder="Target (e.g., *.rs)" required>
                        <input type="text" name="message" placeholder="Check message..." required>
                        <select name="severity">
                            <option value="block">Block</option>
                            <option value="warn">Warn</option>
                            <option value="info">Info</option>
                        </select>
                        <button type="submit">Add</button>
                    </form>
                </div>
            </div>

            <div class="sidebar-footer">
                <p>Ctrl+C to stop</p>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="main-content">
            <div class="toolbar">
                <form id="new-task-form" onsubmit="return handleTaskSubmit(event)">
                    <input type="text" name="title" placeholder="New task..." required>
                    <input type="hidden" name="project" id="new-task-project">
                    <button type="submit">+ Add</button>
                </form>
            </div>

            <div class="kanban">
                <div class="kanban-column" data-status="backlog">
                    <div class="column-header">
                        <h2>Backlog</h2>
                        <span class="count" id="count-backlog">0</span>
                    </div>
                    <div class="column-tasks" id="col-backlog"></div>
                </div>

                <div class="kanban-column" data-status="pending">
                    <div class="column-header">
                        <h2>Pending</h2>
                        <span class="count" id="count-pending">0</span>
                    </div>
                    <div class="column-tasks" id="col-pending"></div>
                </div>

                <div class="kanban-column" data-status="in_progress">
                    <div class="column-header">
                        <h2>In Progress</h2>
                        <span class="count" id="count-in-progress">0</span>
                    </div>
                    <div class="column-tasks" id="col-in-progress"></div>
                </div>

                <div class="kanban-column" data-status="done">
                    <div class="column-header">
                        <h2>Done</h2>
                        <span class="count" id="count-done">0</span>
                    </div>
                    <div class="column-tasks" id="col-done"></div>
                </div>
            </div>
        </main>

    </div>

    <!-- Task Detail Modal -->
    <div id="detail-modal" class="modal" onclick="if(event.target === this) closeDetailModal()">
        <div class="modal-content detail-modal-content">
            <div class="detail-header">
                <span id="detail-id"></span>
                <button class="btn-close" onclick="closeDetailModal()">×</button>
            </div>
            <div class="detail-body">
                <div class="detail-section">
                    <label>Title</label>
                    <div id="detail-title"></div>
                </div>
                <div class="detail-section">
                    <label>Status</label>
                    <div id="detail-status"></div>
                </div>
                <div class="detail-section">
                    <label>Branch</label>
                    <div id="detail-branch"></div>
                </div>
                <div class="detail-section">
                    <label>Blocked By</label>
                    <div id="detail-blocked-by"></div>
                    <div class="add-blocker">
                        <select id="blocker-select"></select>
                        <button onclick="addBlocker()">Add</button>
                    </div>
                </div>
                <div class="detail-section">
                    <label>Blocking</label>
                    <div id="detail-blocking"></div>
                </div>
            </div>
        </div>
    </div>

    <!-- Delete confirmation dialog -->
    <div id="delete-confirm" class="modal">
        <div class="modal-content">
            <p>Delete <span id="delete-task-id"></span>?</p>
            <div class="modal-actions">
                <button onclick="cancelDelete()">Cancel</button>
                <button class="btn-danger" onclick="confirmDelete()">Delete</button>
            </div>
        </div>
    </div>

    <script>
        // Branch color palette
        const BRANCH_COLORS = [
            { bg: '#3b82f6', text: '#fff', name: 'blue' },
            { bg: '#10b981', text: '#fff', name: 'green' },
            { bg: '#f59e0b', text: '#000', name: 'amber' },
            { bg: '#ef4444', text: '#fff', name: 'red' },
            { bg: '#8b5cf6', text: '#fff', name: 'purple' },
            { bg: '#ec4899', text: '#fff', name: 'pink' },
            { bg: '#06b6d4', text: '#000', name: 'cyan' },
            { bg: '#84cc16', text: '#000', name: 'lime' },
        ];

        // State
        let lastCounter = null;
        let polling = true;
        let currentBranch = null;
        let allTasks = [];
        let allProjects = [];
        let currentProject = null; // null means "All"

        // Unwrap API envelope
        function unwrap(response) {
            if (!response.success) {
                console.error('API error:', response.error);
                return null;
            }
            return response.data;
        }

        // Load status
        async function loadStatus() {
            try {
                const response = await fetch('/api/v1/status');
                const envelope = await response.json();
                const data = unwrap(envelope);
                if (data) {
                    currentBranch = data.branch;
                    document.getElementById('checks-count').textContent = data.checks;
                    // Update branch context in header
                    document.getElementById('current-branch-name').textContent = data.branch || 'unknown';
                }
            } catch (e) {
                console.error('Failed to load status:', e);
            }
        }

        // Toggle checks section
        function toggleChecksSection() {
            const content = document.getElementById('checks-content');
            const icon = document.getElementById('checks-toggle-icon');
            content.classList.toggle('collapsed');
            icon.textContent = content.classList.contains('collapsed') ? '▸' : '▾';
        }

        // Load tasks
        async function loadTasks() {
            try {
                const response = await fetch('/api/v1/tasks');
                const envelope = await response.json();
                const data = unwrap(envelope);
                if (data) {
                    allTasks = data.tasks || [];
                    renderKanban();
                    renderProjectList(); // Update task counts
                }
            } catch (e) {
                console.error('Failed to load tasks:', e);
            }
        }

        // Load projects
        async function loadProjects() {
            try {
                const response = await fetch('/api/v1/projects');
                const envelope = await response.json();
                const data = unwrap(envelope);
                if (data) {
                    allProjects = data.projects || [];
                    currentProject = data.current_project || null;
                    renderProjectList();
                    // Update hidden project field for new task form
                    document.getElementById('new-task-project').value = currentProject || '';
                }
            } catch (e) {
                console.error('Failed to load projects:', e);
            }
        }

        function renderProjectList() {
            const container = document.getElementById('projects-list');
            let html = `<div class="project-item ${!currentProject ? 'active' : ''}"
                             onclick="selectProject(null)">
                            <span class="project-name">All Tasks</span>
                            <span class="project-count">${allTasks.length}</span>
                        </div>`;

            for (const project of allProjects) {
                const isActive = currentProject === project.id;
                html += `<div class="project-item ${isActive ? 'active' : ''}"
                              onclick="selectProject('${project.id}')">
                            <span class="project-name">${project.name}</span>
                            <span class="project-count">${project.task_count}</span>
                        </div>`;
            }

            container.innerHTML = html;
        }

        async function selectProject(projectId) {
            try {
                await fetch('/api/v1/projects/select', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ id: projectId })
                });
                currentProject = projectId;
                renderProjectList();
                renderKanban();
                // Update hidden project field for new task form
                document.getElementById('new-task-project').value = currentProject || '';
            } catch (e) {
                console.error('Failed to select project:', e);
            }
        }

        async function promptNewProject() {
            const name = prompt('Enter project name:');
            if (!name || !name.trim()) return;

            try {
                const response = await fetch('/api/v1/projects', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ name: name.trim() })
                });
                const envelope = await response.json();
                const data = unwrap(envelope);
                if (data) {
                    // Reload and select the new project
                    await loadProjects();
                    await selectProject(data.id);
                }
            } catch (e) {
                console.error('Failed to create project:', e);
            }
        }

        function filterTasks(tasks) {
            return tasks.filter(t => {
                // Project filter: if currentProject is set, only show tasks in that project
                return !currentProject || t.project === currentProject;
            });
        }

        function renderKanban() {
            const filtered = filterTasks(allTasks);

            // Sort done by completed_at descending (most recent first)
            const sortByCompletedDesc = (a, b) => {
                if (!a.completed_at && !b.completed_at) return 0;
                if (!a.completed_at) return 1;
                if (!b.completed_at) return -1;
                return b.completed_at.localeCompare(a.completed_at);
            };

            // 4 columns: Backlog, Pending, In Progress, Done
            const backlog = filtered.filter(t => t.status === 'backlog');
            const pending = filtered.filter(t => t.status === 'pending');
            const inProgress = filtered.filter(t => t.status === 'in_progress');
            const done = filtered.filter(t => t.status === 'done').sort(sortByCompletedDesc);

            // Render columns
            document.getElementById('col-backlog').innerHTML = backlog.map(renderTaskCard).join('');
            document.getElementById('col-pending').innerHTML = pending.map(renderTaskCard).join('');
            document.getElementById('col-in-progress').innerHTML = inProgress.map(renderTaskCard).join('');
            document.getElementById('col-done').innerHTML = done.map(renderTaskCard).join('');

            // Update counts
            document.getElementById('count-backlog').textContent = backlog.length;
            document.getElementById('count-pending').textContent = pending.length;
            document.getElementById('count-in-progress').textContent = inProgress.length;
            document.getElementById('count-done').textContent = done.length;

            // Initialize Sortable on columns
            initSortable();

            // Re-apply selection highlight
            updateSelection();

            // Refresh detail panel if open
            if (detailModalOpen && selectedTaskId) {
                openDetailModal(selectedTaskId);
            }
        }

        function renderTaskCard(t) {
            const blockedCount = t.blocked_by?.length || 0;
            const isBlocked = t.blocked && t.status !== 'done';
            const isDone = t.status === 'done';
            const branchColor = t.branch ? getBranchColor(t.branch, getBranchColorIndex(t.branch)).bg : null;

            return `
                <div class="task-card ${isBlocked ? 'blocked' : ''} ${isDone ? 'done' : ''} ${t.current ? 'current' : ''} ${selectedTaskId === t.id ? 'selected' : ''}"
                     data-task-id="${t.id}"
                     data-status="${t.status}"
                     onclick="selectTask('${t.id}')"
                     ondblclick="openDetailModal('${t.id}')"
                     tabindex="0">
                    <div class="task-header">
                        <span class="task-id" ${branchColor ? `style="color: ${branchColor}"` : ''}>${t.id}</span>
                        <div class="task-menu">
                            <button class="btn-menu" onclick="toggleMenu('${t.id}', event)">⋮</button>
                            <div class="menu-dropdown" id="menu-${t.id}">
                                <button onclick="promptDelete('${t.id}')">Delete</button>
                            </div>
                        </div>
                    </div>
                    <div class="task-title">${t.title}</div>
                    <div class="task-footer">
                        ${t.branch ? `<span class="branch-tag">${t.branch}</span>` : ''}
                        ${blockedCount > 0 ? `<span class="blocked-tag" title="${t.blocked_by.join(', ')}">⊘ ${blockedCount}</span>` : ''}
                    </div>
                </div>
            `;
        }

        function getBranchColorIndex(branchName) {
            // Hash the branch name to get a consistent color
            let hash = 0;
            for (let i = 0; i < branchName.length; i++) {
                hash = ((hash << 5) - hash) + branchName.charCodeAt(i);
                hash |= 0;
            }
            return Math.abs(hash) % BRANCH_COLORS.length;
        }

        // Initialize SortableJS
        function initSortable() {
            ['col-backlog', 'col-pending', 'col-in-progress', 'col-done'].forEach(colId => {
                const el = document.getElementById(colId);
                if (el && !el.sortableInstance) {
                    el.sortableInstance = new Sortable(el, {
                        group: 'tasks',
                        animation: 150,
                        ghostClass: 'sortable-ghost',
                        chosenClass: 'sortable-chosen',
                        dragClass: 'sortable-drag',
                        onEnd: handleDragEnd
                    });
                }
            });
        }

        async function handleDragEnd(evt) {
            const taskId = evt.item.dataset.taskId;
            const toColumn = evt.to.id;

            let newStatus;
            if (toColumn === 'col-backlog') {
                newStatus = 'backlog';
            } else if (toColumn === 'col-pending') {
                newStatus = 'pending';
            } else if (toColumn === 'col-in-progress') {
                newStatus = 'in_progress';
            } else if (toColumn === 'col-done') {
                newStatus = 'done';
            }

            // Find task's current status
            const task = allTasks.find(t => t.id === taskId);
            if (!task) return;

            try {
                // Update status if changed
                if (task.status !== newStatus) {
                    if (newStatus === 'backlog') {
                        await fetch(`/api/v1/tasks/${taskId}/backlog`, { method: 'POST' });
                    } else if (newStatus === 'pending') {
                        await fetch(`/api/v1/tasks/${taskId}/reset`, { method: 'POST' });
                    } else if (newStatus === 'in_progress') {
                        await fetch(`/api/v1/tasks/${taskId}/start`, { method: 'POST' });
                    } else if (newStatus === 'done') {
                        await fetch(`/api/v1/tasks/${taskId}/done`, { method: 'POST' });
                    }
                }

                // Auto-link to current branch when moving to Pending or In Progress
                if ((toColumn === 'col-pending' || toColumn === 'col-in-progress') && !task.branch && currentBranch) {
                    await fetch(`/api/v1/tasks/${taskId}/link-branch`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ branch: currentBranch })
                    });
                }

                // Reload to get fresh data
                loadTasks();
                loadStatus();
            } catch (e) {
                console.error('Failed to update task:', e);
                loadTasks(); // Revert UI
            }
        }

        async function linkBranch(taskId, branch) {
            try {
                await fetch(`/api/v1/tasks/${taskId}/link-branch`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ branch: branch })
                });
                loadTasks();
            } catch (e) {
                console.error('Failed to link branch:', e);
            }
        }

        // Selection state
        let selectedTaskId = null;
        let pendingDelete = null;
        let detailModalOpen = false;

        function selectTask(taskId) {
            selectedTaskId = taskId;
            updateSelection();
        }

        function updateSelection() {
            document.querySelectorAll('.task-card').forEach(card => {
                card.classList.toggle('selected', card.dataset.taskId === selectedTaskId);
            });
            // Focus the selected card for keyboard events
            if (selectedTaskId) {
                const card = document.querySelector(`[data-task-id="${selectedTaskId}"]`);
                if (card) card.focus();
            }
        }

        function toggleMenu(taskId, event) {
            event.stopPropagation();
            // Close all other menus
            document.querySelectorAll('.menu-dropdown.open').forEach(m => {
                if (m.id !== `menu-${taskId}`) m.classList.remove('open');
            });
            const menu = document.getElementById(`menu-${taskId}`);
            menu.classList.toggle('open');
        }

        // Close menus when clicking outside
        document.addEventListener('click', (e) => {
            if (!e.target.closest('.task-menu')) {
                document.querySelectorAll('.menu-dropdown.open').forEach(m => m.classList.remove('open'));
            }
        });

        function promptDelete(taskId) {
            pendingDelete = taskId;
            document.getElementById('delete-confirm').classList.add('open');
            document.getElementById('delete-task-id').textContent = taskId;
        }

        function cancelDelete() {
            pendingDelete = null;
            document.getElementById('delete-confirm').classList.remove('open');
        }

        async function confirmDelete() {
            if (!pendingDelete) return;
            const taskId = pendingDelete;
            cancelDelete();
            await deleteTask(taskId);
        }

        async function deleteTask(taskId) {
            // Close any open menu
            document.querySelectorAll('.menu-dropdown.open').forEach(m => m.classList.remove('open'));

            try {
                await fetch(`/api/v1/tasks/${taskId}`, {
                    method: 'DELETE'
                });
                // Clear selection if deleted task was selected
                if (selectedTaskId === taskId) {
                    selectedTaskId = null;
                    closeDetailModal();
                }
                loadTasks();
                loadStatus();
            } catch (e) {
                console.error('Failed to delete task:', e);
            }
        }

        // Detail modal functions
        function openDetailModal(taskId) {
            const task = allTasks.find(t => t.id === taskId);
            if (!task) return;

            selectedTaskId = taskId;
            updateSelection();

            document.getElementById('detail-id').textContent = task.id;
            document.getElementById('detail-title').textContent = task.title;
            document.getElementById('detail-status').textContent = task.status;
            document.getElementById('detail-branch').textContent = task.branch || '—';

            // Blocked by (with remove buttons)
            const blockedBy = task.blocked_by || [];
            document.getElementById('detail-blocked-by').innerHTML = blockedBy.length
                ? blockedBy.map(b => `<span class="dep-tag">${b} <button onclick="removeBlocker('${taskId}', '${b}')">×</button></span>`).join('')
                : '<span class="empty">None</span>';

            // What this task blocks (reverse lookup)
            const blocking = allTasks.filter(t => t.blocked_by?.includes(taskId)).map(t => t.id);
            document.getElementById('detail-blocking').innerHTML = blocking.length
                ? blocking.map(b => `<span class="dep-tag">${b}</span>`).join('')
                : '<span class="empty">None</span>';

            // Populate blocker dropdown (exclude self, already blocking, and done tasks)
            const select = document.getElementById('blocker-select');
            const candidates = allTasks.filter(t => t.id !== taskId && !blockedBy.includes(t.id) && t.status !== 'done');
            select.innerHTML = candidates.length
                ? candidates.map(t => `<option value="${t.id}">${t.id}</option>`).join('')
                : '<option value="">No tasks available</option>';

            document.getElementById('detail-modal').classList.add('open');
            detailModalOpen = true;
        }

        function closeDetailModal() {
            document.getElementById('detail-modal').classList.remove('open');
            detailModalOpen = false;
        }

        async function addBlocker() {
            const blockerId = document.getElementById('blocker-select').value;
            if (!blockerId || !selectedTaskId) return;
            try {
                await fetch(`/api/v1/tasks/${selectedTaskId}/block`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ blocker_id: blockerId })
                });
                loadTasks();
            } catch (e) {
                console.error('Failed to add blocker:', e);
            }
        }

        async function removeBlocker(taskId, blockerId) {
            try {
                await fetch(`/api/v1/tasks/${taskId}/unblock`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ blocker_id: blockerId })
                });
                loadTasks();
            } catch (e) {
                console.error('Failed to remove blocker:', e);
            }
        }

        // Grid-aware keyboard navigation
        document.addEventListener('keydown', (e) => {
            // Don't handle if typing in an input
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

            const columns = ['col-backlog', 'col-pending', 'col-in-progress', 'col-done'];
            const cardsByColumn = columns.map(id => Array.from(document.querySelectorAll(`#${id} .task-card`)));

            // Find current position
            let col = -1, row = -1;
            if (selectedTaskId) {
                for (let c = 0; c < cardsByColumn.length; c++) {
                    const idx = cardsByColumn[c].findIndex(card => card.dataset.taskId === selectedTaskId);
                    if (idx >= 0) { col = c; row = idx; break; }
                }
            }

            switch (e.key) {
                case 'ArrowDown':
                case 'j':
                    e.preventDefault();
                    if (col >= 0 && row < cardsByColumn[col].length - 1) {
                        selectTask(cardsByColumn[col][row + 1].dataset.taskId);
                    } else if (col === -1) {
                        // Select first card in first non-empty column
                        for (const c of cardsByColumn) {
                            if (c.length) { selectTask(c[0].dataset.taskId); break; }
                        }
                    }
                    break;

                case 'ArrowUp':
                case 'k':
                    e.preventDefault();
                    if (col >= 0 && row > 0) {
                        selectTask(cardsByColumn[col][row - 1].dataset.taskId);
                    }
                    break;

                case 'ArrowRight':
                case 'l':
                    e.preventDefault();
                    if (col >= 0 && col < columns.length - 1) {
                        const next = cardsByColumn[col + 1];
                        if (next.length) {
                            selectTask(next[Math.min(row, next.length - 1)].dataset.taskId);
                        }
                    }
                    break;

                case 'ArrowLeft':
                case 'h':
                    e.preventDefault();
                    if (col > 0) {
                        const prev = cardsByColumn[col - 1];
                        if (prev.length) {
                            selectTask(prev[Math.min(row, prev.length - 1)].dataset.taskId);
                        }
                    }
                    break;

                case 'Delete':
                case 'Backspace':
                    if (selectedTaskId) {
                        e.preventDefault();
                        promptDelete(selectedTaskId);
                    }
                    break;

                case 'Enter':
                    if (pendingDelete) {
                        e.preventDefault();
                        confirmDelete();
                    } else if (selectedTaskId && !detailModalOpen) {
                        e.preventDefault();
                        openDetailModal(selectedTaskId);
                    }
                    break;

                case 'Escape':
                    cancelDelete();
                    closeDetailModal();
                    document.querySelectorAll('.menu-dropdown.open').forEach(m => m.classList.remove('open'));
                    break;
            }
        });

        async function handleTaskCreated(event) {
            if (!event.detail.successful) return;

            const form = event.target;
            form.reset();

            // New tasks go to backlog (unlinked) - no auto-linking
            loadTasks();
            loadStatus();
        }

        function toggleCheckForm() {
            document.getElementById('new-check-form').classList.toggle('hidden');
        }

        // Transform checks JSON response
        document.body.addEventListener('htmx:beforeSwap', function(evt) {
            const target = evt.detail.target;
            if (target.id === 'checks') {
                try {
                    const envelope = JSON.parse(evt.detail.xhr.responseText);
                    const data = unwrap(envelope);
                    if (data) {
                        evt.detail.serverResponse = renderChecks(data);
                    }
                } catch (e) {}
            }
        });

        function renderChecks(data) {
            if (!data.checks || data.checks.length === 0) {
                return '<p class="empty">No checks</p>';
            }
            return data.checks.map(c => `
                <div class="check-item ${c.severity}">
                    <span class="check-target">${c.target}</span>
                    <span class="check-severity">${c.severity}</span>
                </div>
            `).join('');
        }

        // Long-polling
        async function poll() {
            const statusEl = document.getElementById('connection-status');

            while (polling) {
                try {
                    statusEl.textContent = 'live';
                    statusEl.className = 'connected';

                    const url = lastCounter === null
                        ? '/api/v1/events'
                        : `/api/v1/events?since=${lastCounter}`;

                    const response = await fetch(url);
                    const envelope = await response.json();
                    const data = unwrap(envelope);

                    if (data && data.changed) {
                        loadTasks();
                        loadStatus();
                        loadProjects();
                        htmx.trigger('#checks', 'load');
                    }

                    if (data) lastCounter = data.counter;
                } catch (e) {
                    statusEl.textContent = 'reconnecting...';
                    statusEl.className = 'disconnected';
                    await new Promise(r => setTimeout(r, 2000));
                }
            }
        }

        // Handle task form submission with project
        async function handleTaskSubmit(event) {
            event.preventDefault();
            const form = event.target;
            const title = form.title.value.trim();
            const project = form.project.value || null;

            if (!title) return false;

            try {
                const response = await fetch('/api/v1/tasks', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ title, project })
                });
                const envelope = await response.json();
                if (envelope.success) {
                    form.reset();
                    form.project.value = currentProject || '';
                    loadTasks();
                    loadStatus();
                    loadProjects(); // Update task counts
                }
            } catch (e) {
                console.error('Failed to create task:', e);
            }

            return false;
        }

        // Initialize
        loadProjects();
        loadStatus();
        loadTasks();
        poll();
    </script>
</body>
</html>
"#;

const STYLE_CSS: &str = r#"
:root {
    --bg: #1a1a2e;
    --surface: #16213e;
    --primary: #0f3460;
    --accent: #e94560;
    --text: #eee;
    --text-dim: #888;
    --success: #4ade80;
    --warning: #fbbf24;
    --info: #60a5fa;
    --sidebar-width: 250px;
    /* Font scale - tighter, more readable */
    --font-xs: 0.75rem;    /* 12px - labels, badges */
    --font-sm: 0.8125rem;  /* 13px - secondary text, IDs */
    --font-base: 0.875rem; /* 14px - body, task titles */
    --font-lg: 1rem;       /* 16px - headings */
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    background: var(--bg);
    color: var(--text);
    height: 100vh;
    overflow: hidden;
}

/* App Layout */
.app-container {
    display: grid;
    grid-template-columns: var(--sidebar-width) 1fr;
    height: 100vh;
}

/* Sidebar */
.sidebar {
    background: var(--surface);
    border-right: 1px solid var(--primary);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
}

.sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid var(--primary);
}

.header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.sidebar-header h1 {
    font-size: 1.25rem;
    color: var(--accent);
}

.header-context {
    font-size: var(--font-xs);
    color: var(--text-dim);
    margin-top: 0.5rem;
}

#current-branch-name {
    color: var(--text);
}

#connection-status {
    font-size: var(--font-xs);
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    text-transform: uppercase;
}

#connection-status.connected {
    background: var(--success);
    color: var(--bg);
}

#connection-status.disconnected {
    background: var(--accent);
    color: var(--text);
}

/* Projects Section */
.projects-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.projects-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    font-size: var(--font-xs);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
}

.btn-new-project {
    background: transparent;
    border: 1px dashed var(--text-dim);
    border-radius: 4px;
    color: var(--text-dim);
    width: 24px;
    height: 24px;
    font-size: var(--font-base);
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
}

.btn-new-project:hover {
    border-color: var(--accent);
    color: var(--accent);
}

.projects-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 0.5rem;
}

.project-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: var(--font-sm);
    transition: background 0.15s;
    margin-bottom: 0.25rem;
}

.project-item:hover {
    background: var(--primary);
}

.project-item.active {
    background: var(--accent);
    color: var(--text);
}

.project-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.project-count {
    font-size: var(--font-xs);
    background: rgba(0, 0, 0, 0.2);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    margin-left: 0.5rem;
}

.project-item.active .project-count {
    background: rgba(255, 255, 255, 0.2);
}

/* Checks Section */
.checks-section {
    border-top: 1px solid var(--primary);
}

.checks-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: var(--font-xs);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    cursor: pointer;
    text-align: left;
}

.checks-toggle:hover {
    background: var(--primary);
}

.checks-toggle .badge {
    margin-left: auto;
}

.toggle-icon {
    font-size: var(--font-xs);
}

.checks-content {
    padding: 0 1rem 1rem;
}

.checks-content.collapsed {
    display: none;
}

.sidebar-section {
    padding: 1rem;
    border-bottom: 1px solid var(--primary);
}

.sidebar-section h3 {
    font-size: var(--font-xs);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
    margin-bottom: 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.badge {
    background: var(--primary);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-size: var(--font-xs);
}

.sidebar-footer {
    margin-top: auto;
    padding: 1rem;
    text-align: center;
    color: var(--text-dim);
    font-size: var(--font-sm);
}

/* Checks in sidebar */
.check-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 0.5rem;
    background: var(--primary);
    border-radius: 4px;
    margin-bottom: 0.35rem;
    font-size: var(--font-sm);
    border-left: 2px solid var(--text-dim);
}

.check-item.block { border-left-color: var(--accent); }
.check-item.warn { border-left-color: var(--warning); }
.check-item.info { border-left-color: var(--success); }

.check-target {
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.check-severity {
    font-size: var(--font-xs);
    text-transform: uppercase;
    color: var(--text-dim);
}

.btn-add-check {
    width: 100%;
    margin-top: 0.5rem;
    background: transparent;
    border: 1px dashed var(--primary);
    color: var(--text-dim);
    padding: 0.4rem;
    font-size: var(--font-sm);
}

.btn-add-check:hover {
    border-color: var(--accent);
    color: var(--accent);
}

#new-check-form {
    flex-direction: column;
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: none;
}

#new-check-form input, #new-check-form select {
    font-size: var(--font-sm);
    padding: 0.4rem;
}

.hidden {
    display: none !important;
}

/* Main Content */
.main-content {
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.toolbar {
    padding: 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--primary);
}

.toolbar form {
    display: flex;
    gap: 0.5rem;
    margin: 0;
    padding: 0;
    border: none;
}

.toolbar input[type="text"] {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--primary);
    color: var(--text);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    font-family: inherit;
    font-size: var(--font-base);
}

.toolbar input:focus {
    outline: none;
    border-color: var(--accent);
}

.toolbar select {
    background: var(--bg);
    border: 1px solid var(--primary);
    color: var(--text);
    padding: 0.5rem;
    border-radius: 4px;
    font-family: inherit;
    font-size: var(--font-base);
}

.toolbar button {
    background: var(--success);
    color: var(--bg);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    font-size: var(--font-base);
    font-weight: 500;
}

.toolbar button:hover {
    opacity: 0.9;
}

/* Kanban Board */
.kanban {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    padding: 1rem;
    flex: 1;
    overflow: hidden;
}

.kanban-column {
    background: var(--surface);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.column-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--primary);
}

.column-header h2 {
    font-size: var(--font-xs);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    margin: 0;
    font-weight: 600;
}

.column-header .count {
    background: var(--primary);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: var(--font-xs);
    color: var(--text-dim);
    font-weight: 500;
}

.column-tasks {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    min-height: 100px;
}

/* Task Cards */
.task-card {
    background: #1e4976;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    cursor: grab;
    transition: transform 0.15s, box-shadow 0.15s, opacity 0.15s;
    opacity: 1;
}

.task-card:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.task-card.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
}

.task-card.current {
    background: linear-gradient(90deg, rgba(233, 69, 96, 0.2) 0%, #1e4976 100%);
}

.task-card.blocked {
    opacity: 0.7;
}

.task-card.done {
    opacity: 0.75;
}

.task-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.4rem;
}

.task-id {
    font-size: var(--font-sm);
    color: var(--text-dim);
    font-weight: 500;
}

.task-menu {
    position: relative;
}

.btn-menu {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: var(--font-lg);
    padding: 0.25rem;
    line-height: 1;
}

.btn-menu:hover {
    color: var(--text);
}

.menu-dropdown {
    display: none;
    position: absolute;
    right: 0;
    top: 100%;
    background: var(--surface);
    border: 1px solid var(--primary);
    border-radius: 4px;
    z-index: 50;
    min-width: 80px;
}

.menu-dropdown.open {
    display: block;
}

.menu-dropdown button {
    display: block;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: transparent;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: var(--font-sm);
    text-align: left;
}

.menu-dropdown button:hover {
    background: var(--primary);
}

.task-title {
    font-size: var(--font-base);
    line-height: 1.3;
    margin-bottom: 0.5rem;
    color: #fff;
}

.task-footer {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
}

.branch-tag {
    font-size: var(--font-xs);
    padding: 0.15rem 0.4rem;
    background: var(--bg);
    color: var(--text-dim);
    border-radius: 3px;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.blocked-tag {
    font-size: var(--font-xs);
    color: var(--warning);
    background: rgba(251, 191, 36, 0.15);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-weight: 600;
}

/* Modal */
.modal {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    z-index: 200;
    align-items: center;
    justify-content: center;
}

.modal.open {
    display: flex;
}

.modal-content {
    background: var(--surface);
    padding: 1.5rem;
    border-radius: 8px;
    text-align: center;
    min-width: 280px;
}

.modal-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
    margin-top: 1rem;
}

.modal-actions button {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    border: 1px solid var(--primary);
    background: var(--bg);
    color: var(--text);
}

.btn-danger {
    background: var(--accent) !important;
    border-color: var(--accent) !important;
    color: white !important;
}

.btn-close {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 1.25rem;
    cursor: pointer;
    padding: 0.25rem;
}

.btn-close:hover {
    color: var(--text);
}

/* Detail Modal */
.detail-modal-content {
    width: 400px;
    max-width: 90vw;
    max-height: 80vh;
    overflow-y: auto;
    text-align: left;
}

.detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--primary);
    font-weight: bold;
}

.detail-body {
    padding: 0;
}

.detail-section {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--primary);
}

.detail-section label {
    font-size: var(--font-xs);
    text-transform: uppercase;
    color: var(--text-dim);
    display: block;
    margin-bottom: 0.25rem;
}

.add-blocker {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
}

.add-blocker select {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--primary);
    color: var(--text);
    padding: 0.35rem;
    border-radius: 4px;
    font-size: var(--font-sm);
}

.add-blocker button {
    background: var(--accent);
    border: none;
    color: white;
    padding: 0.35rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: var(--font-sm);
}

.dep-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--primary);
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-size: var(--font-sm);
    margin-right: 0.25rem;
    margin-bottom: 0.25rem;
}

.dep-tag button {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: var(--font-sm);
    padding: 0;
    margin-left: 0.25rem;
}

.dep-tag button:hover {
    color: var(--accent);
}

/* SortableJS styles */
.sortable-ghost {
    opacity: 0.4;
}

.sortable-chosen {
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.sortable-drag {
    opacity: 1;
}

/* Empty state */
.empty {
    color: var(--text-dim);
    font-style: italic;
    font-size: var(--font-sm);
    padding: 1rem;
    text-align: center;
}

/* Responsive */
@media (max-width: 900px) {
    .app-container {
        grid-template-columns: 1fr;
    }

    .sidebar {
        display: none;
    }

    .kanban {
        grid-template-columns: 1fr;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use noslop::storage::TaskRefs;
    use serial_test::serial;
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();
        // Create .noslop directory
        std::fs::create_dir_all(".noslop/refs/tasks").unwrap();
        temp
    }

    #[test]
    #[serial]
    fn test_file_watcher_detects_task_changes() {
        let _temp = setup();

        // Reset counter
        CHANGE_COUNTER.store(0, Ordering::SeqCst);

        // Start file watcher
        let _watcher = start_file_watcher().unwrap();

        // Give watcher time to initialize
        std::thread::sleep(Duration::from_millis(100));

        let initial_count = CHANGE_COUNTER.load(Ordering::SeqCst);

        // Create a task (this writes to .noslop/refs/tasks/)
        let id = TaskRefs::create("Test task", None).unwrap();

        // Wait for watcher to detect the change
        std::thread::sleep(Duration::from_millis(600));

        let after_create = CHANGE_COUNTER.load(Ordering::SeqCst);
        assert!(
            after_create > initial_count,
            "Counter should increment after task create: {} -> {}",
            initial_count,
            after_create
        );

        // Update the task status (this is what Start/Done buttons do)
        TaskRefs::set_status(&id, "in_progress").unwrap();

        // Wait for watcher to detect the change
        std::thread::sleep(Duration::from_millis(600));

        let after_update = CHANGE_COUNTER.load(Ordering::SeqCst);
        assert!(
            after_update > after_create,
            "Counter should increment after task update: {} -> {}",
            after_create,
            after_update
        );

        // Also test set_current (which Start button also does)
        TaskRefs::set_current(&id).unwrap();

        std::thread::sleep(Duration::from_millis(600));

        let after_set_current = CHANGE_COUNTER.load(Ordering::SeqCst);
        assert!(
            after_set_current > after_update,
            "Counter should increment after set_current: {} -> {}",
            after_update,
            after_set_current
        );
    }
}
