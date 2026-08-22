//! Pantalla de fin de nivel: sirve para haber escapado y para haber sido
//! atrapado.
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
use crate::game::{LevelReport, Outcome};
use crate::render::text;

const BACKGROUND: u32 = 0x05070A;
/// Paleta de cada desenlace.
///
/// La estructura de la pantalla es idéntica en los dos casos; lo único que
/// cambia es el color y el título. Agruparlo así evita repartir `if` por todo el
/// código de dibujo.
struct Palette {
    title: &'static str,
    frame: u32,
    title_color: u32,
    label: u32,
    value: u32,
    hint: u32,
    bar_background: u32,
    bar_fill: u32,
    scanline: u32,
}

/// Verde de terminal: escapaste.
const ESCAPED: Palette = Palette {
    title: "SALIDA ALCANZADA",
    frame: 0x2F7F4F,
    title_color: 0x7CFFB0,
    label: 0x4E7F63,
    value: 0xC8FFDA,
    hint: 0x5A7F68,
    bar_background: 0x11201A,
    bar_fill: 0x40FF80,
    scanline: 0x0A1712,
};

/// Rojo de alarma: te atrapó.
const CAUGHT: Palette = Palette {
    title: "TE ATRAPARON",
    frame: 0x7F2F2F,
    title_color: 0xFF9090,
    label: 0x7F5050,
    value: 0xFFD0D0,
    hint: 0x7F5A5A,
    bar_background: 0x201111,
    bar_fill: 0xFF5050,
    scanline: 0x170A0A,
};

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

pub fn draw(framebuffer: &mut Framebuffer, report: &LevelReport) {
    let palette = match report.outcome {
        Outcome::Escaped => &ESCAPED,
        Outcome::Caught => &CAUGHT,
    };

    framebuffer.set_background_color(BACKGROUND);
    framebuffer.clear();

    let width = framebuffer.width;
    let height = framebuffer.height;

    draw_scanlines(framebuffer, palette);
    draw_frame(framebuffer, palette);

    text::draw_text_centered(
        framebuffer,
        height / 6,
        TITLE_SCALE,
        palette.title_color,
        palette.title,
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

    row(framebuffer, palette, label_x, value_x, y, "NIVEL", report.level_name);

    y += ROW_SPACING;
    row(
        framebuffer,
        palette,
        label_x,
        value_x,
        y,
        "TIEMPO",
        &format_time(report.seconds),
    );

    y += ROW_SPACING;
    let explored = format!("{:>3}%", (report.explored * 100.0).round() as u32);
    row(framebuffer, palette, label_x, value_x, y, "EXPLORADO", &explored);
    bar(framebuffer, palette, bar_x, y, report.explored);

    y += ROW_SPACING;
    let battery = format!("{:>3}%", (report.battery * 100.0).round() as u32);
    row(framebuffer, palette, label_x, value_x, y, "BATERIA", &battery);
    bar(framebuffer, palette, bar_x, y, report.battery);

    let hints = [
        "ENTER   VOLVER AL MENU",
        "R       JUGAR OTRA VEZ",
    ];

    for (i, hint) in hints.iter().enumerate() {
        let hint_y = height - height / 6 + i * (text::text_height(HINT_SCALE) + 14);

        text::draw_text_centered(framebuffer, hint_y, HINT_SCALE, palette.hint, hint);
    }
}

/// Un renglón del informe: etiqueta apagada, valor brillante.
fn row(
    framebuffer: &mut Framebuffer,
    palette: &Palette,
    label_x: usize,
    value_x: usize,
    y: usize,
    label: &str,
    value: &str,
) {
    text::draw_text(framebuffer, label_x, y, ROW_SCALE, palette.label, label);
    text::draw_text(framebuffer, value_x, y, ROW_SCALE, palette.value, value);
}

/// Barra de progreso alineada con el centro del renglón de texto.
fn bar(framebuffer: &mut Framebuffer, palette: &Palette, x: usize, text_y: usize, fraction: f32) {
    // el texto mide más que la barra, así que la barra se centra contra él en
    // vez de apoyarse en su borde superior.
    let y = text_y + text::text_height(ROW_SCALE) / 2 - BAR_HEIGHT / 2;

    framebuffer.rect(x, y, BAR_WIDTH, BAR_HEIGHT, palette.bar_background);

    let filled = (BAR_WIDTH as f32 * fraction.clamp(0.0, 1.0)) as usize;

    framebuffer.rect(x, y, filled, BAR_HEIGHT, palette.bar_fill);
}

/// Líneas horizontales tenues sobre toda la pantalla, como un monitor viejo.
fn draw_scanlines(framebuffer: &mut Framebuffer, palette: &Palette) {
    let height = framebuffer.height;

    for y in (0..height).step_by(SCANLINE_EVERY) {
        framebuffer.fill_rows(y, y + 1, palette.scanline);
    }
}

/// Marco alrededor de la pantalla.
///
/// Se pinta un rectángulo del color del marco y otro más chico del color del
/// fondo encima. Dos `rect` en vez de cuatro tiras.
fn draw_frame(framebuffer: &mut Framebuffer, palette: &Palette) {
    let width = framebuffer.width;
    let height = framebuffer.height;

    framebuffer.rect(
        MARGIN,
        MARGIN,
        width - MARGIN * 2,
        height - MARGIN * 2,
        palette.frame,
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
            palette.scanline,
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
