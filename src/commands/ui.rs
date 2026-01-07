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
use std::path::Path;
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
fn start_file_watcher() -> anyhow::Result<RecommendedWatcher> {
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

    // Watch .noslop directory for task changes
    if Path::new(".noslop").exists() {
        watcher.watch(Path::new(".noslop"), RecursiveMode::Recursive)?;
    }

    // Watch .git/HEAD for branch changes
    if Path::new(".git/HEAD").exists() {
        watcher.watch(Path::new(".git/HEAD"), RecursiveMode::NonRecursive)?;
    }

    // Watch .noslop.toml for check changes
    if Path::new(".noslop.toml").exists() {
        watcher.watch(Path::new(".noslop.toml"), RecursiveMode::NonRecursive)?;
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
</head>
<body hx-ext="json-enc">
    <header>
        <h1>noslop</h1>
        <div id="branch"></div>
        <div id="connection-status" class="connected">live</div>
    </header>

    <main>
        <section id="status-section">
            <h2>Status</h2>
            <div id="status" hx-get="/api/v1/status" hx-trigger="load, refresh from:body" hx-swap="innerHTML">
                Loading...
            </div>
        </section>

        <section id="tasks-section">
            <h2>Tasks</h2>
            <div id="tasks" hx-get="/api/v1/tasks" hx-trigger="load, refresh from:body" hx-swap="innerHTML">
                Loading...
            </div>
            <form id="new-task-form" hx-post="/api/v1/tasks" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) { this.reset(); htmx.trigger('#tasks', 'load'); htmx.trigger('#status', 'load'); }">
                <input type="text" name="title" placeholder="New task title..." required>
                <select name="priority">
                    <option value="p1">P1 (default)</option>
                    <option value="p0">P0 (urgent)</option>
                    <option value="p2">P2</option>
                    <option value="p3">P3 (low)</option>
                </select>
                <button type="submit">Add Task</button>
            </form>
        </section>

        <section id="checks-section">
            <h2>Checks</h2>
            <div id="checks" hx-get="/api/v1/checks" hx-trigger="load, refresh from:body" hx-swap="innerHTML">
                Loading...
            </div>
            <form id="new-check-form" hx-post="/api/v1/checks" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) { this.reset(); htmx.trigger('#checks', 'load'); htmx.trigger('#status', 'load'); }">
                <input type="text" name="target" placeholder="Target (e.g., *.rs, src/**)" required>
                <input type="text" name="message" placeholder="Check message..." required>
                <select name="severity">
                    <option value="block">Block</option>
                    <option value="warn">Warn</option>
                    <option value="info">Info</option>
                </select>
                <button type="submit">Add Check</button>
            </form>
        </section>
    </main>

    <footer>
        <p>Press Ctrl+C in terminal to stop</p>
    </footer>

    <script>
        // Long-polling for real-time updates
        let lastCounter = null;
        let polling = true;

        // Unwrap API envelope - all API responses are { success, data?, error? }
        function unwrap(response) {
            if (!response.success) {
                console.error('API error:', response.error);
                return null;
            }
            return response.data;
        }

        async function poll() {
            const statusEl = document.getElementById('connection-status');

            while (polling) {
                try {
                    statusEl.textContent = 'live';
                    statusEl.className = 'connected';

                    // First request: get current counter without waiting
                    // Subsequent requests: long-poll with since=N
                    const url = lastCounter === null
                        ? '/api/v1/events'
                        : `/api/v1/events?since=${lastCounter}`;

                    const response = await fetch(url);
                    const envelope = await response.json();
                    const data = unwrap(envelope);

                    if (data && data.changed) {
                        // Trigger HTMX refresh
                        document.body.dispatchEvent(new CustomEvent('refresh'));
                    }

                    // Always update counter
                    if (data) lastCounter = data.counter;
                } catch (e) {
                    statusEl.textContent = 'reconnecting...';
                    statusEl.className = 'disconnected';
                    // Wait before retrying on error
                    await new Promise(r => setTimeout(r, 2000));
                }
            }
        }

        // Start polling
        poll();

        // Transform JSON responses into HTML
        document.body.addEventListener('htmx:beforeSwap', function(evt) {
            const target = evt.detail.target;
            try {
                const envelope = JSON.parse(evt.detail.xhr.responseText);
                const data = unwrap(envelope);

                if (!data) {
                    evt.detail.serverResponse = '<p class="error">Error loading data</p>';
                    return;
                }

                if (target.id === 'status') {
                    evt.detail.serverResponse = renderStatus(data);
                } else if (target.id === 'tasks') {
                    evt.detail.serverResponse = renderTasks(data);
                } else if (target.id === 'checks') {
                    evt.detail.serverResponse = renderChecks(data);
                }
            } catch (e) {
                // Not JSON, use as-is
            }
        });

        function renderStatus(data) {
            document.getElementById('branch').textContent = data.branch || '(not in git repo)';
            const current = data.current_task
                ? `<p><strong>Current task:</strong> ${data.current_task}</p>`
                : '';
            return `
                ${current}
                <div class="stats">
                    <div class="stat"><span class="num">${data.tasks.total}</span> tasks</div>
                    <div class="stat"><span class="num">${data.tasks.in_progress}</span> in progress</div>
                    <div class="stat"><span class="num">${data.tasks.pending}</span> pending</div>
                    <div class="stat"><span class="num">${data.tasks.done}</span> done</div>
                    <div class="stat"><span class="num">${data.checks}</span> checks</div>
                </div>
            `;
        }

        function renderTasks(data) {
            if (!data.tasks || data.tasks.length === 0) {
                return '<p class="empty">No tasks</p>';
            }
            return data.tasks.map(t => {
                const statusIcon = t.status === 'done' ? '&#x2713;'
                    : t.status === 'in_progress' ? '&#x25CF;'
                    : t.ready ? '&#x25CB;' : '&#x2298;';
                const current = t.current ? ' current' : '';
                const blocked = t.blocked_by && t.blocked_by.length > 0
                    ? `<span class="blocked">(blocked by: ${t.blocked_by.join(', ')})</span>`
                    : '';
                const actions = t.status === 'pending' && t.ready
                    ? `<button hx-post="/api/v1/tasks/${t.id}/start" hx-swap="none" hx-on::after-request="htmx.trigger('#tasks', 'load'); htmx.trigger('#status', 'load')">Start</button>`
                    : t.status === 'in_progress'
                    ? `<button hx-post="/api/v1/tasks/${t.id}/done" hx-swap="none" hx-on::after-request="htmx.trigger('#tasks', 'load'); htmx.trigger('#status', 'load')">Done</button>`
                    : '';
                return `
                    <div class="task ${t.status}${current}">
                        <span class="status-icon">${statusIcon}</span>
                        <span class="id">[${t.id}]</span>
                        <span class="title">${t.title}</span>
                        ${blocked}
                        <span class="actions">${actions}</span>
                    </div>
                `;
            }).join('');
        }

        function renderChecks(data) {
            if (!data.checks || data.checks.length === 0) {
                return '<p class="empty">No checks configured</p>';
            }
            return data.checks.map(c => `
                <div class="check ${c.severity}">
                    <span class="id">[${c.id}]</span>
                    <span class="target">${c.target}</span>
                    <span class="message">${c.message}</span>
                    <span class="severity">${c.severity}</span>
                </div>
            `).join('');
        }
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
    min-height: 100vh;
    padding: 2rem;
}

