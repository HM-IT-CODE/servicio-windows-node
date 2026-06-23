import { spawnSync } from 'child_process'
import { WinsvcError } from '../utils/errors'

interface RunOptions {
  binaryPath: string
  args:       string[]
  cwd?:       string
}

interface RunResult {
  success: boolean
  stdout:  string
  stderr:  string
  code:    number
}

export class ProcessService {

  run(options: RunOptions): RunResult {
    const result = spawnSync(options.binaryPath, options.args, {
      cwd:      options.cwd,
      encoding: 'utf-8',
      shell:    false,
    })

    return {
      success: result.status === 0,
      stdout:  result.stdout ?? '',
      stderr:  result.stderr ?? '',
      code:    result.status ?? 1,
    }
  }

  runOrThrow(options: RunOptions): string {
    const result = this.run(options)

    if (!result.success) {
      const detail = result.stderr || result.stdout || `exit code ${result.code}`
      throw new WinsvcError(detail.trim(), 'CORE_ERROR')
    }

    return result.stdout.trim()
  }
}
