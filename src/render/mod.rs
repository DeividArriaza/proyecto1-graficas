//! Dibujo del mundo en el framebuffer.
//!
//! `main.rs` no dibuja: sólo decide qué vista corresponde y llama a uno de los
//! submódulos de aquí. Cada submódulo es una vista completa.

pub mod hud;
pub mod lighting;
pub mod minimap;
pub mod text;
pub mod world;

// `topdown.rs` sigue en el disco pero ya no forma parte del programa: era la
// vista cenital a escala 1:1 que servía de depuración antes del minimapa. Se
// deja como referencia; para revivirla basta con declararla aquí de nuevo.

use std::f32::consts::PI;

/// Amplitud del campo de visión (field of view), en radianes.
///
/// Vive aquí porque las dos vistas lo comparten: la 3D lo usa para repartir un
/// rayo por columna de pantalla, y la cenital para dibujar el abanico.
pub const FOV: f32 = PI / 2.0;

/// Color con el que se pinta cada tipo de celda del laberinto.
pub fn cell_color(cell: char) -> u32 {
    match cell {
        '+' => 0x00AAFF,       // columnas
        '-' => 0xFF5555,       // paredes horizontales
        '|' => 0xFF5555,       // paredes verticales
        'g' | 'G' => 0x00FF00, // meta
        _ => 0xFFDDDD,         // cualquier otra cosa
    }
}
