import { ConfigService } from '../services/config.service'
import { logger }        from '../utils/logger'

const configService = new ConfigService()

export function runInit(projectRoot: string): void {
  logger.title('Aruna — Init')

  const configPath = configService.createTemplate(projectRoot)

  logger.success(`Created: ${configPath}`)
  logger.blank()
  logger.info('Edit winsvc.config.json and then run:')
  logger.step('aruna install')
  logger.step('aruna start')
}
