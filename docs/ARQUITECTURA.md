# Arquitectura de `node-winsvc`

> Cómo un binario Rust convierte cualquier app de Node.js en un servicio nativo
> de Windows. Documento de referencia del proyecto
> `D:\2026\node-winsvc-servicio-node`. Fecha de corte: **2026-09-18**.

---

## 1. Qué resuelve

`sc create MiServicio binPath="C:\app\node.exe server.js"` **no funciona**. El
Service Control Manager (SCM) de Windows espera que el proceso que arranca se
registre y le reporte su estado en los primeros 30 segundos. `node.exe` nunca
reporta nada, así que Windows lo mata con *"el servicio no respondió a tiempo"*.

Desde afuera parece que la app está rota, y no lo está: simplemente no habla el
protocolo de servicios de Windows.

`node-winsvc` pone en el medio un **supervisor escrito en Rust** que sí lo habla.

---

## 2. El supervisor de dos niveles

El servicio **no es** la app de Node: es un supervisor que la lanza como hijo.

```
   SCM (Windows)
     │  arranca lo que diga BinaryPathName
     ▼
   node-winsvc-core.exe run --name <svc>     ← el supervisor: habla con el SCM
     │  lanza como proceso hijo
     ▼
   node.exe [nodeArgs] <script>               ← tu app, sin una línea de cambio
```

Qué gana cada lado:

- **La app de Node no cambia.** El mismo `server.js` que corres a mano en la
  terminal es el que corre bajo el servicio.
- **El supervisor** reporta `RUNNING` al SCM, escribe el log y relanza al hijo
  si muere (crash, `process.exit`, kill) cuando `autoRestart` está activo.
- **Si el supervisor muriera**, lo reinicia el propio Windows gracias a las
  acciones de fallo que se configuran al instalar.

Tres niveles de red, y ninguno depende de que alguien esté mirando la pantalla.

---

## 3. Las dos capas del paquete

```
┌─────────────────────────────────┐
│  CLI  (TypeScript, strict)      │   npx node-winsvc <comando>
│  src/cli/*.command.ts           │   lee winsvc.config.json, valida, arma args
└──────────────┬──────────────────┘
               │ spawnSync
               ▼
┌─────────────────────────────────┐
│  Núcleo  (Rust + Win32)         │   bin/node-winsvc-core.exe
│  rust/src/commands/*.rs         │   CreateServiceW / StartServiceW / ...
└──────────────┬──────────────────┘
               │ API Win32 del SCM
               ▼
   Service Control Manager de Windows
```

La capa TypeScript **no toca Windows**: solo lee configuración y traduce a
argumentos de línea de comandos. Toda la interacción con el sistema operativo
vive en Rust. Eso mantiene el CLI trivial de probar y el núcleo sin dependencias
de Node.

### Capa TypeScript — `src/`

| Archivo | Responsabilidad |
|---|---|
| `main.ts` | Entry point: verifica Windows y despacha. |
| `cli/index.ts` | Mapa de comandos + ayuda. |
| `cli/*.command.ts` | Un comando por archivo; arma los args del exe. |
| `models/winsvc-config.model.ts` | `WinsvcConfig` + `DEFAULT_CONFIG`. |
| `services/config.service.ts` | Lee/crea `winsvc.config.json`, resuelve rutas. |
| `services/binary.service.ts` | Localiza `node-winsvc-core.exe`. |
| `services/process.service.ts` | `spawnSync` al binario, traduce errores. |
| `utils/logger.ts`, `utils/errors.ts` | Salida con color, errores tipados. |

### Núcleo Rust — `rust/src/`

| Archivo | Responsabilidad |
|---|---|
| `main.rs` | CLI `clap`: install/uninstall/start/stop/status/run/ping. |
| `models/config.rs` | `InstallArgs`, `ServiceStatus`. |
| `services/scm.rs` | `ScmHandle` / `ServiceHandle` con RAII, `to_wide()`. |
| `services/node_finder.rs` | Encuentra `node.exe` (`where node` + rutas comunes). |
| `services/service_config.rs` | Persiste la config en `%ProgramData%\node-winsvc\<name>.json`. |
| `services/log.rs` | Log mensual del supervisor, con BOM UTF-8. |
| `services/failure_actions.rs` | Acciones de fallo del SCM. |
| `commands/install.rs` | `CreateServiceW` + descripción + acciones de fallo. |
| `commands/run.rs` | **El host del servicio**: `ServiceMain` + supervisión. |
| `commands/ping.rs` | Diagnóstico previo a instalar. |

---

## 4. Las decisiones, y por qué

### El `BinaryPathName` apunta al propio exe, no a `node.exe`

```rust
let bin_path = format!("\"{}\" run --name \"{}\"", self_exe.display(), args.name);
```

Windows arranca `node-winsvc-core.exe run --name <svc>`, no Node. Ese proceso
implementa el protocolo de servicio y **después** lanza Node como hijo. Es la
razón entera de que esto funcione.

### La configuración se persiste en `%ProgramData%`

