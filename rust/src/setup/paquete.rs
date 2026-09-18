//! El formato del paquete: TAR comprimido con Zstandard en un solo flujo.
//!
//! Por que TAR y no ZIP, que es lo obvio: un ZIP comprime **cada archivo por
//! separado**, asi que no puede aprovechar que miles de archivos de
//! `node_modules` se parecen entre si. Un TAR es un solo flujo continuo, y al
//! comprimirlo entero el compresor ve todo el conjunto a la vez. Es lo que
//! Inno Setup llama "compresion solida", y es la razon de que sus instaladores
//! salgan mas pequenos.
//!
//! Medido sobre el mismo caso (node.exe de 80 MB + 7.900 archivos), ver el
//! cuadro en `empaquetar.rs`.
//!
//! El precio: para sacar UN archivo hay que descomprimir todo lo anterior. Da
//! igual aqui, porque un instalador siempre extrae el paquete entero.

use anyhow::{anyhow, Result};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Firma del formato, al principio del paquete. Permite reconocer un paquete
/// antiguo (ZIP) sin intentar descomprimirlo y fallar con un error confuso.
pub const MAGIA: &[u8; 4] = b"NWS1";

// --- Escritura -------------------------------------------------------------
//
// Esta mitad la usa solo el nucleo, al empaquetar; el instalador unicamente
// extrae. De ahi el allow: para el binario del instalador es codigo muerto.

#[allow(dead_code)]
pub struct Constructor {
    tar: tar::Builder<Vec<u8>>,
}

#[allow(dead_code)]
impl Constructor {
    pub fn nuevo() -> Self {
        Self { tar: tar::Builder::new(Vec::new()) }
    }

    /// Agrega un archivo con la ruta que tendra dentro del paquete.
    pub fn agregar(&mut self, origen: &Path, destino: &str) -> Result<()> {
        let datos = std::fs::read(origen)
            .map_err(|e| anyhow!("No se pudo leer {}: {e}", origen.display()))?;

        let mut cabecera = tar::Header::new_gnu();
        cabecera.set_size(datos.len() as u64);
        cabecera.set_mode(0o644);
        cabecera.set_cksum();

        self.tar
            .append_data(&mut cabecera, destino, Cursor::new(datos))
            .map_err(|e| anyhow!("No se pudo empaquetar {destino}: {e}"))?;
        Ok(())
    }

    /// Cierra el TAR y lo comprime entero. Devuelve magia + flujo zstd.
    pub fn terminar(self, nivel: i32) -> Result<Vec<u8>> {
        let tar = self.tar.into_inner()?;

        let comprimido = zstd::encode_all(Cursor::new(tar), nivel)
            .map_err(|e| anyhow!("No se pudo comprimir el paquete: {e}"))?;

        let mut salida = Vec::with_capacity(comprimido.len() + 4);
        salida.extend_from_slice(MAGIA);
        salida.extend_from_slice(&comprimido);
        Ok(salida)
    }
}

// --- Lectura ---------------------------------------------------------------

/// Extrae el paquete en el destino.
pub fn extraer(paquete: &[u8], destino: &Path) -> Result<()> {
    if paquete.len() < 4 || &paquete[..4] != MAGIA {
        return Err(anyhow!(
            "El paquete no tiene el formato esperado (instalador de una version distinta)"
        ));
    }

    std::fs::create_dir_all(destino)?;

    let tar = zstd::decode_all(Cursor::new(&paquete[4..]))
        .map_err(|e| anyhow!("No se pudo descomprimir el paquete: {e}"))?;

    let mut archivo = tar::Archive::new(Cursor::new(tar));

    for entrada in archivo.entries()? {
        let mut entrada = entrada?;

        // Rutas con .. o absolutas: un paquete manipulado podria escribir
        // fuera del destino. Se descartan en vez de confiar en el nombre.
        let ruta = entrada.path()?.into_owned();
        if !ruta_segura(&ruta) {
            continue;
        }

        let salida = destino.join(&ruta);
        if let Some(padre) = salida.parent() {
            std::fs::create_dir_all(padre)?;
        }

        entrada.unpack(&salida)
            .map_err(|e| anyhow!("No se pudo extraer {}: {e}", ruta.display()))?;
    }
    Ok(())
}

fn ruta_segura(ruta: &Path) -> bool {
    use std::path::Component;

    ruta.components().all(|c| matches!(c, Component::Normal(_)))
}

/// Cuenta los archivos de un paquete sin extraerlo. Para informar al usuario.
#[allow(dead_code)]
pub fn contar(paquete: &[u8]) -> Result<usize> {
    let tar = zstd::decode_all(Cursor::new(&paquete[4..]))?;
    let mut archivo = tar::Archive::new(Cursor::new(tar));
    Ok(archivo.entries()?.count())
}

/// Rutas de un directorio, en pares (origen, nombre dentro del paquete).
#[allow(dead_code)]
pub fn recorrer(raiz: &Path, base: &Path) -> Vec<(PathBuf, String)> {
    let mut salida = Vec::new();
    recorrer_en(raiz, base, &mut salida);
    salida
}

#[allow(dead_code)]
fn recorrer_en(ruta: &Path, base: &Path, salida: &mut Vec<(PathBuf, String)>) {
    if ruta.is_file() {
        let relativa = ruta.strip_prefix(base).unwrap_or(ruta);
        salida.push((ruta.to_path_buf(), relativa.to_string_lossy().replace('\\', "/")));
        return;
    }

    let Ok(entradas) = std::fs::read_dir(ruta) else { return };
    for entrada in entradas.filter_map(Result::ok) {
        recorrer_en(&entrada.path(), base, salida);
    }
}
