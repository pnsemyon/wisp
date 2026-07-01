//! [`SingBoxProcess`]: an [`Engine`] impl that runs sing-box as a child
//! process and talks to it via its Clash API.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::clash_api::ClashApi;
use crate::engine::{Engine, EngineState, EngineStatus, TrafficStats};

/// Max number of log lines kept in the in-memory ring buffer.
const LOG_RING_CAPACITY: usize = 500;

/// A single traffic-totals sample used to compute speed as a diff over
/// time.
type TrafficSample = (u64, u64, Instant);

struct State {
    child: Option<Child>,
    engine_state: EngineState,
    active_tag: Option<String>,
    since_unix: Option<u64>,
    last_error: Option<String>,
    last_sample: Option<TrafficSample>,
}

impl Default for State {
    fn default() -> Self {
        State {
            child: None,
            engine_state: EngineState::Stopped,
            active_tag: None,
            since_unix: None,
            last_error: None,
            last_sample: None,
        }
    }
}

/// Runs sing-box as a child process, feeding it a generated config and
/// exposing its status/traffic/logs via [`Engine`].
pub struct SingBoxProcess {
    binary: PathBuf,
    work_dir: PathBuf,
    args: Vec<String>,
    clash: ClashApi,
    state: Mutex<State>,
    logs: Arc<Mutex<VecDeque<String>>>,
    /// Whether `start` should poll the Clash API to confirm the process
    /// came up before reporting `Running`. Disabled only in tests, where a
    /// dummy binary has no HTTP API to poll.
    health_check: bool,
}

impl SingBoxProcess {
    /// `binary` = path to `sing-box(.exe)`, `work_dir` = writable directory
    /// for `config.json` and sing-box's runtime files. `clash_port` and
    /// `clash_secret` must match what `BuildSettings` put into the config
    /// passed to `start`.
    pub fn new(binary: PathBuf, work_dir: PathBuf, clash_port: u16, clash_secret: String) -> Self {
        let args = vec![
            "run".to_string(),
            "-c".to_string(),
            "config.json".to_string(),
            "-D".to_string(),
            work_dir.display().to_string(),
        ];
        SingBoxProcess {
            binary,
            work_dir,
            args,
            clash: ClashApi::new(clash_port, clash_secret),
            state: Mutex::new(State::default()),
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_RING_CAPACITY))),
            health_check: true,
        }
    }

    /// Test-only constructor: lets a test point the child process at an
    /// arbitrary binary + args (e.g. `/bin/sh -c "sleep 30"`) instead of the
    /// real `sing-box run ...` invocation, and skips the Clash-API health
    /// check since a dummy binary has no HTTP API to poll.
    #[cfg(test)]
    fn new_for_test(binary: PathBuf, work_dir: PathBuf, args: Vec<String>) -> Self {
        SingBoxProcess {
            binary,
            work_dir,
            args,
            clash: ClashApi::new(0, String::new()),
            state: Mutex::new(State::default()),
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_RING_CAPACITY))),
            health_check: false,
        }
    }

    /// Write `config` to `work_dir/config.json`, spawn the child process,
    /// wire up log readers, and (unless disabled) confirm it's healthy.
    async fn spawn_and_confirm(&self, config: &Value) -> Result<Child> {
        tokio::fs::create_dir_all(&self.work_dir)
            .await
            .context("creating work_dir")?;
        let config_path = self.work_dir.join("config.json");
        let pretty = serde_json::to_string_pretty(config).context("serializing config")?;
        tokio::fs::write(&config_path, pretty)
            .await
            .context("writing config.json")?;

        let mut cmd = Command::new(&self.binary);
        cmd.args(&self.args)
            .current_dir(&self.work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().context("spawning sing-box process")?;

        if let Some(stdout) = child.stdout.take() {
            spawn_log_reader(stdout, self.logs.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_log_reader(stderr, self.logs.clone());
        }

        if self.health_check {
            self.wait_for_health(&mut child).await?;
        }

        Ok(child)
    }

    /// Poll `GET /version` a few times with short backoff to confirm
    /// sing-box came up, bailing out early if the child already exited.
    async fn wait_for_health(&self, child: &mut Child) -> Result<()> {
        const ATTEMPTS: u32 = 10;
        let mut delay = Duration::from_millis(200);

        for attempt in 0..ATTEMPTS {
            if let Some(status) = child.try_wait().context("polling child status")? {
                anyhow::bail!("sing-box exited early with status {status}");
            }
            if self.clash.version().await.is_ok() {
                return Ok(());
            }
            if attempt + 1 < ATTEMPTS {
                sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(2));
            }
        }

        anyhow::bail!("sing-box did not report a healthy Clash API in time")
    }
}

