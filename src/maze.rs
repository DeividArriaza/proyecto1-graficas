use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufRead, BufReader};

use nalgebra_glm::Vec2;

use crate::player::Player;

pub type Maze = Vec<Vec<char>>;

/// Lado en píxeles de una celda del laberinto.
///
/// Es la escala del mundo: define tanto el tamaño de la celda en la vista
/// cenital como la altura de la pared que se proyecta en la vista 3D.
pub const BLOCK_SIZE: usize = 100;

/// Qué celda contiene el punto (x, y) del mundo.
///
/// Fuera del laberinto devuelve `'|'`: los bordes cuentan como pared por todos
/// lados, y así ni el jugador ni el monstruo pueden salirse del mapa.
///
/// Vive acá y no en quien la usa porque el jugador y el monstruo hacen la misma
/// pregunta con respuestas distintas: el jugador puede pisar la meta —tiene que
/// poder, para ganar—, el monstruo no.
pub fn cell_at(maze: &Maze, x: f32, y: f32) -> char {
    if x < 0.0 || y < 0.0 {
        return '|';
    }

    let i = x as usize / BLOCK_SIZE;
    let j = y as usize / BLOCK_SIZE;

    maze.get(j)
        .and_then(|row| row.get(i))
        .copied()
        .unwrap_or('|')
}

/// Ángulo de vista inicial del jugador, en radianes.
const INITIAL_ANGLE: f32 = PI / 3.0;

/// Carga un laberinto de un archivo de texto.
///
/// Devuelve `None` si el archivo no se puede abrir o leer, en vez de entrar en
/// pánico: quien llama decide qué hacer, y en este proyecto la respuesta es
/// generar uno procedural en su lugar.
pub fn load_maze(filename: &str) -> Option<Maze> {
    let file = File::open(filename).ok()?;
    let reader = BufReader::new(file);

    let mut maze: Maze = Vec::new();

    for line in reader.lines() {
        maze.push(line.ok()?.chars().collect());
    }

    if maze.is_empty() {
        return None;
    }

    Some(maze)
}

/// Saca al jugador del laberinto y devuelve su estado inicial.
///
/// La marca `p` se reemplaza por piso: ya cumplió su función y dejarla haría que
/// el caster la tratara como pared.
///
/// Si no hay marca `p`, el jugador arranca en el centro de la primera celda
/// libre que se encuentre. Antes se usaba (0, 0), que en un laberinto con borde
/// cerrado es justo la esquina de una pared.
pub fn extract_player(maze: &mut Maze) -> Player {
    let mut position: Option<Vec2> = None;

    for (row, line) in maze.iter_mut().enumerate() {
        for (col, character) in line.iter_mut().enumerate() {
            if *character == 'p' || *character == 'P' {
                *character = ' ';
                position = Some(cell_center(col, row));
            }
        }
    }

    let pos = position
        .or_else(|| first_free_cell(maze))
        .unwrap_or_else(|| cell_center(0, 0));

    Player {
        pos,
        a: INITIAL_ANGLE,
    }
}

/// Centro en píxeles de la celda (col, row).
fn cell_center(col: usize, row: usize) -> Vec2 {
    Vec2::new(
        (col * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
        (row * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
    )
}

fn first_free_cell(maze: &Maze) -> Option<Vec2> {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            if cell == ' ' {
                return Some(cell_center(col, row));
            }
        }
    }

    None
}
