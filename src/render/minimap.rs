//! Minimapa de la esquina superior derecha, superpuesto a la vista 3D.
//!
//! Muestra únicamente lo que `discovery` marcó como visto: es la memoria del
//! jugador, no el mapa del laberinto.
//!
//! # De coordenadas del mundo a coordenadas del recuadro
//!
//! El mundo mide `columnas * BLOCK_SIZE` píxeles de ancho, y el minimapa tiene
//! que caber en un recuadro de unos cientos. En vez de una escala fraccionaria
//! se elige un **tamaño entero de celda**: con escala fraccionaria las celdas
//! se truncan a distinto número de píxeles y aparecen rendijas de un píxel
//! entre ellas, que se ven como una rejilla rota. Con `cell_px` entero, las
//! celdas quedan pegadas por construcción.
//!
//! El mismo `cell_px` se usa en los dos ejes, que es lo que impide que el
//! laberinto salga estirado si el mapa no es cuadrado.

use crate::discovery::Discovered;
use crate::flashlight::Flashlight;
use crate::framebuffer::Framebuffer;
use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;
use crate::render::lighting;
use crate::textures::{Texture, TextureSet};

/// Lado máximo del área del mapa, en píxeles de pantalla. El lado real sale de
/// redondear hacia abajo a un múltiplo entero del tamaño de celda.
const MAX_SIZE: usize = 260;

/// Separación entre el minimapa y el borde de la ventana.
const MARGIN: usize = 12;

/// Relleno entre el mapa y el borde de su recuadro.
const PADDING: usize = 6;

/// Grosor del marco del recuadro.
const BORDER: usize = 2;

const PANEL_COLOR: u32 = 0x0E0E10;
const BORDER_COLOR: u32 = 0x44484F;

/// Color del suelo ya descubierto. Apenas más claro que el panel: tiene que
/// leerse como "aquí se puede pasar" sin competir con las paredes.
const FLOOR_COLOR: u32 = 0x2A2E33;

const PLAYER_COLOR: u32 = 0xFFDD33;

/// Lado del punto que marca al jugador, en píxeles.
const PLAYER_DOT: usize = 5;

/// Largo de la aguja que indica hacia dónde mira el jugador, en píxeles.
const HEADING_LENGTH: usize = 14;

/// Atenuación fija de las paredes del minimapa.
///
/// Las texturas a pleno brillo compiten con el punto del jugador y con la vista
/// 3D de atrás. Bajarlas un poco las deja legibles sin que el minimapa grite.
const TEXTURE_DIM: f32 = 0.85;

/// Qué tan atenuado se ve el minimapa con la linterna apagada.
///
/// No se oculta del todo a propósito: el jugador necesita seguir viendo dónde
/// está para poder decidir si vale la pena gastar batería. Lo que se apaga es
/// la nitidez, no la información. Para que desaparezca por completo con la
/// linterna apagada, poner 0.0.
const DIM_WHEN_OFF: f32 = 0.45;

pub fn draw(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    discovered: &Discovered,
    flashlight: &Flashlight,
    textures: &TextureSet,
) {
    let rows = maze.len();
    let cols = maze.iter().map(|row| row.len()).max().unwrap_or(0);

    if rows == 0 || cols == 0 {
        return;
    }

    // el lado entero de celda que hace que el mapa completo entre en el
    // recuadro, sin estirar ninguno de los dos ejes.
    let cell_px = (MAX_SIZE / cols).min(MAX_SIZE / rows).max(1);

    let map_width = cols * cell_px;
    let map_height = rows * cell_px;

    let panel_width = map_width + PADDING * 2;
    let panel_height = map_height + PADDING * 2;

    // esquina superior derecha: se descuenta el ancho del panel del ancho de la
    // ventana. `saturating_sub` cubre el caso de una ventana más angosta que el
    // panel, donde el minimapa simplemente arranca en x = 0.
    let panel_x = framebuffer
        .width
        .saturating_sub(panel_width + MARGIN);
    let panel_y = MARGIN;

    // marco primero y panel encima, desplazado por el grosor del borde: sale
    // más barato que pintar cuatro tiras.
    framebuffer.rect(
        panel_x.saturating_sub(BORDER),
        panel_y.saturating_sub(BORDER),
        panel_width + BORDER * 2,
        panel_height + BORDER * 2,
        BORDER_COLOR,
    );
    framebuffer.rect(panel_x, panel_y, panel_width, panel_height, PANEL_COLOR);

    let dim = if flashlight.on { 1.0 } else { DIM_WHEN_OFF };

    // origen del área de mapa, ya descontado el relleno.
    let map_x = panel_x + PADDING;
    let map_y = panel_y + PADDING;

    draw_known_cells(framebuffer, maze, discovered, textures, map_x, map_y, cell_px, dim);

    // Del mundo al minimapa: los dos usan píxeles, así que el factor es
    // simplemente cuántos píxeles de minimapa vale un píxel del mundo.
    let world_to_map = cell_px as f32 / BLOCK_SIZE as f32;

    let player_x = map_x as f32 + player.pos.x * world_to_map;
    let player_y = map_y as f32 + player.pos.y * world_to_map;

    draw_heading(framebuffer, player, player_x, player_y, dim);
    draw_player_dot(framebuffer, player_x, player_y, dim);
}

