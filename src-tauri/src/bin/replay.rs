fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("Usage: cargo run --bin replay -- tests/replays/normal-cycle.json")
    })?;
    let scenario = serde_json::from_slice(&std::fs::read(path)?)?;
    let trace = arcane_fishing_bot_app::replay::run(scenario)?;
    println!("{}", serde_json::to_string_pretty(&trace)?);
    Ok(())
}
