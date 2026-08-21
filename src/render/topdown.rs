//! Vista cenital: el laberinto completo, el jugador y el abanico de rayos.
//!
//! Es la vista de depuración, a escala 1:1 con el mundo. El minimapa de la
//! esquina será otra cosa: la misma idea pero escalada y superpuesta a la
//! vista en primera persona.

use crate::caster::cast_ray;
use crate::discovery::Discovered;
use crate::framebuffer::Framebuffer;
use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;
use crate::render::{cell_color, FOV};

/// Cantidad de rayos del abanico que se dibuja en esta vista.
const NUM_RAYS: usize = 5;

fn draw_cell(framebuffer: &mut Framebuffer, xo: usize, yo: usize, cell: char) {
    if cell == ' ' {
        return;
    }

    framebuffer.set_current_color(cell_color(cell));

    for x in xo..xo + BLOCK_SIZE {
        for y in yo..yo + BLOCK_SIZE {
            framebuffer.point(x, y);
        }
    }
}

pub fn render(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    discovered: &Discovered,
) {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            // niebla de guerra: lo que todavía no se ha visto se deja en el
            // color de fondo, indistinguible de un pasillo sin explorar.
            if !discovered.is_known(col, row) {
                continue;
            }

            draw_cell(framebuffer, col * BLOCK_SIZE, row * BLOCK_SIZE, cell);
        }
    }

    framebuffer.set_current_color(0xFFFF00);

    let px = player.pos.x as usize;
    let py = player.pos.y as usize;

    for x in px.saturating_sub(3)..=px + 3 {
        for y in py.saturating_sub(3)..=py + 3 {
            framebuffer.point(x, y);
        }
    }

    // lanza un abanico de rayos centrado en la dirección de vista del jugador.
    // El campo de visión (FOV) se reparte de forma pareja entre los NUM_RAYS
    // rayos: el primero apunta a `a - FOV/2`, el último a `a + FOV/2` y el del
    // medio coincide con la dirección de vista.
    for i in 0..NUM_RAYS {
        let ray_fraction = i as f32 / (NUM_RAYS - 1) as f32; // de 0.0 a 1.0
        let angle = player.a - FOV / 2.0 + FOV * ray_fraction;
        cast_ray(framebuffer, maze, player, angle, true);
    }
}
