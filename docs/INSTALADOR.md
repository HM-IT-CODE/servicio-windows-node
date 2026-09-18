# El comando `installer`

Genera un **instalador de Windows** (`.exe`) para tu servicio, a partir del
mismo `winsvc.config.json` que ya usas. Un asistente clásico —Siguiente,
Siguiente, Finalizar— que pide los datos, los comprueba y registra el servicio.

```bash
npx node-winsvc installer               # asistente propio (por omisión)
npx node-winsvc installer --inno        # con Inno Setup, si lo prefieres
npx node-winsvc installer --inno --no-compile   # solo escribe el .iss
```

Hay **dos generadores**. El propio no necesita nada instalado; el de Inno se
mantiene como salida de emergencia, porque lleva 25 años resolviendo casos
límite que este aún no ha pisado.

---

## El asistente propio

Un `.exe` de ~28 MB con todo dentro: la aplicación, sus dependencias de npm,
el núcleo del servicio y —si lo pides— hasta `node.exe`. El servidor de
destino no necesita **nada** instalado.

### Cómo está hecho

```
[ node-winsvc-setup.exe ]   el asistente genérico, ~1 MB, siempre el mismo
[ paquete               ]   TAR de la aplicación, comprimido de una pieza
[ manifiesto            ]   JSON: qué preguntar, qué servicio registrar
[ pie: 8 + 8 + 8 bytes  ]   longitudes + la firma "NWSVSETP"
```

El truco: **un `.exe` con datos pegados detrás sigue siendo un `.exe` válido**.
El cargador de Windows solo lee las cabeceras del principio y la cola le da
igual. El stub se abre a sí mismo, lee los últimos 24 bytes y de ahí deduce
dónde empieza todo. Así funcionan NSIS, Inno y los autoextraíbles de WinRAR.

La interfaz es **Win32 puro**: sin Electron, sin WebView2, sin .NET. Los
botones y el panel lateral se pintan a mano (`BS_OWNERDRAW` + `WM_DRAWITEM`),
porque Win32 solo da botones grises. Son ~1 MB de stub y arranca en cualquier
Windows sin instalar nada — la misma razón por la que el núcleo se enlaza con
el runtime de C estático.

### Por qué TAR y no ZIP

Un ZIP comprime **cada archivo por separado**, así que no aprovecha que miles
de archivos de `node_modules` se parecen entre sí. Un TAR es un solo flujo
continuo, y al comprimirlo entero el compresor ve todo el conjunto a la vez.
Es lo que Inno llama *compresión sólida*.

Medido sobre el mismo caso real (un `node.exe` de 80 MB + 7.900 archivos):

| Formato | Tamaño | Tiempo |
|---|---|---|
| ZIP + deflate | 45,4 MB | ~1 min |
| ZIP + zstd-19 | 37,2 MB | 5 min |
| Inno Setup (LZMA2 sólido) | 26,0 MB | ~4 min |
| **TAR + zstd-19 sólido** | **28,0 MB** | **1m46** |
| TAR + zstd-22 sólido | 27,5 MB | 1m56 |

La diferencia entre 45 y 28 MB **no fue el algoritmo, fue dejar de comprimir
archivo por archivo**. El nivel se ajusta con `installer.compressionLevel`
(1 a 22, por omisión 19; el 22 gana medio mega y pide más de 1 GB de RAM).

El precio de la compresión sólida: para sacar un archivo hay que descomprimir
todo lo anterior. Da igual aquí, porque un instalador siempre extrae el
paquete entero.

---

## Por qué existe

Las herramientas de esta categoría te dejan a mitad de camino: registran el
servicio en **tu** máquina, donde ya está el código, ya corriste `npm install` y
ya tienes el `.env`. En el servidor de destino no tienes nada de eso.

Lo que falta es lo aburrido: copiar archivos, instalar dependencias sin
internet, pedir la contraseña de la base, comprobar que conecta, y recién
entonces registrar el servicio. Este comando lo empaqueta todo en un `.exe` que
alguien que no es programador puede ejecutar.

---

## Configuración

Todo vive en la clave `installer` de `winsvc.config.json`:

```json
{
  "name": "api-logistica",
  "displayName": "API Logistica - Garantias",
  "script": "src/server.js",
  "nodeArgs": ["--expose-gc", "--max-old-space-size=512"],
  "env": { "NODE_ENV": "production" },

  "installer": {
    "appName": "API Logistica",
    "version": "1.0.0",
    "publisher": "Henry Moreno",
    "include": ["src", "scripts", "node_modules", "package.json"],
    "verifyScript": "scripts/probar-conexion.js",
    "prompts": [
      { "key": "DB_HOST",     "label": "Servidor", "default": "localhost", "group": "Base de datos" },
      { "key": "DB_PASSWORD", "label": "Clave",    "secret": true,          "group": "Base de datos" },
      { "key": "PORT",        "label": "Puerto",   "default": "3080",       "group": "Servicio" }
    ]
  }
}
```

