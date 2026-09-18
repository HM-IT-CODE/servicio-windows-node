//! La entrada de "Agregar o quitar programas" y el modo desinstalacion.
//!
//! Sin esto el programa quedaria instalado pero invisible para Windows, y la
//! unica forma de quitarlo seria borrar la carpeta a mano, dejando el servicio
//! registrado apuntando a archivos que ya no existen.

use anyhow::{anyhow, Result};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// Ver la nota en instalar.rs: sin esto cada hijo abre una consola negra.
const SIN_VENTANA: u32 = 0x0800_0000; // CREATE_NO_WINDOW

use crate::setup::manifest::Manifest;

fn clave_registro(nombre: &str) -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{nombre}")
}

/// Se copia a si mismo a la carpeta de instalacion y anota la entrada.
///
/// Tiene que copiarse: si la entrada apuntara al .exe que el usuario ejecuto
/// desde Descargas, dejaria de funcionar en cuanto borrase ese archivo.
pub fn registrar_entrada(destino: &Path, manifest: &Manifest) -> Result<()> {
    let propio = std::env::current_exe()?;
    let copia = destino.join("desinstalar.exe");

    // Si ya existe (reinstalacion), no pasa nada: se sobrescribe.
    std::fs::copy(&propio, &copia)
        .map_err(|e| anyhow!("No se pudo copiar el desinstalador: {e}"))?;

    let clave = clave_registro(&manifest.service_name);

    let valores: [(&str, &str, String); 6] = [
        ("DisplayName",     "REG_SZ", manifest.app_name.clone()),
        ("DisplayVersion",  "REG_SZ", manifest.version.clone()),
        ("Publisher",       "REG_SZ", manifest.publisher.clone()),
        ("InstallLocation", "REG_SZ", destino.display().to_string()),
        ("UninstallString", "REG_SZ", format!("\"{}\" /desinstalar", copia.display())),
        ("NoModify",        "REG_DWORD", "1".to_string()),
    ];

    for (nombre, tipo, valor) in valores {
        let salida = Command::new("reg")
            .args(["add", &format!("HKLM\\{clave}"), "/v", nombre, "/t", tipo, "/d", &valor, "/f"])
            .creation_flags(SIN_VENTANA)
            .output()?;

        if !salida.status.success() {
            return Err(anyhow!(
                "No se pudo escribir la entrada de desinstalacion: {}",
                String::from_utf8_lossy(&salida.stderr).trim()
            ));
        }
    }
    Ok(())
}

/// Modo `/desinstalar`: para el servicio, lo quita, y borra la entrada.
///
/// NO borra la carpeta: el .exe que corre esta dentro de ella, y Windows no
/// deja borrar un archivo en uso. Se le indica al usuario.
pub fn ejecutar(manifest: &Manifest) -> Result<String> {
    let destino = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow!("No se pudo resolver la carpeta de instalacion"))?
        .to_path_buf();

    let core = destino.join("bin").join("node-winsvc-core.exe");

    if core.exists() {
        let _ = Command::new(&core).args(["stop", "--name", &manifest.service_name])
            .creation_flags(SIN_VENTANA).output();
        std::thread::sleep(std::time::Duration::from_millis(2000));

        let salida = Command::new(&core)
            .args(["uninstall", "--name", &manifest.service_name])
            .creation_flags(SIN_VENTANA)
            .output()?;

        if !salida.status.success() {
            return Err(anyhow!(
                "No se pudo quitar el servicio: {}",
                String::from_utf8_lossy(&salida.stderr).trim()
            ));
        }
    }

    let _ = Command::new("reg")
        .args(["delete", &format!("HKLM\\{}", clave_registro(&manifest.service_name)), "/f"])
            .creation_flags(SIN_VENTANA)
        .output();

    Ok(format!(
        "El servicio \"{}\" se quito correctamente.\n\n\
         Los archivos siguen en:\n{}\n\n\
         Puede borrar esa carpeta a mano cuando cierre esta ventana.",
        manifest.service_display,
        destino.display(),
    ))
}
