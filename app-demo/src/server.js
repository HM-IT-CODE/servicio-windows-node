'use strict'

/**
 * App de demostracion: un servidor minimo, SIN dependencias.
 *
 * Existe para ver de punta a punta como Aruna convierte una app de Node en un
 * servicio de Windows. No usa base de datos ni librerias a proposito: si algo
 * falla, el fallo es de Aruna y no de la app.
 */

const http = require('http')
const os   = require('os')

const ARRANQUE = Date.now()
let peticiones = 0

const servidor = http.createServer((req, res) => {
  peticiones += 1

  res.setHeader('Content-Type', 'application/json; charset=utf-8')
  res.end(JSON.stringify({
    ok: true,
    mensaje: 'La app corre como servicio de Windows',
    ruta: req.url,
    pid: process.pid,
    usuario: os.userInfo().username,     // dira SYSTEM cuando sea servicio
    directorio: process.cwd(),
    uptimeSegundos: Math.round((Date.now() - ARRANQUE) / 1000),
    peticiones,
  }, null, 2))
})

const PUERTO = Number(process.env.PORT) || 4100

servidor.listen(PUERTO, () => {
  // Esta linea la captura Aruna y la escribe en el log mensual
  console.log(`app-demo escuchando en http://localhost:${PUERTO} (pid ${process.pid})`)
  console.log(`usuario: ${os.userInfo().username} · directorio: ${process.cwd()}`)
})

// El supervisor mata al hijo cuando el SCM pide parar: hay que soltar el puerto.
process.on('SIGTERM', () => {
  console.log('SIGTERM recibido, cerrando')
  servidor.close(() => process.exit(0))
})

// Para probar que el servicio revive: GET /matar tumba el proceso.
process.on('SIGINT', () => process.exit(0))
