# node-winsvc

[![npm version](https://img.shields.io/npm/v/node-winsvc.svg)](https://www.npmjs.com/package/node-winsvc)
[![npm downloads](https://img.shields.io/npm/dm/node-winsvc.svg)](https://www.npmjs.com/package/node-winsvc)
[![license](https://img.shields.io/npm/l/node-winsvc.svg)](./LICENSE)
[![platform](https://img.shields.io/badge/platform-Windows-blue.svg)](#)
[![built with Rust](https://img.shields.io/badge/core-Rust-orange.svg)](#)

> Register Node.js apps as **native Windows services**. No NSSM. No VBScript. No `npm start` running forever in a forgotten terminal.

`node-winsvc` turns any Node.js application into a first-class Windows service using the **native Win32 Service Control Manager API** — the same mechanism SQL Server, IIS and the Windows kernel use. The heavy lifting is done by a small Rust binary; you drive it with a friendly CLI.

## Install

```bash
npm install --save-dev node-winsvc
# or use it on demand, no install:
npx node-winsvc <command>
```

## Quick start

```bash
npx node-winsvc init       # create winsvc.config.json
npx node-winsvc install    # register as a Windows service  (Administrator)
npx node-winsvc start      # start it                        (Administrator)
npx node-winsvc status     # check it                        (Administrator)
npx node-winsvc stop
npx node-winsvc uninstall
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
| `node-windows`      | Generates VBScript wrappers; fragile, hard to debug.          |

`node-winsvc` registers the service **directly** with `CreateServiceW`. Windows itself supervises the process, restarts it on boot, and shows it in `services.msc`.

---

## Architecture

```
┌─────────────────────────────┐
│  CLI  (TypeScript, strict)  │   npx node-winsvc <command>
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
- **Rust core** — `node-winsvc-core.exe`, a tiny clap CLI that talks to the Win32 Service Control Manager. Shipped *inside* the npm package (`bin/node-winsvc-core.exe`).

---

## Installation

```bash
npm install --save-dev node-winsvc
# or run on demand:
npx node-winsvc <command>
```

> **Windows only.** Requires Administrator privileges for `install` / `uninstall` / `start` / `stop` (the Service Control Manager demands it).

---

## Configuration — `winsvc.config.json`

`npx node-winsvc init` creates this template:

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
| `status`    | Query state (`QueryServiceStatus`) → JSON.            | yes   |

---

## Building from source

```bash
# 1. Build the Rust core
npm run build:rust       # cargo build --release
npm run bundle:exe       # copies the exe into bin/

# 2. Build the TypeScript CLI
npm run build            # tsc → dist/
```

---

## Roadmap

- [ ] Service-runner mode: the Rust core also acts **as** the service host (`StartServiceCtrlDispatcherW` + child-process supervision + auto-restart).
- [ ] Live log tail: `node-winsvc logs`.
- [ ] `npx node-winsvc doctor` — diagnose permissions / Node path / config.
- [ ] Health-check hooks.

---

## License

MIT © Henry Moreno
