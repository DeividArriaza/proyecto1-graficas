use minifb::{Key, MouseMode, Window};
use nalgebra_glm::Vec2;
use std::f32::consts::TAU;

use crate::maze::{cell_at, Maze};

// ---------------------------------------------------------------------------
// Velocidades del jugador, en unidades **por segundo**.
//
// Antes eran por cuadro, lo que ataba la velocidad a los fps: la misma tecla
// movía el doble de lejos en una máquina que corría al doble de rápido. A 60 fps
// coincidían, pero la rúbrica sólo exige 15, y a 15 fps el jugador se habría
// arrastrado a un cuarto de velocidad. Ahora todo se multiplica por el tiempo
// del cuadro y el juego se siente igual sin importar el rendimiento.
// ---------------------------------------------------------------------------

/// Píxeles por segundo caminando. Con bloques de 100 px, son 3 bloques por
/// segundo.
pub const WALK_SPEED: f32 = 300.0;

/// Cuánto multiplica la velocidad el correr.
///
/// En 1.9 correr se siente claramente distinto de caminar sin volver el
/// laberinto incontrolable en los pasillos de un bloque de ancho.
pub const RUN_MULTIPLIER: f32 = 1.9;

/// Radianes por segundo al girar con el teclado. PI = media vuelta por segundo.
pub const ROTATION_SPEED: f32 = std::f32::consts::PI;

/// Radianes de giro por píxel de movimiento del mouse.
///
/// A diferencia del teclado, esto **no** se multiplica por el tiempo del cuadro.
/// El mouse ya entrega un desplazamiento acumulado: si un cuadro tardó el doble,
/// el cursor recorrió el doble de píxeles y el giro sale correcto solo.
/// Multiplicarlo por el delta aplicaría el tiempo dos veces y el giro se
/// volvería más lento cuanto peor fuera el rendimiento.
///
/// En 0.004, cruzar la ventana de 1300 px de lado a lado gira unos 300 grados.
pub const MOUSE_SENSITIVITY: f32 = 0.004;

/// Salto de cursor, en píxeles, que se considera un artefacto y se descarta.
///
/// El cursor puede aparecer de golpe en otro punto al recuperar el foco de la
/// ventana, al volver de otro escritorio o al reengancharse tras salirse de la
/// pantalla. Sin este tope, ese salto se traduciría en un giro brusco de varias
/// vueltas.
const MAX_MOUSE_JUMP: f32 = 200.0;

/// Radio del jugador en píxeles: qué tan cerca de una pared puede quedar.
///
/// Sin este margen el jugador se pega a la pared y la estaca de esa columna
/// crece hasta tapar toda la pantalla. Con 20 px sobre bloques de 100 px, la
/// pared más cercana posible queda a 1/5 de bloque.
pub const COLLISION_MARGIN: f32 = 20.0;

/// Desplazamiento máximo que se evalúa de una sola vez, en píxeles.
///
/// El movimiento de un cuadro se parte en tramos de a lo sumo este tamaño. Sin
/// eso, un cuadro muy largo —correr a 15 fps, o la ventana recién arrastrada—
/// movería al jugador más de 100 px de golpe, y la comprobación de colisión sólo
/// mira el punto de destino: atravesaría una pared completa sin tocarla nunca.
const MAX_SUBSTEP: f32 = 20.0;

/// Seguimiento del mouse para girar la cámara.
///
/// `minifb` no da movimiento relativo ni permite capturar el puntero, sólo la
/// posición absoluta. Así que el desplazamiento se calcula guardando la posición
/// del cuadro anterior y restando.
///
/// Limitación que eso impone: cuando el cursor llega al borde físico de la
/// pantalla deja de moverse, y el giro se detiene hasta que lo traés de vuelta.
/// No hay forma de evitarlo sin capturar el puntero, que `minifb` no expone. El
/// teclado sigue girando sin límite, así que nunca quedás sin poder mirar.
pub struct MouseLook {
    /// Posición horizontal del cuadro anterior. `None` cuando no hay
    /// referencia válida todavía.
    last_x: Option<f32>,
}

impl MouseLook {
    pub fn new() -> Self {
        MouseLook { last_x: None }
    }

    /// Cuánto hay que girar, en radianes, por lo que se movió el mouse.
    ///
    /// Devuelve 0.0 en el primer cuadro: hace falta una posición anterior para
    /// poder restar, y sin ella el giro saldría de la nada.
    pub fn delta_angle(&mut self, window: &mut Window) -> f32 {
        // Sin foco el cursor está en otra aplicación, y su posición no debe
        // contarse. `None` también sirve para olvidar la referencia, así que al
        // volver no se acumula el trayecto que hizo afuera.
        //
        // `Pass` entrega la posición incluso fuera de la ventana, que es lo que
        // se quiere: girar no debería cortarse porque el cursor se pasó del borde
        // por unos píxeles.
        let position = if window.is_active() {
            window.get_mouse_pos(MouseMode::Pass).map(|(x, _)| x)
        } else {
            None
        };

        self.advance(position)
    }

