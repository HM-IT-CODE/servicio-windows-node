//! El asistente, en Win32 puro.
//!
//! Sin dependencias de interfaz: los mismos controles que usa el propio
//! Windows, asi el instalador arranca en cualquier version sin instalar nada.
//! Es la misma razon por la que el nucleo se enlaza con el runtime de C
//! estatico: un instalador que necesita instalar algo antes no sirve.
//!
//! Lo que Win32 NO da y hay que pintar a mano (ver `tema.rs`): el panel
//! lateral con degradado, los botones de color y la lista de pasos. Un boton
//! normal sale gris; con `BS_OWNERDRAW` Windows deja de pintarlo y nos manda
//! `WM_DRAWITEM` para que lo hagamos nosotros.

use std::cell::RefCell;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_DISABLED, ODS_SELECTED, WC_BUTTONW};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::setup::manifest::Manifest;
use crate::setup::tema;

/// Medidas del AREA DE CLIENTE, no de la ventana entera: el borde y la barra
/// de titulo se suman aparte con AdjustWindowRect. Usar el total aqui fue un
/// bug real: los botones se montaban encima del ultimo campo.
const ANCHO: i32 = 720;
const ALTO:  i32 = 500;

const ALTO_BOTON:  i32 = 34;
const ANCHO_BOTON: i32 = 104;
/// Franja inferior donde viven los botones.
const PIE: i32 = 68;

const ID_ATRAS:      isize = 1001;
const ID_SIGUIENTE:  isize = 1002;
const ID_CANCELAR:   isize = 1003;
const ID_CAMPO_BASE: isize = 2000;

/// Gris claro para el texto secundario sobre el panel de color.
const CLARO: COLORREF = COLORREF(0x00D8D0C8);
/// Rojo apagado para el titulo cuando la instalacion falla.
const ROJO: COLORREF = COLORREF(0x004444CC);

/// La marca que firma el instalador.
const MARCA: &str = "Aruna";

struct Estado {
    manifest:  Manifest,
    pagina:    usize,
    destino:   String,
    valores:   Vec<(String, String)>,
    controles: Vec<HWND>,
    terminado: bool,
    cancelado: bool,
    /// Modo desarrollo: se muestra todo pero no se instala nada.
    demo:      bool,
    /// Mensaje del ultimo paso, mostrado en la pagina final.
    resultado: String,
    /// La instalacion fallo: cambia el color del titulo final.
    fallo:     bool,
    /// Los archivos a extraer. Se leen UNA vez al arrancar y se guardan aqui:
    /// releerlos al instalar hacia que la instalacion dependiera de que el
    /// .exe siguiera en su sitio, y no tiene por que estarlo.
    zip:       Vec<u8>,
}

thread_local! {
    static ESTADO: RefCell<Option<Estado>> = const { RefCell::new(None) };
}

fn con_estado<T>(f: impl FnOnce(&Estado) -> T) -> T {
    ESTADO.with(|e| f(e.borrow().as_ref().unwrap()))
}

fn mutar<T>(f: impl FnOnce(&mut Estado) -> T) -> T {
    ESTADO.with(|e| f(e.borrow_mut().as_mut().unwrap()))
}

/// bienvenida + carpeta + una por grupo de campos + instalar.
fn total_paginas(manifest: &Manifest) -> usize {
    2 + manifest.paginas().len() + 1
}

/// Los nombres que se muestran en la lista de pasos del panel lateral.
fn nombres_pasos(manifest: &Manifest) -> Vec<String> {
    let mut pasos = vec!["Bienvenida".to_string(), "Carpeta".to_string()];
    pasos.extend(manifest.paginas().into_iter().map(|(nombre, _)| nombre));
    pasos.push("Instalar".to_string());
    pasos
}

/// Abre el asistente y bloquea hasta que se cierre. La instalacion ocurre
/// dentro, en `ejecutar_instalacion`.
pub fn mostrar(manifest: Manifest, demo: bool, zip: Vec<u8>) {
    let destino = format!(
        "{}\\{}",
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()),
        manifest.dir_name,
    );

    ESTADO.with(|e| {
        *e.borrow_mut() = Some(Estado {
            manifest: manifest.clone(),
            pagina: 0,
            destino,
            valores: Vec::new(),
            controles: Vec::new(),
            terminado: false,
            cancelado: false,
            demo,
            resultado: String::new(),
            fallo: false,
            zip,
        })
    });

    unsafe {
        crear_ventana(&manifest);
        bucle_mensajes();
    }
}

