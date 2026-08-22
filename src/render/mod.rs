//! Dibujo en el framebuffer.
//!
//! `main.rs` no dibuja: arma el estado y llama a los submódulos de aquí. Cada
//! uno resuelve una capa de la pantalla — el mundo, el minimapa, el HUD — y
//! ninguno sabe de los otros.

pub mod hud;
pub mod billboard;
pub mod lighting;
pub mod minimap;
pub mod text;
pub mod world;

use std::f32::consts::PI;

/// Amplitud del campo de visión (field of view), en radianes.
///
/// Vive aquí y no en `world` porque también lo usa el pase de descubrimiento,
/// que necesita saber la apertura del haz de la linterna, y esa apertura está
/// definida en columnas de pantalla — o sea, depende del FOV.
///
/// Bajarlo cierra el encuadre y da claustrofobia; subirlo muestra más laberinto
/// pero estira las paredes en los bordes.
pub const FOV: f32 = PI / 2.0;

/// Altura del ojo del jugador dentro del bloque, en píxeles.
///
/// Media celda: el jugador ve el mundo desde el centro vertical de la pared, y
/// por eso el horizonte cae a media pantalla. Lo comparten la vista en primera
/// persona y la proyección de sprites, que necesita saber dónde está el piso
/// para apoyarlos.
pub const EYE_HEIGHT: f32 = crate::maze::BLOCK_SIZE as f32 / 2.0;
