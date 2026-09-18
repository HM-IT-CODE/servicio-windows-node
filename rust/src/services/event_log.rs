//! Registro en el Visor de eventos de Windows.
//!
//! Es donde un administrador de sistemas mira PRIMERO cuando un servicio falla,
//! antes que cualquier archivo de log. Un servicio que solo escribe a un .log
//! en su carpeta es invisible para las herramientas de monitoreo del sistema.
//!
//! Se registra bajo el origen "node-winsvc" en el log de Aplicacion. Usamos
//! `RegisterEventSourceW` sin instalar un archivo de mensajes: Windows muestra
//! el texto igual, precedido de un aviso de que falta la descripcion del
//! evento. Es un compromiso consciente: instalar un .dll de mensajes obligaria
//! a tocar el registro en cada instalacion, y el texto se lee bien tal cual.

use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::PCWSTR;
use windows::Win32::System::EventLog::{
    DeregisterEventSource, RegisterEventSourceW, ReportEventW,
    EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE, REPORT_EVENT_TYPE,
};

use crate::services::scm::to_wide;

const ORIGEN: &str = "node-winsvc";

/// Si el registro falla una vez (permisos, origen no registrable), dejamos de
/// intentarlo: no vale la pena pagar el coste en cada linea del log.
static DESACTIVADO: AtomicBool = AtomicBool::new(false);

pub fn info(mensaje: &str) {
    escribir(EVENTLOG_INFORMATION_TYPE, 1000, mensaje);
}

pub fn warn(mensaje: &str) {
    escribir(EVENTLOG_WARNING_TYPE, 2000, mensaje);
}

pub fn error(mensaje: &str) {
    escribir(EVENTLOG_ERROR_TYPE, 3000, mensaje);
}

fn escribir(tipo: REPORT_EVENT_TYPE, evento_id: u32, mensaje: &str) {
    if DESACTIVADO.load(Ordering::Relaxed) {
        return;
    }

    let origen_w = to_wide(ORIGEN);

    let handle = unsafe { RegisterEventSourceW(PCWSTR::null(), PCWSTR(origen_w.as_ptr())) };
    let Ok(handle) = handle else {
        DESACTIVADO.store(true, Ordering::Relaxed);
        return;
    };

    let mensaje_w = to_wide(mensaje);
    let cadenas = [PCWSTR(mensaje_w.as_ptr())];

    unsafe {
        let _ = ReportEventW(
            handle,
            tipo,
            0,          // categoria
            evento_id,
            None,       // SID del usuario
            0,          // bytes de datos crudos
            Some(&cadenas),
            None,
        );
        let _ = DeregisterEventSource(handle);
    }
}
