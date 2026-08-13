use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

/// Resultado de lanzar un rayo: qué tan lejos viajó antes de chocar y contra
/// qué chocó. La distancia define la altura de la estaca en la vista 3D y el
/// carácter de impacto define su color.
pub struct Intersect {
    /// Distancia euclidiana recorrida por el rayo desde el jugador.
    pub distance: f32,
    /// Carácter de la celda del laberinto contra la que chocó el rayo.
    pub impact: char,
}

/// ¿El punto (x, y) cae dentro de algo sólido?
///
/// Devuelve `None` si el punto está en espacio libre, o `Some(carácter)` con la
/// celda contra la que se chocó. Salirse del laberinto por cualquier lado
/// cuenta como pared.
fn cell_at(maze: &Maze, block_size: usize, x: f32, y: f32) -> Option<char> {
    if x < 0.0 || y < 0.0 {
        return Some('|');
    }

    let i = x as usize / block_size;
    let j = y as usize / block_size;

    match maze.get(j).and_then(|row| row.get(i)) {
        None => Some('|'),
        Some(&' ') => None,
        Some(&cell) => Some(cell),
    }
}

/// Avanza un rayo desde la posición del jugador en el ángulo `a` hasta chocar
/// con una celda que no sea espacio vacío.
///
/// `draw_line` controla si el recorrido se pinta en el framebuffer: la vista 2D
/// lo quiere (para ver el abanico), la vista 3D no (solo necesita la distancia).
pub fn cast_ray(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    a: f32,
    block_size: usize,
    draw_line: bool,
) -> Intersect {
    let (sin_a, cos_a) = a.sin_cos();

    let mut d = 0.0;

    if draw_line {
        framebuffer.set_current_color(0xFFDDDD);
    }

    // Se avanza de un píxel a la vez. Es tentador dar pasos más largos para
    // ganar velocidad, pero un paso grueso puede rozar la esquina de un bloque
    // sin llegar a muestrear dentro de ella: el rayo cruza el corte diagonal de
    // la esquina en pocos píxeles, el muestreo no lo detecta y sigue de largo
    // hasta una pared mucho más lejana. En pantalla eso aparece como columnas
    // sueltas con un pico. Medido con paso de 10 px: 24 de 14 400 rayos fallaban
    // así, con hasta 221 px de error.
    loop {
        let x = player.pos.x + d * cos_a;
        let y = player.pos.y + d * sin_a;

        if let Some(impact) = cell_at(maze, block_size, x, y) {
            return Intersect { distance: d, impact };
        }

        if draw_line {
            framebuffer.point(x as usize, y as usize);
        }

        d += 1.0;
    }
}
