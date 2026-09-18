//! Estado del servicio, con PID y uptime reales.
//!
//! `QueryServiceStatus` (a secas) no devuelve el PID: hay que usar la variante
//! Ex con `SC_STATUS_PROCESS_INFO`. El PID que da es el del SUPERVISOR, no el
//! del hijo Node, que es lo correcto: es el proceso que Windows controla.
//!
//! Para leer el estado basta `SC_MANAGER_CONNECT`; no exigimos administrador
//! como en install/start/stop.

use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::System::Services::*;
use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::services::scm::{ScmHandle, ServiceHandle};
use crate::services::service_config;

pub fn run(name: &str) -> Result<()> {
    let scm = ScmHandle::open_for_query()?;
    let svc = ServiceHandle::open(&scm, name, SERVICE_QUERY_STATUS)?;

    let info = query_process_info(&svc)?;
    let state = state_to_str(info.dwCurrentState);

    let pid = if info.dwProcessId == 0 { None } else { Some(info.dwProcessId) };
    // GetProcessTimes es lo preciso, pero OpenProcess contra un proceso de
    // LocalSystem exige elevacion. Si no se puede, cae a la marca que deja el
    // supervisor al arrancar, que no necesita permiso alguno.
    let uptime = match pid.and_then(uptime_de) {
        Some(u) => Some(u),
        None if info.dwCurrentState == SERVICE_RUNNING => {
            service_config::segundos_desde_arranque(name).map(formatear)
        }
        None => None,
    };

    println!(
        r#"{{"ok":true,"name":"{}","state":"{}","pid":{},"uptime":{}}}"#,
        name,
        state,
        pid.map(|p| p.to_string()).unwrap_or_else(|| "null".into()),
        uptime.map(|u| format!("\"{u}\"")).unwrap_or_else(|| "null".into()),
    );
    Ok(())
}

/// La variante Ex es la unica que trae el PID del proceso del servicio.
fn query_process_info(svc: &ServiceHandle) -> Result<SERVICE_STATUS_PROCESS> {
    let mut buffer = vec![0u8; std::mem::size_of::<SERVICE_STATUS_PROCESS>()];
    let mut needed = 0u32;

    unsafe {
        QueryServiceStatusEx(
            svc.raw(),
            SC_STATUS_PROCESS_INFO,
            Some(&mut buffer),
            &mut needed,
        )
        .map_err(|e| anyhow!("QueryServiceStatusEx failed: {}", e))?;

        Ok(*(buffer.as_ptr() as *const SERVICE_STATUS_PROCESS))
    }
}

/// Tiempo transcurrido desde que arranco el proceso, como "2d 4h 13m".
fn uptime_de(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };

    let mut creacion = FILETIME::default();
    let (mut salida, mut kernel, mut usuario) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default());

    let ok = unsafe {
        GetProcessTimes(handle, &mut creacion, &mut salida, &mut kernel, &mut usuario).is_ok()
    };
    unsafe { let _ = CloseHandle(handle); }

    if !ok {
        return None;
    }

    let ahora = ahora_filetime()?;
    let inicio = filetime_a_u64(creacion);
    if ahora <= inicio {
        return None;
    }

    // FILETIME cuenta en intervalos de 100 nanosegundos.
    Some(formatear(((ahora - inicio) / 10_000_000) as u64))
}

fn ahora_filetime() -> Option<u64> {
    // Misma escala que el tiempo de creacion del proceso: intervalos de 100 ns
    // desde 1601. Restarlos da el uptime sin convertir a ningun otro formato.
    let ahora = unsafe { GetSystemTimeAsFileTime() };
    Some(filetime_a_u64(ahora))
}

fn filetime_a_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn formatear(segundos: u64) -> String {
    let dias = segundos / 86_400;
    let horas = (segundos % 86_400) / 3_600;
    let minutos = (segundos % 3_600) / 60;

    if dias > 0 {
        format!("{dias}d {horas}h {minutos}m")
    } else if horas > 0 {
        format!("{horas}h {minutos}m")
    } else {
        format!("{minutos}m")
    }
}

fn state_to_str(state: SERVICE_STATUS_CURRENT_STATE) -> &'static str {
    match state {
        SERVICE_STOPPED          => "stopped",
        SERVICE_START_PENDING    => "start_pending",
        SERVICE_STOP_PENDING     => "stop_pending",
        SERVICE_RUNNING          => "running",
        SERVICE_CONTINUE_PENDING => "continue_pending",
        SERVICE_PAUSE_PENDING    => "pause_pending",
        SERVICE_PAUSED           => "paused",
        _                        => "unknown",
    }
}
