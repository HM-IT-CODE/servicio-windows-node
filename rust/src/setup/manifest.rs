//! El manifiesto que el CLI mete dentro del instalador: describe el asistente
//! y el servicio a registrar. Es el equivalente al .iss de Inno, pero en JSON
//! y leido en tiempo de ejecucion en vez de compilado.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campo {
    /// Nombre de la variable en el .env
    pub key: String,
    /// Etiqueta mostrada junto al cuadro de texto
    pub label: String,
    #[serde(default)]
    pub default: String,
    /// Se muestra con asteriscos
    #[serde(default)]
    pub secret: bool,
    /// Pagina del asistente en la que aparece
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    // --- Producto ---
    pub app_name:  String,
    pub version:   String,
    pub publisher: String,
    /// Carpeta bajo Archivos de programa
    pub dir_name:  String,

    // --- Servicio ---
    pub service_name:    String,
    pub service_display: String,
    pub service_desc:    String,
    pub script:          String,
    #[serde(default)]
    pub node_args:    String,
    #[serde(default)]
    pub log_file:     String,
    #[serde(default = "auto")]
    pub start_type:   String,
    #[serde(default = "verdadero")]
    pub auto_restart: bool,

    // --- Asistente ---
    #[serde(default)]
    pub campos: Vec<Campo>,
    /// Variables fijas que van al .env sin preguntar
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Script Node que valida la configuracion antes de registrar el servicio
    #[serde(default)]
    pub verify_script: String,
    /// El paquete trae su propio node.exe en vendor\node.exe
    #[serde(default)]
    pub bundled_node: bool,
}

fn auto() -> String { "auto".to_string() }
fn verdadero() -> bool { true }

impl Manifest {
    pub fn desde_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Manifiesto invalido: {e}"))
    }

    /// Los campos agrupados en paginas, respetando el orden de declaracion.
    pub fn paginas(&self) -> Vec<(String, Vec<Campo>)> {
        let mut paginas: Vec<(String, Vec<Campo>)> = Vec::new();

        for campo in &self.campos {
            let grupo = if campo.group.is_empty() { "Configuracion" } else { &campo.group };

            match paginas.iter_mut().find(|(nombre, _)| nombre == grupo) {
                Some((_, lista)) => lista.push(campo.clone()),
                None => paginas.push((grupo.to_string(), vec![campo.clone()])),
            }
        }
        paginas
    }
}

impl Manifest {
    /// Manifiesto de ejemplo para desarrollar la interfaz sin empaquetar nada.
    ///
    /// Se usa cuando el exe no lleva carga util adosada, que es justo lo que
    /// pasa al hacer `cargo run --bin node-winsvc-setup`. Asi se puede iterar
    /// sobre la ventana en segundos, sin generar un instalador cada vez.
    pub fn demo() -> Self {
        Self {
            app_name:  "API Demo".into(),
            version:   "1.0.0".into(),
            publisher: "node-winsvc".into(),
            dir_name:  "API Demo".into(),

            service_name:    "api-demo".into(),
            service_display: "API Demo - Servicio".into(),
            service_desc:    "Servicio de ejemplo para probar el asistente.".into(),
            script:          "src/server.js".into(),
            node_args:       "--expose-gc".into(),
            log_file:        "logs/service.log".into(),
            start_type:      "auto".into(),
            auto_restart:    true,

            campos: vec![
                Campo { key: "DB_HOST".into(),     label: "Servidor".into(),
                        default: "localhost".into(), secret: false, group: "Base de datos".into() },
                Campo { key: "DB_PORT".into(),     label: "Puerto".into(),
                        default: "1433".into(),      secret: false, group: "Base de datos".into() },
                Campo { key: "DB_NAME".into(),     label: "Base".into(),
                        default: "mi_base".into(),   secret: false, group: "Base de datos".into() },
                Campo { key: "DB_USER".into(),     label: "Usuario".into(),
                        default: "sa".into(),        secret: false, group: "Base de datos".into() },
                Campo { key: "DB_PASSWORD".into(), label: "Contrasena".into(),
                        default: "".into(),          secret: true,  group: "Base de datos".into() },
                Campo { key: "PORT".into(),        label: "Puerto HTTP".into(),
                        default: "3080".into(),      secret: false, group: "Servicio".into() },
            ],
            env: vec![("NODE_ENV".into(), "production".into())],
            verify_script: String::new(),
            bundled_node:  false,
        }
    }
}
