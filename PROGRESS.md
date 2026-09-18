# 📋 node-winsvc — Estado del Proyecto

> **Lee este archivo primero.** Resume qué se construyó, qué falta, y cómo
> continuar sin perder contexto. Fecha de corte: **2026-09-18**.

---

## 🎯 Qué es este proyecto

Paquete npm (`npx node-winsvc`) que registra apps de Node.js como **servicios
nativos de Windows** usando la API Win32 Service Control Manager. Sin NSSM, sin
VBScript, sin `npm start` eterno. Núcleo en **Rust** + CLI en **TypeScript**.

La explicación completa de cómo y por qué está en
[`docs/ARQUITECTURA.md`](./docs/ARQUITECTURA.md).

---

## ✅ Hecho

### Capa TypeScript — `src/`
`main.ts` · `cli/index.ts` + 9 comandos (`init`, `install`, `uninstall`,
`start`, `stop`, `restart`, `status`, `logs`, `doctor`) ·
`models/winsvc-config.model.ts` · `services/{config,binary,process}.service.ts` ·
`utils/{logger,errors}.ts`

Compila limpio con `npx tsc`.

### Núcleo Rust — `rust/src/`
`main.rs` (clap: install/uninstall/start/stop/status/run/**ping**) ·
`models/config.rs` · `services/{scm,node_finder,service_config,**log**,**failure_actions**}.rs` ·
`commands/{install,uninstall,start,stop,status,run,**ping**}.rs`

Compila limpio con `cargo build --release`. Binario en `bin/node-winsvc-core.exe`.

### Endurecido el 2026-09-18 (v0.2.0)
Aplicando el patrón de producción de `api-interna-1` y `api-interna-2`:

- **`rust/.cargo/config.toml` con `+crt-static`** — el binario ya no dependía
  del runtime de C. Era el riesgo más serio: la compilación producía un exe
  dependiente **sin ningún aviso**, y eso se descubre en el servidor destino.
  Comprobado: `grep -a -c VCRUNTIME140 …` → **0**.
- **Acciones de fallo del SCM** — Windows relanza el supervisor al minuto dos
  veces, a los cinco la tercera, y cuenta también las salidas con código de
  error, no solo los crashes.
- **Log mensual con BOM UTF-8** — `logs/servicio-AAAA-MM.log`, el archivo se
  elige al escribir cada línea. La salida del hijo se copia línea a línea desde
  dos hilos, con los eventos del ciclo de vida alrededor.
- **`ping` + `doctor`** — diagnóstico antes de instalar; termina con código != 0
  para que el instalador aborte.
- **`instalar-servicio.cmd`** — doble clic, se eleva solo a administrador.

### ★ API de prueba — `api-logistica/`
API real de Node + SQL Server contra la base **`mi_base`**, sobre el flujo de
garantías. Es el banco de pruebas end-to-end del servicio.

**Probada en vivo el 2026-09-18** contra `localhost/mi_base`: los cuatro tipos
responden, la trazabilidad encadena correctamente
(`INGGAR 30441 → REVGAR 34741 → SALGAR 30149`), y `doctor` pasa entero.

> ⚠️ **El ingreso de garantía es `INGGAR`, con doble G.** No existe ningún
> `INGAR` en la base. La API acepta `INGAR` como alias y lo traduce.

---

### ★ Instalador propio — 2026-09-18 (noche)
Ya **no hace falta Inno Setup**. `installer` genera un `.exe` autocontenido:

- **`rust/src/setup/`** — el asistente en Win32 puro (~1 MB de stub): panel
  lateral turquesa con degradado y lista de pasos, botones pintados a mano
  (`BS_OWNERDRAW`), icono propio generado sin herramientas gráficas.
- **`rust/src/setup/paquete.rs`** — TAR comprimido de una pieza. La compresión
  **sólida** fue lo que bajó el instalador de 45 a 28 MB, no el algoritmo.
- **`rust/src/commands/empaquetar.rs`** — pega el paquete detrás del stub.
- **Modo demo**: `cargo run --bin node-winsvc-setup` abre el asistente con
  datos de ejemplo, sin instalar nada. Sirve para iterar sobre la interfaz.

Comparativa medida (node.exe de 80 MB + 7.900 archivos):

| Formato | Tamaño | Tiempo |
|---|---|---|
| ZIP + deflate | 45,4 MB | ~1 min |
| ZIP + zstd-19 | 37,2 MB | 5 min |
| Inno Setup (LZMA2 sólido) | 26,0 MB | ~4 min |
| **TAR + zstd-19 sólido** | **28,0 MB** | **1m46** |

### Completado el 2026-09-18 (tarde)
- **Comando `installer`** — genera el asistente `.exe` desde `winsvc.config.json`.
  Probado: 6.1 MB, páginas generadas desde `installer.prompts`, verificación
  previa con `scripts/probar-conexion.js`. Ver [docs/INSTALADOR.md](./docs/INSTALADOR.md).
- **Event Log de Windows** — arranque, parada, caída del hijo y fallos de
  arranque van al log de Aplicación. Era lo último que `node-windows` tenía
  y nosotros no.
- **PID y uptime reales en `status`** — `QueryServiceStatusEx`, con marca de
  arranque como respaldo. Ya **no exige administrador** para leer el estado.
- **20 tests con Jest** (`npm test`), en verde.

---

## ⏳ Pendiente

### 1. 🟡 Verificar el uptime con el servicio reiniciado
`status` ya devuelve el PID real. El uptime sale `null` hasta que el servicio se
reinicie con el núcleo nuevo, que es el que escribe la marca de arranque:

```powershell
# consola de Administrador
node dist\main.js restart
node dist\main.js status     # debe traer pid Y uptime
```

También conviene mirar el Visor de eventos (Aplicación, origen `node-winsvc`)
tras ese reinicio, para confirmar que los eventos entran.

### 2. 🟡 Prueba end-to-end como Administrador
Falta el paso que exige elevación:

```powershell
# En api-logistica/, doble clic en instalar-servicio.cmd
# o desde una consola de Administrador:
node ..\dist\main.js install
node ..\dist\main.js start
node ..\dist\main.js status          # debe decir "running"
curl http://localhost:3080/salud     # debe responder ok:true
# y confirmar en services.msc que sigue vivo tras reiniciar Windows
```

### 3. Más tests
- Faltan los de integración: instalar/arrancar/parar/desinstalar un servicio
  dummy. Los unitarios de la capa TS ya están (20, en verde).

### 4. Publicación npm
- `npm pack` → verificar que el `.tgz` incluye `bin/node-winsvc-core.exe`.
- `npm publish` (versión 0.2.0).

### 5. Mejoras menores
- Nadie limpia los logs viejos: rotan por mes pero no se borran.
- El Event Log no instala un archivo de mensajes, así que el Visor de eventos
  antepone un aviso de "falta la descripción". El texto se lee igual.

---

## 🧱 Reglas de arquitectura (Clean Code aplicado)

- Una responsabilidad por archivo. Sin lógica de negocio en `main`.
- Tope de 600 líneas por archivo; en la práctica ninguno pasa de 200.
- En la API: el SQL vive **solo** en los modelos, siempre parametrizado.

---

## 📍 Rutas

- **Este proyecto:** `D:\2026\node-winsvc-servicio-node\`
- **API de prueba:** `D:\2026\node-winsvc-servicio-node\api-logistica\`
- **Proyecto hermano (orquestador multi-MCP):** `D:\2026\RUST\HENRY MORENO-DEV\sentinel-hub\`
