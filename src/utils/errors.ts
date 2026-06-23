export class WinsvcError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message)
    this.name = 'WinsvcError'
  }
}

export class ConfigNotFoundError extends WinsvcError {
  constructor(path: string) {
    super(`winsvc.config.json not found at: ${path}`, 'CONFIG_NOT_FOUND')
  }
}

export class BinaryNotFoundError extends WinsvcError {
  constructor(path: string) {
    super(`node-winsvc-core.exe not found at: ${path}`, 'BINARY_NOT_FOUND')
  }
}

export class NotWindowsError extends WinsvcError {
  constructor() {
    super('node-winsvc only works on Windows', 'NOT_WINDOWS')
  }
}

export class ServiceAlreadyExistsError extends WinsvcError {
  constructor(name: string) {
    super(`Service "${name}" is already installed. Run uninstall first.`, 'SERVICE_EXISTS')
  }
}

export class ServiceNotFoundError extends WinsvcError {
  constructor(name: string) {
    super(`Service "${name}" is not installed.`, 'SERVICE_NOT_FOUND')
  }
}
