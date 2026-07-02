//! Agent CLI runner adapter
//!
//! Runs mining prompts through whatever agent CLI the developer already has
//! installed. noslop owns no LLM access: the runner is a subprocess with a
//! one-line contract — read the prompt on stdin, print the answer on stdout.
//!
//! Detection order: explicit `[discover] runner` config, then `claude` on
//! PATH (invoked as `claude -p`). Other agents work via the config template.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Max wall-clock time for one runner invocation.
const RUNNER_TIMEOUT: Duration = Duration::from_mins(5);

/// A resolved agent CLI command
#[derive(Debug, Clone)]
pub struct Runner {
    argv: Vec<String>,
}

impl Runner {
    /// Resolve a runner from config, falling back to PATH detection.
    ///
    /// Returns `None` when no agent CLI is available.
    #[must_use]
    pub fn detect(config_runner: Option<&str>) -> Option<Self> {
        if let Some(template) = config_runner {
            let argv: Vec<String> = template.split_whitespace().map(str::to_string).collect();
            if argv.is_empty() {
                return None;
            }
            return Some(Self { argv });
        }
        if find_on_path("claude") {
            return Some(Self {
                argv: vec!["claude".to_string(), "-p".to_string()],
            });
        }
        None
    }

    /// Human-readable command line, for status output.
    #[must_use]
    pub fn describe(&self) -> String {
        self.argv.join(" ")
    }

    /// Run the prompt through the agent CLI and return its stdout.
    ///
    /// The child runs in a scratch directory (never the repo) with the
    /// prompt piped to stdin, and is killed after a hard timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be spawned, exits non-zero,
    /// times out, or produces unreadable output.
    pub fn run(&self, prompt: &str) -> anyhow::Result<String> {
        let scratch = std::env::temp_dir().join(format!("noslop-runner-{}", std::process::id()));
        std::fs::create_dir_all(&scratch)?;

        // The child runs in a scratch dir, so a repo-relative runner path
        // ("./scripts/agent.sh") must be resolved before we change cwd.
        let program = if Path::new(&self.argv[0]).is_relative() && self.argv[0].contains('/') {
            std::env::current_dir()?.join(&self.argv[0]).into_os_string()
        } else {
            self.argv[0].clone().into()
        };

        let mut child = Command::new(&program)
            .args(&self.argv[1..])
            .current_dir(&scratch)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to start runner '{}': {e}", self.describe()))?;

        // Feed the prompt from a thread so a slow-reading child can't
        // deadlock against a full stdin pipe.
        let Some(mut stdin) = child.stdin.take() else {
            anyhow::bail!("runner stdin unavailable");
        };
        let prompt_owned = prompt.to_string();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(prompt_owned.as_bytes());
            // stdin drops here, closing the pipe
        });

        let Some(mut stdout) = child.stdout.take() else {
            anyhow::bail!("runner stdout unavailable");
        };
        let reader = std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stdout.read_to_string(&mut buf);
            buf
        });

        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if start.elapsed() > RUNNER_TIMEOUT {
                child.kill()?;
                anyhow::bail!(
                    "runner '{}' timed out after {}s",
                    self.describe(),
                    RUNNER_TIMEOUT.as_secs()
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        };

        writer.join().ok();
        let output = reader.join().unwrap_or_default();

        if !status.success() {
            let mut stderr_buf = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut stderr_buf);
            }
            anyhow::bail!(
                "runner '{}' exited with {status}: {}",
                self.describe(),
                stderr_buf.trim()
            );
        }
        Ok(output)
    }
}

/// Is `bin` an executable on PATH?
fn find_on_path(bin: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| is_executable(&dir.join(bin)))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file() && path.metadata().is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
