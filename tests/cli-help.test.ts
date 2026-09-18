import fs   from 'fs'
import path from 'path'

/**
 * La ayuda tiene que listar TODOS los comandos.
 *
 * La 0.2.0 se publicó con `doctor` e `installer` funcionando pero ausentes de
 * la ayuda: el comando estrella del paquete era invisible para quien lo
 * instalara. Este test existe para que no se repita.
 */
describe('ayuda del CLI', () => {
  const fuente = fs.readFileSync(
    path.join(__dirname, '..', 'src', 'cli', 'index.ts'), 'utf-8',
  )

  /** Los comandos declarados en el tipo CommandName. */
  const comandosDeclarados = (): string[] => {
    const bloque = /type CommandName =([\s\S]*?)const COMMANDS/.exec(fuente)
    if (!bloque) throw new Error('No se encontró el tipo CommandName')

    return [...bloque[1].matchAll(/'([a-z]+)'/g)].map(m => m[1])
  }

  /** Lo que printHelp imprime. */
  const ayuda = (): string => {
    const bloque = /function printHelp\(\)[\s\S]*$/.exec(fuente)
    if (!bloque) throw new Error('No se encontró printHelp')
    return bloque[0]
  }

  it('declara al menos los comandos conocidos', () => {
    expect(comandosDeclarados()).toEqual(
      expect.arrayContaining([
        'init', 'install', 'uninstall', 'start', 'stop',
        'restart', 'status', 'logs', 'doctor', 'installer',
      ]),
    )
  })

  it.each(comandosDeclarados())('lista "%s" en la ayuda', (comando) => {
    expect(ayuda()).toContain(comando)
  })

  it('cada comando del mapa está también en el tipo', () => {
    // El tipo del mapa lleva `=>` dentro, así que hay que buscar de forma
    // perezosa hasta el `= {` de verdad, sin acotar por caracteres.
    const mapa = /const COMMANDS[\s\S]*?= \{([\s\S]*?)\n\}/.exec(fuente)
    if (!mapa) throw new Error('No se encontró el mapa COMMANDS')

    const enMapa = [...mapa[1].matchAll(/^\s+([a-z]+):/gm)].map(m => m[1])

    expect(enMapa.sort()).toEqual(comandosDeclarados().sort())
  })
})
