use std::{fs, path::PathBuf};

use airlet::{mechanism::MechanismPlanner, songs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/airlet-hints.json"));

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }

    let timeline = songs::air::intro_score().expand();
    let hints = MechanismPlanner::default().plan(&timeline);
    fs::write(&out, serde_json::to_string_pretty(&hints)?)?;

    println!("exported {}", out.display());
    Ok(())
}
