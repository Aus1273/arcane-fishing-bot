//! Deterministic controller: no OS calls, sleeping, UI handles or wall-clock reads.
use crate::{
    config::{BotConfig, EnergyFailurePolicy},
    detection::EnergyReading,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Startup,
    Preparing,
    ResettingRod,
    SelectingRod,
    Ready,
    Casting,
    WaitingForBite,
    Reeling,
    ConfirmingCatch,
    CheckEnergy,
    SelectingFood,
    Eating,
    VerifyFeeding,
    RestoringRod,
    Cooldown,
    Recovering,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Click,
    SelectSlot(u8),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Observation {
    pub sequence: u64,
    pub captured_at_ms: u64,
    pub bite: bool,
    pub bite_checked: bool,
    pub caught: bool,
    pub rod_selected: bool,
    pub food_selected: bool,
    pub energy: Option<EnergyReading>,
    /// OCR failure is separate from capture failure so optional monitoring can continue safely.
    pub energy_error: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Controller {
    pub phase: Phase,
    pub reason: String,
    pub fish_caught: u64,
    pub feeds: u64,
    pub errors: u32,
    pub recovery_attempts: u32,
    pub energy: Option<EnergyReading>,
    #[serde(skip)]
    config: BotConfig,
    #[serde(skip)]
    entered: u64,
    #[serde(skip)]
    deadline: u64,
    #[serde(skip)]
    next_click: u64,
    #[serde(skip)]
    last_sequence: u64,
    #[serde(skip)]
    catch_armed: bool,
    #[serde(skip)]
    consecutive_recoveries: u32,
    #[serde(skip)]
    capture_errors: u32,
    #[serde(skip)]
    pre_feed_capacity: u32,
    #[serde(skip)]
    improved_readings: u32,
}

impl Controller {
    pub fn new(config: BotConfig) -> Self {
        Self {
            phase: Phase::Startup,
            reason: "Focus Roblox during the startup delay".into(),
            fish_caught: 0,
            feeds: 0,
            errors: 0,
            recovery_attempts: 0,
            energy: None,
            entered: 0,
            deadline: config.startup_delay_ms,
            config,
            next_click: 0,
            last_sequence: 0,
            catch_armed: false,
            consecutive_recoveries: 0,
            capture_errors: 0,
            pre_feed_capacity: 0,
            improved_readings: 0,
        }
    }

    pub fn terminal(&self) -> bool {
        matches!(self.phase, Phase::Stopped | Phase::Paused)
    }
    pub fn needs_energy(&self) -> bool {
        matches!(self.phase, Phase::CheckEnergy | Phase::VerifyFeeding)
    }
    pub fn needs_bite(&self) -> bool {
        matches!(self.phase, Phase::Ready | Phase::WaitingForBite)
    }
    fn enter(&mut self, phase: Phase, now: u64, timeout: u64, reason: &str) {
        self.phase = phase;
        self.entered = now;
        self.deadline = now.saturating_add(timeout);
        self.reason = reason.into();
    }
    pub fn pause(&mut self, reason: &str) {
        self.phase = Phase::Paused;
        self.reason = reason.into();
    }
    pub fn action_failed(&mut self, reason: &str) {
        self.errors += 1;
        self.pause(reason);
    }
    fn recover(&mut self, now: u64, reason: &str) {
        self.errors += 1;
        if self.consecutive_recoveries >= self.config.recovery_limit {
            self.pause(reason);
            return;
        }
        self.consecutive_recoveries += 1;
        self.recovery_attempts += 1;
        self.enter(Phase::Recovering, now, 2500, reason);
    }

    fn energy_failed(&mut self, now: u64, reason: &str) {
        self.errors += 1;
        if self.phase == Phase::CheckEnergy
            && !self.config.auto_feed_enabled
            && self.config.energy_failure_policy == EnergyFailurePolicy::Continue
        {
            self.enter(
                Phase::Cooldown,
                now,
                400,
                &format!("Energy monitoring skipped: {reason}"),
            );
        } else {
            self.pause(&format!("Energy reading failed: {reason}"));
        }
    }

    /// Call with monotonic milliseconds. Repeated observations may sustain a click
    /// cadence, but cannot confirm a catch, feeding or any selection transition.
    pub fn step(
        &mut self,
        now: u64,
        observation: Option<&Observation>,
        focused: bool,
        cancelled: bool,
    ) -> Vec<Action> {
        if cancelled {
            self.phase = Phase::Stopped;
            self.reason = "Stopped by user".into();
            return vec![];
        }
        if self.terminal() {
            return vec![];
        }
        if self.phase == Phase::Startup && now < self.deadline {
            return vec![];
        }
        if !focused {
            self.pause("Roblox lost focus; restore the game and start a new session");
            return vec![];
        }
        if self.phase == Phase::Startup {
            self.enter(Phase::Preparing, now, 2500, "Checking rod selection");
        }
        let Some(obs) = observation else {
            if now.saturating_sub(self.entered) > self.config.observation_max_age_ms {
                self.pause("No fresh screen capture");
            }
            return vec![];
        };
        if obs.captured_at_ms > now
            || now.saturating_sub(obs.captured_at_ms) > self.config.observation_max_age_ms
        {
            self.pause("Screen observations are stale; input stopped");
            return vec![];
        }
        let fresh = obs.sequence > self.last_sequence && obs.captured_at_ms >= self.entered;
        if obs.sequence > self.last_sequence {
            self.last_sequence = obs.sequence;
            if let Some(error) = &obs.error {
                self.errors += 1;
                self.capture_errors += 1;
                self.reason = format!("Observation failed: {error}");
                if self.capture_errors >= 3 {
                    self.pause(&format!("Repeated observation failures: {error}"));
                }
            } else {
                self.capture_errors = 0;
            }
        }
        if obs.error.is_some() {
            return vec![];
        }
        if fresh && self.needs_energy() {
            if let Some(error) = &obs.energy_error {
                self.energy_failed(now, error);
                return vec![];
            }
        }
        if fresh {
            if let Some(energy) = obs.energy {
                self.energy = Some(energy);
            }
            if !obs.caught {
                self.catch_armed = true;
            }
        }
        if now > self.deadline && self.phase != Phase::Cooldown {
            match self.phase {
                Phase::WaitingForBite | Phase::Reeling | Phase::Ready | Phase::Casting => {
                    self.recover(now, "Fishing phase timed out; resetting rod")
                }
                Phase::CheckEnergy => {
                    self.energy_failed(now, "OCR timed out; inspect the calibration")
                }
                Phase::VerifyFeeding => self.pause(
                    "Food did not produce a verified capacity increase; check food and inventory",
                ),
                _ => self.pause("Could not verify the requested game state before its deadline"),
            }
            return vec![];
        }
        match self.phase {
            Phase::Preparing | Phase::Recovering if fresh => {
                if obs.rod_selected {
                    self.enter(Phase::ResettingRod, now, 2500, "Clearing the previous line");
                } else {
                    self.enter(Phase::SelectingRod, now, 2500, "Equipping the rod");
                }
                return vec![Action::SelectSlot(self.config.rod_slot)];
            }
            Phase::ResettingRod if fresh && !obs.rod_selected => {
                self.enter(
                    Phase::SelectingRod,
                    now,
                    2500,
                    "Restoring the rod after reset",
                );
                return vec![Action::SelectSlot(self.config.rod_slot)];
            }
            Phase::SelectingRod if fresh && obs.rod_selected => {
                self.enter(Phase::Ready, now, 2500, "Rod selection confirmed");
            }
            Phase::Ready if fresh && obs.bite_checked && obs.rod_selected && !obs.bite => {
                self.catch_armed = !obs.caught;
                self.enter(Phase::Casting, now, 2500, "Casting line");
                return vec![Action::Click];
            }
            Phase::Casting
                if now.saturating_sub(self.entered) >= 300 && fresh && obs.rod_selected =>
            {
                self.enter(
                    Phase::WaitingForBite,
                    now,
                    self.config.bite_timeout_ms(),
                    "Waiting for a red bite marker",
                );
            }
            Phase::WaitingForBite if fresh && obs.bite && obs.rod_selected => {
                self.enter(
                    Phase::Reeling,
                    now,
                    self.config.max_fishing_timeout_ms,
                    "Reeling",
                );
                self.next_click = now;
            }
            Phase::Reeling if fresh && obs.caught && self.catch_armed && obs.rod_selected => {
                self.enter(
                    Phase::ConfirmingCatch,
                    now,
                    1500,
                    "Confirming a new catch notification",
                );
            }
            Phase::Reeling if obs.rod_selected && now >= self.next_click => {
                self.next_click = now.saturating_add(self.config.autoclick_interval_ms);
                return vec![Action::Click];
            }
            Phase::ConfirmingCatch if fresh && !obs.caught => {
                // Keep confirmation bounded; a flicker must not replenish the reel timeout.
                self.recover(now, "Catch notification disappeared before confirmation");
            }
            Phase::ConfirmingCatch
                if fresh
                    && obs.caught
                    && obs.rod_selected
                    && now.saturating_sub(self.entered) >= 100 =>
            {
                self.fish_caught += 1;
                self.consecutive_recoveries = 0;
                self.catch_armed = false;
                if self.config.monitors_energy()
                    && self
                        .fish_caught
                        .is_multiple_of(u64::from(self.config.fish_per_feed))
                {
                    self.enter(Phase::CheckEnergy, now, 5000, "Reading Energy capacity");
                } else {
                    self.enter(Phase::Cooldown, now, 400, "Catch confirmed");
                }
            }
            Phase::CheckEnergy if fresh && obs.energy.is_some() => {
                let energy = obs.energy.unwrap();
                if self.config.auto_feed_enabled
                    && energy.capacity < self.config.energy_capacity_feed_below
                {
                    self.pre_feed_capacity = energy.capacity;
                    self.improved_readings = 0;
                    self.enter(Phase::SelectingFood, now, 2500, "Selecting food");
                    if !obs.food_selected {
                        return vec![Action::SelectSlot(self.config.food_slot)];
                    }
                } else {
                    self.enter(Phase::Cooldown, now, 400, "Energy capacity checked");
                }
            }
            Phase::SelectingFood if fresh && obs.food_selected => {
                self.enter(Phase::Eating, now, 2500, "Food selection confirmed");
            }
            Phase::Eating
                if fresh && obs.food_selected && now.saturating_sub(self.entered) >= 200 =>
            {
                self.enter(
                    Phase::VerifyFeeding,
                    now,
                    5000,
                    "Waiting for Energy capacity to increase",
                );
                return vec![Action::Click];
            }
            Phase::VerifyFeeding if fresh && obs.energy.is_some() => {
                if obs.energy.unwrap().capacity > self.pre_feed_capacity {
                    self.improved_readings += 1;
                } else {
                    self.improved_readings = 0;
                }
                if self.improved_readings >= 2 {
                    self.enter(
                        Phase::RestoringRod,
                        now,
                        2500,
                        "Food restored Energy; restoring rod",
                    );
                    if !obs.rod_selected {
                        return vec![Action::SelectSlot(self.config.rod_slot)];
                    }
                }
            }
            Phase::RestoringRod if fresh && obs.rod_selected => {
                self.feeds += 1;
                self.enter(Phase::Cooldown, now, 400, "Rod restored");
            }
            Phase::Cooldown if now >= self.deadline && fresh => {
                self.enter(Phase::Ready, now, 2500, "Ready for the next cast");
            }
            _ => {}
        }
        if fresh
            && matches!(
                self.phase,
                Phase::WaitingForBite | Phase::Reeling | Phase::ConfirmingCatch
            )
            && !obs.rod_selected
        {
            self.recover(now, "Rod selection was lost");
        }
        vec![]
    }
}
