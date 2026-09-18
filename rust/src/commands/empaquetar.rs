//! Construye el instalador final pegando la aplicacion detras del stub.
//!
//! ```text
//!   [ node-winsvc-setup.exe ]   el asistente generico, siempre el mismo
//!   [ zip                   ]   los archivos de la aplicacion
//!   [ manifest              ]   JSON: que preguntar, que servicio registrar
//!   [ pie                   ]   longitudes + firma, 24 bytes
//! ```
//!
//! Un .exe con datos pegados detras sigue siendo un .exe valido: el cargador
//! de Windows solo mira las cabeceras del principio y no le molesta la cola.
//!
//! El empaquetado vive aqui y no en la capa TypeScript para que el paquete npm
//! siga sin dependencias: comprimir en Node exigiria traerse una libreria.

use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::setup::paquete::{self, Constructor};
use crate::setup::payload::FIRMA;

pub struct Args {
    pub stub:     String,
    pub origen:   String,
    pub incluir:  String,
    pub core:     String,
    pub node:     String,
    pub manifest: String,
    pub salida:   String,
    /// Nivel de compresion zstd, 1 a 22.
    pub nivel:    i64,
}

pub fn run(args: &Args) -> Result<()> {
    let stub = Path::new(&args.stub);
    if !stub.exists() {
        return Err(anyhow!("No se encuentra el instalador base: {}", stub.display()));
    }

    let manifest = fs::read_to_string(&args.manifest)
        .map_err(|e| anyhow!("No se pudo leer el manifiesto: {e}"))?;

    let (paquete, archivos) = construir_paquete(args)?;

    // El stub primero, tal cual: es lo que Windows va a ejecutar.
    let mut salida = fs::File::create(&args.salida)
        .map_err(|e| anyhow!("No se pudo crear {}: {e}", args.salida))?;

    salida.write_all(&fs::read(stub)?)?;
    salida.write_all(&paquete)?;
    salida.write_all(manifest.as_bytes())?;

    // El pie: el stub lo lee desde el final para saber donde empieza todo.
    salida.write_all(&(paquete.len() as u64).to_le_bytes())?;
    salida.write_all(&(manifest.len() as u64).to_le_bytes())?;
    salida.write_all(FIRMA)?;

    let total = salida.metadata()?.len();
    println!(
        r#"{{"ok":true,"salida":"{}","bytes":{},"archivos":{}}}"#,
        args.salida.replace('\\', "\\\\"),
        total,
        archivos,
    );
    Ok(())
}

/// Arma el paquete: TAR de todo, comprimido de una pieza.
fn construir_paquete(args: &Args) -> Result<(Vec<u8>, usize)> {
    let mut constructor = Constructor::nuevo();
    let origen = Path::new(&args.origen);
    let mut archivos = 0;

    for entrada in args.incluir.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let ruta = origen.join(entrada.replace('/', "\\"));
        if !ruta.exists() {
            return Err(anyhow!("No se encuentra lo que se pidio incluir: {}", ruta.display()));
        }

        for (archivo, nombre) in paquete::recorrer(&ruta, origen) {
            constructor.agregar(&archivo, &nombre)?;
            archivos += 1;
        }
    }

    // El nucleo del servicio, siempre en bin\ : sin el, el instalador copia
    // los archivos pero no puede registrar nada.
    let core = Path::new(&args.core);
    if !core.exists() {
        return Err(anyhow!("No se encuentra el nucleo del servicio: {}", core.display()));
    }
    constructor.agregar(core, "bin/node-winsvc-core.exe")?;
    archivos += 1;

    // El Node empaquetado, si lo hay: el servidor deja de necesitar Node.
    if !args.node.is_empty() {
        let node = Path::new(&args.node);
        if !node.exists() {
            return Err(anyhow!("No se encuentra node.exe: {}", node.display()));
        }
        constructor.agregar(node, "vendor/node.exe")?;
        archivos += 1;
    }

    Ok((constructor.terminar(args.nivel as i32)?, archivos))
}
