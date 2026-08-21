//! Pantalla de nivel superado.
//!
//! Está armada como la terminal de la instalación: verde de monitor sobre negro,
//! marco, líneas de barrido y barras de progreso. Reutiliza el mismo lenguaje
//! visual que la textura `terminal` que marca la salida, así que llegar a la
//! meta y ver el informe se sienten parte de lo mismo.
//!
//! No es sólo un "ganaste": muestra tiempo, exploración y batería restante. En
//! el modo infinito eso es lo único que le da algo que superar al jugador.

use minifb::{Key, KeyRepeat, Window};

use crate::framebuffer::Framebuffer;
use crate::game::VictoryReport;
use crate::render::text;

const BACKGROUND: u32 = 0x05070A;
const FRAME_COLOR: u32 = 0x2F7F4F;
const TITLE_COLOR: u32 = 0x7CFFB0;
const LABEL_COLOR: u32 = 0x4E7F63;
const VALUE_COLOR: u32 = 0xC8FFDA;
const HINT_COLOR: u32 = 0x5A7F68;

const BAR_BACKGROUND: u32 = 0x11201A;
const BAR_FILL: u32 = 0x40FF80;

/// Color de las líneas de barrido. Muy cerca del fondo: la idea es que se
/// insinúen, no que rayen la pantalla.
const SCANLINE_COLOR: u32 = 0x0A1712;

/// Cada cuántas filas va una línea de barrido.
const SCANLINE_EVERY: usize = 4;

const MARGIN: usize = 28;
const FRAME_THICKNESS: usize = 2;

const TITLE_SCALE: usize = 5;
const ROW_SCALE: usize = 3;
const HINT_SCALE: usize = 2;

/// Separación vertical entre renglones del informe.
const ROW_SPACING: usize = 52;

const BAR_WIDTH: usize = 240;
const BAR_HEIGHT: usize = 16;

/// Qué hacer al salir de esta pantalla.
pub enum Choice {
    /// Volver al menú de niveles.
    Menu,
    /// Jugar otra vez el mismo nivel.
    ///
    /// En los tres niveles fijos sale el mismo laberinto, porque la semilla es
    /// fija. En el modo infinito sale uno nuevo, así que se puede jugar en
    /// cadena sin pasar por el menú.
    Retry,
}

/// Lee el teclado. Devuelve `None` mientras no se elija nada.
pub fn update(window: &Window) -> Option<Choice> {
    if window.is_key_pressed(Key::Enter, KeyRepeat::No)
        || window.is_key_pressed(Key::Space, KeyRepeat::No)
        || window.is_key_pressed(Key::Escape, KeyRepeat::No)
    {
        return Some(Choice::Menu);
    }

    if window.is_key_pressed(Key::R, KeyRepeat::No) {
        return Some(Choice::Retry);
    }

    None
}

pub fn draw(framebuffer: &mut Framebuffer, report: &VictoryReport) {
    framebuffer.set_background_color(BACKGROUND);
    framebuffer.clear();

    let width = framebuffer.width;
    let height = framebuffer.height;

    draw_scanlines(framebuffer);
    draw_frame(framebuffer);

    text::draw_text_centered(
        framebuffer,
        height / 6,
        TITLE_SCALE,
        TITLE_COLOR,
        "SALIDA ALCANZADA",
    );

    // Dos columnas: etiquetas alineadas a la izquierda, valores en una segunda
    // columna fija. Alinear los valores es lo que hace que el bloque se lea como
    // una tabla y no como cuatro frases sueltas.
    //
    // Las restas son saturantes porque son `usize`: en una ventana angosta
    // `width / 2 - 380` daría un desbordamiento y un pánico en vez de un cero.
    let label_x = (width / 2).saturating_sub(380);
    let value_x = (width / 2).saturating_sub(40);
    let bar_x = value_x + 130;

    let mut y = (height / 2).saturating_sub(ROW_SPACING);

    row(framebuffer, label_x, value_x, y, "NIVEL", report.level_name);

    y += ROW_SPACING;
    row(framebuffer, label_x, value_x, y, "TIEMPO", &format_time(report.seconds));

    y += ROW_SPACING;
    let explored = format!("{:>3}%", (report.explored * 100.0).round() as u32);
    row(framebuffer, label_x, value_x, y, "EXPLORADO", &explored);
    bar(framebuffer, bar_x, y, report.explored);

    y += ROW_SPACING;
    let battery = format!("{:>3}%", (report.battery * 100.0).round() as u32);
    row(framebuffer, label_x, value_x, y, "BATERIA", &battery);
    bar(framebuffer, bar_x, y, report.battery);

    let hints = [
        "ENTER   VOLVER AL MENU",
        "R       JUGAR OTRA VEZ",
    ];

    for (i, hint) in hints.iter().enumerate() {
        let hint_y = height - height / 6 + i * (text::text_height(HINT_SCALE) + 14);

        text::draw_text_centered(framebuffer, hint_y, HINT_SCALE, HINT_COLOR, hint);
    }
}

