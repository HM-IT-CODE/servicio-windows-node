export type ServiceStartType = 'auto' | 'manual' | 'disabled'

export interface WinsvcConfig {
  /** Nombre interno del servicio en Windows (sin espacios) */
  name: string
  /** Nombre visible en el panel de Servicios de Windows */
  displayName: string
  /** Descripción visible en el panel de Servicios */
  description: string
  /** Ruta al script Node.js de entrada (relativa al proyecto) */
  script: string
  /** Argumentos adicionales para node.exe */
  nodeArgs: string[]
  /** Variables de entorno inyectadas al proceso */
  env: Record<string, string>
  /** Reiniciar el proceso si falla */
  autoRestart: boolean
  /** Tipo de inicio del servicio */
  startType: ServiceStartType
  /** Archivo de log del servicio (relativo al proyecto) */
  logFile: string
  /** Directorio de trabajo (relativo al proyecto, default: raíz) */
  workingDirectory?: string
}

export const DEFAULT_CONFIG: Omit<WinsvcConfig, 'name' | 'displayName' | 'script'> = {
  description: 'Node.js Windows Service',
  nodeArgs: [],
  env: { NODE_ENV: 'production' },
  autoRestart: true,
  startType: 'auto',
  logFile: 'logs/service.log',
}
