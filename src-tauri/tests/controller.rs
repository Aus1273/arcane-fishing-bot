use arcane_fishing_bot_app::{
    config::macbook_profile,
    detection::EnergyReading,
    engine::{Action, Controller, Observation, Phase},
};
fn config() -> arcane_fishing_bot_app::config::BotConfig {
    let mut c = macbook_profile();
    c.startup_delay_ms = 0;
    c
}
fn obs(t: u64) -> Observation {
    Observation {
        sequence: t + 1,
        captured_at_ms: t,
        rod_selected: true,
        bite_checked: true,
        ..Observation::default()
    }
}
fn tick(c: &mut Controller, t: u64, o: Observation) -> Vec<Action> {
    c.step(t, Some(&o), true, false)
}
fn waiting(c: &mut Controller) {
    let mut o = obs(0);
    o.rod_selected = false;
    assert_eq!(tick(c, 0, o), vec![Action::SelectSlot(4)]);
    tick(c, 100, obs(100));
    assert_eq!(c.phase, Phase::Ready);
    assert_eq!(tick(c, 200, obs(200)), vec![Action::Click]);
    tick(c, 501, obs(501));
    assert_eq!(c.phase, Phase::WaitingForBite);
}
fn reeling(c: &mut Controller) {
    waiting(c);
    let mut o = obs(600);
    o.bite = true;
    tick(c, 600, o);
    assert_eq!(c.phase, Phase::Reeling);
}
fn caught(c: &mut Controller) {
    reeling(c);
    let mut o = obs(700);
    o.caught = true;
    tick(c, 700, o);
    let mut o = obs(810);
    o.caught = true;
    tick(c, 810, o);
    assert_eq!(c.fish_caught, 1);
}

