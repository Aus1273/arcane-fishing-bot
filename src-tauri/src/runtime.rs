//! Session ownership and native adapters. Observe-only never constructs an input device.
use crate::{
    capture,
    config::BotConfig,
    diagnostics::{self, Needs},
    engine::{Action, Controller, Observation, Phase},
    input::{self, NativeInput},
    readiness,
    recording::{RecordingFrame, SessionRecording},
    storage::LifetimeStats,
};
use anyhow::{anyhow, Result};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    #[default]
    Automate,
    Observe,
}
#[derive(Clone, Default, Serialize)]
pub struct SessionMetrics {
    pub capture_ms: u64,
    pub analysis_ms: u64,
    pub frame_age_ms: u64,
    pub frames: u64,
}
#[derive(Clone, Serialize)]
pub struct SessionEvent {
    pub at_ms: u64,
    pub phase: Phase,
    pub reason: String,
    pub actions: Vec<Action>,
    pub observation: Option<Observation>,
}
#[derive(Clone, Serialize)]
pub struct SessionState {
    pub running: bool,
    pub mode: SessionMode,
    pub controller: Controller,
    pub elapsed_ms: u64,
    pub observation: Option<Observation>,
    pub events: VecDeque<SessionEvent>,
    pub metrics: SessionMetrics,
    pub recording_enabled: bool,
    pub recording_frames: usize,
}
impl SessionState {
    fn new(config: BotConfig, mode: SessionMode, record: bool) -> Self {
        Self {
            running: false,
            mode,
            controller: Controller::new(config),
            elapsed_ms: 0,
            observation: None,
            events: VecDeque::new(),
            metrics: SessionMetrics::default(),
            recording_enabled: record,
            recording_frames: 0,
        }
    }
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub stats: LifetimeStats,
    pub session: SessionState,
}
#[derive(Default)]
struct Activity {
    control: Option<JoinHandle<()>>,
    observer: Option<JoinHandle<()>>,
    inspecting: bool,
}
#[derive(Clone)]
pub struct SharedState {
    pub config: Arc<RwLock<BotConfig>>,
    pub stats: Arc<RwLock<LifetimeStats>>,
    pub session: Arc<RwLock<SessionState>>,
    pub recording: Arc<RwLock<Option<SessionRecording>>>,
    pub shortcut_ready: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    shutting_down: Arc<AtomicBool>,
    stop_generation: Arc<AtomicU64>,
    activity: Arc<Mutex<Activity>>,
}
pub struct InspectionGuard(SharedState);
impl Drop for InspectionGuard {
    fn drop(&mut self) {
        self.0.activity.lock().inspecting = false;
    }
}

