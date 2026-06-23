use anyhow::{anyhow, Result};
use windows::core::PCWSTR;
use windows::Win32::System::Services::*;

use crate::models::InstallArgs;
use crate::services::scm::{ScmHandle, to_wide};
use crate::services::node_finder::find_node_exe;
use crate::services::service_config;

pub fn run(args: &InstallArgs) -> Result<()> {
    // Validate Node is installed before registering anything
    find_node_exe()?;

    // Persist config so the `run` host can read it when Windows starts the service
    service_config::save(args)?;

    // The service host is THIS executable in `run` mode (not node.exe directly),
    // so Windows talks to a real service that supervises the Node child process.
    let self_exe = std::env::current_exe()
        .map_err(|e| anyhow!("Cannot resolve own exe path: {}", e))?;
    let bin_path = format!("\"{}\" run --name \"{}\"", self_exe.display(), args.name);

    let scm = ScmHandle::open_with_admin()?;

    let name_w     = to_wide(&args.name);
    let display_w  = to_wide(&args.display);
    let bin_path_w = to_wide(&bin_path);
    let desc_w     = to_wide(&args.description);

    let start_type = match args.start_type.as_str() {
        "manual"   => SERVICE_DEMAND_START,
        "disabled" => SERVICE_DISABLED,
        _          => SERVICE_AUTO_START, // "auto" default
    };

    let svc = unsafe {
        CreateServiceW(
            scm.raw(),
            PCWSTR(name_w.as_ptr()),
            PCWSTR(display_w.as_ptr()),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            start_type,
            SERVICE_ERROR_NORMAL,
            PCWSTR(bin_path_w.as_ptr()),
            PCWSTR::null(), // load order group
            None,           // tag id
            PCWSTR::null(), // dependencies
            PCWSTR::null(), // service account (LocalSystem)
            PCWSTR::null(), // password
        ).map_err(|e| anyhow!("CreateServiceW failed: {}", e))?
    };

    // Set description
    let mut desc = SERVICE_DESCRIPTIONW {
        lpDescription: windows::core::PWSTR(desc_w.as_ptr() as *mut _),
    };
    unsafe {
        let _ = ChangeServiceConfig2W(
            svc,
            SERVICE_CONFIG_DESCRIPTION,
            Some(&mut desc as *mut _ as *mut _),
        );
        let _ = CloseServiceHandle(svc);
    }

    println!(r#"{{"ok":true,"name":"{}","message":"Service installed"}}"#, args.name);
    Ok(())
}