unsafe fn crear_ventana(manifest: &Manifest) {
    let instancia = GetModuleHandleW(None).unwrap();

    // El icono incrustado por build.rs. winresource lo registra con el id 1;
    // sin esto la ventana sale con el icono generico de Windows, aunque el
    // .exe si tenga el suyo en el Explorador.
    let icono = LoadImageW(
        instancia, PCWSTR(1 as *const u16), IMAGE_ICON,
        0, 0, LR_DEFAULTSIZE | LR_SHARED,
    ).map(|h| HICON(h.0)).unwrap_or_default();

    let clase = WNDCLASSW {
        lpfnWndProc: Some(ventana_proc),
        hInstance: instancia.into(),
        lpszClassName: w!("NodeWinsvcSetup"),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
        hIcon: icono,
        hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
        ..Default::default()
    };
    RegisterClassW(&clase);

    // El tamano que pide CreateWindowExW incluye bordes y barra de titulo.
    // AdjustWindowRect calcula cuanto hay que pedir para que el AREA UTIL mida
    // lo que queremos. Sin esto el contenido queda recortado por abajo.
    let estilo = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let mut marco = RECT { left: 0, top: 0, right: ANCHO, bottom: ALTO };
    let _ = AdjustWindowRect(&mut marco, estilo, false);

    let titulo: Vec<u16> = format!("Instalar {}\0", manifest.app_name).encode_utf16().collect();

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("NodeWinsvcSetup"),
        PCWSTR(titulo.as_ptr()),
        estilo,
        CW_USEDEFAULT, CW_USEDEFAULT,
        marco.right - marco.left,
        marco.bottom - marco.top,
        None, None, instancia, None,
    ).unwrap();

    crear_botones(hwnd, instancia.into());
    dibujar_pagina(hwnd);

    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = UpdateWindow(hwnd);
}

unsafe fn crear_botones(padre: HWND, instancia: windows::Win32::Foundation::HINSTANCE) {
    let y = ALTO - PIE + (PIE - ALTO_BOTON) / 2;

    let botones = [
        (w!("Atras"),     ID_ATRAS,     ANCHO - 24 - ANCHO_BOTON * 3 - 20),
        (w!("Siguiente"), ID_SIGUIENTE, ANCHO - 24 - ANCHO_BOTON * 2 - 10),
        (w!("Cancelar"),  ID_CANCELAR,  ANCHO - 24 - ANCHO_BOTON),
    ];

    for (texto, id, x) in botones {
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_BUTTONW,
            texto,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_OWNERDRAW as u32),
            x, y, ANCHO_BOTON, ALTO_BOTON,
            padre, HMENU(id as *mut _), instancia, None,
        );
    }
}

// --- Dibujo ----------------------------------------------------------------

/// El fondo: panel lateral con degradado, lista de pasos y franja del pie.
unsafe fn pintar_fondo(hwnd: HWND, dc: HDC) {
    let (pagina, manifest) = con_estado(|e| (e.pagina, e.manifest.clone()));

    let mut cliente = RECT::default();
    let _ = GetClientRect(hwnd, &mut cliente);

    tema::rellenar(dc, cliente, tema::BLANCO);

    let panel = RECT { left: 0, top: 0, right: tema::PANEL, bottom: cliente.bottom };
    tema::degradado(dc, panel, tema::TURQUESA, tema::TURQUESA_NOCHE);

    let titular = tema::fuente(23, 700);
    let previa = SelectObject(dc, titular);
    tema::texto(dc, 24, 32, tema::PANEL - 44, &manifest.app_name, tema::BLANCO, DT_WORDBREAK);
    SelectObject(dc, previa);
    let _ = DeleteObject(titular);

    let pequena = tema::fuente(13, 400);
    let previa = SelectObject(dc, pequena);
    tema::texto(dc, 24, 84, tema::PANEL - 44,
                &format!("version {}", manifest.version), CLARO, DT_SINGLELINE);
    SelectObject(dc, previa);
    let _ = DeleteObject(pequena);

    pintar_pasos(dc, &manifest, pagina);
    pintar_marca(dc, cliente.bottom);

    let pie = RECT {
        left: tema::PANEL, top: cliente.bottom - PIE,
        right: cliente.right, bottom: cliente.bottom,
    };
    tema::rellenar(dc, pie, tema::HUMO);
}

