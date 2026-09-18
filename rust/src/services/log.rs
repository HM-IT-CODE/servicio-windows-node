//! Log mensual del supervisor.
//!
//! El archivo se elige al escribir CADA linea (`logs/servicio-AAAA-MM.log`),
//! asi rota solo aunque el servicio viva meses. Si el archivo se crea ahora,
//! se le escribe el BOM de UTF-8: sin el, `Get-Content` de PowerShell 5.1 y el
//! Bloc de notas lo leen como ANSI y los acentos salen como `arrancÃ³`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Fija el directorio donde viven los logs. Se llama una vez al arrancar el
/// host del servicio, con una ruta ABSOLUTA: un servicio arranca en System32.
pub fn set_dir(dir: &Path) {
    let _ = LOG_DIR.set(dir.to_path_buf());
}

fn log_path() -> Option<PathBuf> {
    let dir = LOG_DIR.get()?;
    let (year, month, _, _, _, _) = now_utc();
    Some(dir.join(format!("servicio-{year:04}-{month:02}.log")))
}

/// Escribe una linea con marca de tiempo. Nunca falla hacia afuera: si no se
/// puede escribir el log, el servicio tiene que seguir corriendo igual.
pub fn line(msg: &str) {
    let Some(path) = log_path() else { return };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let nuevo = !path.exists();
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    if nuevo {
        let _ = file.write_all(BOM);
    }

    let _ = writeln!(file, "{} {}", timestamp(), msg);
}

/// Copia una linea de salida del hijo tal cual, con su propio prefijo.
pub fn child_line(stream: &str, msg: &str) {
    line(&format!("[{stream}] {msg}"));
}

fn timestamp() -> String {
    let (y, mo, d, h, mi, s) = now_utc();
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Fecha y hora UTC sin dependencias externas: (anio, mes, dia, hora, min, seg).
fn now_utc() -> (u32, u32, u32, u32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let dias_totales = secs / 86_400;
    let resto        = secs % 86_400;
    let (hora, min, seg) = (resto / 3600, (resto % 3600) / 60, resto % 60);

    let (anio, mes, dia) = civil_from_days(dias_totales as i64);
    (anio, mes, dia, hora as u32, min as u32, seg as u32)
}

/// Algoritmo de Howard Hinnant: dias desde epoch -> (anio, mes, dia).
fn civil_from_days(z: i64) -> (u32, u32, u32) {
    let z   = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m   = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y   = if m <= 2 { y + 1 } else { y };
    (y as u32, m, d)
}
