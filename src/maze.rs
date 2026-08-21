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

pub fn load_maze(filename: &str) -> (Maze, Player) {
    let file = File::open(filename).expect("no se pudo abrir el archivo del laberinto");

    let reader = BufReader::new(file);

    let mut maze: Maze = Vec::new();

    let mut player_pos: Option<Vec2> = None;

    for (row, line) in reader.lines().enumerate() {
        let line = line.expect("no se pudo leer una línea del laberinto");

        let mut cells: Vec<char> = Vec::new();

        for (col, character) in line.chars().enumerate() {
            if character == 'p' {
                let x = col * BLOCK_SIZE + BLOCK_SIZE / 2;
                let y = row * BLOCK_SIZE + BLOCK_SIZE / 2;
                player_pos = Some(Vec2::new(x as f32, y as f32));

                cells.push(' ');
            } else {
                cells.push(character);
            }
        }

        maze.push(cells);
    }

    let player = Player {
        pos: player_pos.unwrap_or_else(|| Vec2::new(0.0, 0.0)),
        // ángulo de vista inicial; el jugador podrá girarlo con el teclado.
        a: PI / 3.0,
    };

    (maze, player)
}
