//! Lo que pasa despues de que el usuario pulsa "Instalar": extraer, escribir
//! la configuracion, comprobarla y registrar el servicio.
//!
//! El orden importa. La comprobacion va ANTES de registrar: un .env mal puesto
//! con el servicio ya registrado produce un bucle de arranque y caida que desde
//! fuera parece un fallo del programa.

use anyhow::{anyhow, Result};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Sin esto, CADA proceso hijo abre su propia ventana de consola negra: el
/// instalador no tiene consola (windows_subsystem = "windows"), asi que
/// Windows le crea una a cada uno. Se veia como una sucesion de parpadeos.
const SIN_VENTANA: u32 = 0x0800_0000; // CREATE_NO_WINDOW

use crate::setup::manifest::Manifest;

/// Descomprime el paquete adosado en el destino. Ver `paquete.rs`.
pub fn extraer(paquete: &[u8], destino: &Path) -> Result<()> {
    crate::setup::paquete::extraer(paquete, destino)
}

/// Escribe el .env con las variables fijas y lo que el usuario contesto.
pub fn escribir_env(destino: &Path, manifest: &Manifest, valores: &[(String, String)]) -> Result<()> {
    let mut texto = String::from("# Generado por el instalador\n");

    for (clave, valor) in &manifest.env {
        texto.push_str(&format!("{clave}={valor}\n"));
    }
    for (clave, valor) in valores {
        texto.push_str(&format!("{clave}={valor}\n"));
    }

    let mut archivo = std::fs::File::create(destino.join(".env"))?;
    archivo.write_all(texto.as_bytes())?;
    Ok(())
}

/// Ejecuta el script de comprobacion. Sin script, se da por buena.
pub fn comprobar(destino: &Path, manifest: &Manifest) -> Result<()> {
    if manifest.verify_script.is_empty() {
        return Ok(());
    }

    let node = node_exe(destino, manifest);
    let script = destino.join(manifest.verify_script.replace('/', "\\"));

    let salida = Command::new(node)
        .arg(&script)
        .current_dir(destino)
        .creation_flags(SIN_VENTANA)
        .output()
        .map_err(|e| anyhow!("No se pudo ejecutar la comprobacion: {e}"))?;

    if salida.status.success() {
        return Ok(());
    }

    // El mensaje del script es lo util para el usuario: se lo damos tal cual.
    let detalle = String::from_utf8_lossy(&salida.stderr);
    let detalle = if detalle.trim().is_empty() {
        String::from_utf8_lossy(&salida.stdout).to_string()
    } else {
        detalle.to_string()
    };

    Err(anyhow!("{}", detalle.trim()))
}

/// El node.exe a usar: el empaquetado si lo hay, si no el del sistema.
fn node_exe(destino: &Path, manifest: &Manifest) -> PathBuf {
    if manifest.bundled_node {
        let propio = destino.join("vendor").join("node.exe");
        if propio.exists() {
            return propio;
        }
    }
    PathBuf::from("node")
}

/// Registra el servicio llamando al nucleo que va dentro del paquete.
pub fn registrar_servicio(destino: &Path, manifest: &Manifest) -> Result<()> {
    let core = destino.join("bin").join("node-winsvc-core.exe");
    if !core.exists() {
        return Err(anyhow!("Falta el nucleo del servicio: {}", core.display()));
    }

    // Pararlo antes de borrarlo: DeleteService solo lo MARCA, el proceso sigue
    // vivo reteniendo el puerto y el servicio nuevo arrancaria contra el.
    let _ = Command::new(&core).args(["stop", "--name", &manifest.service_name])
        .creation_flags(SIN_VENTANA).output();
    std::thread::sleep(std::time::Duration::from_millis(2000));
    let _ = Command::new(&core).args(["uninstall", "--name", &manifest.service_name])
        .creation_flags(SIN_VENTANA).output();
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let script   = destino.join(manifest.script.replace('/', "\\"));
    let log_file = if manifest.log_file.is_empty() {
        destino.join("logs").join("service.log")
    } else {
        destino.join(manifest.log_file.replace('/', "\\"))
    };

    let salida = Command::new(&core)
        .args([
            "install",
            "--name",         &manifest.service_name,
            "--display",      &manifest.service_display,
            "--description",  &manifest.service_desc,
            "--script",       &script.display().to_string(),
            "--node-args",    &manifest.node_args,
            "--env",          "{}",
            "--working-dir",  &destino.display().to_string(),
            "--log-file",     &log_file.display().to_string(),
            "--start-type",   &manifest.start_type,
            "--auto-restart", &manifest.auto_restart.to_string(),
        ])
        .current_dir(destino)
        .creation_flags(SIN_VENTANA)
        .output()
        .map_err(|e| anyhow!("No se pudo ejecutar el nucleo: {e}"))?;

    if !salida.status.success() {
        return Err(anyhow!("{}", String::from_utf8_lossy(&salida.stderr).trim()));
    }
    Ok(())
}

pub fn arrancar_servicio(destino: &Path, manifest: &Manifest) -> Result<()> {
    let core = destino.join("bin").join("node-winsvc-core.exe");

    let salida = Command::new(&core)
        .args(["start", "--name", &manifest.service_name])
        .creation_flags(SIN_VENTANA)
        .output()
        .map_err(|e| anyhow!("No se pudo arrancar el servicio: {e}"))?;

    if !salida.status.success() {
        return Err(anyhow!("{}", String::from_utf8_lossy(&salida.stderr).trim()));
    }
    Ok(())
}