/// Keeping construction behind this gate prevents permission prompts and input in observe mode.
fn input_for_mode<T>(mode: SessionMode, create: impl FnOnce() -> Result<T>) -> Result<Option<T>> {
    match mode {
        SessionMode::Automate => create().map(Some),
        SessionMode::Observe => Ok(None),
    }
}
impl SharedState {
    pub fn new() -> Result<Self> {
        Ok(Self::from_saved(BotConfig::load()?, LifetimeStats::load()?))
    }
    fn from_saved(config: BotConfig, stats: LifetimeStats) -> Self {
        let mut session = SessionState::new(config.clone(), SessionMode::Automate, false);
        session.controller.phase = Phase::Stopped;
        session.controller.reason = "Idle".into();
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(stats)),
            session: Arc::new(RwLock::new(session)),
            recording: Arc::new(RwLock::new(None)),
            shortcut_ready: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(true)),
            shutting_down: Arc::new(AtomicBool::new(false)),
            stop_generation: Arc::new(AtomicU64::new(0)),
            activity: Arc::new(Mutex::new(Activity::default())),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let stats = self.stats.read().clone();
        let session = self.session.read().clone();
        Snapshot { stats, session }
    }
    pub fn reserve_inspection(&self) -> Result<InspectionGuard> {
        let mut activity = self.activity.lock();
        if Self::busy(&activity) || self.session.read().running {
            return Err(anyhow!(
                "Stop the session, close the overlay and wait for capture to finish first"
            ));
        }
        activity.inspecting = true;
        Ok(InspectionGuard(self.clone()))
    }
    fn busy(activity: &Activity) -> bool {
        activity.inspecting
            || activity.control.as_ref().is_some_and(|h| !h.is_finished())
            || activity.observer.as_ref().is_some_and(|h| !h.is_finished())
    }
    pub fn save_config(&self, config: BotConfig) -> Result<()> {
        let _guard = self.reserve_inspection()?;
        config.validate()?;
        config.save()?;
        *self.config.write() = config;
        Ok(())
    }
    pub fn stop(&self) {
        self.stop_generation.fetch_add(1, Ordering::SeqCst);
        self.cancelled.store(true, Ordering::SeqCst);
    }
    pub fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        self.stop();
        let handle = self.activity.lock().control.take();
        self.stop();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
        // OS capture may be uninterruptible; the observer owns no input.
    }
    pub fn start(&self, emit: Arc<dyn Fn(Snapshot) + Send + Sync>) -> Result<()> {
        self.start_mode(SessionMode::Automate, false, emit)
    }
    /// Capture this before queueing startup work so a later Stop invalidates it.
    pub fn start_ticket(&self) -> u64 {
        self.stop_generation.load(Ordering::SeqCst)
    }
    /// Keep the cancellation boundary testable without initializing capture or native input.
    fn preflight(
        &self,
        config: &BotConfig,
        mode: SessionMode,
        ticket: u64,
        check: impl FnOnce(&BotConfig, SessionMode, bool) -> readiness::Readiness,
    ) -> Result<()> {
        // Stop during preflight must not be overwritten when workers start.
        self.cancelled.store(false, Ordering::SeqCst);
        if ticket != self.start_ticket() || self.shutting_down.load(Ordering::SeqCst) {
            self.stop();
            return Err(anyhow!("Session start was cancelled"));
        }
        let report = check(config, mode, self.shortcut_ready.load(Ordering::SeqCst));
        if !report.ready {
            self.stop();
            return Err(anyhow!(report.failure_message()));
        }
        if self.cancelled.load(Ordering::SeqCst)
            || self.shutting_down.load(Ordering::SeqCst)
            || ticket != self.start_ticket()
        {
            return Err(anyhow!("Session start was cancelled"));
        }
        Ok(())
    }
    pub fn start_mode(
        &self,
        mode: SessionMode,
        record: bool,
        emit: Arc<dyn Fn(Snapshot) + Send + Sync>,
    ) -> Result<()> {
        self.start_with_ticket(mode, record, emit, self.start_ticket())
    }
    pub fn start_with_ticket(
        &self,
        mode: SessionMode,
        record: bool,
        emit: Arc<dyn Fn(Snapshot) + Send + Sync>,
        ticket: u64,
    ) -> Result<()> {
        let mut activity = self.activity.lock();
        if Self::busy(&activity)
            || self.session.read().running
            || self.shutting_down.load(Ordering::SeqCst)
        {
            return Err(anyhow!(
                "Session, overlay or capture is already active/stopping"
            ));
        }
        for handle in [activity.control.take(), activity.observer.take()]
            .into_iter()
            .flatten()
        {
            let _ = handle.join();
        }
        let config = self.config.read().clone();
        self.preflight(&config, mode, ticket, readiness::check)?;
        let started = Instant::now();
        let latest = Arc::new(RwLock::new(None));
        let metrics = Arc::new(RwLock::new(SessionMetrics::default()));
        let needs = Arc::new(RwLock::new(Needs::default()));
        *self.recording.write() = record.then(|| SessionRecording::new(config.clone(), mode));
        let mut initial = SessionState::new(config.clone(), mode, record);
        initial.running = true;
        *self.session.write() = initial;
        let observer_cancel = self.cancelled.clone();
        let observer_config = config.clone();
        let observer_latest = latest.clone();
        let observer_needs = needs.clone();
        let observer_metrics = metrics.clone();
        activity.observer = Some(thread::spawn(move || {
            let mut sequence = 0;
            while !observer_cancel.load(Ordering::Relaxed) {
                let captured_at_ms = started.elapsed().as_millis() as u64;
                let need = *observer_needs.read();
                let capture_start = Instant::now();
                let image = capture::capture_frame(&observer_config);
                let capture_ms = capture_start.elapsed().as_millis() as u64;
                let analysis_start = Instant::now();
                let result = image.and_then(|image| {
                    // Reserve time to publish an OCR error before the capture becomes stale.
                    // The controller's Continue/Pause policy can then handle that error.
                    let budget = observer_config
                        .observation_max_age_ms
                        .saturating_sub(100)
                        .min(2500);
                    // Until publication the controller still sees the previous frame.
                    // Its expiry, not only the new capture's, limits blocking OCR.
                    let freshness_start = observer_latest
                        .read()
                        .as_ref()
                        .map_or(capture_start, |obs: &Observation| {
                            started + Duration::from_millis(obs.captured_at_ms)
                        });
                    diagnostics::observe_before(
                        &image,
                        &observer_config,
                        need,
                        &observer_cancel,
                        freshness_start + Duration::from_millis(budget),
                    )
                });
                let analysis_ms = analysis_start.elapsed().as_millis() as u64;
                sequence += 1;
                let mut observation = result.unwrap_or_else(|error| Observation {
                    error: Some(error.to_string()),
                    ..Observation::default()
                });
                observation.sequence = sequence;
                observation.captured_at_ms = captured_at_ms;
                *observer_latest.write() = Some(observation);
                *observer_metrics.write() = SessionMetrics {
                    capture_ms,
                    analysis_ms,
                    frames: sequence,
                    frame_age_ms: 0,
                };
                let next =
                    capture_start + Duration::from_millis(observer_config.detection_interval_ms);
                while Instant::now() < next && !observer_cancel.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }));
        let shared = self.clone();
        activity.control = Some(thread::spawn(move || {
            let base = shared.stats.read().clone();
            let mut controller = Controller::new(config);
            let mut log = open_log().ok();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut native = match input_for_mode(mode, NativeInput::new) {
                    Ok(input) => input,
                    Err(error) => {
                        controller.action_failed(&format!("Input initialization failed: {error}"));
                        return;
                    }
                };
                let mut emitted = Instant::now();
                let mut saved = Instant::now();
                let mut previous_phase = controller.phase;
                loop {
                    let now = started.elapsed().as_millis() as u64;
                    let observation = latest.read().clone();
                    let focused = input::roblox_focused().unwrap_or(false);
                    let stop = shared.cancelled.load(Ordering::SeqCst);
                    let actions = controller.step(now, observation.as_ref(), focused, stop);
                    *needs.write() = Needs {
                        energy: controller.needs_energy(),
                        bite: controller.needs_bite(),
                    };
                    let mut action_error = None;
                    if let Some(input) = &mut native {
                        for action in &actions {
                            if shared.cancelled.load(Ordering::SeqCst) {
                                break;
                            }
                            if let Err(error) = input.execute(action) {
                                let reason = error.to_string();
                                controller.action_failed(&reason);
                                action_error = Some(reason);
                                break;
                            }
                        }
                    }
                    if mode == SessionMode::Automate
                        && !controller.terminal()
                        && saved.elapsed() >= Duration::from_secs(30)
                    {
                        if let Err(error) = shared.stats.read().clone().save() {
                            let reason = format!("Statistics checkpoint failed: {error}");
                            controller.action_failed(&reason);
                            action_error = Some(reason);
                        }
                        saved = Instant::now();
                    }
                    if let Some(recording) = shared.recording.write().as_mut() {
                        recording.push(RecordingFrame {
                            at_ms: now,
                            observation: observation.clone(),
                            focused,
                            stop,
                            expected_phase: controller.phase,
                            expected_actions: actions.clone(),
                            expected_fish: controller.fish_caught,
                            expected_feeds: controller.feeds,
                            action_error,
                        });
                    }
                    let phase_changed = controller.phase != previous_phase;
                    if phase_changed
                        || actions
                            .iter()
                            .any(|action| matches!(action, Action::SelectSlot(_)))
                    {
                        let mut session = shared.session.write();
                        if session.events.len() >= 200 {
                            session.events.pop_front();
                        }
                        session.events.push_back(SessionEvent {
                            at_ms: now,
                            phase: controller.phase,
                            reason: controller.reason.clone(),
                            actions,
                            observation: observation.clone(),
                        });
                    }
                    if phase_changed {
                        write_log(
                            &mut log,
                            now,
                            &format!("{mode:?} {:?}: {}", controller.phase, controller.reason),
                        );
                        previous_phase = controller.phase;
                    }
                    if phase_changed
                        || controller.terminal()
                        || emitted.elapsed() >= Duration::from_millis(250)
                    {
                        shared.publish(
                            &base,
                            &controller,
                            now,
                            observation,
                            metrics.read().clone(),
                            false,
                        );
                        emit(shared.snapshot());
                        emitted = Instant::now();
                    }
                    if controller.terminal() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }));
            if outcome.is_err() {
                controller.action_failed("Controller failed unexpectedly; input stopped");
            }
            shared.cancelled.store(true, Ordering::SeqCst);
            if let Some(recording) = shared.recording.write().as_mut() {
                recording.complete = outcome.is_ok()
                    && !recording.truncated
                    && recording
                        .frames
                        .last()
                        .is_some_and(|frame| frame.expected_phase == controller.phase);
            }
            shared.publish(
                &base,
                &controller,
                started.elapsed().as_millis() as u64,
                latest.read().clone(),
                metrics.read().clone(),
                true,
            );
            if mode == SessionMode::Automate {
                if let Err(error) = shared.stats.read().clone().save() {
                    shared.session.write().controller.reason =
                        format!("Stopped; statistics could not be saved: {error}");
                }
            }
            write_log(
                &mut log,
                started.elapsed().as_millis() as u64,
                &format!("{mode:?} ended: {}", controller.reason),
            );
            emit(shared.snapshot());
        }));
        Ok(())
    }
    fn publish(
        &self,
        base: &LifetimeStats,
        controller: &Controller,
        elapsed_ms: u64,
        observation: Option<Observation>,
        mut metrics: SessionMetrics,
        finished: bool,
    ) {
        let mut session = self.session.write();
        if session.mode == SessionMode::Automate {
            let mut stats = base.clone();
            stats.total_fish_caught += controller.fish_caught;
            stats.total_feeds += controller.feeds;
            stats.total_runtime_seconds += elapsed_ms / 1000;
            stats.best_session_fish = stats.best_session_fish.max(controller.fish_caught);
            stats.sessions_completed += u64::from(finished);
            stats.average_fish_per_hour = if stats.total_runtime_seconds > 0 {
                stats.total_fish_caught as f32 * 3600.0 / stats.total_runtime_seconds as f32
            } else {
                0.0
            };
            stats.last_updated = chrono::Local::now().to_rfc3339();
            *self.stats.write() = stats;
        }
        metrics.frame_age_ms = observation
            .as_ref()
            .map_or(0, |obs| elapsed_ms.saturating_sub(obs.captured_at_ms));
        session.running = !finished;
        session.controller = controller.clone();
        session.elapsed_ms = elapsed_ms;
        session.observation = observation;
        session.metrics = metrics;
        session.recording_frames = self
            .recording
            .read()
            .as_ref()
            .map_or(0, |recording| recording.frames.len());
    }
}
fn open_log() -> Result<BufWriter<File>> {
    let dirs = directories::ProjectDirs::from("com", "arcane", "fishing-bot")
        .ok_or_else(|| anyhow!("No application data directory"))?;
    let path = dirs.data_dir().join("logs");
    std::fs::create_dir_all(&path)?;
    Ok(BufWriter::new(File::create(path.join(format!(
        "session_{}.log",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%f")
    )))?))
}
fn write_log(log: &mut Option<BufWriter<File>>, ms: u64, text: &str) {
    if let Some(log) = log {
        let _ = writeln!(log, "{ms}ms {text}");
        let _ = log.flush();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> readiness::Readiness {
        readiness::Readiness {
            ready: true,
            checks: vec![],
        }
    }

    #[test]
    fn shutdown_permanently_rejects_fresh_sessions_before_platform_checks() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config, LifetimeStats::default());
        shared.shutdown();
        for mode in [SessionMode::Automate, SessionMode::Observe] {
            assert!(shared
                .start_mode(
                    mode,
                    true,
                    Arc::new(|_| panic!("must not initialize a session after shutdown"))
                )
                .is_err());
        }
        assert!(shared.cancelled.load(Ordering::SeqCst));
        assert!(shared.shutting_down.load(Ordering::SeqCst));
        assert!(!shared.session.read().running);
        assert!(shared.recording.read().is_none());
        let activity = shared.activity.lock();
        assert!(activity.control.is_none());
        assert!(activity.observer.is_none());
    }

    #[test]
    fn stop_during_successful_preflight_is_not_overwritten() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        let result = shared.preflight(
            &config,
            SessionMode::Observe,
            shared.start_ticket(),
            |_, _, _| {
                assert!(!shared.cancelled.load(Ordering::SeqCst));
                shared.stop();
                ready()
            },
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "Session start was cancelled"
        );
        assert!(shared.cancelled.load(Ordering::SeqCst));
        assert!(!shared.session.read().running);
    }

    #[test]
    fn failed_preflight_restores_cancellation() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        let result = shared.preflight(
            &config,
            SessionMode::Observe,
            shared.start_ticket(),
            |_, _, _| readiness::Readiness {
                ready: false,
                checks: vec![readiness::Check {
                    id: "test".into(),
                    label: "Test".into(),
                    status: "fail".into(),
                    detail: "Prerequisite unavailable".into(),
                }],
            },
        );
        assert_eq!(result.unwrap_err().to_string(), "Prerequisite unavailable");
        assert!(shared.cancelled.load(Ordering::SeqCst));
    }

    #[test]
    fn shutdown_during_preflight_rejects_start_and_finishes_after_lock_release() {
        use std::sync::mpsc;
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        let starter = shared.clone();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let start = thread::spawn(move || {
            // Production start_mode holds this lock throughout preflight.
            let _activity = starter.activity.lock();
            starter.preflight(
                &config,
                SessionMode::Observe,
                starter.start_ticket(),
                |_, _, _| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                    ready()
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let closer = shared.clone();
        let close = thread::spawn(move || closer.shutdown());
        let deadline = Instant::now() + Duration::from_secs(2);
        while !shared.shutting_down.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(shared.shutting_down.load(Ordering::SeqCst));
        release_tx.send(()).unwrap();
        assert_eq!(
            start.join().unwrap().unwrap_err().to_string(),
            "Session start was cancelled"
        );
        close.join().unwrap();
        assert!(shared.cancelled.load(Ordering::SeqCst));
        assert!(!shared.session.read().running);
    }
    #[test]
    fn stop_invalidates_start_queued_before_the_worker_runs() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        let ticket = shared.start_ticket();
        shared.stop();
        let error = shared
            .preflight(&config, SessionMode::Observe, ticket, |_, _, _| {
                panic!("A cancelled start must not check permissions or create workers")
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "Session start was cancelled");
        assert!(shared.cancelled.load(Ordering::SeqCst));
        // An intentional later start can still proceed.
        shared
            .preflight(
                &config,
                SessionMode::Observe,
                shared.start_ticket(),
                |_, _, _| ready(),
            )
            .unwrap();
    }

    #[test]
    fn observe_does_not_construct_input() {
        let result = input_for_mode::<()>(SessionMode::Observe, || {
            panic!("Observe must never initialize input")
        });
        assert!(result.unwrap().is_none());
        assert_eq!(
            input_for_mode(SessionMode::Automate, || Ok(42)).unwrap(),
            Some(42)
        );
    }
    #[test]
    fn observe_does_not_change_lifetime_stats() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        shared.session.write().mode = SessionMode::Observe;
        let mut controller = Controller::new(config);
        controller.fish_caught = 5;
        controller.feeds = 2;
        shared.publish(
            &LifetimeStats::default(),
            &controller,
            5000,
            None,
            SessionMetrics::default(),
            true,
        );
        let stats = shared.stats.read();
        assert_eq!(stats.total_fish_caught, 0);
        assert_eq!(stats.total_feeds, 0);
        assert_eq!(stats.total_runtime_seconds, 0);
        assert_eq!(stats.sessions_completed, 0);
    }
    #[test]
    fn inspection_guard_blocks_input_and_edits_until_released() {
        let config = crate::config::macbook_profile();
        let shared = SharedState::from_saved(config.clone(), LifetimeStats::default());
        let guard = shared.reserve_inspection().unwrap();
        assert!(shared
            .start(Arc::new(|_| panic!("must not start input")))
            .is_err());
        assert!(shared.save_config(config).is_err());
        assert!(shared.reserve_inspection().is_err());
        assert!(shared.cancelled.load(Ordering::SeqCst));
        drop(guard);
        assert!(shared.reserve_inspection().is_ok());
    }
}
