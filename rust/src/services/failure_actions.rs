//! Acciones de fallo del servicio: que hace Windows si el supervisor muere.
//!
//! Es el tercer nivel de red. El supervisor relanza a Node; Windows relanza al
//! supervisor. Por omision Windows solo reacciona a CRASHES, no a una salida
//! con codigo de error, por eso hace falta
//! `SERVICE_CONFIG_FAILURE_ACTIONS_FLAG`.

use anyhow::{anyhow, Result};
use windows::Win32::System::Services::*;

const MINUTO:  u32 = 60_000;
const CINCO:   u32 = 300_000;
/// El contador de fallos se reinicia tras un dia sano (en segundos).
const RESET_S: u32 = 86_400;

/// Reiniciar al minuto las dos primeras veces, a los cinco la tercera.
pub fn configure(service: SC_HANDLE) -> Result<()> {
    let mut acciones = [
        SC_ACTION { Type: SC_ACTION_RESTART, Delay: MINUTO },
        SC_ACTION { Type: SC_ACTION_RESTART, Delay: MINUTO },
        SC_ACTION { Type: SC_ACTION_RESTART, Delay: CINCO  },
    ];

    let mut failure = SERVICE_FAILURE_ACTIONSW {
        dwResetPeriod: RESET_S,
        lpRebootMsg:   windows::core::PWSTR::null(),
        lpCommand:     windows::core::PWSTR::null(),
        cActions:      acciones.len() as u32,
        lpsaActions:   acciones.as_mut_ptr(),
    };

    unsafe {
        ChangeServiceConfig2W(
            service,
            SERVICE_CONFIG_FAILURE_ACTIONS,
            Some(&mut failure as *mut _ as *mut _),
        )
        .map_err(|e| anyhow!("No se pudieron fijar las acciones de fallo: {}", e))?;
    }

    enable_on_non_crash(service)
}

/// Sin esto, un `process.exit(1)` del hijo no cuenta como fallo para Windows.
fn enable_on_non_crash(service: SC_HANDLE) -> Result<()> {
    let mut flag = SERVICE_FAILURE_ACTIONS_FLAG {
        fFailureActionsOnNonCrashFailures: true.into(),
    };

    unsafe {
        ChangeServiceConfig2W(
            service,
            SERVICE_CONFIG_FAILURE_ACTIONS_FLAG,
            Some(&mut flag as *mut _ as *mut _),
        )
        .map_err(|e| anyhow!("No se pudo activar el reinicio ante salidas con error: {}", e))?;
    }
    Ok(())
}
