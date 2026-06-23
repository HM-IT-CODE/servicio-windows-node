/**
 * Public programmatic API.
 * The primary interface is the CLI (`npx node-winsvc <command>`); this module
 * re-exports the configuration types for tooling that wants type-safe configs.
 */
export type { WinsvcConfig, ServiceStartType } from './models/winsvc-config.model'
export { DEFAULT_CONFIG } from './models/winsvc-config.model'
