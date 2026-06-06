//! Host-process helpers shared by the `local` backend: spawning with a
//! readiness probe and recursive directory copy (the local snapshot
//! primitive).

use crate::error::BackendError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// How long to wait for the readiness marker before giving up.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Outcome of watching stdout for the readiness marker.
enum Ready {
    /// Marker seen; carries the parsed endpoint (if the line had `port=`).
    Found(Option<String>),
    /// Stdout closed before the marker — the process exited early.
    Eof,
}

/// Spawn `argv` in `workdir` with `env`, then (when `marker` is non-empty)
/// read stdout until a line contains it, deriving the serving endpoint from
/// a `port=<n>` token if present. Remaining stdout is drained in the
/// background so the child never blocks on a full pipe.
pub async fn spawn_ready(
    backend: &'static str,
    argv: &[String],
    workdir: &Path,
    env: &[(String, String)],
    marker: &str,
) -> Result<(Child, Option<String>), BackendError> {
    let (program, args) = argv.split_first().ok_or_else(|| BackendError::Start {
        backend,
        detail: "empty launch command".into(),
    })?;

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(workdir)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }

    // Only capture stdout when we actually need to watch for readiness.
    let watch = !marker.is_empty();
    cmd.stdout(if watch { Stdio::piped() } else { Stdio::null() });

    let mut child = cmd.spawn().map_err(|e| BackendError::Start {
        backend,
        detail: format!("spawn `{program}` failed: {e}"),
    })?;

    if !watch {
        return Ok((child, None));
    }

    let stdout = child.stdout.take().ok_or_else(|| BackendError::Start {
        backend,
        detail: "child stdout unavailable".into(),
    })?;
    let mut lines = BufReader::new(stdout).lines();
    let needle = marker.to_string();

    let ready = tokio::time::timeout(READY_TIMEOUT, async {
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if line.contains(&needle) => {
                    return Ok(Ready::Found(parse_endpoint(&line)))
                }
                Ok(Some(_)) => continue,
                Ok(None) => return Ok(Ready::Eof),
                Err(e) => return Err(e), // surface real read errors, don't swallow
            }
        }
    })
    .await
    .map_err(|_| BackendError::Timeout {
        backend,
        op: "start",
        secs: READY_TIMEOUT.as_secs(),
    })?
    .map_err(|e| BackendError::Start {
        backend,
        detail: format!("reading stdout: {e}"),
    })?;

    let endpoint = match ready {
        Ready::Found(ep) => ep,
        Ready::Eof => {
            return Err(BackendError::Start {
                backend,
                detail: "process exited before readiness marker".into(),
            })
        }
    };

    // Keep draining stdout so the engine never blocks writing logs. The child
    // is reaped by kill_on_drop, which closes stdout and ends this task.
    tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });

    Ok((child, endpoint))
}

/// Pull `http://127.0.0.1:<port>` out of a banner line carrying `port=<n>`.
fn parse_endpoint(line: &str) -> Option<String> {
    let after = line.split("port=").nth(1)?;
    let port: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    (!port.is_empty()).then(|| format!("http://127.0.0.1:{port}"))
}

/// Recursively copy `src` into `dst` on a blocking thread (the local
/// snapshot/restore primitive). Off the async runtime so a large tree never
/// parks a tokio worker.
pub async fn copy_tree(
    backend: &'static str,
    op: fn(&'static str, String) -> BackendError,
    src: PathBuf,
    dst: PathBuf,
) -> Result<(), BackendError> {
    tokio::task::spawn_blocking(move || copy_dir(&src, &dst))
        .await
        .map_err(|e| op(backend, format!("copy task panicked: {e}")))?
        .map_err(|e| op(backend, format!("copy tree: {e}")))
}

/// Recursive directory copy (sync; call via [`copy_tree`] from async code).
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_port_from_banner() {
        let line = "HOUSTON_ENGINE_LISTENING port=54032 token=abc";
        assert_eq!(
            parse_endpoint(line).as_deref(),
            Some("http://127.0.0.1:54032")
        );
    }

    #[test]
    fn no_port_yields_none() {
        assert_eq!(parse_endpoint("ready, no port here"), None);
    }
}
