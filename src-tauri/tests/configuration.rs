use arcane_fishing_bot_app::{
    config::{macbook_profile, BotConfig},
    storage::save_json,
};
#[test]
fn old_fields_load_without_changing_profile() {
    let config: BotConfig =
        serde_json::from_str(r#"{"fish_per_feed":7,"rod_slot":4,"food_slot":5}"#).unwrap();
    assert_eq!(config.fish_per_feed, 7);
    assert_eq!(config.energy_capacity_feed_below, 0);
    assert!(!config.auto_feed_enabled);
    config.validate().unwrap();
}
#[test]
fn feeding_requires_explicit_capacity_threshold() {
    let mut config = macbook_profile();
    config.auto_feed_enabled = true;
    assert!(config.validate().is_err());
    config.energy_capacity_feed_below = 500;
    assert!(config.validate().is_ok());
}
#[test]
fn rejects_invalid_timings_slots_and_oversized_crops() {
    let mut config = macbook_profile();
    config.fish_per_feed = 0;
    assert!(config.validate().is_err());
    config.fish_per_feed = 5;
    config.rod_slot = config.food_slot;
    assert!(config.validate().is_err());
    config.rod_slot = 4;
    config.detection_interval_ms = 0;
    assert!(config.validate().is_err());
    config.detection_interval_ms = 50;
    config.hunger_region.width = 1000;
    config.hunger_region.height = 1000;
    assert!(config.validate().is_err());
}
#[test]
fn atomic_storage_replaces_existing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stats.json");
    save_json(&path, &vec![1, 2]).unwrap();
    save_json(&path, &vec![3]).unwrap();
    assert_eq!(
        serde_json::from_slice::<Vec<u32>>(&std::fs::read(path).unwrap()).unwrap(),
        vec![3]
    );
}
