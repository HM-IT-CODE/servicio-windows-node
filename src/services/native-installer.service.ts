import fs   from 'fs'
import os   from 'os'
import path from 'path'
import { spawnSync } from 'child_process'
import { WinsvcConfig, InstallerConfig } from '../models/winsvc-config.model'
import { WinsvcError } from '../utils/errors'

/**
 * Builds the self-contained installer: our own wizard, no Inno Setup needed.
 *
 * The heavy lifting (zipping, appending to the stub) happens in the Rust core,
 * so this package keeps zero npm dependencies. Here we only translate the
 * config into a manifest the stub can read at runtime.
 */
export class NativeInstallerService {

  private readonly STUB = 'node-winsvc-setup.exe'

  /** The generic wizard, shipped next to the core inside the npm package. */
  resolveStub(corePath: string): string {
    const stub = path.join(path.dirname(corePath), this.STUB)

    if (!fs.existsSync(stub)) {
      throw new WinsvcError(
        `Installer stub not found: ${stub}\n` +
        `Build it with:  cd rust && cargo build --release`,
        'STUB_NOT_FOUND',
      )
    }
    return stub
  }

  /**
   * Translates winsvc.config.json into the manifest the stub reads.
   * Everything the wizard needs to know at runtime lives here.
   */
  buildManifest(config: WinsvcConfig, installer: InstallerConfig, bundledNode: boolean): object {
    return {
      app_name:  installer.appName,
      version:   installer.version,
      publisher: installer.publisher,
      dir_name:  installer.defaultDirName ?? installer.appName,

      service_name:    config.name,
      service_display: config.displayName,
      service_desc:    config.description,
      script:          config.script,
      node_args:       config.nodeArgs.join(' '),
      log_file:        config.logFile,
      start_type:      config.startType,
      auto_restart:    config.autoRestart,

      campos: (installer.prompts ?? []).map(p => ({
        key:     p.key,
        label:   p.label,
        default: p.default ?? '',
        secret:  p.secret ?? false,
        group:   p.group ?? 'Configuracion',
      })),
      // El manifiesto usa pares, no objeto: conserva el orden de escritura
      // en el .env, que es lo que un humano espera al abrirlo.
      env: Object.entries(config.env),
      verify_script: installer.verifyScript ?? '',
      bundled_node:  bundledNode,
    }
  }

  /** Runs the Rust packer and returns the path to the finished installer. */
  build(
    config: WinsvcConfig,
    installer: InstallerConfig,
    projectRoot: string,
    corePath: string,
    bundledNode: string | null,
  ): string {
    const stub     = this.resolveStub(corePath)
    const manifest = this.buildManifest(config, installer, bundledNode !== null)

    // El manifiesto es temporal: solo viaja del CLI al empaquetador.
    const manifestPath = path.join(os.tmpdir(), `winsvc-${config.name}-${process.pid}.json`)
    fs.writeFileSync(manifestPath, JSON.stringify(manifest), 'utf-8')

    const outDir = path.resolve(projectRoot, installer.outputDir ?? 'instalador')
    fs.mkdirSync(outDir, { recursive: true })
    const salida = path.join(outDir, `instalar-${config.name}.exe`)

    const args = [
      'empaquetar',
      '--stub',     stub,
      '--origen',   projectRoot,
      '--incluir',  (installer.include ?? ['src', 'package.json']).join(','),
      '--core',     corePath,
      '--manifest', manifestPath,
      '--salida',   salida,
      '--nivel',    String(installer.compressionLevel ?? 19),
    ]
    if (bundledNode) {
      args.push('--node', path.resolve(projectRoot, bundledNode))
    }

    try {
      const result = spawnSync(corePath, args, {
        encoding:  'utf-8',
        shell:     false,
        maxBuffer: 64 * 1024 * 1024,
      })

      if (result.status !== 0) {
        const detail = (result.stderr || result.stdout || '').trim()
        throw new WinsvcError(detail || 'Packing failed', 'PACK_ERROR')
      }
    } finally {
      // Lleva la contraseña por defecto si el config la trae: no se queda ahí.
      try { fs.unlinkSync(manifestPath) } catch { /* ya no está */ }
    }

    return salida
  }
}