header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--primary);
}

h1 {
    font-size: 1.5rem;
    color: var(--accent);
}

#branch {
    color: var(--text-dim);
}

#connection-status {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
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

main {
    display: grid;
    gap: 2rem;
}

section {
    background: var(--surface);
    padding: 1.5rem;
    border-radius: 8px;
}

h2 {
    font-size: 1rem;
    color: var(--text-dim);
    margin-bottom: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
}

.stats {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
}

.stat {
    color: var(--text-dim);
}

.stat .num {
    font-size: 1.5rem;
    color: var(--text);
    margin-right: 0.5rem;
}

.task, .check {
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    background: var(--primary);
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.task.current {
    border-left: 3px solid var(--accent);
}

.task.done {
    opacity: 0.6;
}

.status-icon {
    font-size: 1rem;
}

.task.done .status-icon { color: var(--success); }
.task.in_progress .status-icon { color: var(--warning); }

.id {
    color: var(--text-dim);
    font-size: 0.875rem;
}

.title {
    flex: 1;
}

.blocked {
    color: var(--text-dim);
    font-size: 0.875rem;
}

.actions {
    margin-left: auto;
}

button {
    background: var(--accent);
    color: var(--text);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    font-size: 0.875rem;
}

button:hover {
    opacity: 0.9;
}

.check {
    border-left: 3px solid var(--text-dim);
}

.check.block { border-color: var(--accent); }
.check.warn { border-color: var(--warning); }
.check.info { border-color: var(--success); }

.check .target {
    color: var(--accent);
}

.check .message {
    flex: 1;
}

.check .severity {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg);
    border-radius: 4px;
    text-transform: uppercase;
}

.empty {
    color: var(--text-dim);
    font-style: italic;
}

/* Forms */
form {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--primary);
}

form input, form select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--primary);
    padding: 0.5rem;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.875rem;
}

form input[type="text"] {
    flex: 1;
}

form input:focus, form select:focus {
    outline: none;
    border-color: var(--accent);
}

form button[type="submit"] {
    background: var(--success);
    color: var(--bg);
}

footer {
    margin-top: 2rem;
    text-align: center;
    color: var(--text-dim);
    font-size: 0.875rem;
}

@media (min-width: 768px) {
    main {
        grid-template-columns: 1fr 1fr;
    }

    #status-section {
        grid-column: span 2;
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