    /// El cálculo, separado de la ventana para poder verificarlo.
    ///
    /// `None` significa "no hay lectura válida": sin foco, o sin posición. En ese
    /// caso se olvida la referencia y no hay giro.
    fn advance(&mut self, position: Option<f32>) -> f32 {
        let Some(x) = position else {
            self.last_x = None;
            return 0.0;
        };

        let previous = self.last_x.replace(x);

        let Some(previous) = previous else {
            return 0.0;
        };

        let movement = x - previous;

        // Un salto enorme es un artefacto, no un gesto. Se descarta el giro pero
        // la referencia ya quedó actualizada, así que el movimiento siguiente se
        // mide contra la posición nueva y no vuelve a saltar.
        if movement.abs() > MAX_MOUSE_JUMP {
            return 0.0;
        }

        movement * MOUSE_SENSITIVITY
    }

    /// Olvida la posición de referencia.
    ///
    /// Hay que llamarla al volver de la pausa: mientras el juego estaba detenido
    /// el cursor pudo recorrer media pantalla, y sin esto todo ese trayecto se
    /// aplicaría de golpe como un giro al reanudar.
    pub fn reset(&mut self) {
        self.last_x = None;
    }
}

pub struct Player {
    /// Posición en píxeles dentro del laberinto.
    pub pos: Vec2,
    /// Ángulo de vista en radianes.
    pub a: f32,
}

/// ¿La celda que contiene el punto (x, y) bloquea el paso?
///
/// La meta `g` es transitable a propósito: `cast_ray` sí la trata como pared
/// (por eso se ve como un muro al frente), pero el jugador tiene que poder
/// entrar en ella para que se dispare la condición de victoria.
fn is_wall(maze: &Maze, x: f32, y: f32) -> bool {
    let cell = cell_at(maze, x, y);

    cell != ' ' && cell != 'g' && cell != 'G'
}

/// Aplica el desplazamiento (dx, dy) partiéndolo en tramos cortos.
///
/// Partirlo es lo que hace que la colisión aguante cualquier velocidad y
/// cualquier ritmo de cuadros: cada tramo mide como máximo `MAX_SUBSTEP`, así
/// que nunca hay un salto lo bastante grande para cruzar una pared entera.
fn try_move(player: &mut Player, maze: &Maze, dx: f32, dy: f32) {
    let distance = dx.abs().max(dy.abs());
    let steps = (distance / MAX_SUBSTEP).ceil().max(1.0);
    let count = steps as usize;

    let step_x = dx / steps;
    let step_y = dy / steps;

    for _ in 0..count {
        slide(player, maze, step_x, step_y);
    }
}

