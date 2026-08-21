use minifb::{Key, Window};
use nalgebra_glm::Vec2;
use std::f32::consts::{PI, TAU};

use crate::maze::{Maze, BLOCK_SIZE};

// ---------------------------------------------------------------------------
// Velocidades del jugador. Ambas se aplican una vez por cuadro, así que su
// efecto real depende del frame rate (~60 cuadros por segundo con el
// `frame_delay` de 16 ms que usa el ciclo de render).
// ---------------------------------------------------------------------------

/// Píxeles que avanza o retrocede el jugador por cuadro.
/// A 60 fps: 5 px/cuadro = 300 px/s = 3 bloques por segundo.
pub const MOVE_SPEED: f32 = 5.0;

/// Radianes que gira el jugador por cuadro.
/// A 60 fps: PI/60 = 3°/cuadro = 180° por segundo.
/// Subirlo hace el giro más brusco; bajarlo, más suave.
pub const ROTATION_SPEED: f32 = PI / 60.0;

/// Radio del jugador en píxeles: qué tan cerca de una pared puede quedar.
///
/// Sin este margen el jugador se pega a la pared y la estaca de esa columna
/// crece hasta tapar toda la pantalla. Con 20 px sobre bloques de 100 px, la
/// pared más cercana posible queda a 1/5 de bloque.
pub const COLLISION_MARGIN: f32 = 20.0;

pub struct Player {
    /// Posición en píxeles dentro del laberinto.
    pub pos: Vec2,
    /// Ángulo de vista en radianes.
    pub a: f32,
}

/// ¿La celda que contiene el punto (x, y) bloquea el paso?
///
/// La meta `g` es transitable a propósito: `cast_ray` sí la trata como pared
/// (por eso se ve como un muro verde al frente), pero el jugador tiene que
/// poder entrar en ella para que se dispare la condición de victoria.
fn is_wall(maze: &Maze, x: f32, y: f32) -> bool {
    // fuera del laberinto por el lado negativo: se trata como pared.
    if x < 0.0 || y < 0.0 {
        return true;
    }

    let i = x as usize / BLOCK_SIZE;
    let j = y as usize / BLOCK_SIZE;

    match maze.get(j).and_then(|row| row.get(i)) {
        Some(&cell) => cell != ' ' && cell != 'g' && cell != 'G',
        // fuera del laberinto por el lado positivo: también es pared.
        None => true,
    }
}

/// Aplica el desplazamiento (dx, dy) evaluando cada eje por separado.
///
/// Evaluarlos por separado es lo que permite *deslizarse* a lo largo de una
/// pared: si avanzas en diagonal contra un muro vertical, el eje X se bloquea
/// pero el eje Y sigue libre, en vez de frenar al jugador por completo.
///
/// El punto que se consulta no es el destino exacto sino el destino corrido
/// `COLLISION_MARGIN` píxeles en la dirección del movimiento, de modo que el
/// jugador se detenga *antes* de tocar la pared y nunca quede dentro de ella.
fn try_move(player: &mut Player, maze: &Maze, dx: f32, dy: f32) {
    if dx != 0.0 {
        let probe_x = player.pos.x + dx + COLLISION_MARGIN * dx.signum();
        if !is_wall(maze, probe_x, player.pos.y) {
            player.pos.x += dx;
        }
    }

    if dy != 0.0 {
        let probe_y = player.pos.y + dy + COLLISION_MARGIN * dy.signum();
        if !is_wall(maze, player.pos.x, probe_y) {
            player.pos.y += dy;
        }
    }
}

pub fn process_events(window: &Window, player: &mut Player, maze: &Maze) {
    // A y D solo cambian el ángulo de vista: el jugador gira sobre su eje, así
    // que girar nunca puede meterlo dentro de una pared.
    if window.is_key_down(Key::A) {
        player.a -= ROTATION_SPEED;
    }

    if window.is_key_down(Key::D) {
        player.a += ROTATION_SPEED;
    }

    // El ángulo se mantiene dentro de [0, 2PI) para que no crezca sin límite
    // tras muchos giros y pierda precisión en f32.
    player.a = player.a.rem_euclid(TAU);

    // W y S se mueven sobre la dirección de vista, de modo que avanzar siempre
    // ocurre hacia donde el jugador está viendo:
    //     x += velocidad * cos(a)
    //     y += velocidad * sin(a)
    let (sin_a, cos_a) = player.a.sin_cos();

    if window.is_key_down(Key::W) {
        try_move(
            player,
            maze,
            MOVE_SPEED * cos_a,
            MOVE_SPEED * sin_a,
        );
    }

    if window.is_key_down(Key::S) {
        try_move(
            player,
            maze,
            -MOVE_SPEED * cos_a,
            -MOVE_SPEED * sin_a,
        );
    }
}
