use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

use crate::models::InstallArgs;

/// Persists the install config so the `run` host can read it when Windows
/// launches the service. Stored under %ProgramData%\node-winsvc\<name>.json
fn config_dir() -> PathBuf {
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    PathBuf::from(base).join("node-winsvc")
}

fn config_path(name: &str) -> PathBuf {
    config_dir().join(format!("{name}.json"))
}

pub fn save(args: &InstallArgs) -> Result<()> {
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(args)?;
    fs::write(config_path(&args.name), json)?;
    Ok(())
}

pub fn load(name: &str) -> Result<InstallArgs> {
    let path = config_path(name);
    let json = fs::read_to_string(&path)
        .map_err(|_| anyhow!("Service config not found: {}", path.display()))?;
    let args = serde_json::from_str(&json)?;
    Ok(args)
}

pub fn remove(name: &str) -> Result<()> {
    let path = config_path(name);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
