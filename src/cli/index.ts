import { runInit }      from './init.command'
import { runInstall }   from './install.command'
import { runUninstall } from './uninstall.command'
import { runStart }     from './start.command'
import { runStop }      from './stop.command'
import { runRestart }   from './restart.command'
import { runStatus }    from './status.command'
import { runLogs }      from './logs.command'
import { runDoctor }    from './doctor.command'
import { runInstaller } from './installer.command'
import { logger }       from '../utils/logger'

type CommandName =
  | 'init' | 'install' | 'uninstall'
  | 'start' | 'stop' | 'restart'
  | 'status' | 'logs' | 'doctor' | 'installer'

const COMMANDS: Record<CommandName, (root: string) => void> = {
  init:      runInit,
  install:   runInstall,
  uninstall: runUninstall,
  start:     runStart,
  stop:      runStop,
  restart:   runRestart,
  status:    runStatus,
  logs:      runLogs,
  doctor:    runDoctor,
  installer: runInstaller,
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
  logger.title('Aruna — servicios de Windows para Node.js')
  console.log('Usage: aruna <command>\n')
  console.log('Commands:')
  console.log('  init        Create winsvc.config.json in current directory')
  console.log('  install     Register app as a Windows service')
  console.log('  uninstall   Remove the Windows service')
  console.log('  start       Start the service')
  console.log('  stop        Stop the service')
  console.log('  restart     Restart the service (stop + start)')
  console.log('  status      Show current service status')
  console.log('  logs        Print the service log  (-f to follow, -n <N> lines)')
  console.log('  doctor      Diagnose config, core binary, Node and script paths')
  console.log('  installer   Build a double-click installer .exe for your app\n')
  console.log('Example:')
  console.log('  npx aruna init')
  console.log('  npx aruna doctor')
  console.log('  npx aruna install')
  console.log('  npx aruna start')
  console.log('  npx aruna installer   # one .exe to install it anywhere')
}