#[test]
fn normal_cycle_requires_distinct_confirmation_frames() {
    let mut c = Controller::new(config());
    reeling(&mut c);
    assert_eq!(tick(&mut c, 610, obs(610)), vec![Action::Click]);
    assert!(tick(&mut c, 620, obs(620)).is_empty());
    let mut o = obs(700);
    o.caught = true;
    tick(&mut c, 700, o.clone());
    tick(&mut c, 850, o);
    assert_eq!(c.fish_caught, 0);
    let mut o = obs(860);
    o.caught = true;
    tick(&mut c, 860, o);
    assert_eq!(c.fish_caught, 1);
    assert_eq!(c.phase, Phase::Cooldown);
}
#[test]
fn held_notification_does_not_count_again() {
    let mut c = Controller::new(config());
    caught(&mut c);
    for t in [1300, 1400, 1800] {
        let mut o = obs(t);
        o.caught = true;
        tick(&mut c, t, o);
    }
    let mut o = obs(1900);
    o.caught = true;
    o.bite = true;
    tick(&mut c, 1900, o);
    for t in [2000, 2200, 2400] {
        let mut o = obs(t);
        o.caught = true;
        tick(&mut c, t, o);
    }
    assert_eq!(c.fish_caught, 1);
    assert_eq!(c.phase, Phase::Reeling);
    tick(&mut c, 2500, obs(2500));
    for t in [2600, 2750] {
        let mut o = obs(t);
        o.caught = true;
        tick(&mut c, t, o);
    }
    assert_eq!(c.fish_caught, 2);
}
#[test]
fn reel_timeout_resets_rod_without_blind_casting() {
    let mut c = Controller::new(config());
    reeling(&mut c);
    assert!(tick(&mut c, 26000, obs(26000)).is_empty());
    assert_eq!(c.phase, Phase::Recovering);
    assert_eq!(tick(&mut c, 26100, obs(26100)), vec![Action::SelectSlot(4)]);
    let mut o = obs(26200);
    o.rod_selected = false;
    assert_eq!(tick(&mut c, 26200, o), vec![Action::SelectSlot(4)]);
    assert!(tick(&mut c, 26300, obs(26300)).is_empty());
    assert_eq!(tick(&mut c, 26400, obs(26400)), vec![Action::Click]);
}
#[test]
fn focus_loss_and_stale_frames_stop_clicks() {
    let mut c = Controller::new(config());
    reeling(&mut c);
    assert!(c.step(700, Some(&obs(700)), false, false).is_empty());
    assert_eq!(c.phase, Phase::Paused);
    let mut c = Controller::new(config());
    reeling(&mut c);
    assert!(tick(&mut c, 5000, obs(600)).is_empty());
    assert_eq!(c.phase, Phase::Paused);
}
#[test]
fn cancellation_in_every_phase_produces_no_input() {
    for phase in [
        Phase::Startup,
        Phase::Preparing,
        Phase::ResettingRod,
        Phase::SelectingRod,
        Phase::Ready,
        Phase::Casting,
        Phase::WaitingForBite,
        Phase::Reeling,
        Phase::ConfirmingCatch,
        Phase::CheckEnergy,
        Phase::SelectingFood,
        Phase::Eating,
        Phase::VerifyFeeding,
        Phase::RestoringRod,
        Phase::Cooldown,
        Phase::Recovering,
    ] {
        let mut c = Controller::new(config());
        c.phase = phase;
        assert!(c.step(500, Some(&obs(500)), true, true).is_empty());
        assert_eq!(c.phase, Phase::Stopped);
    }
}
#[test]
fn feed_uses_capacity_and_counts_only_verified_restoration() {
    let mut config = config();
    config.fish_per_feed = 1;
    config.auto_feed_enabled = true;
    config.energy_capacity_feed_below = 500;
    let mut c = Controller::new(config);
    caught(&mut c);
    assert_eq!(c.phase, Phase::CheckEnergy);
    let mut o = obs(900);
    o.energy = Some(EnergyReading {
        usable: 400,
        capacity: 400,
    });
    assert_eq!(tick(&mut c, 900, o), vec![Action::SelectSlot(5)]);
    let mut o = obs(1000);
    o.rod_selected = false;
    o.food_selected = true;
    tick(&mut c, 1000, o);
    let mut o = obs(1250);
    o.rod_selected = false;
    o.food_selected = true;
    assert_eq!(tick(&mut c, 1250, o), vec![Action::Click]);
    assert_eq!(c.feeds, 0);
    let mut o = obs(1400);
    o.rod_selected = false;
    o.food_selected = true;
    o.energy = Some(EnergyReading {
        usable: 450,
        capacity: 450,
    });
    tick(&mut c, 1400, o.clone());
    assert_eq!(c.feeds, 0);
    tick(&mut c, 1500, o);
    assert_eq!(c.feeds, 0); // repeated OCR result cannot verify
    let mut o = obs(1600);
    o.rod_selected = false;
    o.food_selected = true;
    o.energy = Some(EnergyReading {
        usable: 450,
        capacity: 450,
    });
    assert_eq!(tick(&mut c, 1600, o), vec![Action::SelectSlot(4)]);
    assert_eq!(c.feeds, 1);
    tick(&mut c, 1700, obs(1700));
    assert_eq!(c.phase, Phase::Cooldown);
}
#[test]
fn low_usable_energy_with_high_capacity_does_not_feed() {
    let mut config = config();
    config.fish_per_feed = 1;
    config.auto_feed_enabled = true;
    config.energy_capacity_feed_below = 500;
    let mut c = Controller::new(config);
    caught(&mut c);
    let mut o = obs(900);
    o.energy = Some(EnergyReading {
        usable: 100,
        capacity: 950,
    });
    assert!(tick(&mut c, 900, o).is_empty());
    assert_eq!(c.phase, Phase::Cooldown);
}
#[test]
fn failed_ocr_or_input_never_counts_a_feed() {
    let mut config = config();
    config.fish_per_feed = 1;
    let mut c = Controller::new(config);
    caught(&mut c);
    for t in [900, 1000, 1100] {
        let mut o = obs(t);
        o.error = Some("OCR unavailable".into());
        assert!(tick(&mut c, t, o).is_empty());
    }
    assert_eq!(c.phase, Phase::Paused);
    assert_eq!(c.feeds, 0);
    let mut c = Controller::new(self::config());
    reeling(&mut c);
    c.action_failed("Input failed");
    assert!(tick(&mut c, 900, obs(900)).is_empty());
    assert_eq!(c.phase, Phase::Paused);
}
#[test]
fn recovery_attempts_are_bounded() {
    let mut config = config();
    config.recovery_limit = 0;
    let mut c = Controller::new(config);
    waiting(&mut c);
    tick(&mut c, 70000, obs(70000));
    assert_eq!(c.phase, Phase::Paused);
}
#[test]
fn json_replays_match_expected_actions() {
    for entry in
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/replays")).unwrap()
    {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|v| v == "json") {
            let scenario = serde_json::from_slice(&std::fs::read(entry.path()).unwrap()).unwrap();
            arcane_fishing_bot_app::replay::run(scenario).unwrap();
        }
    }
}

#[test]
fn late_acknowledgement_cannot_extend_selection_deadline() {
    let mut c = Controller::new(config());
    let mut o = obs(0);
    o.rod_selected = false;
    tick(&mut c, 0, o);
    assert!(tick(&mut c, 3000, obs(3000)).is_empty());
    assert_eq!(c.phase, Phase::Paused);
}
#[test]
fn unmeasured_bite_flag_cannot_authorize_cast() {
    let mut c = Controller::new(config());
    let mut o = obs(0);
    o.rod_selected = false;
    tick(&mut c, 0, o);
    tick(&mut c, 100, obs(100));
    let mut o = obs(200);
    o.bite_checked = false;
    assert!(tick(&mut c, 200, o).is_empty());
    assert_eq!(c.phase, Phase::Ready);
}
#[test]
fn rod_loss_during_confirmation_does_not_count() {
    let mut c = Controller::new(config());
    reeling(&mut c);
    let mut o = obs(700);
    o.caught = true;
    tick(&mut c, 700, o);
    let mut o = obs(850);
    o.caught = true;
    o.rod_selected = false;
    tick(&mut c, 850, o);
    assert_eq!(c.fish_caught, 0);
    assert_eq!(c.phase, Phase::Recovering);
}
