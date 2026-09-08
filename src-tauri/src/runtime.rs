//! Session ownership, native adapters and persistence. The controller is independently replayable.
use crate::{
    capture,
    config::BotConfig,
    diagnostics::{self, Needs},
    engine::{Action, Controller, Observation, Phase},
    input::{self, NativeInput},
    storage::LifetimeStats,
};
use anyhow::{anyhow, Result};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub struct SessionState {
    pub running: bool,
    pub controller: Controller,
    pub elapsed_ms: u64,
    pub observation: Option<Observation>,
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
    cancelled: Arc<AtomicBool>,
    activity: Arc<Mutex<Activity>>,
}

pub struct InspectionGuard(SharedState);
impl Drop for InspectionGuard {
    fn drop(&mut self) {
        self.0.activity.lock().inspecting = false;
    }
}

impl SharedState {
    pub fn new() -> Result<Self> {
        let config = BotConfig::load()?;
        let mut controller = Controller::new(config.clone());
        controller.phase = Phase::Stopped;
        controller.reason = "Idle".into();
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(LifetimeStats::load()?)),
            session: Arc::new(RwLock::new(SessionState {
                running: false,
                controller,
                elapsed_ms: 0,
                observation: None,
            })),
            cancelled: Arc::new(AtomicBool::new(true)),
            activity: Arc::new(Mutex::new(Activity::default())),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            stats: self.stats.read().clone(),
            session: self.session.read().clone(),
        }
    }
    pub fn reserve_inspection(&self) -> Result<InspectionGuard> {
        let mut activity = self.activity.lock();
        if Self::busy(&activity) || self.session.read().running {
            return Err(anyhow!(
                "Stop the session and wait for capture to finish first"
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
        self.cancelled.store(true, Ordering::SeqCst);
    }
    pub fn shutdown(&self) {
        self.stop();
        let handle = self.activity.lock().control.take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
        // OS capture may not be interruptible. It owns no input and is not joined here.
    }
    pub fn start(&self, emit: Arc<dyn Fn(Snapshot) + Send + Sync>) -> Result<()> {
        let mut activity = self.activity.lock();
        if Self::busy(&activity) || self.session.read().running {
            return Err(anyhow!("Session or capture is already active/stopping"));
        }
        for handle in [activity.control.take(), activity.observer.take()]
            .into_iter()
            .flatten()
        {
            let _ = handle.join();
        }
        let config = self.config.read().clone();
        config.validate()?;
        if config.calibration.is_none() {
            return Err(anyhow!("This controller requires a calibrated screen and hotbar profile. Select the MacBook profile or configure one first."));
        }
        let started = Instant::now();
        let latest = Arc::new(RwLock::new(None));
        let needs = Arc::new(RwLock::new(Needs::default()));
        self.cancelled.store(false, Ordering::SeqCst);
        *self.session.write() = SessionState {
            running: true,
            controller: Controller::new(config.clone()),
            elapsed_ms: 0,
            observation: None,
        };
        let observer_cancel = self.cancelled.clone();
        let observer_config = config.clone();
        let observer_latest = latest.clone();
        let observer_needs = needs.clone();
        activity.observer = Some(thread::spawn(move || {
            let mut sequence = 0;
            while !observer_cancel.load(Ordering::Relaxed) {
                let captured_at_ms = started.elapsed().as_millis() as u64;
                let need = *observer_needs.read();
                let result = capture::capture_frame(&observer_config).and_then(|image| {
                    diagnostics::observe(&image, &observer_config, need, &observer_cancel)
                });
                sequence += 1;
                let mut observation = result.unwrap_or_else(|error| Observation {
                    error: Some(error.to_string()),
                    ..Observation::default()
                });
                observation.sequence = sequence;
                observation.captured_at_ms = captured_at_ms;
                *observer_latest.write() = Some(observation);
                let next =
                    Instant::now() + Duration::from_millis(observer_config.detection_interval_ms);
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
                let mut input = match NativeInput::new() {
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
                    let focus = input::roblox_focused().unwrap_or(false);
                    let actions = controller.step(
                        now,
                        observation.as_ref(),
                        focus,
                        shared.cancelled.load(Ordering::SeqCst),
                    );
                    *needs.write() = Needs {
                        energy: controller.needs_energy(),
                        bite: controller.needs_bite(),
                    };
                    for action in actions {
                        if shared.cancelled.load(Ordering::SeqCst) {
                            break;
                        }
                        if let Err(error) = input.execute(&action) {
                            controller.action_failed(&error.to_string());
                            break;
                        }
                        if matches!(action, Action::SelectSlot(_)) {
                            write_log(&mut log, now, &format!("Input {action:?}"));
                        }
                    }
                    let phase_changed = controller.phase != previous_phase;
                    if phase_changed {
                        write_log(
                            &mut log,
                            now,
                            &format!("{:?}: {}", controller.phase, controller.reason),
                        );
                        previous_phase = controller.phase;
                    }
                    shared.publish(&base, &controller, now, observation, false);
                    if phase_changed || emitted.elapsed() >= Duration::from_millis(250) {
                        emit(shared.snapshot());
                        emitted = Instant::now();
                    }
                    if controller.terminal() {
                        break;
                    }
                    if saved.elapsed() >= Duration::from_secs(30) {
                        if let Err(error) = shared.stats.read().clone().save() {
                            controller
                                .action_failed(&format!("Statistics checkpoint failed: {error}"));
                        }
                        saved = Instant::now();
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }));
            if outcome.is_err() {
                controller.action_failed("Controller failed unexpectedly; input stopped");
            }
            shared.cancelled.store(true, Ordering::SeqCst);
            shared.publish(
                &base,
                &controller,
                started.elapsed().as_millis() as u64,
                latest.read().clone(),
                true,
            );
            let stats = shared.stats.read().clone();
            if let Err(error) = stats.save() {
                shared.session.write().controller.reason =
                    format!("Stopped; statistics could not be saved: {error}");
            }
            write_log(
                &mut log,
                started.elapsed().as_millis() as u64,
                &format!("Session ended: {}", controller.reason),
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
        finished: bool,
    ) {
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
        *self.session.write() = SessionState {
            running: !finished,
            controller: controller.clone(),
            elapsed_ms,
            observation,
        };
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
