//! Local Web UI command
//!
//! Provides a local HTTP server for managing tasks and viewing checks.

use std::io::Cursor;

use tiny_http::{Header, Method, Response, Server};

use crate::noslop_file;
use noslop::storage::TaskRefs;

/// Start the local web UI server
pub fn ui(port: u16, open: bool) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let server = Server::http(&addr).map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;

    println!("Starting noslop UI...");
    println!("Open http://localhost:{port} in your browser");
    println!();
    println!("Press Ctrl+C to stop");

    if open {
        // Try to open browser
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

    for request in server.incoming_requests() {
        let response = handle_request(&request);
        let _ = request.respond(response);
    }

    Ok(())
}

fn handle_request(request: &tiny_http::Request) -> Response<Cursor<Vec<u8>>> {
    let path = request.url();
    let method = request.method();

    match (method, path) {
        // Static pages
        (&Method::Get, "/") => serve_html(INDEX_HTML),
        (&Method::Get, "/style.css") => serve_css(STYLE_CSS),

        // REST API
        (&Method::Get, "/api/status") => api_status(),
        (&Method::Get, "/api/tasks") => api_tasks(),
        (&Method::Get, "/api/checks") => api_checks(),

        // API routes with path params
        _ if method == &Method::Get && path.starts_with("/api/tasks/") => {
            let id = path.strip_prefix("/api/tasks/").unwrap_or("");
            api_task_show(id)
        },
        _ if method == &Method::Post
            && path.starts_with("/api/tasks/")
            && path.ends_with("/start") =>
        {
            let id = path
                .strip_prefix("/api/tasks/")
                .and_then(|s| s.strip_suffix("/start"))
                .unwrap_or("");
            api_task_start(id)
        },
        _ if method == &Method::Post
            && path.starts_with("/api/tasks/")
            && path.ends_with("/done") =>
        {
            let id = path
                .strip_prefix("/api/tasks/")
                .and_then(|s| s.strip_suffix("/done"))
                .unwrap_or("");
            api_task_done(id)
        },

        // 404
        _ => not_found(),
    }
}

// =============================================================================
// Response helpers
// =============================================================================

