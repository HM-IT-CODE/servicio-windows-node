# 📋 node-winsvc — Estado del Proyecto (para Antigravity / continuación)

> **Lee este archivo primero.** Resume qué se construyó, qué falta, y cómo continuar
> sin perder contexto. Fecha de corte: **2026-06-23**.

---

## 🎯 Qué es este proyecto

Paquete npm (`npx node-winsvc`) que registra apps de Node.js como **servicios nativos
de Windows** usando la API Win32 Service Control Manager. Sin NSSM, sin VBScript, sin
`npm start` eterno. Núcleo en **Rust** + CLI en **TypeScript**.

Slogan: *"Modern replacement for node-windows / NSSM."*

---

## ✅ Hecho (compila limpio)

### Capa TypeScript — 100%
```
src/
  main.ts                         ← entry point (assertWindows + dispatch)
  models/winsvc-config.model.ts   ← WinsvcConfig + DEFAULT_CONFIG
  utils/logger.ts                 ← logger con colores
  utils/errors.ts                 ← errores tipados (WinsvcError)
  services/config.service.ts      ← lee/crea winsvc.config.json
  services/binary.service.ts      ← localiza node-winsvc-core.exe
  services/process.service.ts     ← spawnSync al binario Rust
  cli/init.command.ts             ← crea config template
  cli/install.command.ts          ← arma args y llama Rust
  cli/uninstall.command.ts
  cli/start.command.ts
  cli/stop.command.ts
  cli/status.command.ts
  cli/index.ts                    ← COMMANDS map + dispatch + help
```

### Núcleo Rust (`node-winsvc-core.exe`) — 100% (CLI + service host)
```
rust/src/
  main.rs                  ← clap CLI (install/uninstall/start/stop/status/run)
  models/config.rs         ← InstallArgs
  services/scm.rs          ← ScmHandle, ServiceHandle (RAII), to_wide()
  services/node_finder.rs  ← find_node_exe()
  services/service_config.rs ← persiste config en %ProgramData%\node-winsvc\<name>.json
  commands/install.rs      ← CreateServiceW; BinaryPathName = self.exe run --name <svc>
  commands/uninstall.rs    ← DeleteService + borra config persistida
  commands/start.rs        ← StartServiceW
  commands/stop.rs         ← ControlService(STOP)
  commands/status.rs       ← QueryServiceStatus → JSON
  commands/run.rs          ← ★ SERVICE HOST: ServiceMain + supervisión del hijo Node
```

**Estado de compilación:** `cargo build` ✅ + `cargo build --release` ✅. Binario
empaquetado en `bin/node-winsvc-core.exe`. `--help` muestra el comando `run`.

### ★ Modo service-runner (run.rs) — HECHO 2026-06-23
- `StartServiceCtrlDispatcherW` → conecta con el SCM
- `service_main` (extern "system") + `RegisterServiceCtrlHandlerW`
- Reporta `START_PENDING` → `RUNNING` → `STOP_PENDING` → `STOPPED` vía `SetServiceStatus`
- `supervise()`: lanza `node [nodeArgs] script` como proceso hijo, hace polling con
  `try_wait()`, **reinicia** si crashea y `auto_restart=true`, mata al hijo al recibir STOP
- Inyecta `env` (parseado del JSON), `working_dir`, y redirige stdout/stderr al `log_file`
- Globals seguros: `OnceLock<String>` (nombre), `AtomicBool` (stop), `AtomicIsize` (status handle)

---

## ⏳ Pendiente (lo que falta para v1.0)

### 1. 🟡 Prueba end-to-end real (como Administrador)
El service-runner ya está implementado y compila. Falta probarlo en vivo con un
servicio real para confirmar que Windows lo marca `running`:
```powershell
# 1. compilar TS
npm run build
# 2. crear una app node dummy (server.js que escuche un puerto)
# 3. node-winsvc init   → editar winsvc.config.json
# 4. (terminal Admin) node dist\main.js install
# 5. node dist\main.js start
# 6. node dist\main.js status   → debe decir "running"
# 7. confirmar en services.msc que aparece y sigue vivo
```

### 2. Tests
- Jest para la capa TS (config parsing, arg building).
- Test de integración: instalar/arrancar/parar/desinstalar un servicio dummy.

### 3. Publicación npm
- `README.md` ✅ ya creado (inglés).
- `git init` + commit (⚠️ aún SIN git — protegerlo ya).
- Crear repo GitHub.
- `npm pack` → verificar que el `.tgz` incluye `bin/node-winsvc-core.exe`.
- `npm publish`.

### 4. Mejoras menores
- `scm::open_with_admin` usa `SC_MANAGER_ALL_ACCESS`; `status` podría usar
  `SC_MANAGER_CONNECT` para no exigir admin en lectura.
- `status` podría devolver el PID real del proceso hijo Node.

---

## 🚀 Cómo continuar (siguiente sesión)

1. **`git init` + primer commit** — el proyecto aún no tiene control de versiones.
2. `npm run build` (compila TS → dist/) y probar end-to-end como Admin (ver arriba).
3. `npm pack` y revisar contenido del paquete.
4. Crear repo GitHub + `npm publish`.

---

## 🧱 Reglas de arquitectura (Clean Code aplicado)

- main ≤ 50 líneas · router/cli ≤ 30 · command/controller ≤ 100 · service ≤ 150 · model ≤ 50 · util ≤ 60
- Una responsabilidad por archivo. Sin lógica de negocio en `main`.

---

## 📍 Rutas

- **Este proyecto:** `D:\2026\node-winsvc-servicio-node\`
- **Proyecto hermano (orquestador multi-MCP):** `D:\2026\RUST\HENRY MORENO-DEV\sentinel-hub\`
