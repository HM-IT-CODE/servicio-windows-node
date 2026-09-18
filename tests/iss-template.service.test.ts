import { IssTemplateService } from '../src/services/iss-template.service'
import { WinsvcConfig, InstallerConfig } from '../src/models/winsvc-config.model'

describe('IssTemplateService', () => {
  const service = new IssTemplateService()

  const config: WinsvcConfig = {
    name: 'api-demo',
    displayName: 'API Demo',
    description: 'Servicio de prueba',
    script: 'src/server.js',
    nodeArgs: ['--expose-gc'],
    env: { NODE_ENV: 'production' },
    autoRestart: true,
    startType: 'auto',
    logFile: 'logs/service.log',
  }

  const installer: InstallerConfig = {
    appName: 'API Demo',
    version: '1.0.0',
    publisher: 'Pruebas',
  }

  const core = 'C:\\bin\\node-winsvc-core.exe'

  it('genera las secciones obligatorias de Inno', () => {
    const iss = service.build(config, installer, core)

    expect(iss).toContain('[Setup]')
    expect(iss).toContain('[Files]')
    expect(iss).toContain('[Code]')
    expect(iss).toContain('AppName=API Demo')
  })

  it('exige elevacion, porque registrar un servicio la necesita', () => {
    expect(service.build(config, installer, core)).toContain('PrivilegesRequired=admin')
  })

  it('deriva un AppId estable del nombre del servicio', () => {
    const uno = service.build(config, installer, core)
    const dos = service.build(config, installer, core)

    const extraer = (iss: string): string => /AppId=\{\{([^}]+)\}/.exec(iss)![1]

    expect(extraer(uno)).toBe(extraer(dos))
  })

  it('da AppIds distintos a servicios distintos', () => {
    const otro = { ...config, name: 'api-otra' }
    const extraer = (iss: string): string => /AppId=\{\{([^}]+)\}/.exec(iss)![1]

    expect(extraer(service.build(config, installer, core)))
      .not.toBe(extraer(service.build(otro, installer, core)))
  })

  it('respeta un AppId puesto a mano', () => {
    const iss = service.build(config, { ...installer, appId: 'MI-GUID-FIJO' }, core)

    expect(iss).toContain('AppId={{MI-GUID-FIJO}')
  })

  it('escribe OutputDir=. para no anidar carpetas', () => {
    // Inno resuelve OutputDir contra el .iss, no contra el proyecto.
    expect(service.build(config, installer, core)).toContain('OutputDir=.')
  })

  it('crea una pagina del asistente por cada grupo de campos', () => {
    const conPrompts: InstallerConfig = {
      ...installer,
      prompts: [
        { key: 'DB_HOST', label: 'Servidor', default: 'localhost', group: 'Base' },
        { key: 'DB_PASSWORD', label: 'Clave', secret: true, group: 'Base' },
        { key: 'PORT', label: 'Puerto', default: '3080', group: 'Servicio' },
      ],
    }

    const iss = service.build(config, conPrompts, core)

    expect(iss).toContain('Pagina0: TInputQueryWizardPage')
    expect(iss).toContain('Pagina1: TInputQueryWizardPage')
    expect(iss).not.toContain('Pagina2:')
  })

  it('marca como contrasena solo los campos secretos', () => {
    const conPrompts: InstallerConfig = {
      ...installer,
      prompts: [
        { key: 'DB_USER', label: 'Usuario', group: 'Base' },
        { key: 'DB_PASSWORD', label: 'Clave', secret: true, group: 'Base' },
      ],
    }

    const iss = service.build(config, conPrompts, core)

    expect(iss).toContain("Pagina0.Add('Usuario:', False)")
    expect(iss).toContain("Pagina0.Add('Clave:', True)")
  })

  it('vuelca al .env tanto env como los campos preguntados', () => {
    const conPrompts: InstallerConfig = {
      ...installer,
      prompts: [{ key: 'DB_HOST', label: 'Servidor', default: 'localhost', group: 'Base' }],
    }

    const iss = service.build(config, conPrompts, core)

    expect(iss).toContain("'NODE_ENV=production'")
    expect(iss).toContain("'DB_HOST=' + Trim(Pagina0.Values[0])")
  })

  it('sin verifyScript la comprobacion pasa siempre', () => {
    const iss = service.build(config, installer, core)

    expect(iss).toContain('function Verificar(): Boolean;')
    expect(iss).toContain('Result := True;')
  })

  it('con verifyScript ejecuta el script y exige codigo 0', () => {
    const iss = service.build(config, { ...installer, verifyScript: 'scripts/probar.js' }, core)

    expect(iss).toContain('scripts\\probar.js')
    expect(iss).toContain('(Codigo = 0)')
  })

  it('para el servicio antes de desregistrarlo', () => {
    // DeleteService solo lo MARCA: sin el stop, el proceso sigue reteniendo el puerto.
    const iss = service.build(config, installer, core)
    const stop = iss.indexOf('stop --name api-demo')
    const uninstall = iss.indexOf('uninstall --name api-demo')

    expect(stop).toBeGreaterThan(-1)
    expect(stop).toBeLessThan(uninstall)
  })

  it('pasa los nodeArgs al registro del servicio', () => {
    expect(service.build(config, installer, core)).toContain('--node-args "--expose-gc"')
  })
})
