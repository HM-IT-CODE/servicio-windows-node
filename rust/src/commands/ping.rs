//! Diagnostico previo a instalar. Termina con codigo != 0 si algo falla, para
//! que `instalar-servicio.cmd` aborte ANTES de registrar nada. Mas vale
//! descubrir un script mal puesto aqui que con el servicio cayendose en bucle.

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::services::node_finder::find_node_exe;

/// `ping --script <ruta>` (script opcional: sin el, solo comprueba Node).
pub fn run(script: &str) -> Result<()> {
    let mut fallos: Vec<String> = Vec::new();

    match find_node_exe() {
        Ok(node) => println!("[ok]    node.exe: {}", node.display()),
        Err(e)   => fallos.push(format!("node.exe no encontrado: {e}")),
    }

    if !script.is_empty() {
        let path = Path::new(script);
        if path.exists() {
            println!("[ok]    script: {}", path.display());
        } else {
            fallos.push(format!("el script no existe: {}", path.display()));
        }
    }

    match std::env::var("ProgramData") {
        Ok(dir) => println!("[ok]    ProgramData: {dir}"),
        Err(_)  => fallos.push("la variable ProgramData no esta definida".to_string()),
    }

    if fallos.is_empty() {
        println!("[ok]    Todo listo para instalar el servicio.");
        return Ok(());
    }

    for f in &fallos {
        eprintln!("[FALLO] {f}");
    }
    Err(anyhow!("{} comprobacion(es) fallaron", fallos.len()))
}