/// Un renglón del informe: etiqueta apagada, valor brillante.
fn row(
    framebuffer: &mut Framebuffer,
    label_x: usize,
    value_x: usize,
    y: usize,
    label: &str,
    value: &str,
) {
    text::draw_text(framebuffer, label_x, y, ROW_SCALE, LABEL_COLOR, label);
    text::draw_text(framebuffer, value_x, y, ROW_SCALE, VALUE_COLOR, value);
}

/// Barra de progreso alineada con el centro del renglón de texto.
fn bar(framebuffer: &mut Framebuffer, x: usize, text_y: usize, fraction: f32) {
    // el texto mide más que la barra, así que la barra se centra contra él en
    // vez de apoyarse en su borde superior.
    let y = text_y + text::text_height(ROW_SCALE) / 2 - BAR_HEIGHT / 2;

    framebuffer.rect(x, y, BAR_WIDTH, BAR_HEIGHT, BAR_BACKGROUND);

    let filled = (BAR_WIDTH as f32 * fraction.clamp(0.0, 1.0)) as usize;

    framebuffer.rect(x, y, filled, BAR_HEIGHT, BAR_FILL);
}

/// Líneas horizontales tenues sobre toda la pantalla, como un monitor viejo.
fn draw_scanlines(framebuffer: &mut Framebuffer) {
    let height = framebuffer.height;

    for y in (0..height).step_by(SCANLINE_EVERY) {
        framebuffer.fill_rows(y, y + 1, SCANLINE_COLOR);
    }
}

/// Marco verde alrededor de la pantalla.
///
/// Se pinta un rectángulo del color del marco y otro más chico del color del
/// fondo encima. Dos `rect` en vez de cuatro tiras.
fn draw_frame(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width;
    let height = framebuffer.height;

    framebuffer.rect(
        MARGIN,
        MARGIN,
        width - MARGIN * 2,
        height - MARGIN * 2,
        FRAME_COLOR,
    );

    framebuffer.rect(
        MARGIN + FRAME_THICKNESS,
        MARGIN + FRAME_THICKNESS,
        width - (MARGIN + FRAME_THICKNESS) * 2,
        height - (MARGIN + FRAME_THICKNESS) * 2,
        BACKGROUND,
    );

    // el interior tapó las líneas de barrido, así que se repintan adentro.
    for y in (MARGIN + FRAME_THICKNESS..height - MARGIN - FRAME_THICKNESS).step_by(SCANLINE_EVERY) {
        framebuffer.rect(
            MARGIN + FRAME_THICKNESS,
            y,
            width - (MARGIN + FRAME_THICKNESS) * 2,
            1,
            SCANLINE_COLOR,
        );
    }
}

/// Segundos a `MM:SS`.
///
/// No se acota a 59 minutos: si alguien se pierde una hora en el laberinto, el
/// contador sigue subiendo en vez de dar la vuelta.
fn format_time(seconds: f32) -> String {
    let total = seconds.max(0.0) as u32;

    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_tiempo_se_formatea_como_minutos_y_segundos() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(9.4), "00:09");
        assert_eq!(format_time(84.0), "01:24");
        assert_eq!(format_time(599.9), "09:59");

        // más de una hora no da la vuelta: el contador sigue subiendo.
        assert_eq!(format_time(3661.0), "61:01");

        // un tiempo negativo no debería existir, pero no debe entrar en pánico.
        assert_eq!(format_time(-5.0), "00:00");
    }
}
