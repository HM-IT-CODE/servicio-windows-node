#!/usr/bin/env node
import path from 'path'
import { dispatch }      from './cli/index'
import { NotWindowsError } from './utils/errors'
import { logger }          from './utils/logger'

function assertWindows(): void {
  if (process.platform !== 'win32') throw new NotWindowsError()
}

function resolveProjectRoot(): string {
  return process.cwd()
}

function main(): void {
  assertWindows()

  const [, , command] = process.argv
  const projectRoot   = resolveProjectRoot()

  if (!command) {
    dispatch('--help', projectRoot)
    return
  }

  try {
    dispatch(command, projectRoot)
  } catch (err) {
    logger.error(err instanceof Error ? err.message : String(err))
    process.exit(1)
  }
}

main()
