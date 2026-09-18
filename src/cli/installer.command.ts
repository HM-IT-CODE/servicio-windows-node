import fs from 'fs'
import { ConfigService }    from '../services/config.service'
import { BinaryService }    from '../services/binary.service'
import { InstallerService } from '../services/installer.service'
import { NativeInstallerService } from '../services/native-installer.service'
import { logger }           from '../utils/logger'

const configService    = new ConfigService()
const binaryService    = new BinaryService()
const installerService = new InstallerService()
const nativeService    = new NativeInstallerService()

/**
 * Generates a Windows installer (.exe) for the service described by
 * winsvc.config.json.
 *
 * Flags:
 *   `--inno`        build with Inno Setup instead of the built-in wizard
 *   `--no-compile`  (Inno only) write the .iss script without compiling
 */
export function runInstaller(projectRoot: string): void {
  logger.title('Aruna — Installer')

  const config    = configService.load(projectRoot)
  const defaults  = installerService.defaults(config)
  const installer = { ...defaults, ...(config.installer ?? {}) }
  const corePath  = binaryService.resolvePath()

  if (!process.argv.includes('--inno')) {
    buildNative(config, installer, projectRoot, corePath)
    return
  }

  const compile = !process.argv.includes('--no-compile')

  logger.step(`Building installer for "${installer.appName}" (Inno Setup)...`)

  const result = installerService.build(config, installer, projectRoot, corePath, compile)

  logger.success(`Script: ${result.issPath}`)

  if (result.exePath) {
    logger.success(`Installer: ${result.exePath}`)
    logger.blank()
    logger.info('Copy that single .exe to the target server and double-click it.')
    return
  }

  if (!compile) {
    logger.info('Skipped compilation (--no-compile).')
    return
  }

  logger.warn('Inno Setup not found — the .iss script was written but not compiled.')
  logger.info('Install it from https://jrsoftware.org/isdl.php and run this again,')
  logger.info('or compile manually:  ISCC.exe "' + result.issPath + '"')
}

/**
 * The built-in wizard: our own stub with the app appended. Needs nothing
 * installed — not Inno Setup here, not Node on the target server.
 */
function buildNative(
  config: ReturnType<ConfigService['load']>,
  installer: ReturnType<InstallerService['defaults']>,
  projectRoot: string,
  corePath: string,
): void {
  logger.step(`Building installer for "${installer.appName}"...`)

  const bundled = installerService.prepareNode(installer, projectRoot)
  if (bundled) logger.step('Bundling node.exe — the target server will need nothing.')

  const exePath = nativeService.build(config, installer, projectRoot, corePath, bundled)
  const mb = (fs.statSync(exePath).size / 1024 / 1024).toFixed(1)

  logger.success(`Installer: ${exePath}  (${mb} MB)`)
  logger.blank()
  logger.info('Copy that single .exe to the target server and double-click it.')
}
