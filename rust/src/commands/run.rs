//! Modo host del servicio. Windows lanza `node-winsvc-core.exe run --name <svc>`
//! y ESTE codigo implementa el protocolo Win32 de servicios: registra un
//! manejador de control, reporta RUNNING, y luego lanza y supervisa el proceso
//! hijo de Node, relanzandolo si cae (cuando auto_restart) hasta que el SCM
//! pida parar.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::System::Services::*;

use crate::models::InstallArgs;
use crate::services::event_log;
use crate::services::log;
use crate::services::node_finder::find_node_exe_en;
use crate::services::scm::to_wide;
use crate::services::service_config;

static SERVICE_NAME:   OnceLock<String> = OnceLock::new();
static STOP_REQUESTED: AtomicBool       = AtomicBool::new(false);
static STATUS_HANDLE:  AtomicIsize      = AtomicIsize::new(0);

const POLL_INTERVAL_MS: u64 = 500;
const RESTART_DELAY_MS: u64 = 1000;

/// Entry point for `run --name <svc>`. Blocks until the service stops.
pub fn run(name: &str) -> Result<()> {
    let _ = SERVICE_NAME.set(name.to_string());

    let mut name_w = to_wide(name);
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: PWSTR(name_w.as_mut_ptr()),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW::default(), // null terminator
    ];

    unsafe {
        StartServiceCtrlDispatcherW(table.as_ptr())
            .map_err(|e| anyhow!("StartServiceCtrlDispatcherW failed: {}", e))?;
    }
    Ok(())
}

/// Called by the SCM on a dedicated thread once the service process starts.
unsafe extern "system" fn service_main(_argc: u32, _argv: *mut PWSTR) {
    let name = SERVICE_NAME.get().cloned().unwrap_or_default();
    let name_w = to_wide(&name);

    let handle = match RegisterServiceCtrlHandlerW(PCWSTR(name_w.as_ptr()), Some(control_handler)) {
        Ok(h) => h,
        Err(_) => return,
    };
    STATUS_HANDLE.store(handle.0 as isize, Ordering::SeqCst);

    report_status(SERVICE_START_PENDING, 0, 3000);
    report_status(SERVICE_RUNNING, SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN, 0);

    supervise(&name);

    log::line("servicio detenido");
    event_log::info(&format!("El servicio \"{name}\" se ha detenido."));
    report_status(SERVICE_STOPPED, 0, 0);
}

/// Called by the SCM when it wants to control the service (stop/shutdown).
unsafe extern "system" fn control_handler(control: u32) {
    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            STOP_REQUESTED.store(true, Ordering::SeqCst);
            report_status(SERVICE_STOP_PENDING, 0, 5000);
        }
        _ => {}
    }
}

fn report_status(state: SERVICE_STATUS_CURRENT_STATE, accepted: u32, wait_hint: u32) {
    let handle = SERVICE_STATUS_HANDLE(STATUS_HANDLE.load(Ordering::SeqCst) as *mut core::ffi::c_void);
    let status = SERVICE_STATUS {
        dwServiceType:             SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState:            state,
        dwControlsAccepted:        accepted,
        dwWin32ExitCode:           0,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint:              0,
        dwWaitHint:                wait_hint,
    };
    unsafe { let _ = SetServiceStatus(handle, &status); }
}

