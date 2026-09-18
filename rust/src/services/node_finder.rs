use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Carpeta donde el instalador deja un Node empaquetado, si lo trae.
const NODE_EMPAQUETADO: &str = r"vendor
ode.exe";

/// Localiza node.exe, prefiriendo el que venga DENTRO de la instalacion.
///
/// Un Node empaquetado junto al servicio hace que el servidor no necesite
/// tener Node instalado, y ademas fija la version: una actualizacion del Node
/// del sistema no puede romper el servicio de un dia para otro.
pub fn find_node_exe_en(working_dir: &str) -> Result<PathBuf> {
    if !working_dir.is_empty() {
        let propio = Path::new(working_dir).join(NODE_EMPAQUETADO);
        if propio.exists() {
            return Ok(propio);
        }
    }
    find_node_exe()
}

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
