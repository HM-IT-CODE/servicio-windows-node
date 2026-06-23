use anyhow::Result;
use windows::Win32::System::Services::*;

use crate::services::scm::{ScmHandle, ServiceHandle};

pub fn run(name: &str) -> Result<()> {
    let scm = ScmHandle::open_with_admin()?;
    let svc = ServiceHandle::open(&scm, name, SERVICE_START)?;

    unsafe {
        StartServiceW(svc.raw(), None)
            .map_err(|e| anyhow::anyhow!("StartServiceW failed: {}", e))?;
    }

    println!(r#"{{"ok":true,"name":"{}","message":"Service started"}}"#, name);
    Ok(())
}
