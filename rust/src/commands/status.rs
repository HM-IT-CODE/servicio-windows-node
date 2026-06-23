use anyhow::Result;
use windows::Win32::System::Services::*;

use crate::services::scm::{ScmHandle, ServiceHandle};

pub fn run(name: &str) -> Result<()> {
    let scm = ScmHandle::open_with_admin()?;
    let svc = ServiceHandle::open(&scm, name, SERVICE_QUERY_STATUS)?;

    let mut status = SERVICE_STATUS::default();
    unsafe {
        QueryServiceStatus(svc.raw(), &mut status)
            .map_err(|e| anyhow::anyhow!("QueryServiceStatus failed: {}", e))?;
    }

    let state = state_to_str(status.dwCurrentState);

    println!(
        r#"{{"ok":true,"name":"{}","state":"{}","pid":null}}"#,
        name, state
    );
    Ok(())
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
