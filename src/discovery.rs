//! Qué partes del laberinto ya vio el jugador.
//!
//! Es la niebla de guerra del minimapa: una celda no se dibuja hasta que la
//! linterna la alcanzó, o hasta que el jugador la pisó.
//!
//! El descubrimiento corre en un pase propio, aparte del render. Sería tentador
//! marcar celdas dentro de `cast_ray`, aprovechando que ya recorre el mapa,
//! pero eso obligaría a pasarle estado mutable del juego al caster y marcaría
//! la misma celda miles de veces por cuadro. Este pase usa 64 rayos gruesos en
//! vez de 1300 finos: para saber *qué celda* se ve no hace falta resolución de
//! píxel.

use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;
use crate::render::lighting;

/// Rayos del pase de descubrimiento. La vista 3D usa 1300 porque necesita una
/// columna de pantalla por rayo; aquí sólo hay que tocar cada celda visible.
const DISCOVERY_RAYS: usize = 64;

/// Cuánto avanza cada rayo por paso, en píxeles.
///
/// Un cuarto de celda: imposible saltarse una celda de 100 px, y 25 veces más
/// barato que el paso de 1 px que necesita el render para no perder esquinas.
const DISCOVERY_STEP: f32 = BLOCK_SIZE as f32 / 4.0;

/// ¿Lo descubierto se recuerda para siempre?
///
/// En `true` el minimapa acumula todo lo visto, como la memoria del jugador.
/// En `false` sólo muestra lo que la linterna alumbra en este instante: más
/// tenso, pero el minimapa titila sin parar y deja de servir para orientarse.
const PERSISTENT_MEMORY: bool = true;

pub struct Discovered {
    /// Una casilla por celda del laberinto. Plano y no `Vec<Vec<bool>>`: una
    /// sola asignación y sin salto de puntero por fila.
    cells: Vec<bool>,
    width: usize,
    height: usize,
}

impl Discovered {
    /// Arranca con todo el laberinto sin descubrir.
    ///
    /// El ancho es el de la fila más larga: `load_maze` no rellena las filas
    /// cortas, así que las filas del laberinto pueden ser disparejas.
    pub fn new(maze: &Maze) -> Self {
        let height = maze.len();
        let width = maze.iter().map(|row| row.len()).max().unwrap_or(0);

        Discovered {
            cells: vec![false; width * height],
            width,
            height,
        }
    }

    pub fn is_known(&self, i: usize, j: usize) -> bool {
        if i >= self.width || j >= self.height {
            return false;
        }

        self.cells[j * self.width + i]
    }

    fn mark(&mut self, i: usize, j: usize) {
        if i < self.width && j < self.height {
            self.cells[j * self.width + i] = true;
        }
    }

    /// Marca la celda que el jugador ocupa ahora mismo.
    ///
    /// Se llama siempre, con linterna o sin ella: por ahí pasaste, lo tanteaste
    /// a oscuras. Así el minimapa nunca pierde el rastro de por dónde viniste,
    /// aunque hayas caminado con la batería apagada.
    pub fn mark_player(&mut self, player: &Player) {
        let i = player.pos.x as usize / BLOCK_SIZE;
        let j = player.pos.y as usize / BLOCK_SIZE;

        self.mark(i, j);
    }

    /// Revela lo que el haz de la linterna alcanza desde donde está el jugador.
    ///
    /// El abanico cubre la apertura del haz, no el campo de visión completo:
    /// fuera del cono la luz ambiental es casi nula, así que revelar todo el FOV
    /// mostraría en el minimapa cosas que en pantalla no se distinguen.
    ///
    /// `reach` se escala afuera con la intensidad de la linterna, de modo que
    /// una batería agonizante descubre menos sin necesitar código aparte.
    pub fn reveal_from(&mut self, maze: &Maze, player: &Player, reach: f32) {
        if !PERSISTENT_MEMORY {
            self.cells.fill(false);
        }

        if reach <= 0.0 {
            return;
        }

        let half_angle = lighting::beam_half_angle();

        for r in 0..DISCOVERY_RAYS {
            let fraction = r as f32 / (DISCOVERY_RAYS - 1) as f32; // de 0.0 a 1.0
            let angle = player.a - half_angle + 2.0 * half_angle * fraction;

            self.march(maze, player, angle, reach);
        }
    }

    /// Avanza un rayo marcando cada celda que cruza, hasta chocar o agotar el
    /// alcance.
    fn march(&mut self, maze: &Maze, player: &Player, angle: f32, reach: f32) {
        let (sin_a, cos_a) = angle.sin_cos();
        let mut d = 0.0;

        while d <= reach {
            let x = player.pos.x + d * cos_a;
            let y = player.pos.y + d * sin_a;

            if x < 0.0 || y < 0.0 {
                return;
            }

            let i = x as usize / BLOCK_SIZE;
            let j = y as usize / BLOCK_SIZE;

            match maze.get(j).and_then(|row| row.get(i)) {
                // fuera del laberinto: no hay nada más que descubrir por aquí.
                None => return,
                Some(&cell) => {
                    self.mark(i, j);

                    // la pared sí se marca antes de cortar: su cara es
                    // justamente lo que el jugador está viendo.
                    if cell != ' ' {
                        return;
                    }
                }
            }

            d += DISCOVERY_STEP;
        }
    }
}