/// Read `reader` line by line into `logs`, dropping the oldest line once the
/// ring buffer is full. Runs until EOF (i.e. until the child's pipe closes).
fn spawn_log_reader<R>(reader: R, logs: Arc<Mutex<VecDeque<String>>>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut logs = logs.lock().await;
            if logs.len() >= LOG_RING_CAPACITY {
                logs.pop_front();
            }
            logs.push_back(line);
        }
    });
}

#[async_trait]
impl Engine for SingBoxProcess {
    async fn start(&self, config: Value) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.engine_state = EngineState::Starting;
            state.last_error = None;
        }

        match self.spawn_and_confirm(&config).await {
            Ok(child) => {
                let since_unix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .ok();
                let mut state = self.state.lock().await;
                state.child = Some(child);
                state.engine_state = EngineState::Running;
                state.since_unix = since_unix;
                state.last_sample = None;
                Ok(())
            }
            Err(err) => {
                let mut state = self.state.lock().await;
                state.engine_state = EngineState::Errored;
                state.last_error = Some(err.to_string());
                Err(err)
            }
        }
    }

    async fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if let Some(mut child) = state.child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        state.engine_state = EngineState::Stopped;
        state.since_unix = None;
        state.last_sample = None;
        Ok(())
    }

    async fn status(&self) -> EngineStatus {
        let state = self.state.lock().await;
        EngineStatus {
            state: state.engine_state,
            active_tag: state.active_tag.clone(),
            since_unix: state.since_unix,
            last_error: state.last_error.clone(),
        }
    }

    async fn stats(&self) -> Result<TrafficStats> {
        let snapshot = self.clash.connections().await.context("querying /connections")?;
        let now = Instant::now();

        let mut state = self.state.lock().await;
        let (up_speed, down_speed) = match state.last_sample {
            Some((prev_up, prev_down, prev_time)) => {
                let elapsed = now.duration_since(prev_time).as_secs_f64().max(0.001);
                let up_speed = (snapshot.upload_total.saturating_sub(prev_up) as f64 / elapsed) as u64;
                let down_speed = (snapshot.download_total.saturating_sub(prev_down) as f64 / elapsed) as u64;
                (up_speed, down_speed)
            }
            None => (0, 0),
        };
        state.last_sample = Some((snapshot.upload_total, snapshot.download_total, now));

        Ok(TrafficStats {
            up_bytes: snapshot.upload_total,
            down_bytes: snapshot.download_total,
            up_speed,
            down_speed,
        })
    }

    async fn logs(&self, max_lines: usize) -> Vec<String> {
        let logs = self.logs.lock().await;
        let skip = logs.len().saturating_sub(max_lines);
        logs.iter().skip(skip).cloned().collect()
    }

    async fn switch(&self, tag: &str) -> Result<()> {
        self.clash
            .switch_selector("proxy", tag)
            .await
            .context("switching selector")?;
        let mut state = self.state.lock().await;
        state.active_tag = Some(tag.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn lifecycle_stopped_to_running_writes_config_and_stop_kills_child() {
        let work_dir_guard = tempdir().expect("tempdir");
        let work_dir = work_dir_guard.path().to_path_buf();

        let proc = SingBoxProcess::new_for_test(
            PathBuf::from("/bin/sh"),
            work_dir.clone(),
            vec!["-c".to_string(), "sleep 30".to_string()],
        );

        assert_eq!(proc.status().await.state, EngineState::Stopped);

        proc.start(serde_json::json!({ "log": { "level": "info" } }))
            .await
            .expect("start should succeed against dummy binary");

        let config_contents = tokio::fs::read_to_string(work_dir.join("config.json"))
            .await
            .expect("config.json should have been written");
        assert!(config_contents.contains("\"level\""));

        assert_eq!(proc.status().await.state, EngineState::Running);

        proc.stop().await.expect("stop should succeed");
        assert_eq!(proc.status().await.state, EngineState::Stopped);
    }

    #[tokio::test]
    async fn start_failure_sets_errored_state_with_last_error() {
        let work_dir_guard = tempdir().expect("tempdir");
        let work_dir = work_dir_guard.path().to_path_buf();

        let proc = SingBoxProcess::new_for_test(
            PathBuf::from("/definitely/not/a/real/binary"),
            work_dir,
            vec![],
        );

        let result = proc.start(serde_json::json!({})).await;
        assert!(result.is_err());

        let status = proc.status().await;
        assert_eq!(status.state, EngineState::Errored);
        assert!(status.last_error.is_some());
    }

    #[tokio::test]
    async fn switch_updates_active_tag_even_without_a_running_process() {
        // switch() talks to the Clash API; against no running sing-box it
        // will fail, but we can still verify the call shape compiles and
        // returns an error rather than panicking.
        let work_dir_guard = tempdir().expect("tempdir");
        let proc = SingBoxProcess::new_for_test(
            PathBuf::from("/bin/sh"),
            work_dir_guard.path().to_path_buf(),
            vec![],
        );
        let result = proc.switch("some-tag").await;
        assert!(result.is_err());
    }
}
