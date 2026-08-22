//! Dibujo de sprites como *billboard*: imágenes planas que siempre encaran al
//! jugador, escaladas según su distancia.
//!
//! # Del mundo a la pantalla
//!
//! El sprite está en coordenadas del laberinto y hay que llevarlo a coordenadas
//! de cámara. Se toma su posición relativa al jugador y se rota por el ángulo de
//! vista, lo que da dos números útiles:
//!
//! - `forward`: cuánto hay hacia adelante. Es la distancia perpendicular al
//!   plano de proyección, o sea la misma magnitud que guarda el búfer de
//!   profundidad de las paredes, así que se pueden comparar directo.
//! - `side`: cuánto hay hacia el costado. Dividido por `forward` da la tangente
//!   del ángulo respecto al centro de la vista, que es exactamente lo que la
//!   proyección de las paredes convierte en columna de pantalla.
//!
//! # Oclusión
//!
//! Cada columna del sprite se compara contra la distancia de la pared en esa
//! columna. Si la pared está más cerca, la columna no se dibuja. Eso es lo que
//! hace que el monstruo desaparezca detrás de una esquina en vez de flotar
//! encima del laberinto.

use crate::flashlight::Flashlight;
use crate::framebuffer::Framebuffer;
use crate::monster::Monster;
use crate::player::Player;
use crate::render::lighting;
use crate::render::{EYE_HEIGHT, FOV};
use crate::sprites::SpriteSheet;
use crate::textures;

/// Alto del sprite en unidades del mundo.
///
/// Algo menos que el bloque de 100 px, para que no toque el techo y se lea como
/// una criatura de pie en un pasillo.
const SPRITE_WORLD_HEIGHT: f32 = 88.0;

/// Fila de la hoja que se usa como ciclo de animación.
///
/// Las hojas de estos bancos traen varias filas con poses distintas —caminar,
/// atacar, morir—. Aquí sólo se necesita una; cambiar este número cambia la
/// animación sin tocar nada más.
const ANIMATION_ROW: usize = 0;

/// Distancia mínima para dibujar, en píxeles.
///
/// Más cerca que esto el sprite se agranda sin límite y su proyección deja de
/// tener sentido. Además evita dividir por algo cercano a cero.
const MIN_DISTANCE: f32 = 20.0;

pub fn draw(
    framebuffer: &mut Framebuffer,
    depth: &[f32],
    player: &Player,
    monster: &Monster,
    sheet: &SpriteSheet,
    flashlight: &Flashlight,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let half_height = height as f32 / 2.0;

    let distance_to_projection_plane = (width as f32 / 2.0) / (FOV / 2.0).tan();
    let half_plane = (FOV / 2.0).tan();

    // posición del sprite relativa al jugador, rotada al espacio de cámara.
    let relative = monster.pos - player.pos;
    let (sin_a, cos_a) = player.a.sin_cos();

    let forward = relative.x * cos_a + relative.y * sin_a;
    let side = -relative.x * sin_a + relative.y * cos_a;

    // detrás del jugador, o prácticamente encima: no se dibuja.
    if forward < MIN_DISTANCE {
        return;
    }

    // Columna central. `side / forward` es la tangente del ángulo respecto al
    // centro de la vista; dividirla por `half_plane` la lleva al rango [-1, 1]
    // que usan las columnas de pantalla.
    let screen_x = (side / forward) / half_plane;
    let center = (screen_x + 1.0) * 0.5 * (width - 1) as f32;

    let sprite_height = (SPRITE_WORLD_HEIGHT / forward) * distance_to_projection_plane;
    let sprite_width = sprite_height * sheet.aspect();

    // El sprite se apoya en el piso, no se centra en el horizonte. La fila del
    // piso a esta distancia sale de la misma relación que usa el degradado del
    // suelo, invertida:
    //     y = horizonte + altura_del_ojo / distancia * dpp
    let floor = half_height + (EYE_HEIGHT / forward) * distance_to_projection_plane;
    let top = floor - sprite_height;

    let first_column = (center - sprite_width / 2.0).floor().max(0.0) as usize;
    let last_column = (center + sprite_width / 2.0).ceil().min(width as f32) as usize;

    let ambient = lighting::ambient(forward);
    let frame = monster.frame();

    // El rango vertical y el borde izquierdo no dependen de la columna, así que
    // se calculan una vez y no una por cada una de las cientos de columnas.
    let left_edge = center - sprite_width / 2.0;
    let first_row = top.floor().max(0.0) as usize;
    let last_row = floor.ceil().min(height as f32) as usize;

    for x in first_column..last_column {
        // Oclusión: si la pared de esta columna está más cerca, el sprite queda
        // detrás y esta franja no se ve.
        if depth.get(x).is_some_and(|&wall| wall < forward) {
            continue;
        }

        // posición horizontal dentro del cuadro, de 0.0 a 1.0.
        let u = (x as f32 - left_edge) / sprite_width;

        // el haz se evalúa en la columna real, así que el monstruo se ilumina
        // sólo cuando la linterna lo alcanza.
        let column_screen_x = 2.0 * x as f32 / (width - 1) as f32 - 1.0;
        let beam = lighting::beam(forward, column_screen_x) * flashlight.intensity();

        for y in first_row..last_row {
            let v = (y as f32 - top) / sprite_height;

            let texel = sheet.sample(frame, ANIMATION_ROW, u, v);

            // Sin composición alfa: el píxel se dibuja o no se dibuja. Es lo que
            // corresponde acá, porque el fondo ya está pintado y mezclar
            // exigiría leerlo de vuelta.
            if !textures::is_visible(texel) {
                continue;
            }

            let screen_y = (y as f32 - half_height) / half_height;
            let light = ambient + beam * lighting::beam_vertical(screen_y);

            framebuffer.pixel(x, y, lighting::apply(texel, light));
        }
    }
}
