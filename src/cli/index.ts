import { runInit }      from './init.command'
import { runInstall }   from './install.command'
import { runUninstall } from './uninstall.command'
import { runStart }     from './start.command'
import { runStop }      from './stop.command'
import { runStatus }    from './status.command'
import { logger }       from '../utils/logger'

type CommandName = 'init' | 'install' | 'uninstall' | 'start' | 'stop' | 'status'

const COMMANDS: Record<CommandName, (root: string) => void> = {
  init:      runInit,
  install:   runInstall,
  uninstall: runUninstall,
  start:     runStart,
  stop:      runStop,
  status:    runStatus,
}

export function dispatch(command: string, projectRoot: string): void {
  const handler = COMMANDS[command as CommandName]

  if (!handler) {
    logger.error(`Unknown command: "${command}"`)
    printHelp()
    process.exit(1)
  }

  handler(projectRoot)
}

function printHelp(): void {
  logger.title('node-winsvc — Native Windows Service Manager for Node.js')
  console.log('Usage: node-winsvc <command>\n')
  console.log('Commands:')
  console.log('  init        Create winsvc.config.json in current directory')
  console.log('  install     Register app as a Windows service')
  console.log('  uninstall   Remove the Windows service')
  console.log('  start       Start the service')
  console.log('  stop        Stop the service')
  console.log('  status      Show current service status\n')
  console.log('Example:')
  console.log('  npx node-winsvc init')
  console.log('  npx node-winsvc install')
  console.log('  npx node-winsvc start')
}
