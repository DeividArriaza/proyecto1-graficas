mod caster;
mod framebuffer;
mod maze;
mod player;
mod render;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::time::{Duration, Instant};

use crate::framebuffer::Framebuffer;
use crate::maze::{load_maze, BLOCK_SIZE};
use crate::player::process_events;

/// Duración objetivo de un cuadro: 16 ms ~ 60 cuadros por segundo.
const FRAME_TIME: Duration = Duration::from_millis(16);

/// Qué vista se está dibujando.
enum View {
    TwoD,
    ThreeD,
}

fn main() {
    let window_width = 1300;
    let window_height = 900;
    let framebuffer_width = 1300;
    let framebuffer_height = 900;

    let (maze, mut player) = load_maze("./maze.txt", BLOCK_SIZE);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    framebuffer.set_background_color(render::world::sky_color());

    let mut view = View::ThreeD;

    let mut window = Window::new(
        "Maze Runner",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        process_events(&window, &mut player, &maze, BLOCK_SIZE);

        // `M` alterna entre la vista cenital y la vista en primera persona.
        if window.is_key_pressed(Key::M, KeyRepeat::No) {
            view = match view {
                View::TwoD => View::ThreeD,
                View::ThreeD => View::TwoD,
            };
        }

        // ¿el jugador llegó a la meta? Se traduce su posición en píxeles a la
        // celda que ocupa y se revisa si esa celda es la marca `g`.
        let i = player.pos.x as usize / BLOCK_SIZE;
        let j = player.pos.y as usize / BLOCK_SIZE;
        if maze.get(j).and_then(|row| row.get(i)) == Some(&'g') {
            println!("¡Meta alcanzada! Fin del juego.");
            break;
        }

        match view {
            // La vista 2D deja huecos, así que sí necesita limpiar antes.
            View::TwoD => {
                framebuffer.clear();
                render::topdown::render(&mut framebuffer, &maze, &player);
            }
            // La vista 3D pinta cielo y piso sobre la pantalla completa: el
            // `clear()` previo sería 1.17 millones de píxeles sobrescritos.
            View::ThreeD => render::world::render(&mut framebuffer, &maze, &player),
        }

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        // Se duerme solo lo que falte para completar el cuadro, no un tiempo
        // fijo encima del render. Con un `sleep` fijo de 16 ms, un render de
        // 10 ms daba 26 ms por cuadro (38 fps) en vez de los 60 esperados.
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - elapsed);
        }
    }
}
