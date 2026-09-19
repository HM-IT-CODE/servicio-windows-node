# Aruna

[![npm version](https://img.shields.io/npm/v/aruna-winsvc.svg)](https://www.npmjs.com/package/aruna-winsvc)
[![npm downloads](https://img.shields.io/npm/dm/aruna-winsvc.svg)](https://www.npmjs.com/package/aruna-winsvc)
[![license](https://img.shields.io/npm/l/aruna-winsvc.svg)](./LICENSE)
[![platform](https://img.shields.io/badge/platform-Windows-blue.svg)](#)
[![built with Rust](https://img.shields.io/badge/core-Rust-orange.svg)](#)

> Register Node.js apps as **native Windows services** — and ship them as a double-click installer. No NSSM. No .NET. No `npm start` running forever in a forgotten terminal.

`aruna` turns any Node.js application into a first-class Windows service using the **native Win32 Service Control Manager API** — the same mechanism SQL Server, IIS and the Windows kernel use. The heavy lifting is done by a small Rust binary; you drive it with a friendly CLI.

## Install

```bash
npm install --save-dev aruna-winsvc
# or use it on demand, no install:
npx aruna <command>
```

## Two things nobody else does

**1. It builds your installer.**

```bash
npx aruna installer   →   instalar-my-api.exe
```

One `.exe` with everything inside: your code, `node_modules`, the service core
and — with `"bundleNode": true` — `node.exe` itself. The target server then
needs **nothing installed**. Double-click, answer a couple of questions, done.

It uninstalls like any Windows program too: it registers in *Add or remove
programs*, stops the service and unregisters it cleanly.

**2. It verifies before it registers.**

The wizard writes your `.env`, runs your check script, and registers the service
**only if the check passes**. A bad `.env` with the service already installed
produces a crash-loop that looks like a broken program from the outside.

## Quick start

```bash
npx aruna init       # create winsvc.config.json
npx aruna doctor     # diagnose before installing
npx aruna install    # register as a Windows service  (Administrator)
npx aruna start      # start it                        (Administrator)
npx aruna status     # check it                        (Administrator)
npx aruna logs -f    # tail the service log
npx aruna restart    # stop + start                    (Administrator)
npx aruna stop
npx aruna uninstall
npx aruna installer  # build a double-click installer .exe
```

A minimal `winsvc.config.json`:

```json
{
  "name": "my-api",
  "displayName": "My Node API",
  "description": "Production Node.js API service",
  "script": "dist/server.js",
  "nodeArgs": [],
  "env": { "NODE_ENV": "production", "PORT": "8080" },
  "autoRestart": true,
  "startType": "auto",
  "logFile": "logs/service.log",
  "workingDirectory": "."
}
```

That's it — your app now starts on boot, survives logoff, restarts on crash, and shows up in `services.msc`.

---

## Why

The usual ways to keep a Node app alive on Windows are all compromises:

| Approach            | Problem                                                        |
| ------------------- | ------------------------------------------------------------- |
| `npm start` in a terminal | Dies when the session closes. Not a real service.       |
| `pm2`               | Cross-platform, but not a native Windows service; needs pm2 itself running. |
| `NSSM`              | External `.exe` you must ship and trust; abandoned-ish.       |
| `node-windows`      | Last published 2021, still `1.0.0-beta`. Bundles winsw.exe, which needs **.NET Framework 2.0** — an optional, off-by-default feature on Windows Server. |

`aruna` registers the service **directly** with `CreateServiceW`. Windows itself supervises the process, restarts it on boot, and shows it in `services.msc`.

---

## Architecture

```
┌─────────────────────────────┐
│  CLI  (TypeScript, strict)  │   npx aruna <command>
│  src/cli/*.command.ts       │
└──────────────┬──────────────┘
               │ spawnSync (args)
               ▼
┌─────────────────────────────┐
│  Core  (Rust + Win32)       │   node-winsvc-core.exe
│  rust/src/commands/*.rs     │   CreateServiceW / StartServiceW / ...
└──────────────┬──────────────┘
               │ Win32 SCM API
               ▼
   Windows Service Control Manager
```

- **TypeScript layer** — reads `winsvc.config.json`, validates, locates the bundled binary, and calls it. Clean Code: models / services / cli / utils separated, one responsibility per file.
- **Rust core** — `node-winsvc-core.exe`, a tiny clap CLI that talks to the Win32 Service Control Manager. Shipped *inside* the npm package.
- **Installer wizard** — `node-winsvc-setup.exe`, a ~1 MB Win32 wizard. `installer` appends your app to it (TAR + solid zstd) to produce a single distributable `.exe`. No Electron, no WebView2, no .NET.

**Zero runtime dependencies.** The binaries link the C runtime statically, so
they start on a clean Windows Server with nothing installed. Verified on every
build:

```bash
grep -a -c VCRUNTIME140 bin/node-winsvc-core.exe   # must print 0
```

---

## Installation

```bash
npm install --save-dev aruna-winsvc
# or run on demand:
npx aruna <command>
```

> **Windows only.** Requires Administrator privileges for `install` / `uninstall` / `start` / `stop` (the Service Control Manager demands it).

---

## Configuration — `winsvc.config.json`

`npx aruna init` creates this template:

```json
{
  "name": "my-api",
  "displayName": "My Node API",
  "description": "Production Node.js API service",
  "script": "dist/server.js",
  "nodeArgs": [],
  "env": { "NODE_ENV": "production", "PORT": "8080" },
  "autoRestart": true,
  "startType": "auto",
  "logFile": "logs/service.log",
  "workingDirectory": "."
}
```

| Field              | Meaning                                                         |
| ------------------ | -------------------------------------------------------------- |
| `name`             | Internal Windows service name (no spaces).                     |
| `displayName`      | Name shown in `services.msc`.                                  |
| `description`      | Description shown in the Services panel.                       |
| `script`           | Entry `.js` relative to the project.                          |
| `nodeArgs`         | Extra args passed to `node.exe`.                              |
| `env`              | Environment variables injected into the process.              |
| `autoRestart`      | Restart the process if it crashes.                            |
| `startType`        | `auto` \| `manual` \| `disabled`.                            |
| `logFile`          | stdout/stderr log file (relative to project).                |
| `workingDirectory` | Working directory (default: project root).                   |

---

## Commands

| Command     | What it does                                          | Admin |
| ----------- | ----------------------------------------------------- | ----- |
| `init`      | Create `winsvc.config.json`.                          | no    |
| `install`   | Register the service (`CreateServiceW`).              | yes   |
| `uninstall` | Remove the service (`DeleteService`).                 | yes   |
| `start`     | Start the service (`StartServiceW`).                  | yes   |
| `stop`      | Stop the service (`ControlService` STOP).             | yes   |
| `restart`   | Stop then start the service.                          | yes   |
| `status`    | State + real PID + uptime (`QueryServiceStatusEx`).   | no    |
| `logs`      | Print the service log (`-f` follow, `-n <N>` lines).  | no    |
| `doctor`    | Diagnose config, core binary, Node and script paths.  | no    |
| `installer` | Build a double-click installer `.exe` (wizard included).| no    |

---

## Building from source

```bash
# 1. Build the Rust core
npm run build:rust       # cargo build --release
npm run bundle:exe       # copies the exe into bin/

# 2. MANDATORY: verify the binary is self-contained
grep -a -c VCRUNTIME140 rust/target/release/node-winsvc-core.exe   # must print 0

# 3. Build the TypeScript CLI
npm run build            # tsc → dist/
```

Step 2 is not optional. Without `rust/.cargo/config.toml` the build silently
produces an exe that depends on the C runtime and fails to start on a clean
server, with a Windows error message that does not say why.

## Why not `node-windows`?

It is the de-facto standard, and it still works — but it was last published in
2021, never left `1.0.0-beta`, and bundles `winsw.exe`, which requires
**.NET Framework 2.0** (from 2005). On Windows Server that is an optional
feature turned *off* by default: `npm i node-windows` installs fine and the
service then fails to start until someone enables .NET 3.5.

See [`docs/COMPARATIVA.md`](./docs/COMPARATIVA.md) for the full comparison,
including what this project still lacks.

## Docs and example

- [`docs/ARQUITECTURA.md`](./docs/ARQUITECTURA.md) — how the two-level
  supervisor works, and why each decision was made (Spanish).
- [`docs/INSTALADOR.md`](./docs/INSTALADOR.md) — the `installer` command:
  building a wizard `.exe` straight from your config (Spanish).
- [`api-logistica/`](./api-logistica) — a real Node.js API (SQL Server, warranty
  document flow) used as the end-to-end test bed. Double-click
  `instalar-servicio.cmd` and it runs as a native Windows service.

---

## Roadmap

- [x] Service-runner mode: the Rust core acts **as** the service host (`StartServiceCtrlDispatcherW` + child-process supervision + auto-restart).
- [x] Live log tail: `aruna logs -f`.
- [x] `aruna restart`.
- [x] `npx aruna doctor` — diagnose permissions / Node path / config.
- [x] Double-click installer (`instalar-servicio.cmd`), self-elevating.
- [x] Monthly log rotation with UTF-8 BOM + child output captured line by line.
- [x] SCM failure actions (restart on crash *and* on non-zero exit).
- [x] Statically linked CRT — the binary is self-contained.
- [x] `aruna installer` — generates a wizard `.exe` from your config,
      with prompts, `.env` generation and pre-flight verification.
- [x] Real PID + uptime in `status`, without requiring Administrator.
- [x] Windows Event Log support.
- [ ] Log retention (files rotate monthly but are never deleted).
- [ ] Health-check hooks.

---

## License

MIT © Henry Moreno
