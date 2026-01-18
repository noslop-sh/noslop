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

use include_dir::{Dir, include_dir};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tiny_http::{Header, Method, Response, Server};

use crate::server::tiny_http::handle_api_request;
use noslop::api::{ApiResponse, EventsData};

/// Embedded Svelte UI build
static UI_BUILD: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/build");

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

        // API routes - delegate to server adapter
        let response = if path.starts_with("/api") {
            handle_api_request(&mut request)
        } else {
            // Static file serving from embedded Svelte build
            match &method {
                &Method::Get => serve_static(&path),
                _ => not_found(),
            }
        };

        let _ = request.respond(response);
    }

    Ok(())
}

// =============================================================================
// STATIC FILE SERVING
// =============================================================================

fn serve_static(path: &str) -> Response<Cursor<Vec<u8>>> {
    // Normalize path: remove leading slash, default to index.html
    let file_path = if path == "/" || path.is_empty() {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    // Try to get the file from the embedded directory
    if let Some(file) = UI_BUILD.get_file(file_path) {
        let content = file.contents().to_vec();
        let content_type = guess_mime_type(file_path);
        Response::from_data(content)
            .with_header(Header::from_bytes("Content-Type", content_type).unwrap())
    } else {
        // For SPA routing, serve index.html for any unmatched path
        // (SvelteKit handles client-side routing)
        if let Some(index) = UI_BUILD.get_file("index.html") {
            let content = index.contents().to_vec();
            Response::from_data(content).with_header(
                Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap(),
            )
        } else {
            not_found()
        }
    }
}

fn guess_mime_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
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
    use noslop::paths;

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

    // Watch .noslop directory for task changes (in main worktree)
    let noslop_dir = paths::noslop_dir();
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
    let noslop_toml = paths::noslop_toml();
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

    #[test]
    fn test_embedded_ui_contains_index() {
        // Verify the embedded UI contains index.html
        assert!(UI_BUILD.get_file("index.html").is_some(), "UI build should contain index.html");
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(guess_mime_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(guess_mime_type("app.js"), "application/javascript; charset=utf-8");
        assert_eq!(guess_mime_type("style.css"), "text/css; charset=utf-8");
        assert_eq!(guess_mime_type("data.json"), "application/json; charset=utf-8");
    }
}
