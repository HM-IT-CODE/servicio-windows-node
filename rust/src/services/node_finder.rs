use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;

/// Localiza el ejecutable node.exe en el sistema
pub fn find_node_exe() -> Result<PathBuf> {
    // Intentar via `where node` (disponible en Windows)
    let output = Command::new("where")
        .arg("node")
        .output()
        .map_err(|_| anyhow!("Cannot run `where node`"))?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout);
        let first_line = path_str.lines().next().unwrap_or("").trim().to_string();
        if !first_line.is_empty() {
            return Ok(PathBuf::from(first_line));
        }
    }

    // Fallback: buscar en rutas comunes de instalación
    let common_paths = [
        r"C:\Program Files\nodejs\node.exe",
        r"C:\Program Files (x86)\nodejs\node.exe",
    ];

    common_paths.iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("node.exe not found. Is Node.js installed?"))
}
