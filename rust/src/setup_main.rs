//! El instalador generico: el "stub".
//!
//! Se compila UNA vez y no sabe nada de ninguna aplicacion. El comando
//! `node-winsvc installer --nativo` le pega detras un zip con los archivos y un
//! manifiesto JSON, y eso lo convierte en el instalador de esa aplicacion.
//!
//! Subsistema Windows (sin consola): ver el atributo de abajo. Sin el, al
//! ejecutarlo aparece una ventana negra detras del asistente.

#![windows_subsystem = "windows"]

// El instalador comparte `models` y `services` con el nucleo, pero solo usa
// una parte: sin este allow el compilador marca como muerto todo lo que usa
// el otro binario. Marcarlo AQUI y no dentro del modulo mantiene los avisos
// vivos para el nucleo, que si los usa enteros.
#[allow(dead_code)] mod models;
#[allow(dead_code)] mod services;
mod setup;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_ICONINFORMATION};

use setup::manifest::Manifest;

fn main() {
    // Sin carga util adosada estamos en desarrollo (`cargo run`): se abre el
    // asistente con datos de ejemplo y NO se instala nada. Permite iterar sobre
    // la ventana en segundos, sin generar un instalador cada vez.
    let (manifest, demo, zip) = match setup::payload::leer() {
        Ok(payload) => match Manifest::desde_json(&payload.manifest) {
            Ok(m) => (m, false, payload.paquete),
            Err(e) => {
                setup::ui::avisar(HWND::default(), &e.to_string(), "Instalador", MB_ICONERROR);
                std::process::exit(1);
            }
        },
        Err(_) => (Manifest::demo(), true, Vec::new()),
    };

    if std::env::args().any(|a| a == "/desinstalar") {
        desinstalar(&manifest);
        return;
    }

    // En demo no se pide elevacion: no se va a tocar el sistema.
    if !demo && !es_administrador() {
        relanzar_como_administrador();
        return;
    }

    setup::ui::mostrar(manifest, demo, zip);
}

fn desinstalar(manifest: &Manifest) {
    if !es_administrador() {
        relanzar_como_administrador();
        return;
    }

    match setup::desinstalar::ejecutar(manifest) {
        Ok(mensaje) => setup::ui::avisar(
            HWND::default(), &mensaje, "Desinstalar", MB_ICONINFORMATION),
        Err(e) => setup::ui::avisar(
            HWND::default(), &e.to_string(), "Desinstalar", MB_ICONERROR),
    }
}

/// Registrar un servicio exige elevacion. En vez de incrustar un manifiesto en
/// el exe (que obligaria a un script de compilacion), se comprueba en caliente
/// y el proceso se relanza con "runas", igual que hace un .cmd que se eleva.
fn es_administrador() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevacion = TOKEN_ELEVATION::default();
        let mut tamano = 0u32;

        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevacion as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut tamano,
        ).is_ok();

        let _ = windows::Win32::Foundation::CloseHandle(token);
        ok && elevacion.TokenIsElevated != 0
    }
}

fn relanzar_como_administrador() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else { return };
    let exe_w: Vec<u16> = format!("{}\0", exe.display()).encode_utf16().collect();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_w: Vec<u16> = format!("{}\0", args.join(" ")).encode_utf16().collect();

    unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("runas"),
            PCWSTR(exe_w.as_ptr()),
            PCWSTR(args_w.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
    }
}