/// La marca del instalador, al pie del panel lateral. Va discreta a proposito:
/// el protagonista es el producto que se esta instalando, no la herramienta.
unsafe fn pintar_marca(dc: HDC, alto: i32) {
    let fuente = tema::fuente(13, 600);
    let previa = SelectObject(dc, fuente);
    tema::texto(dc, 24, alto - 44, tema::PANEL - 40, MARCA, CLARO, DT_SINGLELINE);
    SelectObject(dc, previa);
    let _ = DeleteObject(fuente);
}

/// La lista de pasos: un punto por pagina, el actual mas grande y en negrita.
unsafe fn pintar_pasos(dc: HDC, manifest: &Manifest, actual: usize) {
    let pasos = nombres_pasos(manifest);
    let normal = tema::fuente(14, 400);
    let negrita = tema::fuente(14, 700);

    let mut y = 140;
    for (i, paso) in pasos.iter().enumerate() {
        let es_actual = i == actual;
        let hecho = i < actual;

        let color = if es_actual || hecho { tema::BLANCO } else { CLARO };
        tema::circulo(dc, 30, y + 8, if es_actual { 5 } else { 3 }, color);

        let f = if es_actual { negrita } else { normal };
        let previa = SelectObject(dc, f);
        tema::texto(dc, 46, y, tema::PANEL - 62, paso, color, DT_SINGLELINE | DT_END_ELLIPSIS);
        SelectObject(dc, previa);

        y += 34;
    }

    let _ = DeleteObject(normal);
    let _ = DeleteObject(negrita);
}

/// Un boton pintado a mano. Win32 solo nos da el DC y el rectangulo.
unsafe fn pintar_boton(item: &DRAWITEMSTRUCT) {
    let principal = item.CtlID as isize == ID_SIGUIENTE;
    let pulsado = (item.itemState.0 & ODS_SELECTED.0) != 0;
    let apagado = (item.itemState.0 & ODS_DISABLED.0) != 0;

    let fondo = if apagado {
        tema::HUMO
    } else if principal {
        if pulsado { tema::TURQUESA_OSCURO } else { tema::TURQUESA }
    } else if pulsado {
        tema::BORDE
    } else {
        tema::BLANCO
    };

    tema::rellenar(item.hDC, item.rcItem, fondo);

    // Borde sutil solo en los secundarios: el turquesa se defiende solo.
    if !principal {
        let lapiz = CreatePen(PS_SOLID, 1, tema::BORDE);
        let lapiz_previo = SelectObject(item.hDC, lapiz);
        let brocha_previa = SelectObject(item.hDC, GetStockObject(NULL_BRUSH));

        let _ = Rectangle(item.hDC, item.rcItem.left, item.rcItem.top,
                          item.rcItem.right, item.rcItem.bottom);

        SelectObject(item.hDC, lapiz_previo);
        SelectObject(item.hDC, brocha_previa);
        let _ = DeleteObject(lapiz);
    }

    let color = if apagado {
        tema::GRIS
    } else if principal {
        tema::BLANCO
    } else {
        tema::TINTA
    };

    let mut texto = [0u16; 64];
    let largo = GetWindowTextW(item.hwndItem, &mut texto);

    let fuente = tema::fuente(15, if principal { 600 } else { 400 });
    let previa = SelectObject(item.hDC, fuente);

    SetBkMode(item.hDC, TRANSPARENT);
    SetTextColor(item.hDC, color);

    let mut rect = item.rcItem;
    DrawTextW(item.hDC, &mut texto[..largo as usize], &mut rect,
              DT_CENTER | DT_VCENTER | DT_SINGLELINE);

    SelectObject(item.hDC, previa);
    let _ = DeleteObject(fuente);
}

