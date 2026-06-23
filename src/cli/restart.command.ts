import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

export function runRestart(projectRoot: string): void {
  logger.title('node-winsvc — Restart')

  const config     = configService.load(projectRoot)
  const binaryPath = binaryService.resolvePath()

  logger.step(`Stopping service "${config.name}"...`)
  // Ignore failure here: the service may already be stopped.
  processService.run({ binaryPath, args: ['stop', '--name', config.name] })

  logger.step(`Starting service "${config.name}"...`)
  processService.runOrThrow({ binaryPath, args: ['start', '--name', config.name] })

  logger.success(`Service "${config.name}" restarted.`)
}
