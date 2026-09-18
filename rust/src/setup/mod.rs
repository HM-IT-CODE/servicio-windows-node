//! El instalador propio: un .exe autocontenido que lleva la aplicacion dentro,
//! muestra un asistente y registra el servicio. Sustituye a Inno Setup sin
//! pedirle nada al servidor de destino.

pub mod desinstalar;
pub mod instalar;
pub mod manifest;
pub mod paquete;
pub mod payload;
pub mod tema;
pub mod ui;

use anyhow::Result;
use std::path::Path;

use manifest::Manifest;

/// La instalacion completa, en el orden que importa.
///
/// La comprobacion va ANTES de registrar el servicio: si el .env esta mal, es
/// preferible quedarse con los archivos copiados y ningun servicio, que con un
/// servicio registrado arrancando y cayendose en bucle.
pub fn ejecutar(
    manifest: &Manifest,
    destino: &str,
    valores: &[(String, String)],
    zip: &[u8],
) -> Result<()> {
    let destino = Path::new(destino);

    instalar::extraer(zip, destino)?;
    instalar::escribir_env(destino, manifest, valores)?;
    instalar::comprobar(destino, manifest)?;
    instalar::registrar_servicio(destino, manifest)?;
    instalar::arrancar_servicio(destino, manifest)?;

    desinstalar::registrar_entrada(destino, manifest)?;
    Ok(())
}
