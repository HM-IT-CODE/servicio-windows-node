use anyhow::{anyhow, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::System::Services::*;

pub struct ScmHandle(SC_HANDLE);

impl ScmHandle {
    /// Abre el Service Control Manager con permisos para crear/eliminar servicios
    pub fn open_with_admin() -> Result<Self> {
        let handle = unsafe {
            OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
                .map_err(|e| anyhow!("Cannot open SCM (run as Administrator): {}", e))?
        };
        Ok(Self(handle))
    }

    pub fn raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ScmHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseServiceHandle(self.0); }
    }
}

pub struct ServiceHandle(SC_HANDLE);

impl ServiceHandle {
    pub fn open(scm: &ScmHandle, name: &str, access: u32) -> Result<Self> {
        let name_w = to_wide(name);
        let handle = unsafe {
            OpenServiceW(scm.raw(), PCWSTR(name_w.as_ptr()), access)
                .map_err(|_| anyhow!("Service \"{}\" not found", name))?
        };
        Ok(Self(handle))
    }

    pub fn raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseServiceHandle(self.0); }
    }
}

pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
