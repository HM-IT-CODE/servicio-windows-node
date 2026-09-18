import fs   from 'fs'
import path from 'path'
import { ConfigService }  from '../services/config.service'
import { BinaryService }  from '../services/binary.service'
import { ProcessService } from '../services/process.service'
import { logger }         from '../utils/logger'

const configService  = new ConfigService()
const binaryService  = new BinaryService()
const processService = new ProcessService()

/**
 * Diagnoses the setup BEFORE installing: config, binary, Node, script path.
 * Exits non-zero if anything fails, so `instalar-servicio.cmd` can abort
 * instead of registering a service that will crash-loop.
 */
export function runDoctor(projectRoot: string): void {
  logger.title('Aruna — Doctor')

  const config = configService.load(projectRoot)
  logger.success(`Config: winsvc.config.json → "${config.name}"`)

  const binaryPath = binaryService.resolvePath()
  logger.success(`Core:   ${binaryPath}`)

  const scriptPath = configService.resolveScriptPath(config, projectRoot)
  if (!fs.existsSync(scriptPath)) {
    logger.error(`Script not found: ${scriptPath}`)
    process.exit(1)
  }

  const workingDir = configService.resolveWorkingDir(config, projectRoot)
  if (!fs.existsSync(workingDir)) {
    logger.error(`Working directory not found: ${workingDir}`)
    process.exit(1)
  }

  const result = processService.run({
    binaryPath,
    args: ['ping', '--script', scriptPath],
  })

  console.log(result.stdout.trim())

  if (!result.success) {
    logger.error(result.stderr.trim() || 'Core checks failed.')
    process.exit(1)
  }

  logger.success(`Logs:   ${path.resolve(projectRoot, path.dirname(config.logFile))}`)
  logger.success('Ready to install.')
}
