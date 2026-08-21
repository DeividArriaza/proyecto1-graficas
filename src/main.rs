mod caster;
mod discovery;
mod flashlight;
mod fps;
mod framebuffer;
mod maze;
mod player;
mod render;
mod textures;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::time::{Duration, Instant};

use crate::discovery::Discovered;
use crate::flashlight::Flashlight;
use crate::fps::FpsCounter;
use crate::framebuffer::Framebuffer;
use crate::maze::{load_maze, BLOCK_SIZE};
use crate::player::process_events;
use crate::render::lighting;
use crate::textures::TextureSet;

/// Duración objetivo de un cuadro: 16 ms ~ 60 cuadros por segundo.
const FRAME_TIME: Duration = Duration::from_millis(16);

fn main() {
    let window_width = 1300;
    let window_height = 900;
    let framebuffer_width = 1300;
    let framebuffer_height = 900;

    let (maze, mut player) = load_maze("./maze.txt");

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    framebuffer.set_background_color(render::world::ceiling_color());

    let mut fps_counter = FpsCounter::new();
    let mut flashlight = Flashlight::new();
    let mut discovered = Discovered::new(&maze);
    let textures = TextureSet::load();

    let mut window = Window::new(
        "Maze Runner",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();
        let fps = fps_counter.tick();

        process_events(&window, &mut player, &maze);

        // `M` prende y apaga la linterna. El desgaste corre aparte: gasta
        // mientras está encendida y recarga, más despacio, mientras no.
        if window.is_key_pressed(Key::M, KeyRepeat::No) {
            flashlight.toggle();
        }

        flashlight.update(fps_counter.delta_seconds());

        // La celda propia se recuerda siempre; el resto sólo si hay luz que lo
        // alcance. El alcance se escala con la intensidad, así que la batería
        // baja descubre menos.
        discovered.mark_player(&player);

        if flashlight.on {
            discovered.reveal_from(
                &maze,
                &player,
                lighting::beam_reach() * flashlight.intensity(),
            );
        }

        // ¿el jugador llegó a la meta? Se traduce su posición en píxeles a la
        // celda que ocupa y se revisa si esa celda es la marca de meta.
        let i = player.pos.x as usize / BLOCK_SIZE;
        let j = player.pos.y as usize / BLOCK_SIZE;
        if matches!(maze.get(j).and_then(|row| row.get(i)), Some(&('g' | 'G'))) {
            println!("¡Meta alcanzada! Fin del juego.");
            break;
        }

        // La vista pinta techo y piso sobre la pantalla completa, así que no
        // necesita `clear()` previo: serían 1.17 millones de píxeles
        // sobrescritos para nada.
        render::world::render(&mut framebuffer, &maze, &player, &flashlight, &textures);

        // Los sobreimpresos van al final, para quedar encima de la vista.
        render::minimap::draw(
            &mut framebuffer,
            &maze,
            &player,
            &discovered,
            &flashlight,
            &textures,
        );
        render::hud::draw_fps(&mut framebuffer, fps);
        render::hud::draw_flashlight(&mut framebuffer, &flashlight);

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
