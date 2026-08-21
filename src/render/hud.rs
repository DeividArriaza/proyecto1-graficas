//! Sobreimpresos que van encima de la vista del juego: por ahora el contador de
//! FPS, más adelante el minimapa.
//!
//! Todo lo de aquí se dibuja *después* de la vista, para quedar por encima.

use crate::flashlight::Flashlight;
use crate::framebuffer::Framebuffer;
use crate::render::text;

/// Margen entre el borde de la pantalla y el sobreimpreso.
const MARGIN: usize = 12;

/// Relleno entre el texto y el borde de su recuadro de fondo.
const PADDING: usize = 6;

/// Escala de la fuente del contador.
const SCALE: usize = 3;

const TEXT_COLOR: u32 = 0xFFEE55;
const PANEL_COLOR: u32 = 0x111111;

/// Dibuja el contador de FPS en la esquina superior izquierda.
///
/// Lleva un recuadro oscuro de fondo porque el texto queda encima de la vista
/// 3D, y sin él el amarillo se pierde sobre el cielo o sobre una pared clara.
pub fn draw_fps(framebuffer: &mut Framebuffer, fps: f32) {
    // se recorta el valor para que el texto no cambie de ancho de golpe si el
    // promedio se dispara en el primer cuadro.
    let label = format!("FPS {:.0}", fps.clamp(0.0, 999.0));

    let width = text::text_width(&label, SCALE);
    let height = text::text_height(SCALE);

    framebuffer.rect(
        MARGIN,
        MARGIN,
        width + PADDING * 2,
        height + PADDING * 2,
        PANEL_COLOR,
    );

    text::draw_text(
        framebuffer,
        MARGIN + PADDING,
        MARGIN + PADDING,
        SCALE,
        TEXT_COLOR,
        &label,
    );
}

/// Ancho de la barra de batería en píxeles.
const BATTERY_WIDTH: usize = 180;

/// Alto de la barra de batería en píxeles.
const BATTERY_HEIGHT: usize = 14;

/// Colores de la carga según qué tan vacía está.
const BATTERY_FULL: u32 = 0x66DD66;
const BATTERY_LOW: u32 = 0xDDAA33;
const BATTERY_CRITICAL: u32 = 0xDD4444;

/// Color del hueco de la barra, lo que ya se gastó.
const BATTERY_EMPTY: u32 = 0x2A2A2A;

/// Dibuja el estado de la linterna debajo del contador de FPS.
///
/// La barra cambia de color al bajar la carga porque un jugador mirando al
/// frente no está leyendo un número: el cambio de verde a rojo se nota por el
/// rabillo del ojo, la cifra no.
pub fn draw_flashlight(framebuffer: &mut Framebuffer, flashlight: &Flashlight) {
    let label = if flashlight.on { "LINTERNA ON" } else { "LINTERNA OFF" };

    // debajo del recuadro del contador de FPS.
    let y = MARGIN + text::text_height(SCALE) + PADDING * 2 + MARGIN;

    let label_scale = SCALE - 1;
    let label_width = text::text_width(label, label_scale);
    let panel_width = label_width.max(BATTERY_WIDTH) + PADDING * 2;
    let panel_height = text::text_height(label_scale) + BATTERY_HEIGHT + PADDING * 3;

    framebuffer.rect(MARGIN, y, panel_width, panel_height, PANEL_COLOR);

    let text_color = if flashlight.on { TEXT_COLOR } else { 0x777777 };
    text::draw_text(framebuffer, MARGIN + PADDING, y + PADDING, label_scale, text_color, label);

    // el hueco completo primero, y encima la parte cargada.
    let bar_y = y + PADDING * 2 + text::text_height(label_scale);
    framebuffer.rect(MARGIN + PADDING, bar_y, BATTERY_WIDTH, BATTERY_HEIGHT, BATTERY_EMPTY);

    let charge_color = if flashlight.battery > 0.5 {
        BATTERY_FULL
    } else if flashlight.battery > 0.2 {
        BATTERY_LOW
    } else {
        BATTERY_CRITICAL
    };

    let charge_width = (BATTERY_WIDTH as f32 * flashlight.battery.clamp(0.0, 1.0)) as usize;
    framebuffer.rect(MARGIN + PADDING, bar_y, charge_width, BATTERY_HEIGHT, charge_color);
}
