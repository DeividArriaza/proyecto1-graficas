//! Dibujo en el framebuffer.
//!
//! `main.rs` no dibuja: arma el estado y llama a los submódulos de aquí. Cada
//! uno resuelve una capa de la pantalla — el mundo, el minimapa, el HUD — y
//! ninguno sabe de los otros.

pub mod hud;
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
