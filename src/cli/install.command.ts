import path from 'path'
import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

export function runInstall(projectRoot: string): void {
  logger.title('node-winsvc — Install')

  logger.step('Reading winsvc.config.json...')
  const config = configService.load(projectRoot)

  logger.step(`Locating node-winsvc-core.exe...`)
  const binaryPath = binaryService.resolvePath()

  const scriptPath  = configService.resolveScriptPath(config, projectRoot)
  const workingDir  = configService.resolveWorkingDir(config, projectRoot)
  const envJson     = JSON.stringify(config.env)
  const nodeArgs    = config.nodeArgs.join(' ')

  logger.step(`Registering service "${config.displayName}"...`)

  processService.runOrThrow({
    binaryPath,
    args: [
      'install',
      '--name',        config.name,
      '--display',     config.displayName,
      '--description', config.description,
      '--script',      scriptPath,
      '--node-args',   nodeArgs,
      '--env',         envJson,
      '--working-dir', workingDir,
      '--log-file',    path.resolve(projectRoot, config.logFile),
      '--start-type',  config.startType,
      '--auto-restart', config.autoRestart.toString(),
    ],
  })

  logger.success(`Service "${config.name}" installed successfully.`)
  logger.info(`Run: node-winsvc start`)
}
