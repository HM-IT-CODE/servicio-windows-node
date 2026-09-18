import fs   from 'fs'
import os   from 'os'
import path from 'path'
import { ConfigService } from '../src/services/config.service'
import { ConfigNotFoundError } from '../src/utils/errors'

describe('ConfigService', () => {
  let raiz: string
  const service = new ConfigService()

  beforeEach(() => {
    raiz = fs.mkdtempSync(path.join(os.tmpdir(), 'winsvc-test-'))
  })

  afterEach(() => {
    fs.rmSync(raiz, { recursive: true, force: true })
  })

  const escribir = (config: object): void => {
    fs.writeFileSync(path.join(raiz, 'winsvc.config.json'), JSON.stringify(config), 'utf-8')
  }

  it('falla con un error claro si no hay config', () => {
    expect(() => service.load(raiz)).toThrow(ConfigNotFoundError)
  })

  it('rellena los valores por defecto que falten', () => {
    escribir({ name: 'api', displayName: 'API', script: 'src/server.js' })

    const config = service.load(raiz)

    expect(config.autoRestart).toBe(true)
    expect(config.startType).toBe('auto')
    expect(config.logFile).toBe('logs/service.log')
    expect(config.nodeArgs).toEqual([])
  })

  it('respeta lo que el usuario puso por encima de los defectos', () => {
    escribir({
      name: 'api', displayName: 'API', script: 'src/server.js',
      autoRestart: false, startType: 'manual', nodeArgs: ['--expose-gc'],
    })

    const config = service.load(raiz)

    expect(config.autoRestart).toBe(false)
    expect(config.startType).toBe('manual')
    expect(config.nodeArgs).toEqual(['--expose-gc'])
  })

  it('resuelve el script a ruta absoluta', () => {
    escribir({ name: 'api', displayName: 'API', script: 'src/server.js' })
    const config = service.load(raiz)

    const resuelto = service.resolveScriptPath(config, raiz)

    expect(path.isAbsolute(resuelto)).toBe(true)
    expect(resuelto).toBe(path.resolve(raiz, 'src/server.js'))
  })

  it('deja en paz un script que ya es absoluto', () => {
    const absoluto = path.join(raiz, 'otro', 'server.js')
    escribir({ name: 'api', displayName: 'API', script: absoluto })
    const config = service.load(raiz)

    expect(service.resolveScriptPath(config, raiz)).toBe(absoluto)
  })

  it('usa la raiz del proyecto como directorio de trabajo por omision', () => {
    escribir({ name: 'api', displayName: 'API', script: 'src/server.js' })
    const config = service.load(raiz)

    expect(service.resolveWorkingDir(config, raiz)).toBe(path.resolve(raiz))
  })

  it('crea una plantilla que se puede volver a leer', () => {
    const ruta = service.createTemplate(raiz)

    expect(fs.existsSync(ruta)).toBe(true)
    expect(() => service.load(raiz)).not.toThrow()
  })
})