Cuando Windows arranca el servicio no hay línea de comandos con la config ni
directorio de trabajo útil (un servicio arranca en `System32`). Por eso
`install` guarda un JSON en `%ProgramData%\node-winsvc\<name>.json` y `run` lo
lee al arrancar. `uninstall` lo borra.

### El directorio de trabajo es explícito

`workingDirectory` del config se resuelve a ruta absoluta en el CLI y se
inyecta con `cmd.current_dir()`. Sin esto, el `.env` de tu app no aparece y las
rutas relativas escriben en lugares absurdos dentro de `System32`.

### El log es mensual y con BOM

`logs\servicio-AAAA-MM.log`, y el archivo se elige **al escribir cada línea**,
así rota solo aunque el servicio viva meses. El BOM de UTF-8 no es capricho:
sin él, `Get-Content` de PowerShell 5.1 y el Bloc de notas leen el archivo como
ANSI y los acentos salen como `arrancÃ³`.

La salida del hijo se copia línea a línea al log desde el supervisor, con dos
hilos leyendo su `stdout` y su `stderr`. Va por el supervisor y no directo al
archivo para que el mes se decida en cada línea: así el log rota aunque el hijo
viva meses.

### Acciones de fallo configuradas al instalar

Reiniciar al minuto las dos primeras veces, a los cinco la tercera, y el
contador se reinicia tras un día sano. Además se activa el reinicio ante
**salidas con código de error**, no solo ante crashes: por omisión Windows
ignora un `process.exit(1)`.

### Corre como LocalSystem

`CreateServiceW` recibe `NULL` como cuenta. Si el servicio necesita
credenciales de red (un recurso compartido, autenticación integrada contra SQL
Server), hay que cambiarlo por una cuenta de dominio. Para conectar a SQL
Server con usuario y contraseña del `.env`, LocalSystem alcanza.

### El `ping` antes de instalar

El instalador prueba las dependencias y aborta si fallan. Más vale descubrir un
`.env` mal puesto ahí que con el servicio arrancando y cayéndose en bucle.

---

## 5. El binario tiene que ser autocontenido

```toml
# rust/.cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

Comprobación obligatoria antes de copiar el exe a cualquier servidor:

```bash
grep -a -c VCRUNTIME140 rust/target/release/node-winsvc-core.exe   # tiene que dar 0
```

No es opcional. Sin ese archivo, la compilación produce un exe dependiente del
runtime de C **sin ningún aviso**, y eso se descubre en el servidor destino,
donde no arranca y el mensaje de Windows no dice por qué.

---

## 6. Ciclo de vida completo

```
npx node-winsvc init       →  crea winsvc.config.json
npx node-winsvc doctor     →  ping: Node, script, permisos, config
npx node-winsvc install    →  guarda config + CreateServiceW + failure actions
npx node-winsvc start      →  StartServiceW
   └─ Windows lanza: node-winsvc-core.exe run --name <svc>
        └─ RegisterServiceCtrlHandlerW
        └─ SetServiceStatus(START_PENDING → RUNNING)
        └─ supervise(): spawn node.exe, poll cada 500 ms
              ├─ hijo muere + autoRestart → relanza al segundo
              └─ llega STOP → kill hijo, SetServiceStatus(STOPPED)
npx node-winsvc status     →  QueryServiceStatus → JSON
npx node-winsvc logs -f    →  tail del log
npx node-winsvc restart    →  stop + start
npx node-winsvc uninstall  →  DeleteService + borra config persistida
```

---

## 7. Comandos del día a día

```powershell
sc query  MiServicio                  # estado
sc start  MiServicio
sc stop   MiServicio
npx node-winsvc status

# el log, en vivo
powershell Get-Content .\logs\servicio-*.log -Tail 20 -Wait
```

Para **actualizar el exe**: detener el servicio, copiar encima, arrancar.
Windows mantiene el archivo bloqueado mientras corre, así que copiar sin
detener falla.

---

## 8. Lo que falta

- **Los logs no se borran nunca.** Rotan por mes, pero nadie limpia los viejos.
  Si el servicio es hablador conviene una tarea programada que borre los de más
  de seis meses.
- **`status` no devuelve el PID real** del proceso hijo de Node ni el uptime.
- **`status` exige administrador** porque `open_with_admin` pide
  `SC_MANAGER_ALL_ACCESS`; para leer bastaría `SC_MANAGER_CONNECT`.
- **Sin tests automatizados.** Ni unitarios de la capa TS ni de integración.

---

## 9. Adoptar este patrón en otro proyecto Rust

1. Copiar `rust/src/commands/run.rs` y `rust/src/services/scm.rs`. Cambiar el
   nombre del servicio y lo que lanza el hijo.
2. Copiar `rust/.cargo/config.toml` (runtime de C estático) y **comprobarlo**.
3. En `main.rs`, agregar los brazos `run` y `ping`.
4. Copiar `instalar-servicio.cmd` y reemplazar nombres de exe y servicio.
5. Que el programa tenga un subcomando **`ping`** que pruebe sus dependencias y
   termine con código distinto de cero si algo falla. El instalador lo usa.
