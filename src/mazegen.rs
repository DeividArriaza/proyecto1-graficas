//! Generación de laberintos.
//!
//! # Cómo se construye
//!
//! El laberinto se genera sobre una rejilla de **celdas**, y después se dibuja
//! en la rejilla de **caracteres** que usa el resto del programa. Las dos no son
//! lo mismo: una celda de laberinto ocupa un carácter, pero entre dos celdas
//! vecinas hay otro carácter que es la pared que las separa. Un laberinto de
//! `cols` x `rows` celdas se dibuja en `2*cols+1` x `2*rows+1` caracteres.
//!
//! La celda (cx, cy) cae en el carácter (2cx+1, 2cy+1) — siempre en índices
//! impares. Los índices pares son paredes.
//!
//! # DFS para generar, BFS para la meta
//!
//! El recorrido en profundidad (*recursive backtracking*) produce un laberinto
//! perfecto: un único camino entre cualquier par de celdas, sin ciclos ni zonas
//! aisladas. Lo valioso es que **la conectividad sale garantizada por
//! construcción**: no hay que generar, verificar que se puede resolver y
//! reintentar. Nunca produce un mapa inválido.
//!
//! Después, un recorrido en anchura desde el inicio mide la distancia a todas
//! las celdas y la meta se coloca en la más lejana. Sin eso la meta podría caer
//! a tres pasos del inicio y el nivel duraría cinco segundos.

use crate::maze::Maze;

/// Generador de números pseudoaleatorios xorshift64.
///
/// Escrito a mano en vez de agregar el crate `rand`: son doce líneas y evita una
/// dependencia entera. Además, al ser determinista para una semilla dada, la
/// misma semilla produce siempre el mismo laberinto — que es exactamente lo que
/// hace posible que los niveles fijos sean *niveles* y no una sorpresa distinta
/// en cada arranque.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // el estado nunca puede ser cero: xorshift se queda pegado en cero.
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;

        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;

        self.0 = x;

        x
    }

    /// Entero en el rango [0, n).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }

        (self.next_u64() % n as u64) as usize
    }

    /// Entero en el rango [low, high].
    pub fn between(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }
}

/// Las cuatro direcciones, como desplazamiento en celdas.
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Genera un laberinto de `cols` x `rows` celdas, con `p` en una esquina y `g`
/// en la celda más lejana.
pub fn generate(cols: usize, rows: usize, seed: u64) -> Maze {
    let cols = cols.max(2);
    let rows = rows.max(2);

    let mut rng = Rng::new(seed);
    let mut maze = walled_grid(cols, rows);

    carve(&mut maze, cols, rows, &mut rng);

    // el inicio va en la esquina desde la que arrancó el recorrido.
    maze[1][1] = 'p';

    let (goal_x, goal_y) = farthest_cell(&maze, cols, rows, (0, 0));
    maze[2 * goal_y + 1][2 * goal_x + 1] = 'g';

    maze
}

/// Rejilla de caracteres con todas las paredes puestas y las celdas vacías.
///
/// Los tres tipos de pared se reparten según la paridad del índice, de modo que
/// el laberinto usa las tres texturas sin tener que decidir nada: esquinas con
/// `+`, tramos horizontales con `-`, tramos verticales con `|`.
fn walled_grid(cols: usize, rows: usize) -> Maze {
    let width = 2 * cols + 1;
    let height = 2 * rows + 1;

    let mut maze = Vec::with_capacity(height);

    for y in 0..height {
        let mut line = Vec::with_capacity(width);

        for x in 0..width {
            line.push(match (x % 2 == 0, y % 2 == 0) {
                (true, true) => '+',   // cruce de paredes
                (false, true) => '-',  // tramo horizontal
                (true, false) => '|',  // tramo vertical
                (false, false) => ' ', // interior de una celda
            });
        }

        maze.push(line);
    }

    maze
}

/// Abre pasillos con un recorrido en profundidad.
///
/// La pila es explícita y no recursiva: un laberinto grande puede encadenar
/// miles de celdas y la recursión desbordaría la pila del programa.
fn carve(maze: &mut Maze, cols: usize, rows: usize, rng: &mut Rng) {
    let mut visited = vec![false; cols * rows];
    let mut stack: Vec<(usize, usize)> = Vec::with_capacity(cols * rows);

    visited[0] = true;
    stack.push((0, 0));

    while let Some(&(cx, cy)) = stack.last() {
        // vecinos que todavía no se visitaron.
        let mut options = [(0usize, 0usize); 4];
        let mut count = 0;

        for (dx, dy) in DIRECTIONS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                continue;
            }

            let (nx, ny) = (nx as usize, ny as usize);

            if !visited[ny * cols + nx] {
                options[count] = (nx, ny);
                count += 1;
            }
        }

        // callejón sin salida: se retrocede.
        if count == 0 {
            stack.pop();
            continue;
        }

        let (nx, ny) = options[rng.below(count)];

        // Se borra la pared que separa las dos celdas. En caracteres, esa pared
        // está justo a mitad de camino entre los centros de ambas.
        let wall_x = cx + nx + 1;
        let wall_y = cy + ny + 1;
        maze[wall_y][wall_x] = ' ';

        visited[ny * cols + nx] = true;
        stack.push((nx, ny));
    }
}

