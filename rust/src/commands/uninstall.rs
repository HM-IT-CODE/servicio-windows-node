use anyhow::Result;
use windows::Win32::System::Services::*;

use crate::services::scm::{ScmHandle, ServiceHandle};
use crate::services::service_config;

pub fn run(name: &str) -> Result<()> {
    let scm = ScmHandle::open_with_admin()?;
    let svc = ServiceHandle::open(&scm, name, SERVICE_ALL_ACCESS)?;

    unsafe {
        DeleteService(svc.raw())
            .map_err(|e| anyhow::anyhow!("DeleteService failed: {}", e))?;
    }

    // Best-effort: remove the persisted config
    let _ = service_config::remove(name);

    println!(r#"{{"ok":true,"name":"{}","message":"Service uninstalled"}}"#, name);
    Ok(())
}
