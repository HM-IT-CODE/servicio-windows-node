# Changelog

All notable changes to **node-winsvc** are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-09-18

Hardening pass based on the production pattern used by `api-interna-1`
and `api-interna-2`.

### Added
- **`doctor` command** — diagnoses config, core binary, Node, script and
  working directory before installing. Exits non-zero so installers can abort.
- **`ping` subcommand in the Rust core** — dependency check used by `doctor`.
- **SCM failure actions** — Windows restarts the supervisor after 1 min, 1 min,
  then 5 min, resetting after a healthy day. `SERVICE_CONFIG_FAILURE_ACTIONS_FLAG`
  is enabled so a non-zero exit counts as a failure, not just a crash.
- **Monthly log rotation with UTF-8 BOM** — `logs/servicio-AAAA-MM.log`. The
  file is picked per written line, so it rotates even if the child runs for
  months. The BOM keeps accents readable in PowerShell 5.1 and Notepad.
- **Child output captured line by line** — two threads pump the child's stdout
  and stderr into the supervisor log, with lifecycle events (spawn, exit code,
  restart, stop) recorded around them.
- **`instalar-servicio.cmd`** — self-elevating double-click installer that
  checks everything before registering anything.
- **`docs/ARQUITECTURA.md`** — the two-level supervisor explained, decision by
  decision.
- **`api-logistica/`** — a real Node.js + SQL Server API used as the end-to-end
  test bed, installable as a service with a double click.
- **Built-in installer wizard** — `installer` now ships its own Win32 wizard
  (`node-winsvc-setup.exe`, ~1 MB) with the app appended to it, so **no Inno
  Setup is needed**. Turquoise sidebar with step list, owner-drawn buttons,
  embedded icon. `--inno` still uses Inno Setup if preferred.
  Packaging is TAR + solid zstd: 28 MB for a 7,900-file app with `node.exe`
  bundled, against 45 MB with per-file ZIP compression and 26 MB for Inno.
- **`installer` command** — builds a Windows installer wizard (.exe) straight
  from `winsvc.config.json`. Declare `installer.prompts` and the wizard asks for
  them, writes them to a `.env`, runs `installer.verifyScript`, and registers the
  service only if verification passes. Bundles `node_modules`, so the target
  server needs no internet. Compiles with Inno Setup if installed; otherwise it
  writes the `.iss` for you to compile elsewhere. `--no-compile` skips the build.
  As far as we could tell, no other npm package in this category generates its
  own installer.

- **`installer.bundleNode`** — ships `node.exe` inside the installer
  (`vendor/node.exe`), which the service prefers over the system Node. The
  target server then needs **nothing** installed, and the Node version is
  pinned. ~6 MB → ~26 MB.
- **Windows Event Log** — the supervisor now reports start, stop, child crash
  and startup failures to the Application log, where a sysadmin looks first.
  This was the last feature `node-windows` had that we lacked.
- **Real PID and uptime in `status`** — via `QueryServiceStatusEx`. Uptime falls
  back to a startup marker when `OpenProcess` is denied, so reading the status
  no longer needs elevation at all (`status` now opens the SCM with
  `SC_MANAGER_CONNECT` instead of `SC_MANAGER_ALL_ACCESS`).
- **Test suite** — 20 Jest tests covering config parsing/defaults and installer
  script generation.

### Fixed
- **`nodeArgs` starting with `-` broke `install`.** With
  `"nodeArgs": ["--expose-gc"]`, clap parsed the value as its own flag and died
  with `unexpected argument '--expose-gc' found`. Fixed with
  `allow_hyphen_values` on `--node-args` and `--env`.
- **The binary was not self-contained.** Added `rust/.cargo/config.toml` with
  `+crt-static`. Without it the build silently produced an exe depending on
  VCRUNTIME140.dll, which fails to start on a clean server with an error message
  that does not say why. Verify with
  `grep -a -c VCRUNTIME140 rust/target/release/node-winsvc-core.exe` → must be 0.

## [0.1.1] - 2026-06-23

### Added
- **`restart` command** — stops and starts the service in one step.
- **`logs` command** — prints the tail of the service log file.
  Supports `-f` / `--follow` to stream new output and `-n <N>` to set how many
  lines to show (default 50).

## [0.1.0] - 2026-06-23

First public release. 🎉

### Added
- **Native Windows service registration** via the Win32 Service Control Manager
  API (`CreateServiceW` / `DeleteService` / `StartServiceW` / `ControlService` /
  `QueryServiceStatus`) — no NSSM, no VBScript, no external dependencies.
- **Service host mode** (`run`): the bundled Rust binary acts as the service
  process itself (`StartServiceCtrlDispatcherW` + `ServiceMain`), supervising the
  Node.js child process, reporting status to Windows, and **auto-restarting** it
  on crash when `autoRestart` is enabled.
- **CLI commands:** `init`, `install`, `uninstall`, `start`, `stop`, `status`.
- **Config file** `winsvc.config.json`: name, displayName, description, script,
  nodeArgs, env, autoRestart, startType, logFile, workingDirectory.
- **Env injection, working directory, and log redirection** for the hosted app.
- TypeScript CLI (clean-code layered) + Rust core, shipped together; the Rust
  binary (`node-winsvc-core.exe`) is bundled inside the npm package.

### Notes
- Windows only. `install` / `uninstall` / `start` / `stop` / `status` require
  Administrator privileges (enforced by the Service Control Manager).

[0.1.1]: https://github.com/HM-IT-CODE/servicio-windows-node/releases/tag/v0.1.1
[0.1.0]: https://github.com/HM-IT-CODE/servicio-windows-node/releases/tag/v0.1.0
