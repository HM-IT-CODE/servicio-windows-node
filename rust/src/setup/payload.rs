//! Carga util adosada al propio ejecutable.
//!
//! El instalador que se distribuye es UN solo archivo:
//!
//! ```text
//!   [ stub.exe ]            el instalador generico, siempre el mismo
//!   [ zip      ]            los archivos de la aplicacion
//!   [ manifest ]            JSON: que preguntar, que servicio registrar
//!   [ pie      ]            longitudes + firma, 24 bytes al final
//! ```
//!
//! El stub se lee a si mismo, encuentra el pie al final y de ahi deduce donde
//! empiezan el manifiesto y el zip. Es la tecnica clasica de auto-extraible:
//! un .exe con datos pegados detras sigue siendo un .exe valido para Windows,
//! porque el cargador PE solo mira las cabeceras del principio.

use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Firma que marca un stub ya rellenado. Si no esta, el exe es el stub pelado.
pub const FIRMA: &[u8; 8] = b"NWSVSETP";

/// Longitudes (u64 + u64) + firma.
const PIE_LEN: u64 = 8 + 8 + 8;

pub struct Payload {
    /// El paquete TAR+zstd con los archivos. Ver `paquete.rs`.
    pub paquete:  Vec<u8>,
    pub manifest: String,
}

/// Lee la carga util del ejecutable indicado.
pub fn leer_de(exe: &Path) -> Result<Payload> {
    let mut archivo = File::open(exe)
        .map_err(|e| anyhow!("No se pudo abrir el instalador: {e}"))?;

    let total = archivo.metadata()?.len();
    if total < PIE_LEN {
        return Err(anyhow!("El instalador esta incompleto"));
    }

    // El pie, al final del archivo
    archivo.seek(SeekFrom::End(-(PIE_LEN as i64)))?;
    let mut pie = [0u8; PIE_LEN as usize];
    archivo.read_exact(&mut pie)?;

    if &pie[16..24] != FIRMA {
        return Err(anyhow!(
            "Este archivo no contiene una aplicacion: es el instalador base sin rellenar"
        ));
    }

    let paquete_len  = u64::from_le_bytes(pie[0..8].try_into().unwrap());
    let manifest_len = u64::from_le_bytes(pie[8..16].try_into().unwrap());

    if paquete_len + manifest_len + PIE_LEN > total {
        return Err(anyhow!("El instalador esta corrupto: longitudes fuera de rango"));
    }

    // paquete y manifiesto van justo antes del pie, en ese orden
    let inicio = total - PIE_LEN - manifest_len - paquete_len;

    archivo.seek(SeekFrom::Start(inicio))?;
    let mut paquete = vec![0u8; paquete_len as usize];
    archivo.read_exact(&mut paquete)?;

    let mut manifest = vec![0u8; manifest_len as usize];
    archivo.read_exact(&mut manifest)?;

    Ok(Payload {
        paquete,
        manifest: String::from_utf8(manifest)
            .map_err(|_| anyhow!("El manifiesto no es UTF-8 valido"))?,
    })
}

/// Lee la carga util del ejecutable en curso.
pub fn leer() -> Result<Payload> {
    let exe = std::env::current_exe()
        .map_err(|e| anyhow!("No se pudo resolver la ruta del instalador: {e}"))?;
    leer_de(&exe)
}