/// Mueve un tramo, evaluando cada eje por separado.
///
/// Evaluarlos por separado es lo que permite *deslizarse* a lo largo de una
/// pared: si avanzás en diagonal contra un muro vertical, el eje X se bloquea
/// pero el eje Y sigue libre, en vez de frenar al jugador por completo.
///
/// El punto que se consulta no es el destino exacto sino el destino corrido
/// `COLLISION_MARGIN` píxeles en la dirección del movimiento, de modo que el
/// jugador se detenga *antes* de tocar la pared y nunca quede dentro de ella.
fn slide(player: &mut Player, maze: &Maze, dx: f32, dy: f32) {
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

/// ¿Se está corriendo?
///
/// Sirven Shift y Ctrl, de cualquiera de los dos lados. Ctrl no es un capricho:
/// es el respaldo por si el juego termina corriendo sobre el backend de Wayland
/// de `minifb`, donde Shift rompe la lectura de WASD —traduce las teclas con los
/// modificadores aplicados, no encuentra los keysyms en mayúscula y descarta el
/// evento, dejando la tecla pegada—. Por eso el `Cargo.toml` fuerza X11. Ctrl no
/// cambia el nivel del teclado en ninguno de los dos backends, así que funciona
/// siempre.
pub fn is_running(window: &Window) -> bool {
    window.is_key_down(Key::LeftShift)
        || window.is_key_down(Key::RightShift)
        || window.is_key_down(Key::LeftCtrl)
        || window.is_key_down(Key::RightCtrl)
}

pub fn process_events(
    window: &mut Window,
    player: &mut Player,
    maze: &Maze,
    mouse: &mut MouseLook,
    delta: f32,
) {
    // A y D solo cambian el ángulo de vista: el jugador gira sobre su eje, así
    // que girar nunca puede meterlo dentro de una pared.
    if window.is_key_down(Key::A) {
        player.a -= ROTATION_SPEED * delta;
    }

    if window.is_key_down(Key::D) {
        player.a += ROTATION_SPEED * delta;
    }

    // El mouse gira sólo en horizontal, que es lo único que este raycaster puede
    // representar: la cámara no tiene inclinación vertical, el horizonte está
    // fijo a media pantalla.
    player.a += mouse.delta_angle(window);

    // El ángulo se mantiene dentro de [0, 2PI) para que no crezca sin límite
    // tras muchos giros y pierda precisión en f32.
    player.a = player.a.rem_euclid(TAU);

    // W y S se mueven sobre la dirección de vista, de modo que avanzar siempre
    // ocurre hacia donde el jugador está viendo:
    //     x += velocidad * cos(a)
    //     y += velocidad * sin(a)
    let (sin_a, cos_a) = player.a.sin_cos();

    let speed = if is_running(window) {
        WALK_SPEED * RUN_MULTIPLIER
    } else {
        WALK_SPEED
    } * delta;

    if window.is_key_down(Key::W) {
        try_move(player, maze, speed * cos_a, speed * sin_a);
    }

    if window.is_key_down(Key::S) {
        try_move(player, maze, -speed * cos_a, -speed * sin_a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Laberinto de 5x3 con un pasillo horizontal en el medio.
    ///
    ///     +-+-+
    ///     |   |
    ///     +-+-+
    fn corridor() -> Maze {
        vec![
            "+-+-+".chars().collect(),
            "|   |".chars().collect(),
            "+-+-+".chars().collect(),
        ]
    }

    fn player_at(x: f32, y: f32) -> Player {
        Player {
            pos: Vec2::new(x, y),
            a: 0.0,
        }
    }

    #[test]
    fn el_borde_del_laberinto_es_pared() {
        let maze = corridor();

        assert!(is_wall(&maze, -1.0, 150.0), "fuera por la izquierda");
        assert!(is_wall(&maze, 150.0, -1.0), "fuera por arriba");
        assert!(is_wall(&maze, 9999.0, 150.0), "fuera por la derecha");
        assert!(is_wall(&maze, 150.0, 9999.0), "fuera por abajo");
    }

    #[test]
    fn la_meta_es_transitable() {
        let maze: Maze = vec!["|g|".chars().collect()];

        assert!(!is_wall(&maze, 150.0, 50.0), "la meta debe poder pisarse");
        assert!(is_wall(&maze, 50.0, 50.0), "la pared no");
    }

    #[test]
    fn no_se_atraviesa_una_pared() {
        let maze = corridor();

        // en el centro del pasillo, empujando hacia la pared de arriba
        let mut player = player_at(250.0, 150.0);
        try_move(&mut player, &maze, 0.0, -500.0);

        assert!(
            player.pos.y > 100.0,
            "quedó en y={}, dentro de la pared de arriba",
            player.pos.y
        );
    }

    #[test]
    fn se_desliza_a_lo_largo_de_una_pared() {
        let maze = corridor();

        // diagonal contra la pared de arriba: el eje Y se bloquea, el X avanza.
        let mut player = player_at(250.0, 150.0);
        let before_x = player.pos.x;

        try_move(&mut player, &maze, 30.0, -500.0);

        assert!(
            player.pos.x > before_x,
            "el eje X debería seguir libre, quedó en {}",
            player.pos.x
        );
        assert!(player.pos.y > 100.0, "el eje Y debería estar bloqueado");
    }

    /// Sin subdivisión del movimiento, un desplazamiento mayor que el grosor de
    /// una pared la cruzaría sin tocarla: la comprobación sólo mira el destino.
    /// Esto es lo que puede pasar corriendo con un cuadro largo.
    #[test]
    fn un_salto_enorme_no_atraviesa_la_pared() {
        let maze = corridor();

        let mut player = player_at(250.0, 150.0);
        try_move(&mut player, &maze, 0.0, -5000.0);

        assert!(
            player.pos.y > 100.0,
            "atravesó la pared de un salto: quedó en y={}",
            player.pos.y
        );

        // y tampoco hacia el otro lado
        let mut player = player_at(250.0, 150.0);
        try_move(&mut player, &maze, 5000.0, 0.0);

        assert!(
            player.pos.x < 400.0,
            "atravesó la pared derecha: quedó en x={}",
            player.pos.x
        );
    }

    #[test]
    fn el_movimiento_libre_avanza_lo_pedido() {
        // laberinto ancho, sin nada que estorbe cerca
        let maze: Maze = (0..5)
            .map(|_| "          ".chars().collect::<Vec<char>>())
            .collect();

        let mut player = player_at(250.0, 250.0);
        try_move(&mut player, &maze, 40.0, 0.0);

        assert!(
            (player.pos.x - 290.0).abs() < 0.001,
            "esperaba x=290, quedó en {}",
            player.pos.x
        );
    }

    #[test]
    fn el_margen_de_colision_deja_espacio() {
        let maze = corridor();

        // empujar hasta el fondo contra la pared de arriba, muchas veces
        let mut player = player_at(250.0, 150.0);
        for _ in 0..50 {
            try_move(&mut player, &maze, 0.0, -5.0);
        }

        // la pared de arriba termina en y=100; con margen de 20 no debería
        // acercarse más que eso.
        assert!(
            player.pos.y >= 100.0 + COLLISION_MARGIN - 5.0,
            "se pegó demasiado a la pared: y={}",
            player.pos.y
        );
    }
}

#[cfg(test)]
mod mouse_tests {
    use super::*;

    #[test]
    fn el_primer_cuadro_no_gira() {
        let mut mouse = MouseLook::new();

        // sin posición anterior no hay nada que restar
        assert_eq!(mouse.advance(Some(640.0)), 0.0);
    }

    #[test]
    fn el_desplazamiento_se_traduce_en_giro() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(600.0));

        let turn = mouse.advance(Some(700.0));

        assert!(
            (turn - 100.0 * MOUSE_SENSITIVITY).abs() < 1e-6,
            "100 px deberían girar {} rad, dio {turn}",
            100.0 * MOUSE_SENSITIVITY
        );
    }

    #[test]
    fn el_giro_sigue_la_direccion_del_mouse() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(600.0));
        assert!(mouse.advance(Some(650.0)) > 0.0, "a la derecha, giro positivo");

        mouse.advance(Some(650.0));
        assert!(mouse.advance(Some(600.0)) < 0.0, "a la izquierda, giro negativo");
    }

    #[test]
    fn quedarse_quieto_no_gira() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(500.0));

        for _ in 0..10 {
            assert_eq!(mouse.advance(Some(500.0)), 0.0);
        }
    }

    /// Un salto grande —recuperar el foco, volver de otro escritorio— no debe
    /// traducirse en varias vueltas de golpe.
    #[test]
    fn un_salto_enorme_se_descarta() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(100.0));

        assert_eq!(
            mouse.advance(Some(100.0 + MAX_MOUSE_JUMP + 1.0)),
            0.0,
            "el salto debe ignorarse"
        );

        // pero la referencia quedó actualizada: el movimiento siguiente se mide
        // contra la posición nueva, no contra la vieja.
        let turn = mouse.advance(Some(100.0 + MAX_MOUSE_JUMP + 11.0));

        assert!(
            (turn - 10.0 * MOUSE_SENSITIVITY).abs() < 1e-6,
            "tras el salto, 10 px deberían girar normal; dio {turn}"
        );
    }

    /// Perder el foco tiene que borrar la referencia. Si no, el trayecto que el
    /// cursor hizo en otra aplicación se aplicaría entero al volver.
    #[test]
    fn perder_el_foco_olvida_la_referencia() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(100.0));
        assert_eq!(mouse.advance(None), 0.0, "sin lectura no hay giro");

        // al volver, el primer cuadro es de referencia otra vez
        assert_eq!(mouse.advance(Some(900.0)), 0.0, "no debe girar de golpe");

        // y a partir de ahí funciona normal
        assert!(mouse.advance(Some(910.0)) > 0.0);
    }

    #[test]
    fn reiniciar_tambien_olvida_la_referencia() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(100.0));
        mouse.reset();

        assert_eq!(mouse.advance(Some(800.0)), 0.0, "tras reset no debe girar");
    }

    /// Cruzar la ventana de lado a lado debería dar una vuelta parcial pero
    /// amplia. Si este número se va muy lejos, la sensibilidad quedó mal.
    #[test]
    fn cruzar_la_pantalla_gira_una_vuelta_parcial() {
        let mut mouse = MouseLook::new();

        mouse.advance(Some(0.0));

        let mut total = 0.0;
        for x in 1..1300 {
            total += mouse.advance(Some(x as f32));
        }

        let degrees = total.to_degrees();

        assert!(
            (240.0..360.0).contains(&degrees),
            "cruzar 1300 px giró {degrees:.0} grados; se esperaba entre 240 y 360"
        );
    }
}
