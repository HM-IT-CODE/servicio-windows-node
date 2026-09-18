//! La paleta y los ayudantes de dibujo.
//!
//! Win32 no trae nada de esto: los botones y el fondo salen del gris del
//! sistema salvo que uno los pinte. Aqui esta todo lo que hace que el
//! asistente no parezca de 1998, y sigue sin costar ni una dependencia.

use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::*;

// --- Paleta ----------------------------------------------------------------
// COLORREF es 0x00BBGGRR: el azul va PRIMERO, al reves que en HTML.

/// Turquesa principal, el de los botones y el panel.
pub const TURQUESA: COLORREF = COLORREF(0x00A6B814);       // #14B8A6
/// Turquesa oscuro, para el degradado y el boton pulsado.
pub const TURQUESA_OSCURO: COLORREF = COLORREF(0x0088940D); // #0D9488
/// Mas oscuro todavia, el fondo del panel lateral abajo.
pub const TURQUESA_NOCHE: COLORREF = COLORREF(0x004A3F0F);  // #0F3F4A

pub const BLANCO: COLORREF = COLORREF(0x00FFFFFF);
/// Texto principal.
pub const TINTA: COLORREF = COLORREF(0x00241F1F);          // #1F1F24
/// Texto secundario.
pub const GRIS: COLORREF = COLORREF(0x00807672);           // #726F80
/// Borde de los cuadros de texto.
pub const BORDE: COLORREF = COLORREF(0x00E0DEDC);
/// Fondo de un boton secundario.
pub const HUMO: COLORREF = COLORREF(0x00F6F4F3);

// --- Medidas ---------------------------------------------------------------

/// Ancho del panel lateral de color.
pub const PANEL: i32 = 210;
/// Margen izquierdo del contenido.
pub const MARGEN: i32 = PANEL + 32;

/// Pinta un degradado vertical franja a franja.
///
/// Existe `GradientFill` en msimg32, pero obliga a enlazar otra biblioteca
/// para algo que son ocho lineas. Interpolar a mano no se nota y no ata nada.
pub unsafe fn degradado(dc: HDC, rect: RECT, arriba: COLORREF, abajo: COLORREF) {
    let alto = (rect.bottom - rect.top).max(1);

    let (r1, g1, b1) = componentes(arriba);
    let (r2, g2, b2) = componentes(abajo);

    for y in 0..alto {
        let t = y as f32 / alto as f32;

        let color = COLORREF(
            (mezcla(b1, b2, t) as u32) << 16
                | (mezcla(g1, g2, t) as u32) << 8
                | mezcla(r1, r2, t) as u32,
        );

        let franja = RECT {
            left: rect.left, top: rect.top + y,
            right: rect.right, bottom: rect.top + y + 1,
        };
        rellenar(dc, franja, color);
    }
}

fn componentes(color: COLORREF) -> (u8, u8, u8) {
    let v = color.0;
    ((v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8)
}

fn mezcla(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

/// Rellena un rectangulo de color plano.
pub unsafe fn rellenar(dc: HDC, rect: RECT, color: COLORREF) {
    let brocha = CreateSolidBrush(color);
    FillRect(dc, &rect, brocha);
    let _ = DeleteObject(brocha);
}

/// Dibuja un texto en la posicion dada, con la fuente que tenga el DC.
pub unsafe fn texto(dc: HDC, x: i32, y: i32, ancho: i32, contenido: &str, color: COLORREF, formato: DRAW_TEXT_FORMAT) {
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, color);

    let mut w: Vec<u16> = contenido.encode_utf16().collect();
    let mut rect = RECT { left: x, top: y, right: x + ancho, bottom: y + 400 };

    DrawTextW(dc, &mut w, &mut rect, formato);
}

/// Crea una fuente Segoe UI del tamano y peso pedidos.
pub unsafe fn fuente(alto: i32, peso: i32) -> HFONT {
    CreateFontW(
        alto, 0, 0, 0, peso, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        FONT_OUTPUT_PRECISION::default().0 as u32,
        FONT_CLIP_PRECISION::default().0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        (FF_DONTCARE.0 | VARIABLE_PITCH.0) as u32,
        windows::core::w!("Segoe UI"),
    )
}

/// Un circulo relleno, para los puntos de la lista de pasos.
pub unsafe fn circulo(dc: HDC, x: i32, y: i32, radio: i32, color: COLORREF) {
    let brocha = CreateSolidBrush(color);
    let lapiz = CreatePen(PS_SOLID, 1, color);

    let brocha_previa = SelectObject(dc, brocha);
    let lapiz_previo = SelectObject(dc, lapiz);

    let _ = Ellipse(dc, x - radio, y - radio, x + radio, y + radio);

    SelectObject(dc, brocha_previa);
    SelectObject(dc, lapiz_previo);
    let _ = DeleteObject(brocha);
    let _ = DeleteObject(lapiz);
}
