use anyhow::Result;
use windows::Win32::System::Services::*;

use crate::services::scm::{ScmHandle, ServiceHandle};

pub fn run(name: &str) -> Result<()> {
    let scm = ScmHandle::open_with_admin()?;
    let svc = ServiceHandle::open(&scm, name, SERVICE_STOP)?;

    let mut status = SERVICE_STATUS::default();
    unsafe {
        ControlService(svc.raw(), SERVICE_CONTROL_STOP, &mut status)
            .map_err(|e| anyhow::anyhow!("ControlService STOP failed: {}", e))?;
    }

    println!(r#"{{"ok":true,"name":"{}","message":"Stop signal sent"}}"#, name);
    Ok(())
}
