import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

export function runStop(projectRoot: string): void {
  logger.title('node-winsvc — Stop')

  const config     = configService.load(projectRoot)
  const binaryPath = binaryService.resolvePath()

  logger.step(`Stopping service "${config.name}"...`)
  processService.runOrThrow({ binaryPath, args: ['stop', '--name', config.name] })

  logger.success(`Service "${config.name}" stopped.`)
}
