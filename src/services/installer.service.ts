import fs   from 'fs'
import path from 'path'
import { spawnSync } from 'child_process'
import { WinsvcConfig, InstallerConfig } from '../models/winsvc-config.model'
import { IssTemplateService } from './iss-template.service'
import { WinsvcError } from '../utils/errors'

/** Where Inno Setup usually lands. Checked in order. */
const ISCC_PATHS = [
  'C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe',
  'C:\\Program Files\\Inno Setup 6\\ISCC.exe',
  'C:\\Program Files (x86)\\Inno Setup 5\\ISCC.exe',
]

export interface BuildResult {
  issPath:  string
  exePath?: string
  compiled: boolean
}

export class InstallerService {

  private template = new IssTemplateService()

  /** Finds the Inno Setup compiler, or null if it is not installed. */
  findCompiler(): string | null {
    const fromPath = spawnSync('where', ['ISCC'], { encoding: 'utf-8', shell: false })
    if (fromPath.status === 0) {
      const first = (fromPath.stdout ?? '').split(/\r?\n/)[0].trim()
      if (first && fs.existsSync(first)) return first
    }
    return ISCC_PATHS.find(p => fs.existsSync(p)) ?? null
  }

  /** Writes the .iss next to the output directory, then compiles it if it can. */
  build(
    config: WinsvcConfig,
    installer: InstallerConfig,
    projectRoot: string,
    corePath: string,
    compile: boolean,
  ): BuildResult {
    const bundled = this.prepareNode(installer, projectRoot)
    const script  = this.template.build(config, installer, corePath, bundled)
    const issDir  = path.resolve(projectRoot, installer.outputDir ?? 'instalador')
    const issPath = path.join(issDir, `${config.name}.iss`)

    fs.mkdirSync(issDir, { recursive: true })
    // El .iss resuelve las rutas de [Files] contra el proyecto, no contra sí
    // mismo: por eso se inyecta el origen en vez de usar rutas relativas.
    fs.writeFileSync(issPath, `#define Origen "${projectRoot}"\n\n${script}`, 'utf-8')

    if (!compile) return { issPath, compiled: false }

    const compiler = this.findCompiler()
    if (!compiler) {
      return { issPath, compiled: false }
    }

    // maxBuffer alto: ISCC imprime una línea por archivo, y con node_modules
    // eso desborda el megabyte por omisión de spawnSync, que entonces MATA el
    // proceso y deja status en null aunque la compilación fuera bien.
    const result = spawnSync(compiler, [issPath], {
      encoding:  'utf-8',
      shell:     false,
      maxBuffer: 64 * 1024 * 1024,
    })

    if (result.status !== 0) {
      const detail = (result.stdout ?? '').split(/\r?\n/).filter(Boolean).slice(-8).join('\n')
      throw new WinsvcError(`Inno Setup failed:\n${detail}`, 'ISCC_ERROR')
    }

    // El .iss lleva OutputDir=. así que el exe sale junto al script.
    const exePath = path.join(issDir, `instalar-${config.name}.exe`)

    return { issPath, exePath: fs.existsSync(exePath) ? exePath : undefined, compiled: true }
  }

  /**
   * Copies node.exe into `vendor/` so the target server needs nothing
   * installed. Returns the path to include, or null when not requested.
   *
   * The whole Node installation is not copied — just node.exe, which is
   * self-contained. npm and the global modules are not needed to run a service.
   */
  prepareNode(installer: InstallerConfig, projectRoot: string): string | null {
    if (!installer.bundleNode) return null

    const origen = typeof installer.bundleNode === 'string'
      ? path.resolve(projectRoot, installer.bundleNode)
      : process.execPath

    if (!fs.existsSync(origen)) {
      throw new WinsvcError(`node.exe not found for bundling: ${origen}`, 'NODE_NOT_FOUND')
    }

    const destino = path.join(projectRoot, 'vendor', 'node.exe')
    fs.mkdirSync(path.dirname(destino), { recursive: true })

    // Copiar sólo si cambió: son ~80 MB y esto corre en cada build.
    const yaEsta = fs.existsSync(destino)
      && fs.statSync(destino).size === fs.statSync(origen).size

    if (!yaEsta) fs.copyFileSync(origen, destino)

    return 'vendor/node.exe'
  }

  /** Sensible defaults so `installer` works with no extra configuration. */
  defaults(config: WinsvcConfig): InstallerConfig {
    return {
      appName:   config.displayName,
      version:   '1.0.0',
      publisher: 'Aruna',
      outputDir: 'instalador',
      include:   ['src', 'package.json', 'node_modules'],
      language:  'es',
    }
  }
}
