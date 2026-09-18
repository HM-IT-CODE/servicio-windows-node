import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

export function runStart(projectRoot: string): void {
  logger.title('Aruna — Start')

  const config     = configService.load(projectRoot)
  const binaryPath = binaryService.resolvePath()

  logger.step(`Starting service "${config.name}"...`)
  processService.runOrThrow({ binaryPath, args: ['start', '--name', config.name] })

  logger.success(`Service "${config.name}" is running.`)
  logger.info(`Check status: aruna status`)
}
