mod caster;
mod framebuffer;
mod maze;
mod player;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

use crate::caster::cast_ray;
use crate::framebuffer::Framebuffer;
use crate::maze::{load_maze, Maze};
use crate::player::{process_events, Player};

const BLOCK_SIZE: usize = 100;

/// Cantidad de rayos del abanico que se dibuja en la vista 2D.
const NUM_RAYS: usize = 5;

/// Amplitud del campo de visión (field of view), en radianes.
const FOV: f32 = PI / 3.0;

const SKY_COLOR: u32 = 0x333355;
const FLOOR_COLOR: u32 = 0x554433;

/// Duración objetivo de un cuadro: 16 ms ~ 60 cuadros por segundo.
const FRAME_TIME: Duration = Duration::from_millis(16);

/// Qué vista se está dibujando.
enum View {
    TwoD,
    ThreeD,
}

fn cell_color(cell: char) -> u32 {
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
/// Sin uso por ahora: se desactivó el sombreado en `render_world`.
#[allow(dead_code)]
fn shade(color: u32, distance: f32) -> u32 {
    let factor = (1.0 - distance / 900.0).clamp(0.25, 1.0);

    let r = ((color >> 16) & 0xFF) as f32 * factor;
    let g = ((color >> 8) & 0xFF) as f32 * factor;
    let b = (color & 0xFF) as f32 * factor;

    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

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

/// Vista cenital: el laberinto completo, el jugador y el abanico de rayos.
fn render_maze(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player) {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
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
        cast_ray(framebuffer, maze, player, angle, BLOCK_SIZE, true);
    }
}

/// Vista en primera persona: un rayo por cada columna de píxeles, y cada rayo
/// se proyecta como una estaca vertical cuya altura depende de la distancia.
fn render_world(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player) {
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

fn main() {
    let window_width = 1300;
    let window_height = 900;
    let framebuffer_width = 1300;
    let framebuffer_height = 900;

    let (maze, mut player) = load_maze("./maze.txt", BLOCK_SIZE);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    framebuffer.set_background_color(SKY_COLOR);

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
                render_maze(&mut framebuffer, &maze, &player);
            }
            // La vista 3D pinta cielo y piso sobre la pantalla completa: el
            // `clear()` previo sería 1.17 millones de píxeles sobrescritos.
            View::ThreeD => render_world(&mut framebuffer, &maze, &player),
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
