# Changelog

All notable changes to **node-winsvc** are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
