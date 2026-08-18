//! Dibujo del mundo en el framebuffer.
//!
//! `main.rs` no dibuja: sólo decide qué vista corresponde y llama a uno de los
//! submódulos de aquí. Cada submódulo es una vista completa.

pub mod topdown;
pub mod world;

use std::f32::consts::PI;

/// Amplitud del campo de visión (field of view), en radianes.
///
/// Vive aquí porque las dos vistas lo comparten: la 3D lo usa para repartir un
/// rayo por columna de pantalla, y la cenital para dibujar el abanico.
pub const FOV: f32 = PI / 2;

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

/// Oscurece un color según la distancia, para dar sensación de profundidad.
/// El factor va de 1.0 (pegado al jugador) hacia 0.0 (lejano).
///
/// Sin uso por ahora: se desactivó el sombreado en `world::render`.
#[allow(dead_code)]
pub fn shade(color: u32, distance: f32) -> u32 {
    let factor = (1.0 - distance / 900.0).clamp(0.25, 1.0);

    let r = ((color >> 16) & 0xFF) as f32 * factor;
    let g = ((color >> 8) & 0xFF) as f32 * factor;
    let b = (color & 0xFF) as f32 * factor;

    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
