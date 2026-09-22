# 📋 Aruna (`aruna-winsvc`) — Estado del Proyecto

> **Lee este archivo primero.** Resume qué se construyó, qué falta, y cómo
> continuar sin perder contexto. Fecha de corte: **2026-09-18**.

---

## 🎯 Qué es este proyecto

Paquete npm (`npx aruna`) que registra apps de Node.js como **servicios
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

### ★ API de prueba — `app-demo/`
Servidor Node mínimo, sin base de datos, que sirve de banco de pruebas
end-to-end: `npx aruna installer` lo convierte en un `.exe` de doble clic que lo
registra como servicio. Si algo falla ahí, es de Aruna y no de la app.

> El sistema interno que se usó como primer banco de pruebas (una API real
> contra SQL Server) se movió a su propio repo privado el 2026-09-22: en un
> repo público no va nada del cliente — ni IPs, ni dominios, ni reglas de red.

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

## ✅ Validación en vivo — 2026-09-18, noche

Las cuatro pruebas que faltaban, hechas sobre `app-demo` instalada con el
instalador propio en `C:\Program Files\App Demo`:

| Prueba | Resultado |
|---|---|
| Matar el proceso hijo | Revivió en 1 s, con PID nuevo |
| **Reiniciar Windows** | Arrancó solo: `00:07:34 arrancando servicio "app-demo"` |
| Corre como servicio real | La API responde `"usuario": "SYSTEM"` |
| Apagado limpio | `stop solicitado por el SCM` antes del reinicio |
| Event Log | Eventos `node-winsvc` 1000 y 2000 en el Visor, Aplicación |
| `status` con uptime | `{"state":"running","pid":5136,"uptime":"12m"}` |

**El hallazgo más valioso fue un accidente.** Al matar el hijo, el puerto 4100
no se había liberado todavía y el relanzamiento falló:

```
00:06:06 [err] EADDRINUSE  port 4100
00:06:07 el hijo termino con codigo exit code: 1
00:06:07 relanzando el hijo en 1s
00:06:08 hijo lanzado (pid 37072)
```

El supervisor lo resolvió solo, sin intervención. Es resiliencia demostrada en
una condición real, no simulada.

> Ojo al leer los PID: `status` devuelve el del **supervisor** (el proceso que
> Windows controla), mientras que la API reporta el del **hijo** de Node. Es
> correcto que no coincidan.

---

## ⏳ Pendiente

### 1. 🔴 Probarlo en un segundo servidor
Es lo único que de verdad falta. Todo lo anterior está probado en la máquina
donde se construyó, que es justo donde siempre funciona todo. Con el paquete ya
publicado es un par de comandos:

```powershell
npm i -D aruna-winsvc
npx aruna doctor
```

O directamente copiar un `.exe` generado con `aruna installer` y darle doble
clic. `app-demo/` sirve de banco de pruebas: no toca base de datos, así que un
fallo allí es de Aruna y no de la app.

### 2. Detalles menores detectados
- El `FileDescription` del núcleo dice "Instalador de servicios de Windows para
  Node.js", y en el Administrador de tareas eso confunde: ese proceso es el
  **supervisor**, no el instalador. Debería decir "Aruna — supervisor".
- Los binarios internos siguen llamándose `node-winsvc-core.exe` y
  `node-winsvc-setup.exe`, de antes del cambio de marca. Renombrarlos toca el
  empaquetador y el instalador; funciona igual, pero queda inconsistente.

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
- **API de prueba:** `D:\2026\node-winsvc-servicio-node\app-demo\`
- **Sistema interno (repo privado aparte):** `D:\2026\api-logistica\`
- **Proyecto hermano (orquestador multi-MCP):** `D:\2026\RUST\HENRY MORENO-DEV\sentinel-hub\`
