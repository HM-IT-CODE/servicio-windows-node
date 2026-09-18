# Comparativa: qué producto tenemos

> Evaluación honesta de `node-winsvc` frente a lo que ya existe.
> Fecha: **2026-09-18**. Los datos marcados con ✱ se midieron en esta máquina;
> el resto viene de npm y de la documentación de cada proyecto.

---

## 1. Quién compite

| Herramienta | Qué es | Estado |
|---|---|---|
| **node-windows** | Paquete npm. El líder de esta categoría. | `1.0.0-beta.8`, publicado hace **4 años**. 26.224 descargas/semana, 172 paquetes dependientes. |
| **NSSM** | `.exe` externo que envuelve cualquier programa. | Último release **2017**. |
| **WinSW** | Envoltorio en .NET, el que usa Jenkins. | Activo. Configuración en XML/YAML. |
| **pm2 + pm2-windows-service** | Envuelve pm2, que a su vez envuelve tu app. | Activo. Capas sobre capas. |
| **node-winsvc** | Este proyecto. Núcleo Rust + CLI TypeScript. | En desarrollo. |

El dato que más pesa: **el líder lleva cuatro años sin publicar y nunca salió
de beta**, mientras 26.000 proyectos por semana siguen descargándolo. La
demanda está medida; la oferta está congelada.

---

## 2. Lo que necesita el servidor de destino

Este es el eje que de verdad separa a unos de otros, porque es donde se pierden
las tardes.

| | node-windows | NSSM | WinSW | node-winsvc |
|---|---|---|---|---|
| Node.js instalado | ✔ requerido | ✔ requerido | ✔ requerido | **opcional** ✱ |
| Runtime adicional | **.NET Framework 2.0** ✱ | ninguno | .NET | **ninguno** ✱ |
| Copiar el código a mano | ✔ | ✔ | ✔ | ✖ |
| `npm install` en el servidor | ✔ | ✔ | ✔ | ✖ |
| Crear el `.env` a mano | ✔ | ✔ | ✔ | ✖ |

**El detalle de .NET importa más de lo que parece.** `node-windows` embebe
`winsw.exe` (58 KB ✱), que declara `v2.0.50727` y enlaza `mscoree.dll` ✱ — o
sea, exige .NET Framework 2.0, de 2005. En Windows Server 2019/2022/2025 eso es
una **característica opcional desactivada por defecto**: `npm i node-windows`
instala bien y el servicio falla al arrancar hasta que alguien habilite .NET 3.5
desde el panel de características.

Es exactamente el problema del `VCRUNTIME140.dll` que este proyecto evita
enlazando el runtime de C estáticamente, comprobado en cada compilación ✱.

---

## 3. Capacidades

| | node-windows | NSSM | WinSW | node-winsvc |
|---|---|---|---|---|
| Registro nativo en el SCM | ✔ | ✔ | ✔ | ✔ |
| Reinicio ante caída | ✔ | ✔ | ✔ | ✔ |
| Acciones de fallo del SCM | ✔ | ✔ | ✔ | ✔ |
| Event Log de Windows | ✔ | ✖ | ✔ | ✔ |
| Rotación de logs | ✖ | ✔ | ✔ | ✔ mensual, con BOM |
| PID y uptime reales | ✖ | ✔ | ✔ | ✔ |
| Configuración declarativa | ✖ script JS | ✖ flags | ✔ XML | ✔ **JSON versionado** |
| Verificación previa | ✖ | ✖ | ✖ | ✔ `doctor` / `ping` |
| **Genera su instalador** | ✖ | ✖ | ✖ | ✔ |
| Dependencias npm | 18 | — | — | **0** ✱ |
| Multiplataforma | node-mac / node-linux | ✖ | ✖ | ✖ solo Windows |

---

## 4. Las dos cosas que no hace nadie más

### El instalador generado

```bash
npx node-winsvc installer   →   instalar-tu-api.exe   (28 MB) ✱
```

Un `.exe` que copia el código, trae `node_modules` sin necesitar internet, pide
la contraseña de la base, **comprueba que conecta**, y solo entonces registra el
servicio. Con `bundleNode`, mete también `node.exe`: el servidor no necesita
nada instalado.

Las otras herramientas te dejan a mitad de camino: registran el servicio en
**tu** máquina, donde ya tienes todo. El trabajo real está en el servidor de
destino, y eso nadie lo empaqueta.

### Comprobar antes de registrar

`verifyScript` corre **después** de escribir el `.env` y **antes** de registrar
el servicio. Si falla, no queda nada registrado.

Un `.env` mal puesto con el servicio ya instalado produce un bucle de arranque
y caída que desde fuera parece un fallo del programa. Descubierto durante la
instalación es un mensaje de error; descubierto después, es una hora de
diagnóstico. NSSM y node-windows registran y que haya suerte.

---

## 5. Compresión del instalador ✱

Medido sobre el caso real de `api-logistica`: `node.exe` de 80 MB + 7.900
archivos de `node_modules`.

| Formato | Tamaño | Tiempo |
|---|---|---|
| ZIP + deflate | 45,4 MB | ~1 min |
| ZIP + zstd-19 | 37,2 MB | 5 min |
| Inno Setup (LZMA2 sólido) | 26,0 MB | ~4 min |
| **TAR + zstd-19 sólido** | **28,0 MB** | **1m46** |
| TAR + zstd-22 sólido | 27,5 MB | 1m56 |

Lo que bajó el tamaño de 45 a 28 MB **no fue el algoritmo, fue dejar de
comprimir archivo por archivo**. Inno sigue 2 MB por delante porque LZMA2
comprime algo mejor que zstd, pero tarda el doble.

---

## 6. Lo que nos falta

Sin adornos:

- **Rodaje.** Funciona en una máquina. Cuentas de dominio, dependencias entre
  servicios, servidores con el PATH raro, rutas UNC: nada de eso está probado.
  Sale usándolo, no anticipándolo.
- **Solo Windows.** `node-windows` tiene hermanos para Mac y Linux con la misma
  API. Aquí no, y el diseño (Win32 crudo) hace que nunca los tenga.
- **Tests de integración.** Hay 20 unitarios ✱ de la capa TypeScript; falta
  instalar/arrancar/parar/desinstalar un servicio de verdad en CI.
- **Limpieza de logs.** Rotan por mes pero nadie borra los viejos.
- **El Event Log no instala un archivo de mensajes**, así que el Visor antepone
  un aviso de "falta la descripción". El texto se lee igual.
- **Nombre y distribución.** Nadie lo conoce. `node-windows` tiene cuatro años
  de SEO y el nombre de Corey Butler (autor de *NVM for Windows*) detrás.

---

## 7. Veredicto

**Como herramienta interna: lista y se paga sola.** Cada API que se despliegue
de ahora en adelante cuesta un doble clic en vez de una tarde, y no depende de
un binario abandonado en 2017 ni de un runtime de 2005.

**Como paquete público: el hueco existe y está medido**, pero ocuparlo no es
automático. 26.000 descargas semanales no se mudan por ser mejor: se mudan
cuando alguien con audiencia lo descubre, o cuando el líder rompe algo. Y si
Corey Butler vuelve y publica el 1.0 estable, el hueco se cierra solo.

El argumento de venta, si algún día se publica, se escribe solo:

> Cero dependencias. Cero runtimes. Configuración versionada con tu proyecto.
> Comprueba antes de registrar. Y trae su propio instalador.

Lo que decidiría si merece la pena: **desplegarlo en un segundo servidor real**.
Si sobrevive sin tocar el código, está listo. Eso dirá más que todo lo anterior.