/// El texto de cada pagina se PINTA, no se crea como control: asi se controla
/// la tipografia y el color sin pelear con el gris del sistema.
unsafe fn pintar_contenido(dc: HDC) {
    let (pagina, manifest, resultado, fallo, demo) =
        con_estado(|e| (e.pagina, e.manifest.clone(), e.resultado.clone(), e.fallo, e.demo));

    let paginas = manifest.paginas();
    let ancho = ANCHO - tema::MARGEN - 24;

    let (titulo, cuerpo) = if pagina == 0 {
        (format!("Instalar {}", manifest.app_name),
         format!(
            "Este asistente instalara {} y lo registrara como servicio de Windows.\n\n\
             El servicio arrancara solo cada vez que se encienda el equipo, y se\n\
             reiniciara si el programa se cae.\n\n\
             Pulse Siguiente para continuar.",
            manifest.app_name))
    } else if pagina == 1 {
        ("Carpeta de destino".to_string(),
         "Se instalara en la siguiente carpeta:".to_string())
    } else if pagina < 2 + paginas.len() {
        (paginas[pagina - 2].0.clone(),
         "Indique los datos necesarios para que el servicio funcione.".to_string())
    } else if resultado.is_empty() {
        ("Listo para instalar".to_string(),
         format!(
            "Se copiaran los archivos, se comprobara la configuracion y se\n\
             registrara el servicio \"{}\".\n\n\
             La comprobacion se hace ANTES de registrar nada: si algo esta mal,\n\
             no queda un servicio a medias.{}",
            manifest.service_display,
            if demo { "\n\nMODO DEMO: no se instalara nada." } else { "" }))
    } else if fallo {
        ("No se pudo completar".to_string(), resultado.clone())
    } else {
        ("Instalacion completada".to_string(), resultado.clone())
    };

    let f_titulo = tema::fuente(27, 600);
    let previa = SelectObject(dc, f_titulo);
    tema::texto(dc, tema::MARGEN, 42, ancho, &titulo,
                if fallo { ROJO } else { tema::TINTA },
                DT_SINGLELINE | DT_END_ELLIPSIS);
    SelectObject(dc, previa);
    let _ = DeleteObject(f_titulo);

    let f_cuerpo = tema::fuente(15, 400);
    let previa = SelectObject(dc, f_cuerpo);
    tema::texto(dc, tema::MARGEN, 84, ancho, &cuerpo, tema::GRIS, DT_WORDBREAK);
    SelectObject(dc, previa);
    let _ = DeleteObject(f_cuerpo);

    // Las etiquetas de los campos, alineadas con sus cuadros de texto
    let f_etiqueta = tema::fuente(14, 600);
    let previa = SelectObject(dc, f_etiqueta);

    if pagina == 1 {
        tema::texto(dc, tema::MARGEN, 128, ancho, "Carpeta", tema::TINTA, DT_SINGLELINE);
    } else if pagina >= 2 && pagina < 2 + paginas.len() {
        let (_, campos) = &paginas[pagina - 2];
        let mut y = 132;
        for campo in campos {
            tema::texto(dc, tema::MARGEN, y, ancho, &campo.label, tema::TINTA, DT_SINGLELINE);
            y += 58;
        }
    }

    SelectObject(dc, previa);
    let _ = DeleteObject(f_etiqueta);
}

/// Borra los controles de la pagina anterior y crea los de la actual.
unsafe fn dibujar_pagina(hwnd: HWND) {
    limpiar_controles();

    let (pagina, manifest, destino) =
        con_estado(|e| (e.pagina, e.manifest.clone(), e.destino.clone()));

    let instancia: windows::Win32::Foundation::HINSTANCE = GetModuleHandleW(None).unwrap().into();
    let paginas = manifest.paginas();

    if pagina == 1 {
        campo_texto(hwnd, instancia, 150, &destino, ID_CAMPO_BASE, false);
    } else if pagina >= 2 && pagina < 2 + paginas.len() {
        let (_, campos) = &paginas[pagina - 2];

        let mut y = 132;
        for (i, campo) in campos.iter().enumerate() {
            campo_texto(hwnd, instancia, y + 22, &campo.default,
                        ID_CAMPO_BASE + i as isize, campo.secret);
            y += 58;
        }
    }

    actualizar_botones(hwnd, pagina, total_paginas(&manifest));
    let _ = InvalidateRect(hwnd, None, true);
}

