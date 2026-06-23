//! Service host mode. Windows launches `node-winsvc-core.exe run --name <svc>`,
//! and THIS code implements the Win32 service protocol: it registers a control
//! handler, reports RUNNING, then spawns and supervises the Node.js child process,
//! restarting it on crash (if auto_restart) until the SCM asks it to stop.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::System::Services::*;

use crate::models::InstallArgs;
use crate::services::node_finder::find_node_exe;
use crate::services::scm::to_wide;
use crate::services::service_config;

static SERVICE_NAME:   OnceLock<String> = OnceLock::new();
static STOP_REQUESTED: AtomicBool       = AtomicBool::new(false);
static STATUS_HANDLE:  AtomicIsize      = AtomicIsize::new(0);

const POLL_INTERVAL_MS: u64 = 500;
const RESTART_DELAY_MS:  u64 = 1000;

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
    let node = match find_node_exe() {
        Ok(n) => n,
        Err(_) => return,
    };

    loop {
        let mut child = match spawn_child(&node, &cfg) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Poll the child until it exits or a stop is requested
        loop {
            if STOP_REQUESTED.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            match child.try_wait() {
                Ok(Some(_)) => break,         // child exited on its own
                Ok(None)    => std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS)),
                Err(_)      => break,
            }
        }

        if STOP_REQUESTED.load(Ordering::SeqCst) || !cfg.auto_restart {
            return;
        }
        std::thread::sleep(Duration::from_millis(RESTART_DELAY_MS));
    }
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

    redirect_logs(&mut cmd, &cfg.log_file);

    cmd.spawn().map_err(|e| anyhow!("Cannot spawn Node child: {}", e))
}

fn redirect_logs(cmd: &mut Command, log_file: &str) {
    if log_file.is_empty() {
        return;
    }
    if let Some(parent) = Path::new(log_file).parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(log_file) {
        if let Ok(clone) = file.try_clone() {
            cmd.stdout(Stdio::from(file));
            cmd.stderr(Stdio::from(clone));
        }
    }
}
