import fs   from 'fs'
import path from 'path'
import { BinaryNotFoundError } from '../utils/errors'

const BINARY_NAME = 'node-winsvc-core.exe'

export class BinaryService {

  /** Localiza el exe Rust dentro del paquete npm instalado */
  resolvePath(): string {
    const candidates = [
      // Cuando se instala como paquete npm global/local
      path.join(__dirname, '..', '..', 'bin', BINARY_NAME),
      // Desarrollo local (antes de npm publish)
      path.join(__dirname, '..', '..', '..', 'bin', BINARY_NAME),
    ]

    const found = candidates.find(p => fs.existsSync(p))
    if (!found) throw new BinaryNotFoundError(candidates[0])

    return found
  }

  exists(): boolean {
    try {
      this.resolvePath()
      return true
    } catch {
      return false
    }
  }
}