unsafe fn actualizar_botones(hwnd: HWND, pagina: usize, total: usize) {
    let terminado = con_estado(|e| e.terminado);

    if let Ok(atras) = GetDlgItem(hwnd, ID_ATRAS as i32) {
        let _ = EnableWindow(atras, pagina > 0 && !terminado);
    }
    if let Ok(cancelar) = GetDlgItem(hwnd, ID_CANCELAR as i32) {
        let _ = ShowWindow(cancelar, if terminado { SW_HIDE } else { SW_SHOW });
    }
    if let Ok(siguiente) = GetDlgItem(hwnd, ID_SIGUIENTE as i32) {
        let texto = if terminado {
            w!("Finalizar")
        } else if pagina == total - 1 {
            w!("Instalar")
        } else {
            w!("Siguiente")
        };
        let _ = SetWindowTextW(siguiente, texto);
    }
}

// --- Controles -------------------------------------------------------------

unsafe fn limpiar_controles() {
    mutar(|e| {
        for control in e.controles.drain(..) {
            let _ = DestroyWindow(control);
        }
    });
}

unsafe fn campo_texto(
    padre: HWND,
    instancia: windows::Win32::Foundation::HINSTANCE,
    y: i32,
    valor: &str,
    id: isize,
    secreto: bool,
) {
    let w: Vec<u16> = format!("{valor}\0").encode_utf16().collect();
    let mut estilo = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
    if secreto {
        estilo |= WINDOW_STYLE(ES_PASSWORD as u32);
    }

    let hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR(w.as_ptr()),
        estilo,
        tema::MARGEN, y, ANCHO - tema::MARGEN - 24, 30,
        padre, HMENU(id as *mut _), instancia, None,
    ).unwrap();

    let fuente = tema::fuente(15, 400);
    SendMessageW(hwnd, WM_SETFONT, WPARAM(fuente.0 as usize), LPARAM(1));

    mutar(|e| e.controles.push(hwnd));
}

unsafe fn leer_control(padre: HWND, id: isize) -> String {
    let Ok(hwnd) = GetDlgItem(padre, id as i32) else { return String::new() };

    let largo = GetWindowTextLengthW(hwnd) + 1;
    let mut buffer = vec![0u16; largo as usize];
    let leidos = GetWindowTextW(hwnd, &mut buffer);

    String::from_utf16_lossy(&buffer[..leidos as usize])
}

/// Guarda lo escrito en la pagina actual antes de movernos.
unsafe fn guardar_pagina(hwnd: HWND) {
    let (pagina, manifest) = con_estado(|e| (e.pagina, e.manifest.clone()));

    if pagina == 1 {
        let destino = leer_control(hwnd, ID_CAMPO_BASE);
        if !destino.trim().is_empty() {
            mutar(|e| e.destino = destino.trim().to_string());
        }
        return;
    }

    let paginas = manifest.paginas();
    if pagina >= 2 && pagina < 2 + paginas.len() {
        let (_, campos) = &paginas[pagina - 2];
        let leidos: Vec<(String, String)> = campos.iter().enumerate()
            .map(|(i, campo)| (campo.key.clone(), leer_control(hwnd, ID_CAMPO_BASE + i as isize)))
            .collect();

        mutar(|e| {
            for (clave, valor) in leidos {
                // Reemplaza si ya estaba: el usuario puede volver atras.
                e.valores.retain(|(k, _)| *k != clave);
                e.valores.push((clave, valor));
            }
        });
    }
}

/// Comprueba la pagina actual. Devuelve el error a mostrar, si lo hay.
unsafe fn validar(hwnd: HWND) -> Option<String> {
    let (pagina, manifest) = con_estado(|e| (e.pagina, e.manifest.clone()));

    if pagina == 1 && leer_control(hwnd, ID_CAMPO_BASE).trim().is_empty() {
        return Some("Indique la carpeta de instalacion.".into());
    }

    let paginas = manifest.paginas();
    if pagina >= 2 && pagina < 2 + paginas.len() {
        let (_, campos) = &paginas[pagina - 2];
        for (i, campo) in campos.iter().enumerate() {
            // Un campo secreto puede quedar vacio a proposito; los demas no.
            if !campo.secret && leer_control(hwnd, ID_CAMPO_BASE + i as isize).trim().is_empty() {
                return Some(format!("Indique {}.", campo.label.to_lowercase()));
            }
        }
    }
    None
}

