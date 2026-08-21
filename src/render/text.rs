//! Fuente de mapa de bits para dibujar texto en el framebuffer.
//!
//! `minifb` sólo entrega un buffer de píxeles: no hay texto, ni fuentes, ni
//! nada parecido. Así que la fuente se define a mano. Cada glifo es una
//! rejilla de 5 x 7 píxeles guardada como 7 bytes, uno por fila, donde los 5
//! bits bajos dicen qué píxeles se prenden (bit 4 = píxel más a la izquierda).
//!
//! Este módulo es la base del contador de FPS, y luego de las pantallas de
//! bienvenida y de victoria.

use crate::framebuffer::Framebuffer;

/// Ancho de un glifo en píxeles de la rejilla.
pub const GLYPH_WIDTH: usize = 5;

/// Alto de un glifo en píxeles de la rejilla.
pub const GLYPH_HEIGHT: usize = 7;

/// Separación horizontal entre glifos, en píxeles de la rejilla.
const GLYPH_SPACING: usize = 1;

/// Glifo que se usa para cualquier carácter que la fuente no conozca: una caja
/// hueca. Se ve raro a propósito, para que el faltante salte a la vista.
const UNKNOWN: [u8; GLYPH_HEIGHT] = [
    0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
];

/// Mapa de bits de un carácter. Las minúsculas se dibujan con el glifo de la
/// mayúscula: la fuente no tiene caja baja.
fn glyph(c: char) -> [u8; GLYPH_HEIGHT] {
    match c.to_ascii_uppercase() {
        ' ' => [0, 0, 0, 0, 0, 0, 0],

        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b10001, 0b00001, 0b00010, 0b00100, 0b00100, 0b00100],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],

        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b01110, 0b00001],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],

        // La `Ñ` se guarda aparte porque no es ASCII: `to_ascii_uppercase` la
        // deja igual, así que el patrón tiene que coincidir con las dos cajas.
        'ñ' | 'Ñ' => [0b01010, 0b00000, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001],

        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '<' => [0b00001, 0b00010, 0b00100, 0b01000, 0b00100, 0b00010, 0b00001],
        '>' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],

        _ => UNKNOWN,
    }
}

/// Cuánto avanza el cursor de un carácter al siguiente, en píxeles de pantalla.
fn advance(scale: usize) -> usize {
    (GLYPH_WIDTH + GLYPH_SPACING) * scale
}

/// Ancho total que va a ocupar `text` en pantalla, para poder centrarlo.
///
/// No incluye la separación que sigue al último glifo.
pub fn text_width(text: &str, scale: usize) -> usize {
    let count = text.chars().count();

    if count == 0 {
        return 0;
    }

    count * advance(scale) - GLYPH_SPACING * scale
}

/// Alto que va a ocupar cualquier texto en pantalla.
pub fn text_height(scale: usize) -> usize {
    GLYPH_HEIGHT * scale
}

/// Dibuja un glifo con la esquina superior izquierda en (x, y).
///
/// `scale` multiplica el tamaño: cada píxel de la rejilla se vuelve un cuadrado
/// de `scale` x `scale` píxeles de pantalla. A 5 x 7 la fuente es diminuta en
/// una ventana de 1300 x 900, así que el HUD usa escala 2 o 3.
fn draw_char(framebuffer: &mut Framebuffer, x: usize, y: usize, scale: usize, color: u32, c: char) {
    let rows = glyph(c);

    for (row, bits) in rows.iter().enumerate() {
        for col in 0..GLYPH_WIDTH {
            // el bit más significativo de los 5 es el píxel de la izquierda.
            let on = bits & (1 << (GLYPH_WIDTH - 1 - col)) != 0;

            if on {
                framebuffer.rect(
                    x + col * scale,
                    y + row * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

/// Dibuja `text` con la esquina superior izquierda en (x, y).
pub fn draw_text(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    scale: usize,
    color: u32,
    text: &str,
) {
    for (i, c) in text.chars().enumerate() {
        draw_char(framebuffer, x + i * advance(scale), y, scale, color, c);
    }
}

/// Dibuja `text` centrado horizontalmente en el framebuffer, a la altura `y`.
///
/// Sin uso todavía: lo van a usar las pantallas de bienvenida y de victoria.
#[allow(dead_code)]
pub fn draw_text_centered(
    framebuffer: &mut Framebuffer,
    y: usize,
    scale: usize,
    color: u32,
    text: &str,
) {
    let width = text_width(text, scale);
    let x = framebuffer.width.saturating_sub(width) / 2;

    draw_text(framebuffer, x, y, scale, color, text);
}
