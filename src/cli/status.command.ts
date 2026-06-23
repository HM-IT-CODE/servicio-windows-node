import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

export function runStatus(projectRoot: string): void {
  logger.title('node-winsvc — Status')

  const config     = configService.load(projectRoot)
  const binaryPath = binaryService.resolvePath()

  const output = processService.runOrThrow({
    binaryPath,
    args: ['status', '--name', config.name, '--json'],
  })

  const status = JSON.parse(output) as { state: string; pid?: number; uptime?: string }

  const stateColor = status.state === 'Running' ? '🟢' : '🔴'
  logger.info(`${stateColor} ${config.displayName}`)
  logger.info(`   State:  ${status.state}`)
  if (status.pid)    logger.info(`   PID:    ${status.pid}`)
  if (status.uptime) logger.info(`   Uptime: ${status.uptime}`)
}