/// Pinta una casilla por cada celda ya descubierta.
///
/// Las que no se han visto no se dibujan: quedan del color del panel, así que un
/// pasillo sin explorar y una pared sin explorar se ven igual. Eso es
/// exactamente lo que se quiere — el jugador no puede deducir la forma del
/// laberinto sin haberlo alumbrado.
///
/// Las paredes se dibujan con su textura encogida a la casilla, la misma que
/// usa la vista 3D. Así el minimapa y el mundo hablan el mismo idioma: la pared
/// de franjas amarillas se ve amarilla en los dos lados, sin una tabla de
/// colores aparte que haya que mantener sincronizada a mano.
///
/// El suelo transitable sí queda de color plano, y a propósito: es lo que
/// distingue de un vistazo por dónde se puede caminar.
fn draw_known_cells(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    discovered: &Discovered,
    textures: &TextureSet,
    map_x: usize,
    map_y: usize,
    cell_px: usize,
    dim: f32,
) {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            if !discovered.is_known(col, row) {
                continue;
            }

            let x0 = map_x + col * cell_px;
            let y0 = map_y + row * cell_px;

            if cell == ' ' {
                framebuffer.rect(
                    x0,
                    y0,
                    cell_px,
                    cell_px,
                    lighting::apply(FLOOR_COLOR, dim),
                );

                continue;
            }

            draw_textured_cell(framebuffer, textures.for_cell(cell), x0, y0, cell_px, dim);
        }
    }
}

/// Encoge una textura completa dentro de una casilla de `cell_px` de lado.
///
/// Es un muestreo por punto, sin promediar: a este tamaño una textura de 64x64
/// se reduce a ~20 px de lado, así que la mayoría de los píxeles originales se
/// descartan y aparece algo de aliasing. Promediar cada bloque quedaría más
/// suave, pero el patrón —las franjas, las juntas del panel— se distingue igual
/// y así el costo se mantiene en un muestreo por píxel dibujado.
fn draw_textured_cell(
    framebuffer: &mut Framebuffer,
    texture: &Texture,
    x0: usize,
    y0: usize,
    cell_px: usize,
    dim: f32,
) {
    let light = dim * TEXTURE_DIM;
    let inverse = 1.0 / cell_px as f32;

    for y in 0..cell_px {
        let v = y as f32 * inverse;

        for x in 0..cell_px {
            let u = x as f32 * inverse;

            let texel = texture.texel_uv(u, v);

            framebuffer.pixel(x0 + x, y0 + y, lighting::apply(texel, light));
        }
    }
}

/// Aguja que apunta hacia donde mira el jugador.
///
/// Sin ella el punto amarillo no dice nada: sabés dónde estás pero no hacia
/// dónde vas a caminar si presionás W, que es la mitad de la información útil
/// de un minimapa.
fn draw_heading(
    framebuffer: &mut Framebuffer,
    player: &Player,
    player_x: f32,
    player_y: f32,
    dim: f32,
) {
    let (sin_a, cos_a) = player.a.sin_cos();

    framebuffer.set_current_color(lighting::apply(PLAYER_COLOR, dim));

    // un punto por píxel de largo. Para una línea de 14 px no hace falta
    // Bresenham: el paso de 1 px sobre el eje más largo no deja huecos porque
    // ninguna de las dos componentes crece más rápido que el parámetro.
    for step in 0..HEADING_LENGTH {
        let x = player_x + cos_a * step as f32;
        let y = player_y + sin_a * step as f32;

        // la aguja puede salirse del panel al mirar hacia afuera del mapa;
        // `point` recorta contra la ventana, pero no contra el recuadro, así
        // que los negativos se descartan aquí antes de convertir a usize.
        if x >= 0.0 && y >= 0.0 {
            framebuffer.point(x as usize, y as usize);
        }
    }
}

/// Punto que marca al jugador, centrado en su posición.
fn draw_player_dot(framebuffer: &mut Framebuffer, player_x: f32, player_y: f32, dim: f32) {
    let half = PLAYER_DOT as f32 / 2.0;

    framebuffer.rect(
        (player_x - half).max(0.0) as usize,
        (player_y - half).max(0.0) as usize,
        PLAYER_DOT,
        PLAYER_DOT,
        lighting::apply(PLAYER_COLOR, dim),
    );
}
