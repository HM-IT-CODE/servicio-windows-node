import fs   from 'fs'
import path from 'path'
import { ConfigService } from '../services/config.service'
import { logger }        from '../utils/logger'

const configService = new ConfigService()

const DEFAULT_LINES = 50

/**
 * Prints the tail of the service log file.
 * Flags (read from argv): `-f` / `--follow` to stream, `-n <N>` for line count.
 */
export function runLogs(projectRoot: string): void {
  logger.title('node-winsvc — Logs')

  const config  = configService.load(projectRoot)
  const logPath = path.resolve(projectRoot, config.logFile)

  if (!fs.existsSync(logPath)) {
    logger.warn(`Log file not found yet: ${logPath}`)
    logger.info('It is created once the service runs and writes output.')
    return
  }

  const { follow, lines } = parseArgs()

  printTail(logPath, lines)

  if (follow) {
    logger.info(`Following ${logPath} (Ctrl+C to stop)...`)
    streamFrom(logPath)
  }
}

function parseArgs(): { follow: boolean; lines: number } {
  const argv   = process.argv.slice(3)
  const follow = argv.includes('-f') || argv.includes('--follow')

  const nIndex = argv.findIndex(a => a === '-n' || a === '--lines')
  const lines  = nIndex >= 0 ? Number(argv[nIndex + 1]) || DEFAULT_LINES : DEFAULT_LINES

  return { follow, lines }
}

function printTail(logPath: string, lines: number): void {
  const content = fs.readFileSync(logPath, 'utf-8')
  const all     = content.split(/\r?\n/).filter(Boolean)
  all.slice(-lines).forEach(line => console.log(line))
}

function streamFrom(logPath: string): void {
  let size = fs.statSync(logPath).size
  fs.watchFile(logPath, { interval: 500 }, (curr) => {
    if (curr.size <= size) { size = curr.size; return }
    const stream = fs.createReadStream(logPath, { start: size, end: curr.size })
    stream.on('data', chunk => process.stdout.write(chunk))
    size = curr.size
  })
}
