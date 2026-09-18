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
  /** Generación del instalador .exe (comando `installer`) */
  installer?: InstallerConfig
}

/** Un campo que el asistente le pide al usuario y escribe en el `.env`. */
export interface InstallerPrompt {
  /** Nombre de la variable en el .env (p. ej. "DB_HOST") */
  key: string
  /** Etiqueta mostrada en el asistente */
  label: string
  /** Valor propuesto */
  default?: string
  /** Campo de contraseña: se muestra con asteriscos */
  secret?: boolean
  /** Página del asistente en la que aparece (agrupa campos) */
  group?: string
}

export interface InstallerConfig {
  /** Nombre del producto en el asistente y en "Agregar o quitar programas" */
  appName: string
  /** Versión mostrada en el instalador */
  version: string
  /** Editor / empresa */
  publisher: string
  /**
   * GUID único de la aplicación. Windows identifica la instalación por él:
   * dos apps con el mismo AppId se pisan en "Agregar o quitar programas".
   * Si se omite, `installer` genera uno estable a partir del nombre.
   */
  appId?: string
  /** Carpeta de salida del .exe (relativa al proyecto) */
  outputDir?: string
  /** Carpeta de instalación propuesta, bajo Archivos de programa */
  defaultDirName?: string
  /** Archivos y carpetas a incluir, relativos al proyecto */
  include?: string[]
  /** Campos que el asistente pregunta y vuelca al .env */
  prompts?: InstallerPrompt[]
  /** Script Node que valida la configuración antes de registrar el servicio */
  verifyScript?: string
  /** Idioma del asistente */
  language?: 'es' | 'en'
  /**
   * Empaqueta node.exe dentro del instalador, en `vendor/node.exe`.
   * El servidor de destino deja de necesitar Node instalado, y la versión
   * queda fijada: una actualización del Node del sistema no puede romper el
   * servicio. Suma ~80 MB al .exe.
   *
   * `true` usa el node.exe que esté ejecutando el CLI; una cadena es la ruta
   * a un node.exe concreto (útil para fijar una versión distinta).
   */
  bundleNode?: boolean | string
  /**
   * Nivel de compresión zstd del instalador propio, de 1 a 22 (por omisión 19).
   * Medido sobre un caso real de 80 MB + 7.900 archivos: el 10 da 41 MB en dos
   * minutos y el 19 da 37 MB en cinco. Descomprimir cuesta lo mismo en ambos,
   * así que el nivel solo afecta a quien empaqueta.
   */
  compressionLevel?: number
}

export const DEFAULT_CONFIG: Omit<WinsvcConfig, 'name' | 'displayName' | 'script'> = {
  description: 'Node.js Windows Service',
  nodeArgs: [],
  env: { NODE_ENV: 'production' },
  autoRestart: true,
  startType: 'auto',
  logFile: 'logs/service.log',
}