/// Spawns the Node child and keeps it alive until a stop is requested.
fn supervise(name: &str) {
    let cfg = match service_config::load(name) {
        Ok(c) => c,
        Err(_) => return,
    };

    log::set_dir(&log_dir(&cfg));
    service_config::marcar_arranque(name);
    log::line(&format!("arrancando servicio \"{name}\""));
    event_log::info(&format!("El servicio \"{name}\" esta arrancando."));

    let node = match find_node_exe_en(&cfg.working_dir) {
        Ok(n) => n,
        Err(e) => {
            log::line(&format!("ERROR no se encontro node.exe: {e}"));
            event_log::error(&format!(
                "El servicio \"{name}\" no pudo arrancar: no se encontro node.exe. {e}"
            ));
            return;
        }
    };
    log::line(&format!("node.exe: {}", node.display()));

    loop {
        let mut child = match spawn_child(&node, &cfg) {
            Ok(c) => c,
            Err(e) => {
                log::line(&format!("ERROR no se pudo lanzar el hijo: {e}"));
                event_log::error(&format!(
                    "El servicio \"{name}\" no pudo lanzar el proceso de Node: {e}"
                ));
                return;
            }
        };
        log::line(&format!("hijo lanzado (pid {}): {}", child.id(), cfg.script));
        capture_output(&mut child);

        // Poll the child until it exits or a stop is requested
        loop {
            if STOP_REQUESTED.load(Ordering::SeqCst) {
                log::line("stop solicitado por el SCM, matando al hijo");
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            match child.try_wait() {
                Ok(Some(st)) => {
                    log::line(&format!("el hijo termino con codigo {st}"));
                    event_log::warn(&format!(
                        "El proceso de Node del servicio \"{name}\" termino inesperadamente ({st})."
                    ));
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS)),
                Err(e) => {
                    log::line(&format!("ERROR consultando al hijo: {e}"));
                    break;
                }
            }
        }

        if STOP_REQUESTED.load(Ordering::SeqCst) {
            return;
        }
        if !cfg.auto_restart {
            log::line("auto_restart desactivado, no se relanza");
            event_log::error(&format!(
                "El servicio \"{name}\" se detiene: el proceso cayo y auto_restart esta desactivado."
            ));
            return;
        }
        log::line("relanzando el hijo en 1s");
        std::thread::sleep(Duration::from_millis(RESTART_DELAY_MS));
    }
}

/// Los logs viven junto al `log_file` configurado; si no hay, en
/// `<working_dir>\logs`. Siempre ruta absoluta: un servicio arranca en System32.
fn log_dir(cfg: &InstallArgs) -> PathBuf {
    if !cfg.log_file.is_empty() {
        if let Some(parent) = Path::new(&cfg.log_file).parent() {
            if !parent.as_os_str().is_empty() {
                return parent.to_path_buf();
            }
        }
    }
    if !cfg.working_dir.is_empty() {
        return Path::new(&cfg.working_dir).join("logs");
    }
    PathBuf::from(r"C:\ProgramData\node-winsvc\logs")
}

fn spawn_child(node: &Path, cfg: &InstallArgs) -> Result<Child> {
    let mut cmd = Command::new(node);

    // node [nodeArgs] <script>
    for arg in cfg.node_args.split_whitespace() {
        cmd.arg(arg);
    }
    cmd.arg(&cfg.script);

    if !cfg.working_dir.is_empty() {
        cmd.current_dir(&cfg.working_dir);
    }

    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&cfg.env) {
        for (k, v) in map {
            cmd.env(k, v);
        }
    }

    // La salida va por el supervisor y no directo a un archivo, para que el mes
    // se decida en CADA linea: asi el log rota aunque el hijo viva meses.
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    cmd.spawn().map_err(|e| anyhow!("Cannot spawn Node child: {}", e))
}

/// Dos hilos que copian la salida del hijo, linea a linea, al log mensual.
fn capture_output(child: &mut Child) {
    if let Some(out) = child.stdout.take() {
        std::thread::spawn(move || pump_stdout(out));
    }
    if let Some(err) = child.stderr.take() {
        std::thread::spawn(move || pump_stderr(err));
    }
}

fn pump_stdout(out: ChildStdout) {
    for linea in BufReader::new(out).lines().map_while(Result::ok) {
        log::child_line("out", &linea);
    }
}

fn pump_stderr(err: ChildStderr) {
    for linea in BufReader::new(err).lines().map_while(Result::ok) {
        log::child_line("err", &linea);
    }
}