pub fn avisar(hwnd: HWND, texto: &str, titulo: &str, icono: MESSAGEBOX_STYLE) {
    let t: Vec<u16> = format!("{texto}\0").encode_utf16().collect();
    let c: Vec<u16> = format!("{titulo}\0").encode_utf16().collect();
    unsafe {
        MessageBoxW(hwnd, PCWSTR(t.as_ptr()), PCWSTR(c.as_ptr()), MB_OK | icono);
    }
}

// --- Bucle de mensajes -----------------------------------------------------

unsafe fn bucle_mensajes() {
    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

extern "system" fn ventana_proc(hwnd: HWND, mensaje: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match mensaje {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                pintar_fondo(hwnd, dc);
                pintar_contenido(dc);
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            // Un boton con BS_OWNERDRAW llega aqui para que lo pintemos.
            WM_DRAWITEM => {
                let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
                pintar_boton(item);
                LRESULT(1)
            }
            WM_COMMAND => {
                match (wparam.0 & 0xFFFF) as isize {
                    ID_CANCELAR => {
                        mutar(|e| if !e.terminado { e.cancelado = true });
                        let _ = DestroyWindow(hwnd);
                    }
                    ID_ATRAS => {
                        guardar_pagina(hwnd);
                        mutar(|e| e.pagina = e.pagina.saturating_sub(1));
                        dibujar_pagina(hwnd);
                    }
                    ID_SIGUIENTE => siguiente(hwnd),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_CTLCOLOREDIT => {
                let dc = HDC(wparam.0 as *mut _);
                SetBkMode(dc, OPAQUE);
                SetBkColor(dc, tema::BLANCO);
                SetTextColor(dc, tema::TINTA);
                LRESULT(GetStockObject(WHITE_BRUSH).0 as isize)
            }
            // Lo pinta WM_PAINT entero; responder aqui evita el parpadeo.
            WM_ERASEBKGND => LRESULT(1),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, mensaje, wparam, lparam),
        }
    }
}

/// El boton Siguiente / Instalar / Finalizar, segun donde estemos.
unsafe fn siguiente(hwnd: HWND) {
    let (pagina, manifest, terminado) =
        con_estado(|e| (e.pagina, e.manifest.clone(), e.terminado));

    if terminado {
        let _ = DestroyWindow(hwnd);
        return;
    }

    if let Some(error) = validar(hwnd) {
        avisar(hwnd, &error, "Falta un dato", MB_ICONWARNING);
        return;
    }
    guardar_pagina(hwnd);

    if pagina == total_paginas(&manifest) - 1 {
        ejecutar_instalacion(hwnd);
        return;
    }

    mutar(|e| e.pagina += 1);
    dibujar_pagina(hwnd);
}

/// Corre la instalacion y deja el resultado en la pagina final.
unsafe fn ejecutar_instalacion(hwnd: HWND) {
    let (manifest, destino, valores, demo, zip) = con_estado(|e| {
        (e.manifest.clone(), e.destino.clone(), e.valores.clone(), e.demo, e.zip.clone())
    });

    let resultado = if demo {
        Ok(())
    } else {
        crate::setup::ejecutar(&manifest, &destino, &valores, &zip)
    };

    let (texto, fallo) = match resultado {
        Ok(()) if demo => (
            format!(
                "MODO DEMO: no se instalo nada.\n\n\
                 Esto es lo que se habria hecho:\n\n\
                 Carpeta:  {destino}\n\
                 Servicio: {}\n\n\
                 Valores recogidos:\n{}",
                manifest.service_display,
                valores.iter()
                    .map(|(k, v)| format!("   {k} = {}", if v.is_empty() { "(vacio)" } else { v }))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            false,
        ),
        Ok(()) => (
            format!(
                "El servicio \"{}\" esta corriendo y arrancara solo cada vez\n\
                 que se encienda el equipo.\n\n\
                 Carpeta: {destino}",
                manifest.service_display,
            ),
            false,
        ),
        Err(e) => (
            format!(
                "{e}\n\n\
                 Los archivos quedaron copiados en:\n{destino}\n\n\
                 Corrija el problema y vuelva a ejecutar este instalador.",
            ),
            true,
        ),
    };

    mutar(|e| {
        e.terminado = true;
        e.resultado = texto;
        e.fallo = fallo;
    });
    dibujar_pagina(hwnd);
}