fn serve_html(content: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(content.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

fn serve_css(content: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(content.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/css; charset=utf-8").unwrap())
}

fn json_response(json: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(json.to_string().into_bytes())
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn not_found() -> Response<Cursor<Vec<u8>>> {
    Response::from_data(b"Not Found".to_vec()).with_status_code(404)
}

// =============================================================================
// API handlers
// =============================================================================

fn api_status() -> Response<Cursor<Vec<u8>>> {
    let branch = get_current_branch();
    let tasks = TaskRefs::list().unwrap_or_default();
    let current = TaskRefs::current().ok().flatten();
    let checks = load_check_count();

    let pending = tasks.iter().filter(|(_, t)| t.status == "pending").count();
    let in_progress = tasks.iter().filter(|(_, t)| t.status == "in_progress").count();
    let done = tasks.iter().filter(|(_, t)| t.status == "done").count();

    json_response(serde_json::json!({
        "branch": branch,
        "current_task": current,
        "tasks": {
            "total": tasks.len(),
            "pending": pending,
            "in_progress": in_progress,
            "done": done
        },
        "checks": checks
    }))
}

fn api_tasks() -> Response<Cursor<Vec<u8>>> {
    let tasks = TaskRefs::list().unwrap_or_default();
    let current = TaskRefs::current().ok().flatten();

    let json_tasks: Vec<_> = tasks
        .iter()
        .map(|(id, t)| {
            serde_json::json!({
                "id": id,
                "title": t.title,
                "status": t.status,
                "priority": t.priority,
                "blocked_by": t.blocked_by,
                "current": current.as_ref() == Some(id),
                "ready": t.is_ready(&tasks),
            })
        })
        .collect();

    json_response(serde_json::json!({ "tasks": json_tasks }))
}

fn api_task_show(id: &str) -> Response<Cursor<Vec<u8>>> {
    let tasks = TaskRefs::list().unwrap_or_default();

    if let Ok(Some(task)) = TaskRefs::get(id) {
        let current = TaskRefs::current().ok().flatten();
        json_response(serde_json::json!({
            "found": true,
            "id": id,
            "title": task.title,
            "status": task.status,
            "priority": task.priority,
            "blocked_by": task.blocked_by,
            "ready": task.is_ready(&tasks),
            "current": current.as_deref() == Some(id),
            "created_at": task.created_at,
            "notes": task.notes,
        }))
    } else {
        json_response(serde_json::json!({ "found": false, "id": id }))
    }
}

fn api_task_start(id: &str) -> Response<Cursor<Vec<u8>>> {
    if TaskRefs::get(id).ok().flatten().is_none() {
        return json_response(serde_json::json!({ "success": false, "error": "Task not found" }));
    }

    if let Err(e) = TaskRefs::set_status(id, "in_progress") {
        return json_response(serde_json::json!({ "success": false, "error": e.to_string() }));
    }

    if let Err(e) = TaskRefs::set_current(id) {
        return json_response(serde_json::json!({ "success": false, "error": e.to_string() }));
    }

    json_response(serde_json::json!({
        "success": true,
        "id": id,
        "status": "in_progress"
    }))
}

fn api_task_done(id: &str) -> Response<Cursor<Vec<u8>>> {
    if TaskRefs::get(id).ok().flatten().is_none() {
        return json_response(serde_json::json!({ "success": false, "error": "Task not found" }));
    }

    if let Err(e) = TaskRefs::set_status(id, "done") {
        return json_response(serde_json::json!({ "success": false, "error": e.to_string() }));
    }

    // Clear current if this was the current task
    if TaskRefs::current().ok().flatten().as_deref() == Some(id) {
        let _ = TaskRefs::clear_current();
    }

    json_response(serde_json::json!({
        "success": true,
        "id": id,
        "status": "done"
    }))
}

fn api_checks() -> Response<Cursor<Vec<u8>>> {
    let path = std::path::Path::new(".noslop.toml");
    if !path.exists() {
        return json_response(serde_json::json!({ "checks": [] }));
    }

    if let Ok(file) = noslop_file::load_file(path) {
        let checks: Vec<_> = file
            .checks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "target": c.target,
                    "message": c.message,
                    "severity": c.severity,
                })
            })
            .collect();
        json_response(serde_json::json!({ "checks": checks }))
    } else {
        json_response(serde_json::json!({ "checks": [] }))
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn get_current_branch() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn load_check_count() -> usize {
    let path = std::path::Path::new(".noslop.toml");
    if !path.exists() {
        return 0;
    }
    noslop_file::load_file(path).map(|f| f.checks.len()).unwrap_or(0)
}

// =============================================================================
// Embedded static files
// =============================================================================

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>noslop</title>
    <link rel="stylesheet" href="/style.css">
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
</head>
<body>
    <header>
        <h1>noslop</h1>
        <div id="branch"></div>
    </header>

    <main>
        <section id="status-section">
            <h2>Status</h2>
            <div id="status" hx-get="/api/status" hx-trigger="load, every 5s" hx-swap="innerHTML">
                Loading...
            </div>
        </section>

        <section id="tasks-section">
            <h2>Tasks</h2>
            <div id="tasks" hx-get="/api/tasks" hx-trigger="load, every 5s" hx-swap="innerHTML">
                Loading...
            </div>
        </section>

        <section id="checks-section">
            <h2>Checks</h2>
            <div id="checks" hx-get="/api/checks" hx-trigger="load" hx-swap="innerHTML">
                Loading...
            </div>
        </section>
    </main>

    <footer>
        <p>Press Ctrl+C in terminal to stop</p>
    </footer>

    <script>
        // Transform JSON responses into HTML
        document.body.addEventListener('htmx:beforeSwap', function(evt) {
            const target = evt.detail.target;
            try {
                const data = JSON.parse(evt.detail.xhr.responseText);

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
                    ? `<button hx-post="/api/tasks/${t.id}/start" hx-swap="none" hx-on::after-request="htmx.trigger('#tasks', 'load'); htmx.trigger('#status', 'load')">Start</button>`
                    : t.status === 'in_progress'
                    ? `<button hx-post="/api/tasks/${t.id}/done" hx-swap="none" hx-on::after-request="htmx.trigger('#tasks', 'load'); htmx.trigger('#status', 'load')">Done</button>`
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