| Campo | Para qué |
|---|---|
| `appName` | Nombre en el asistente y en *Agregar o quitar programas*. |
| `version`, `publisher` | Datos del producto. |
| `appId` | GUID único. Si lo omites, se deriva del nombre del servicio (estable entre compilaciones). |
| `defaultDirName` | Carpeta propuesta bajo Archivos de programa. |
| `outputDir` | Dónde dejar el `.iss` y el `.exe` (por defecto `instalador/`). |
| `include` | Qué copiar. Incluye `node_modules` si el servidor no tiene internet. |
| `prompts` | Los campos que pregunta el asistente. Se escriben al `.env`. |
| `verifyScript` | Script Node que valida la configuración. Si falla, **no registra el servicio**. |
| `language` | `es` (por defecto) o `en`. |
| `bundleNode` | Mete `node.exe` dentro del instalador. Ver abajo. |

### `prompts`

Cada campo va al `.env` con el nombre de `key`. `group` los reparte en páginas
del asistente; `secret: true` lo muestra con asteriscos.

Las variables de `env` del config también se escriben al `.env`, así que lo fijo
va ahí y lo que cambia por servidor va en `prompts`.

### `verifyScript`

Es la pieza que hace esto distinto. Un script Node que se ejecuta **después** de
escribir el `.env` y **antes** de registrar el servicio, con el directorio de
trabajo en la carpeta de instalación. Si sale con código distinto de cero, el
asistente se detiene y avisa.

```js
// scripts/probar-conexion.js — sale 0 si todo está bien, != 0 si no
const sql = require('mssql')
require('dotenv').config()

sql.connect({ /* ... leído del .env ... */ })
  .then(() => { console.log('OK'); process.exit(0) })
  .catch((e) => { console.error(e.message); process.exit(1) })
```

Es el mismo principio que el subcomando `ping` del núcleo: un `.env` mal puesto
con el servicio **ya registrado** produce un bucle de arranque y caída que desde
fuera parece un fallo del programa. Descubierto durante la instalación es un
mensaje de error; descubierto después, es una hora de diagnóstico.

### `bundleNode`

```json
"bundleNode": true
```

Copia `node.exe` a `vendor/node.exe` y lo mete en el instalador. El servicio lo
prefiere sobre el Node del sistema (`node_finder.rs`), así que **el servidor de
destino no necesita tener Node instalado**: doble clic sobre un Windows recién
formateado y la API queda corriendo.

Dos cosas que se ganan:

- **Cero requisitos en el servidor.** El asistente deja incluso de comprobar que
  Node exista, porque lo trae.
- **La versión de Node queda fijada.** Una actualización del Node del sistema no
  puede romper el servicio de un día para otro.

Lo que cuesta: el `.exe` pasa de ~6 MB a ~26 MB. El `node.exe` son 80 MB, pero
comprime bien con lzma2.

Con una cadena en vez de `true` se fija otro binario:
`"bundleNode": "vendor/node-v20.11.0.exe"`.

Node es MIT, así que redistribuirlo está permitido; conviene incluir su licencia
junto al producto.

---

## Qué hace el asistente generado

1. Comprueba que **Node esté instalado**; si no, se niega a seguir.
2. Pide la carpeta de destino.
3. Muestra una página por cada `group` de `prompts`.
4. Copia los archivos de `include` y el núcleo `node-winsvc-core.exe`.
5. Escribe el `.env` con `env` + lo que pusiste en el asistente.
6. Ejecuta `verifyScript`. **Si falla, se detiene aquí.**
7. Para y desregistra cualquier versión anterior del servicio, registra la nueva
   y la arranca.

Al desinstalar, para y quita el servicio **antes** de borrar los archivos: si no,
Windows deja el `.exe` bloqueado y la desinstalación queda a medias.

---

## Requisitos

**El asistente propio no necesita nada.** El de Inno necesita
[Inno Setup 6](https://jrsoftware.org/isdl.php) (gratis); si no está, el
comando no falla: escribe el `.iss` y te dice cómo compilarlo en otra máquina.

En ambos casos, quien recibe el `.exe` no necesita nada: ni npm, ni este
paquete, ni Inno Setup. Y con `bundleNode`, ni siquiera Node.

---

## Cuatro detalles que costaron encontrar

**El tamaño de una ventana Win32 incluye bordes y barra de título.** Pasarle
directamente las medidas que quieres deja el área útil ~40 px más corta, y los
botones acaban encima del último campo. Se corrige con `AdjustWindowRect`.

**Un proceso hijo abre su propia consola** si el padre no tiene una. El
instalador es `windows_subsystem = "windows"`, así que cada `node.exe` o
`node-winsvc-core.exe` que lanzaba hacía parpadear una ventana negra. Se apaga
con `CREATE_NO_WINDOW` (`0x0800_0000`) en `creation_flags`.

**Un icono de 256×256 en BMP crudo ocupa 256 KB** y duplicaba el tamaño del
stub. Dentro de un `.ico` se puede guardar como PNG: 285 KB → 17 KB.

**`OutputDir` de Inno es relativo al `.iss`, no al proyecto** — poner ahí la
ruta del config produce `instalador\instalador\` — y **`spawnSync` tiene un
búfer de 1 MB**, que ISCC desborda imprimiendo una línea por archivo: entonces
Node **mata el proceso** y devuelve código `null` aunque la compilación fuera
bien. El servicio usa `maxBuffer: 64 MB`.
