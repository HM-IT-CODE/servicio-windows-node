const COLORS = {
  reset:   '\x1b[0m',
  green:   '\x1b[32m',
  red:     '\x1b[31m',
  yellow:  '\x1b[33m',
  blue:    '\x1b[34m',
  cyan:    '\x1b[36m',
  gray:    '\x1b[90m',
  bold:    '\x1b[1m',
} as const

export const logger = {
  info:    (msg: string) => console.log(`${COLORS.cyan}ℹ${COLORS.reset}  ${msg}`),
  success: (msg: string) => console.log(`${COLORS.green}✔${COLORS.reset}  ${msg}`),
  warn:    (msg: string) => console.log(`${COLORS.yellow}⚠${COLORS.reset}  ${msg}`),
  error:   (msg: string) => console.error(`${COLORS.red}✖${COLORS.reset}  ${msg}`),
  step:    (msg: string) => console.log(`${COLORS.gray}→${COLORS.reset}  ${msg}`),
  title:   (msg: string) => console.log(`\n${COLORS.bold}${COLORS.blue}${msg}${COLORS.reset}\n`),
  blank:   ()            => console.log(),
}
