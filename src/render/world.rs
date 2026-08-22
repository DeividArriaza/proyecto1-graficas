//! Vista en primera persona: un rayo por cada columna de píxeles, y cada rayo
//! se proyecta como una estaca vertical cuya altura depende de la distancia.

use crate::caster::cast_ray;
use crate::flashlight::Flashlight;
use crate::framebuffer::Framebuffer;
use crate::maze::{Maze, BLOCK_SIZE};
use crate::player::Player;
use crate::render::lighting;
use crate::render::{EYE_HEIGHT, FOV};
use crate::textures::TextureSet;

/// Color base del techo. Es un interior, no hay cielo.
const CEILING_COLOR: u32 = 0x14161A;

/// Color base del piso.
const FLOOR_COLOR: u32 = 0x3A3630;

/// Cuánto se oscurecen las caras norte y sur respecto a las este y oeste.
///
/// Truco clásico y casi gratis: dos paredes perpendiculares con la misma
/// textura, iluminadas igual, se funden en una sola mancha y el mundo se ve
/// plano. Con las caras a distinto brillo, las esquinas se leen como esquinas.
/// El DDA es lo que hace esto posible, porque sabe qué eje cruzó cada rayo.
const HORIZONTAL_FACE_SHADE: f32 = 0.72;

/// Dibuja la vista y devuelve la distancia de la pared en cada columna.
///
/// Ese vector es el búfer de profundidad, y es lo que permite dibujar sprites
/// después: sin él, un monstruo detrás de una pared se pintaría encima de ella y
/// se vería atravesándola. La distancia es la perpendicular, la misma con la que
/// se proyectan las estacas, así que se compara directo contra la del sprite.
pub fn render(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    flashlight: &Flashlight,
    textures: &TextureSet,
) -> Vec<f32> {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let half_height = height as f32 / 2.0;

    // Distancia del ojo al plano de proyección. La mitad del ancho de la
    // pantalla abarca medio campo de visión, así que:
    //     dpp = (ancho / 2) / tan(FOV / 2)
    let distance_to_projection_plane = (width as f32 / 2.0) / (FOV / 2.0).tan();

    let intensity = flashlight.intensity();

    draw_ceiling_and_floor(
        framebuffer,
        distance_to_projection_plane,
        half_height,
        intensity,
    );

    // Media pantalla, en unidades del plano de proyección. La columna 0 cae en
    // -half_plane y la última en +half_plane.
    let half_plane = (FOV / 2.0).tan();

    let mut depth = Vec::with_capacity(width);

    // UN RAYO POR COLUMNA
    // El reparto no es de ángulos parejos sino de posiciones parejas sobre el
    // plano de proyección, que es lo que corresponde: la columna de pantalla de
    // un ángulo θ es `dpp * tan(θ)`, no `dpp * θ`. Repartir el FOV linealmente
    // en el ángulo comprime la imagen hacia los bordes; el efecto es leve a 60°
    // pero visible a 90°. Se invierte la relación con `atan`.
    for i in 0..width {
        // de -1.0 (borde izquierdo) a +1.0 (borde derecho).
        let screen_x = 2.0 * i as f32 / (width - 1) as f32 - 1.0;
        let angle = player.a + (screen_x * half_plane).atan();

        let intersect = cast_ray(maze, player, angle);

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

        depth.push(distance);

        // Proyección en perspectiva por triángulos semejantes: la pared mide
        // BLOCK_SIZE en el mundo y está a `distance` del ojo.
        let stake_height = (BLOCK_SIZE as f32 / distance) * distance_to_projection_plane;

        // la estaca se centra verticalmente: el horizonte queda a media pantalla.
        let stake_top = (half_height - stake_height / 2.0).max(0.0) as usize;
        let stake_bottom = (half_height + stake_height / 2.0).min(height as f32) as usize;

        // Las dos aportaciones de luz se separan porque la ambiental es
        // constante en toda la columna, pero el haz decae hacia arriba y hacia
        // abajo: hay que recalcularlo píxel por píxel.
        let ambient = lighting::ambient(distance);
        let beam = lighting::beam(distance, screen_x) * intensity;

        let face = if intersect.vertical {
            1.0
        } else {
            HORIZONTAL_FACE_SHADE
        };

        let texture = textures.for_cell(intersect.impact);
        let texture_column = texture.column_of(intersect.tx);

        // El mapeo vertical se calcula contra el tope SIN recortar. Si se usara
        // `stake_top`, que ya está limitado a 0, las paredes cercanas —donde la
        // estaca se sale de la pantalla— mostrarían la textura estirada en vez
        // de recortada.
        let unclamped_top = half_height - stake_height / 2.0;

        for y in stake_top..stake_bottom {
            // altura relativa dentro de la pared, de 0.0 arriba a 1.0 abajo.
            let v = (y as f32 - unclamped_top) / stake_height;

            let screen_y = (y as f32 - half_height) / half_height;
            let light = ambient + beam * lighting::beam_vertical(screen_y);

            let texel = texture.column_texel(texture_column, v);

            framebuffer.pixel(i, y, lighting::apply(texel, light * face));
        }
    }

    depth
}

/// Pinta techo y piso con su propio degradado de luz por distancia.
///
/// Un color plano arruina la penumbra: por más que las paredes se oscurezcan al
/// fondo, un piso uniforme delata que no hay profundidad. Cada fila de pantalla
/// corresponde a una distancia concreta del piso, así que se calcula esa
/// distancia y se ilumina la fila entera de un solo `fill`.
///
/// Entre las dos mitades cubren la pantalla completa, así que esta vista no
/// necesita `clear()` previo.
fn draw_ceiling_and_floor(
    framebuffer: &mut Framebuffer,
    distance_to_projection_plane: f32,
    half_height: f32,
    intensity: f32,
) {
    let height = framebuffer.height;

    for y in (half_height as usize + 1)..height {
        // A qué distancia del jugador está el punto del piso que se ve en esta
        // fila. Por triángulos semejantes, con el ojo a media pared:
        //     d = dpp * altura_del_ojo / (y - horizonte)
        // Justo en el horizonte el denominador es cero y la distancia infinita,
        // por eso el rango arranca una fila más abajo.
        let rows_below_horizon = y as f32 - half_height;
        let distance = distance_to_projection_plane * EYE_HEIGHT / rows_below_horizon;

        // El cono se evalúa en el centro horizontal porque la fila se pinta
        // completa de un solo `fill`; la caída vertical sí es exacta, y es la
        // que domina en piso y techo.
        let screen_y = rows_below_horizon / half_height;
        let light = lighting::ambient(distance)
            + lighting::beam(distance, 0.0) * intensity * lighting::beam_vertical(screen_y);

        // el techo es el espejo del piso: misma distancia, misma luz.
        let mirrored = (half_height * 2.0 - y as f32) as usize;

        framebuffer.fill_rows(y, y + 1, lighting::apply(FLOOR_COLOR, light));
        framebuffer.fill_rows(mirrored, mirrored + 1, lighting::apply(CEILING_COLOR, light));
    }

    // la fila exacta del horizonte queda sin pintar por la división por cero:
    // se le da el color más lejano posible.
    let horizon = half_height as usize;
    let far = lighting::ambient(f32::MAX);
    framebuffer.fill_rows(horizon, horizon + 1, lighting::apply(FLOOR_COLOR, far));
}