/// Celda más lejana del inicio, medida en cantidad de pasos.
///
/// Recorrido en anchura: la primera vez que se llega a una celda es siempre por
/// el camino más corto, así que la última celda alcanzada es la más lejana.
fn farthest_cell(maze: &Maze, cols: usize, rows: usize, start: (usize, usize)) -> (usize, usize) {
    let mut visited = vec![false; cols * rows];
    let mut queue = std::collections::VecDeque::new();

    visited[start.1 * cols + start.0] = true;
    queue.push_back(start);

    let mut last = start;

    while let Some((cx, cy)) = queue.pop_front() {
        last = (cx, cy);

        for (dx, dy) in DIRECTIONS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                continue;
            }

            let (nx, ny) = (nx as usize, ny as usize);

            if visited[ny * cols + nx] {
                continue;
            }

            // sólo se puede pasar si la pared entre las dos celdas está abierta.
            if maze[cy + ny + 1][cx + nx + 1] != ' ' {
                continue;
            }

            visited[ny * cols + nx] = true;
            queue.push_back((nx, ny));
        }
    }

    last
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Cuenta las celdas alcanzables desde el inicio y las compara con el total.
    ///
    /// Es la propiedad que hace valioso al recorrido en profundidad: si algo
    /// quedara aislado, este conteo lo delata.
    fn reachable(maze: &Maze, cols: usize, rows: usize) -> usize {
        let mut visited = vec![false; cols * rows];
        let mut stack = vec![(0usize, 0usize)];
        visited[0] = true;
        let mut count = 1;

        while let Some((cx, cy)) = stack.pop() {
            for (dx, dy) in DIRECTIONS {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if visited[ny * cols + nx] || maze[cy + ny + 1][cx + nx + 1] != ' ' {
                    continue;
                }
                visited[ny * cols + nx] = true;
                count += 1;
                stack.push((nx, ny));
            }
        }

        count
    }

    #[test]
    fn todas_las_celdas_son_alcanzables() {
        for seed in 1..40u64 {
            let cols = 4 + (seed as usize % 13);
            let rows = 4 + (seed as usize % 9);
            let maze = generate(cols, rows, seed);

            assert_eq!(maze.len(), 2 * rows + 1, "alto inesperado");
            assert_eq!(maze[0].len(), 2 * cols + 1, "ancho inesperado");
            assert_eq!(
                reachable(&maze, cols, rows),
                cols * rows,
                "semilla {seed}: quedaron celdas aisladas"
            );
        }
    }

    #[test]
    fn hay_inicio_y_meta_y_no_coinciden() {
        for seed in 1..40u64 {
            let maze = generate(9, 7, seed);

            let flat: String = maze.iter().flatten().collect();
            assert_eq!(flat.matches('p').count(), 1, "semilla {seed}: inicio");
            assert_eq!(flat.matches('g').count(), 1, "semilla {seed}: meta");
        }
    }

    /// Distancia en pasos desde el inicio hasta cada celda alcanzable.
    fn distances(maze: &Maze, cols: usize, rows: usize) -> Vec<Option<usize>> {
        let mut dist = vec![None; cols * rows];
        let mut queue = std::collections::VecDeque::new();

        dist[0] = Some(0);
        queue.push_back((0usize, 0usize));

        while let Some((cx, cy)) = queue.pop_front() {
            let here = dist[cy * cols + cx].unwrap();

            for (dx, dy) in DIRECTIONS {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if dist[ny * cols + nx].is_some() || maze[cy + ny + 1][cx + nx + 1] != ' ' {
                    continue;
                }
                dist[ny * cols + nx] = Some(here + 1);
                queue.push_back((nx, ny));
            }
        }

        dist
    }

    #[test]
    fn la_meta_es_la_celda_mas_lejana() {
        // Lo que importa no es la distancia en linea recta —una celda puede
        // estar al lado y a treinta pasos caminando— sino la distancia por el
        // camino. Se comprueba que ninguna otra celda quede mas lejos que la
        // meta, que es exactamente lo que promete el recorrido en anchura.
        for seed in 1..40u64 {
            let (cols, rows) = (10, 8);
            let maze = generate(cols, rows, seed);

            let mut goal = None;
            for (y, row) in maze.iter().enumerate() {
                for (x, &c) in row.iter().enumerate() {
                    if c == 'g' {
                        goal = Some(((x - 1) / 2, (y - 1) / 2));
                    }
                }
            }

            let goal = goal.expect("el laberinto no tiene meta");
            let dist = distances(&maze, cols, rows);
            let goal_distance = dist[goal.1 * cols + goal.0].expect("meta inalcanzable");
            let max_distance = dist.iter().flatten().copied().max().unwrap();

            assert_eq!(
                goal_distance, max_distance,
                "semilla {seed}: la meta esta a {goal_distance} pasos pero hay celdas a {max_distance}"
            );

            // ademas, en un mapa de 10x8 la celda mas lejana nunca queda a la
            // vuelta de la esquina.
            assert!(
                goal_distance > 10,
                "semilla {seed}: meta a solo {goal_distance} pasos"
            );
        }
    }

    /// Imprime un laberinto generado. Se corre con:
    ///     cargo test -- --nocapture muestra_un_laberinto
    #[test]
    fn muestra_un_laberinto() {
        let maze = generate(12, 9, 0x00C0_FFEE);

        for row in &maze {
            println!("{}", row.iter().collect::<String>());
        }
    }

    #[test]
    fn la_misma_semilla_da_el_mismo_laberinto() {
        assert_eq!(generate(11, 8, 777), generate(11, 8, 777));
        assert_ne!(generate(11, 8, 777), generate(11, 8, 778));
    }
}
