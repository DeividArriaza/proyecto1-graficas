//! Estados del juego y qué nivel se está jugando.
//!
//! `main` no decide nada de esto: pregunta en qué pantalla está y despacha.

pub mod pause;
pub mod victory;
pub mod welcome;

use crate::discovery::Discovered;
use crate::flashlight::Flashlight;
use crate::maze::{extract_player, load_maze, Maze};
use crate::mazegen;
use crate::player::{MouseLook, Player};

/// Qué pantalla está activa.
///
/// Las variantes no llevan datos a propósito. `main` guarda la sesión y el
/// informe de victoria en `Option`s aparte, y así puede reasignar la pantalla
/// dentro del mismo `match` sin pelearse con el préstamo de los datos.
pub enum Screen {
    /// Menú de selección de nivel.
    Welcome,
    /// Dentro del laberinto.
    Playing,
    /// Menú de pausa. La sesión sigue viva, sólo se dejó de actualizar.
    Paused,
    /// Informe de nivel superado.
    Victory,
}

/// Lo que se muestra al superar un nivel.
///
/// Es `Copy` para que la pantalla de victoria pueda sacar una copia y liberar el
/// `Option` que lo guarda antes de arrancar la partida siguiente.
#[derive(Clone, Copy)]
pub struct VictoryReport {
    /// Índice en `LEVELS`, para poder reintentar el mismo nivel.
    pub level: usize,
    pub level_name: &'static str,
    pub seconds: f32,
    /// Fracción del laberinto transitable que se llegó a ver, de 0.0 a 1.0.
    pub explored: f32,
    /// Batería restante, de 0.0 a 1.0.
    pub battery: f32,
}

/// Un nivel ofrecido en el menú.
pub struct Level {
    /// Como aparece en el menú.
    pub name: &'static str,
    /// Archivo del que se carga. Si no existe, el nivel se genera.
    ///
    /// Vacío significa que nunca se intenta cargar de disco.
    pub path: &'static str,
    /// Tamaño en celdas cuando hay que generarlo.
    pub cells: (usize, usize),
    /// Semilla fija, o `None` para sacarla del reloj.
    ///
    /// Una semilla fija hace que el nivel sea siempre el mismo laberinto, que es
    /// lo que lo vuelve un *nivel* y no una sorpresa distinta en cada arranque.
    /// `None` es el modo infinito.
    pub seed: Option<u64>,
}

/// Los niveles del menú, en orden.
///
/// Los tres primeros intentan cargar un archivo y, si no está, generan uno con
/// semilla fija. Ese respaldo es lo que permite que el menú funcione hoy y que
/// reemplazar un nivel sea copiar un `.txt` a `assets/levels/`.
pub const LEVELS: [Level; 4] = [
    Level {
        name: "NIVEL 1 - ALMACEN",
        path: "assets/levels/level1.txt",
        cells: (8, 6),
        seed: Some(0x1A2B_3C4D),
    },
    Level {
        name: "NIVEL 2 - PASILLOS",
        path: "assets/levels/level2.txt",
        cells: (12, 9),
        seed: Some(0x00C0_FFEE),
    },
    Level {
        name: "NIVEL 3 - SUBSUELO",
        path: "assets/levels/level3.txt",
        cells: (16, 12),
        seed: Some(0xDEAD_BEEF),
    },
    Level {
        name: "MODO INFINITO",
        path: "",
        cells: (0, 0),
        seed: None,
    },
];

/// Todo lo que cambia mientras se juega un nivel.
///
/// Existe para que empezar una partida sea construir una de estas y tirar la
/// anterior: no hay que acordarse de reiniciar la linterna, ni el descubrimiento,
/// ni la posición del jugador por separado.
pub struct Session {
    /// Índice del nivel en `LEVELS`. Lo necesita el informe de victoria para
    /// ofrecer reintentar.
    pub level: usize,
    /// Segundos jugados en este nivel.
    pub elapsed: f32,
    pub maze: Maze,
    pub player: Player,
    pub discovered: Discovered,
    pub flashlight: Flashlight,
    /// Vive en la sesión y no en `main` para que al empezar un nivel el
    /// seguimiento del mouse arranque limpio. Si se conservara entre partidas,
    /// el trayecto que hizo el cursor por el menú se aplicaría de golpe como un
    /// giro al entrar.
    pub mouse: MouseLook,
}

impl Session {
    /// Arranca una partida del nivel en la posición `index` de `LEVELS`.
    ///
    /// Toma el índice y no una referencia al nivel porque hay que recordarlo
    /// para el reintento, y así no queda la posibilidad de que el índice
    /// guardado y el nivel cargado se desincronicen.
    pub fn start(index: usize) -> Self {
        let level = &LEVELS[index];

        let mut maze = level_maze(level);
        let player = extract_player(&mut maze);
        let discovered = Discovered::new(&maze);

        Session {
            level: index,
            elapsed: 0.0,
            maze,
            player,
            discovered,
            flashlight: Flashlight::new(),
            mouse: MouseLook::new(),
        }
    }

    /// Resumen de la partida, para la pantalla de victoria.
    pub fn report(&self) -> VictoryReport {
        VictoryReport {
            level: self.level,
            level_name: LEVELS[self.level].name,
            seconds: self.elapsed,
            explored: self.discovered.explored_fraction(&self.maze),
            battery: self.flashlight.battery,
        }
    }
}

/// Consigue el laberinto de un nivel: del archivo si existe, generado si no.
fn level_maze(level: &Level) -> Maze {
    if !level.path.is_empty() {
        if let Some(maze) = load_maze(level.path) {
            return maze;
        }

        println!("nivel {} no encontrado; se genera", level.path);
    }

    match level.seed {
        Some(seed) => mazegen::generate(level.cells.0, level.cells.1, seed),
        None => infinite_maze(),
    }
}

/// Laberinto del modo infinito: semilla y tamaño distintos cada vez.
fn infinite_maze() -> Maze {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5EED);

    // el tamaño también sale de la semilla, así que ni las dimensiones se
    // repiten entre partidas.
    let mut rng = mazegen::Rng::new(seed);
    let cols = rng.between(12, 20);
    let rows = rng.between(9, 14);

    mazegen::generate(cols, rows, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Los cuatro niveles del menú tienen que poder arrancar, con o sin archivo
    /// en disco. El respaldo procedural es justo lo que hace que esto pase hoy.
    #[test]
    fn todos_los_niveles_arrancan() {
        for (index, level) in LEVELS.iter().enumerate() {
            let session = Session::start(index);

            assert!(!session.maze.is_empty(), "{}: laberinto vacío", level.name);

            // El jugador tiene que quedar en piso, nunca dentro de una pared.
            let i = session.player.pos.x as usize / crate::maze::BLOCK_SIZE;
            let j = session.player.pos.y as usize / crate::maze::BLOCK_SIZE;
            let cell = session.maze[j][i];

            assert_eq!(cell, ' ', "{}: el jugador arranca en '{cell}'", level.name);

            // y tiene que haber una meta a la que llegar.
            let has_goal = session
                .maze
                .iter()
                .flatten()
                .any(|&c| c == 'g' || c == 'G');

            assert!(has_goal, "{}: sin meta", level.name);
        }
    }

    /// El modo infinito no puede repetir el mismo laberinto.
    #[test]
    fn el_modo_infinito_cambia() {
        let first = infinite_maze();
        let second = infinite_maze();

        assert_ne!(first, second, "el modo infinito repitió el laberinto");
    }
}
