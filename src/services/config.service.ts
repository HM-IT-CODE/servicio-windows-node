import fs   from 'fs'
import path from 'path'
import { WinsvcConfig, DEFAULT_CONFIG } from '../models/winsvc-config.model'
import { ConfigNotFoundError } from '../utils/errors'

const CONFIG_FILENAME = 'winsvc.config.json'

export class ConfigService {

  load(projectRoot: string): WinsvcConfig {
    const configPath = path.join(projectRoot, CONFIG_FILENAME)

    if (!fs.existsSync(configPath)) {
      throw new ConfigNotFoundError(configPath)
    }

    const raw  = fs.readFileSync(configPath, 'utf-8')
    const data = JSON.parse(raw) as Partial<WinsvcConfig>

    return { ...DEFAULT_CONFIG, ...data } as WinsvcConfig
  }

  createTemplate(projectRoot: string): string {
    const configPath = path.join(projectRoot, CONFIG_FILENAME)

    const template: WinsvcConfig = {
      name:             'my-node-service',
      displayName:      'My Node.js Service',
      description:      'Node.js production service',
      script:           'dist/index.js',
      nodeArgs:         [],
      env:              { NODE_ENV: 'production', PORT: '3000' },
      autoRestart:      true,
      startType:        'auto',
      logFile:          'logs/service.log',
      workingDirectory: '.',
    }

    fs.writeFileSync(configPath, JSON.stringify(template, null, 2), 'utf-8')
    return configPath
  }

  resolveScriptPath(config: WinsvcConfig, projectRoot: string): string {
    return path.isAbsolute(config.script)
      ? config.script
      : path.resolve(projectRoot, config.script)
  }

  resolveWorkingDir(config: WinsvcConfig, projectRoot: string): string {
    const workDir = config.workingDirectory ?? '.'
    return path.isAbsolute(workDir)
      ? workDir
      : path.resolve(projectRoot, workDir)
  }
}
