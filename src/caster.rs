//! Lanzamiento de rayos sobre la rejilla del laberinto.
//!
//! Este módulo es geometría pura: no conoce el framebuffer ni dibuja nada. Entra
//! un ángulo, sale contra qué chocó el rayo y a qué distancia.
//!
//! # Por qué DDA y no pasos de tamaño fijo
//!
//! La forma obvia de encontrar la pared es avanzar de a un píxel preguntando
//! "¿ya choqué?". Funciona, pero tiene dos defectos:
//!
//! - **Es caro.** Un rayo que cruza medio laberinto da cientos de iteraciones,
//!   casi todas dentro de la misma celda vacía.
//! - **No sabe dónde pegó.** El punto de impacto queda hasta un píxel *pasado*
//!   la cara de la pared, y no hay forma confiable de saber cuál de las dos
//!   caras se cruzó. Sin ese dato no se puede texturizar: la textura tiembla y
//!   las caras se pintan con la coordenada equivocada.
//!
//! DDA (*Digital Differential Analyzer*) salta de frontera de rejilla a
//! frontera de rejilla. Da tantas iteraciones como celdas cruce —decenas, no
//! cientos— y cae **exactamente** sobre la cara, sabiendo por construcción qué
//! eje cruzó.

use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;

/// Resultado de lanzar un rayo.
pub struct Intersect {
    /// Distancia euclidiana recorrida por el rayo desde el jugador.
    pub distance: f32,
    /// Carácter de la celda del laberinto contra la que chocó el rayo.
    pub impact: char,
    /// Dónde pegó sobre la cara de la pared, de 0.0 a 1.0. Es la coordenada
    /// horizontal de textura.
    pub tx: f32,
    /// ¿Cruzó una frontera vertical de la rejilla?
    ///
    /// `true` para las caras que mira desde el este o el oeste, `false` para las
    /// que mira desde el norte o el sur. Sirve para darles brillo distinto y que
    /// las esquinas se lean como esquinas.
    pub vertical: bool,
}

/// Tope de celdas que un rayo puede cruzar antes de rendirse.
///
/// Es una red de seguridad, no un límite de diseño: sin ella, un laberinto sin
/// bordes cerrados dejaría el bucle girando para siempre. 512 celdas son 51 200
/// píxeles, mucho más de lo que cualquier mapa razonable mide de diagonal.
const MAX_CELLS: usize = 512;

/// ¿La celda (i, j) es sólida?
///
/// Devuelve `None` si es espacio libre, o `Some(carácter)` con lo que hay ahí.
/// Salirse del laberinto por cualquier lado cuenta como pared.
fn cell_at(maze: &Maze, i: i32, j: i32) -> Option<char> {
    if i < 0 || j < 0 {
        return Some('|');
    }

    match maze.get(j as usize).and_then(|row| row.get(i as usize)) {
        None => Some('|'),
        Some(&' ') => None,
        Some(&cell) => Some(cell),
    }
}

pub fn cast_ray(maze: &Maze, player: &Player, a: f32) -> Intersect {
    let (sin_a, cos_a) = a.sin_cos();
    let block = BLOCK_SIZE as f32;

    // celda donde arranca el rayo.
    let mut map_i = (player.pos.x / block).floor() as i32;
    let mut map_j = (player.pos.y / block).floor() as i32;

    // Si el jugador terminó dentro de algo sólido, se devuelve distancia cero en
    // vez de entrar al bucle. No debería pasar —las colisiones lo evitan— pero
    // si pasara, el bucle nunca encontraría una cara que cruzar.
    if let Some(impact) = cell_at(maze, map_i, map_j) {
        return Intersect {
            distance: 0.0,
            impact,
            tx: 0.0,
            vertical: true,
        };
    }

    // Cuánto hay que avanzar sobre el rayo para cruzar una celda completa en
    // cada eje. Con la componente en cero da infinito, que es la respuesta
    // correcta: un rayo perfectamente horizontal nunca cruza una línea
    // horizontal, y el infinito hace que la comparación de más abajo nunca lo
    // elija.
    let delta_x = (block / cos_a).abs();
    let delta_y = (block / sin_a).abs();

    let step_i: i32 = if cos_a < 0.0 { -1 } else { 1 };
    let step_j: i32 = if sin_a < 0.0 { -1 } else { 1 };

    // distancia desde el jugador hasta la primera frontera de cada eje.
    let mut side_x = first_boundary(player.pos.x, map_i, cos_a, block);
    let mut side_y = first_boundary(player.pos.y, map_j, sin_a, block);

    for _ in 0..MAX_CELLS {
        // Se cruza siempre la frontera que esté más cerca. La distancia hasta
        // ella, *antes* de sumarle el delta, es justo la distancia a la cara de
        // la celda en la que estamos entrando.
        let (distance, vertical) = if side_x < side_y {
            let d = side_x;
            side_x += delta_x;
            map_i += step_i;
            (d, true)
        } else {
            let d = side_y;
            side_y += delta_y;
            map_j += step_j;
            (d, false)
        };

        if let Some(impact) = cell_at(maze, map_i, map_j) {
            return Intersect {
                distance,
                impact,
                tx: texture_x(player, cos_a, sin_a, distance, vertical, block),
                vertical,
            };
        }
    }

    // Tope alcanzado. Se reporta como pared lejanísima para que la columna se
    // pinte de algo en vez de quedar sin definir.
    Intersect {
        distance: MAX_CELLS as f32 * block,
        impact: '|',
        tx: 0.0,
        vertical: true,
    }
}

/// Distancia sobre el rayo hasta la primera frontera de rejilla de un eje.
///
/// `position` y `cell` son la coordenada y el índice de celda de ese eje, y
/// `direction` la componente del rayo. Con dirección cero no hay frontera que
/// cruzar nunca, y la respuesta es infinito.
fn first_boundary(position: f32, cell: i32, direction: f32, block: f32) -> f32 {
    if direction > 0.0 {
        // hacia adelante: falta llegar al borde superior de la celda.
        ((cell + 1) as f32 * block - position) / direction
    } else if direction < 0.0 {
        // hacia atrás: falta retroceder hasta el borde inferior.
        (position - cell as f32 * block) / -direction
    } else {
        f32::INFINITY
    }
}

/// Dónde pegó el rayo sobre la cara de la pared, de 0.0 a 1.0.
fn texture_x(
    player: &Player,
    cos_a: f32,
    sin_a: f32,
    distance: f32,
    vertical: bool,
    block: f32,
) -> f32 {
    let hit_x = player.pos.x + distance * cos_a;
    let hit_y = player.pos.y + distance * sin_a;

    // Sobre una cara vertical el rayo se desplaza a lo largo de `y`, y al
    // revés: lo que recorre la cara es el eje que *no* se cruzó.
    let along_face = if vertical { hit_y } else { hit_x };

    let mut tx = (along_face / block).fract();

    // `fract` conserva el signo, así que en coordenadas negativas devuelve algo
    // entre -1.0 y 0.0.
    if tx < 0.0 {
        tx += 1.0;
    }

    // Sin esto la textura sale espejeada en las caras opuestas: el mismo muro
    // visto desde un lado y desde el otro mostraría la imagen invertida.
    if (vertical && cos_a > 0.0) || (!vertical && sin_a < 0.0) {
        tx = 1.0 - tx;
    }

    // se deja justo por debajo de 1.0 para que al multiplicar por el ancho de la
    // textura nunca caiga fuera del último píxel.
    tx.clamp(0.0, 0.999_999)
}
