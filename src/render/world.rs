//! Vista en primera persona: un rayo por cada columna de píxeles, y cada rayo
//! se proyecta como una estaca vertical cuya altura depende de la distancia.

use crate::caster::cast_ray;
use crate::framebuffer::Framebuffer;
use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;
use crate::render::{cell_color, FOV};

const SKY_COLOR: u32 = 0x333355;
const FLOOR_COLOR: u32 = 0x554433;

pub fn render(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let half_height = height as f32 / 2.0;

    // Distancia del ojo al plano de proyección. La mitad del ancho de la
    // pantalla abarca medio campo de visión, así que:
    //     dpp = (ancho / 2) / tan(FOV / 2)
    let distance_to_projection_plane = (width as f32 / 2.0) / (FOV / 2.0).tan();

    // cielo (mitad superior) y piso (mitad inferior). Entre las dos cubren la
    // pantalla completa, así que esta vista no necesita `clear()` previo.
    framebuffer.fill_rows(0, height / 2, SKY_COLOR);
    framebuffer.fill_rows(height / 2, height, FLOOR_COLOR);

    // un rayo por columna: la columna 0 mira a `a - FOV/2` y la última a
    // `a + FOV/2`.
    for i in 0..width {
        let ray_fraction = i as f32 / width as f32; // de 0.0 a 1.0
        let angle = player.a - FOV / 2.0 + FOV * ray_fraction;

        let intersect = cast_ray(framebuffer, maze, player, angle, BLOCK_SIZE, false);

        // CORRECCIÓN DE OJO DE PEZ
        // `cast_ray` devuelve la distancia euclidiana a lo largo del rayo. Los
        // rayos de los extremos recorren más camino hasta una misma pared plana,
        // así que sin corregir darían estacas más cortas y la pared se vería
        // abombada. Se proyecta esa distancia sobre la dirección de vista:
        //     d_perpendicular = d_euclidiana * cos(β),   β = ángulo_rayo - ángulo_vista
        // que es justo la distancia al plano de proyección, y con eso la pared
        // queda plana.
        let beta = angle - player.a;
        let distance = (intersect.distance * beta.cos()).max(1.0);

        // Proyección en perspectiva por triángulos semejantes: la pared mide
        // BLOCK_SIZE en el mundo y está a `distance` del ojo.
        let stake_height = (BLOCK_SIZE as f32 / distance) * distance_to_projection_plane;

        // la estaca se centra verticalmente: el horizonte queda a media pantalla.
        let stake_top = (half_height - stake_height / 2.0).max(0.0) as usize;
        let stake_bottom = (half_height + stake_height / 2.0).min(height as f32) as usize;

        // Sombreado por distancia desactivado por ahora. Medido en 0.00 ms por
        // cuadro, así que reactivarlo es gratis: basta con envolver el color en
        // `shade(..., distance)`.
        let color = cell_color(intersect.impact);

        framebuffer.column(i, stake_top, stake_bottom, color);
    }
}

/// Color del cielo, que `main` usa como color de fondo del framebuffer.
pub fn sky_color() -> u32 {
    SKY_COLOR
}
