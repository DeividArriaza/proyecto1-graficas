//! El monstruo del laberinto: dónde está y en qué cuadro de su animación va.
//!
//! No dibuja nada y no sabe qué imagen lo representa. Sólo lleva su posición en
//! el mundo y el avance de su ciclo de animación; `render::billboard` se encarga
//! de proyectarlo.
//!
//! # Cómo se mueve
//!
//! Patrulla, no persigue. Camina en línea recta hasta topar con una pared y ahí
//! elige otra dirección, evitando volver por donde vino salvo en un callejón sin
//! salida. Eso es todo: no hay búsqueda de caminos ni conocimiento de dónde está
//! el jugador.
//!
//! Es deliberado, y no sólo por simplicidad. Un monstruo que te persigue exige
//! un juego distinto —hay que poder escapar, esconderse, morir de forma
//! entendible—. Uno que ronda a ciegas por los pasillos da la misma tensión con
//! una fracción de las reglas: el peligro está en no saber dónde está, y eso ya
//! lo resuelve la oscuridad.

use nalgebra_glm::Vec2;

use crate::maze::{cell_at, Maze, BLOCK_SIZE};
use crate::mazegen::Rng;

/// Cuánto dura cada cuadro de la animación, en segundos.
///
/// 0.18 s da unos 5.5 cuadros por segundo. Con 4 cuadros el ciclo completo dura
/// 0.72 s, que se lee como algo respirando y no como un parpadeo nervioso.
const FRAME_SECONDS: f32 = 0.18;

/// A qué distancia mínima del jugador puede aparecer, en píxeles.
///
/// Tres bloques: lo bastante lejos para que no esté encima al empezar, pero sin
/// exigir tanto que en un laberinto chico no haya dónde ponerlo.
const MIN_SPAWN_DISTANCE: f32 = 3.0 * BLOCK_SIZE as f32;

/// A qué distancia mínima de la meta puede aparecer, en píxeles.
///
/// Hace falta porque el criterio de "la celda más lejana del jugador" es casi el
/// mismo con el que `mazegen` coloca la meta: las dos terminan en el extremo
/// opuesto del laberinto. Sin esta restricción el monstruo puede quedar pegado a
/// la salida y bloquearla, y entonces ganar exige caminar hacia él.
const MIN_GOAL_DISTANCE: f32 = 4.0 * BLOCK_SIZE as f32;

/// A qué distancia el monstruo atrapa al jugador, en píxeles.
///
/// Algo más que el radio de colisión del jugador (20 px), para que el momento en
/// que te atrapa coincida con verlo encima y no con haberlo atravesado.
const CATCH_DISTANCE: f32 = 38.0;

/// A qué velocidad patrulla, en píxeles por segundo.
///
/// El jugador camina a 300 px/s, así que 45 es siete veces más lento: se mueve lo
/// suficiente para que el laberinto no se sienta un museo, pero nunca te alcanza
/// si estás huyendo. La amenaza es cruzártelo, no que te corra.
const PATROL_SPEED: f32 = 45.0;

/// Cuánto espacio deja respecto a las paredes, en píxeles.
///
/// Sin margen el sprite se mete en la pared y se ve la mitad recortada al pasar
/// por un pasillo angosto.
const WALL_MARGIN: f32 = 26.0;

/// Las cuatro direcciones en las que puede caminar.
const DIRECTIONS: [(f32, f32); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];

pub struct Monster {
    pub pos: Vec2,
    /// Hacia dónde camina. Siempre es una de las cuatro cardinales.
    direction: Vec2,
    /// Cuadro actual de la animación.
    frame: usize,
    /// Tiempo acumulado en el cuadro actual.
    timer: f32,
    /// Generador propio, sembrado con su posición inicial.
    ///
    /// Que sea determinista importa: el mismo nivel produce la misma ronda, así
    /// que reintentar un nivel fijo no cambia el recorrido del monstruo y las
    /// pruebas pueden verificarlo.
    rng: Rng,
}

impl Monster {
    /// Lo coloca en la celda transitable más lejana del jugador que no sea la
    /// meta.
    ///
    /// La meta se excluye porque ahí termina el nivel: un monstruo encima de la
    /// salida sería lo último que se ve, y por un cuadro. La celda más lejana
    /// suele ser un callejón sin salida, que es justo donde uno no quiere
    /// encontrarse algo.
    ///
    /// Devuelve `None` si el laberinto no tiene ninguna celda lo bastante lejos.
    pub fn spawn(maze: &Maze, player: Vec2) -> Option<Monster> {
        let goal = find_goal(maze);

        // Se intenta respetar las dos distancias mínimas y, si el laberinto es
        // demasiado chico para eso, se relaja la de la meta. Un monstruo cerca de
        // la salida es peor que uno lejos, pero mejor que ninguno.
        let pos = pick_cell(maze, player, goal, MIN_GOAL_DISTANCE)
            .or_else(|| pick_cell(maze, player, goal, 0.0))?;

        // la semilla sale de la posición: distinta por nivel, igual entre
        // partidas del mismo nivel.
        let seed = (pos.x as u64).wrapping_mul(73_856_093) ^ (pos.y as u64).wrapping_mul(19_349_663);

        let mut monster = Monster {
            pos,
            direction: Vec2::new(1.0, 0.0),
            frame: 0,
            timer: 0.0,
            rng: Rng::new(seed),
        };

        // arranca mirando a un pasillo libre, no contra una pared.
        monster.turn(maze);

        Some(monster)
    }

    /// Camina y avanza la animación.
    pub fn update(&mut self, maze: &Maze, delta: f32, frames: usize) {
        self.walk(maze, delta);
        self.animate(delta, frames);
    }

    /// Avanza en línea recta y, al topar con algo, elige otra dirección.
    ///
    /// El paso se comprueba contra el punto de destino corrido `WALL_MARGIN` en
    /// la dirección del movimiento, igual que el del jugador: así se detiene
    /// antes de tocar la pared en vez de quedar incrustado en ella.
    fn walk(&mut self, maze: &Maze, delta: f32) {
        let step = PATROL_SPEED * delta;
        let target = self.pos + self.direction * step;
        let probe = target + self.direction * WALL_MARGIN;

        if is_blocked(maze, probe) {
            self.turn(maze);
            return;
        }

        self.pos = target;
    }

    /// Elige una dirección libre, evitando deshacer el camino.
    ///
    /// Volver por donde vino queda como último recurso: sin esa preferencia, en
    /// un pasillo el monstruo se quedaría vibrando entre dos celdas. En un
    /// callejón sin salida el reverso es la única salida, y entonces sí se toma.
    fn turn(&mut self, maze: &Maze) {
        let reverse = -self.direction;

        let mut options = [Vec2::new(0.0, 0.0); 4];
        let mut count = 0;
        let mut reverse_is_free = false;

        for (dx, dy) in DIRECTIONS {
            let candidate = Vec2::new(dx, dy);
            let probe = self.pos + candidate * (WALL_MARGIN + PATROL_SPEED);

            if is_blocked(maze, probe) {
                continue;
            }

            // el reverso se guarda aparte, para usarlo sólo si no hay otra
            if (candidate - reverse).norm() < 0.01 {
                reverse_is_free = true;
                continue;
            }

            options[count] = candidate;
            count += 1;
        }

        if count > 0 {
            self.direction = options[self.rng.below(count)];
        } else if reverse_is_free {
            self.direction = reverse;
        }
        // si nada está libre, se queda quieto y lo reintenta el cuadro siguiente
    }

    /// Avanza la animación. `frames` es cuántos cuadros tiene el ciclo.
    ///
    /// El temporizador se descuenta en un `while` y no en un `if`: con un cuadro
    /// muy largo —o un ciclo muy rápido— podría haber que saltar más de un cuadro
    /// de golpe, y un `if` dejaría la animación arrastrándose detrás del tiempo
    /// real.
    fn animate(&mut self, delta: f32, frames: usize) {
        if frames == 0 {
            return;
        }

        self.timer += delta;

        while self.timer >= FRAME_SECONDS {
            self.timer -= FRAME_SECONDS;
            self.frame = (self.frame + 1) % frames;
        }
    }

    pub fn frame(&self) -> usize {
        self.frame
    }

    /// ¿Alcanzó al jugador?
    ///
    /// Se compara la distancia al cuadrado para no calcular una raíz cuadrada en
    /// cada cuadro. No cambia el resultado: comparar distancias y comparar sus
    /// cuadrados da lo mismo mientras las dos sean positivas.
    pub fn catches(&self, player: Vec2) -> bool {
        let offset = self.pos - player;

        offset.x * offset.x + offset.y * offset.y <= CATCH_DISTANCE * CATCH_DISTANCE
    }
}

/// ¿El monstruo puede pararse en este punto?
///
/// Todo lo que no sea piso lo bloquea, **incluida la meta**. Que la salida sea
/// infranqueable para él es lo que garantiza que nunca se pare encima y la
/// vuelva intomable.
fn is_blocked(maze: &Maze, point: Vec2) -> bool {
    cell_at(maze, point.x, point.y) != ' '
}

/// Centro en píxeles de la celda (col, row).
fn cell_center(col: usize, row: usize) -> Vec2 {
    Vec2::new(
        (col * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
        (row * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
    )
}

/// Dónde está la meta, si el laberinto tiene una.
fn find_goal(maze: &Maze) -> Option<Vec2> {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            if cell == 'g' || cell == 'G' {
                return Some(cell_center(col, row));
            }
        }
    }

    None
}

/// La celda transitable más lejana del jugador que respete las dos distancias
/// mínimas.
///
/// `min_goal` se pasa por parámetro para poder reintentar con la restricción
/// relajada cuando el laberinto no da para más.
fn pick_cell(maze: &Maze, player: Vec2, goal: Option<Vec2>, min_goal: f32) -> Option<Vec2> {
    let mut best: Option<(f32, Vec2)> = None;

    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            if cell != ' ' {
                continue;
            }

            let center = cell_center(col, row);
            let from_player = (center - player).norm();

            if from_player < MIN_SPAWN_DISTANCE {
                continue;
            }

            if let Some(goal) = goal {
                if (center - goal).norm() < min_goal {
                    continue;
                }
            }

            if best.is_none_or(|(far, _)| from_player > far) {
                best = Some((from_player, center));
            }
        }
    }

    best.map(|(_, pos)| pos)
}

/// Constructor mínimo para pruebas.
#[cfg(test)]
fn test_monster(x: f32, y: f32) -> Monster {
    Monster {
        pos: Vec2::new(x, y),
        direction: Vec2::new(1.0, 0.0),
        frame: 0,
        timer: 0.0,
        rng: Rng::new(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::game::{Session, LEVELS};

    /// Los cuatro niveles tienen que poder alojar al monstruo, y siempre lejos.
    #[test]
    fn aparece_lejos_en_todos_los_niveles() {
        for (index, level) in LEVELS.iter().enumerate() {
            let session = Session::start(index);

            let monster = session
                .monster
                .as_ref()
                .unwrap_or_else(|| panic!("{}: sin monstruo", level.name));

            let distance = (monster.pos - session.player.pos).norm();

            assert!(
                distance >= MIN_SPAWN_DISTANCE,
                "{}: monstruo a {distance:.0} px, se pedían {MIN_SPAWN_DISTANCE:.0}",
                level.name
            );

            // y tiene que estar en piso, nunca dentro de una pared.
            let i = monster.pos.x as usize / BLOCK_SIZE;
            let j = monster.pos.y as usize / BLOCK_SIZE;

            assert_eq!(
                session.maze[j][i], ' ',
                "{}: el monstruo quedó dentro de '{}'",
                level.name, session.maze[j][i]
            );
        }
    }

    /// La animación tiene que dar la vuelta y no salirse del rango de cuadros.
    #[test]
    fn la_animacion_cicla() {
        let mut monster = test_monster(0.0, 0.0);

        // avanzar justo un cuadro
        monster.animate(FRAME_SECONDS, 4);
        assert_eq!(monster.frame(), 1);

        // un salto grande no debe dejar la animación atrasada: se consumen todos
        // los cuadros que quepan en ese tiempo.
        monster.animate(FRAME_SECONDS * 2.0, 4);
        assert_eq!(monster.frame(), 3);

        // y da la vuelta en vez de desbordar
        monster.animate(FRAME_SECONDS, 4);
        assert_eq!(monster.frame(), 0);

        // con cero cuadros no debe entrar en pánico ni dividir por cero
        monster.animate(FRAME_SECONDS, 0);
        assert_eq!(monster.frame(), 0);
    }
}

#[cfg(test)]
mod catch_tests {
    use super::*;

    fn monster_at(x: f32, y: f32) -> Monster {
        test_monster(x, y)
    }

    #[test]
    fn atrapa_de_cerca_y_no_de_lejos() {
        let monster = monster_at(200.0, 200.0);

        assert!(monster.catches(Vec2::new(200.0, 200.0)), "encima");
        assert!(monster.catches(Vec2::new(220.0, 200.0)), "a 20 px");
        assert!(!monster.catches(Vec2::new(300.0, 200.0)), "a 100 px");
        assert!(!monster.catches(Vec2::new(200.0, 400.0)), "a 200 px");
    }

    /// El límite se comprueba en las dos direcciones porque se calcula con
    /// distancias al cuadrado: un signo mal puesto ahí pasaría desapercibido
    /// salvo justo en el borde.
    #[test]
    fn el_limite_de_captura_es_simetrico() {
        let monster = monster_at(200.0, 200.0);

        let inside = CATCH_DISTANCE - 1.0;
        let outside = CATCH_DISTANCE + 1.0;

        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            assert!(
                monster.catches(Vec2::new(200.0 + dx * inside, 200.0 + dy * inside)),
                "debería atrapar a {inside} px en dirección ({dx}, {dy})"
            );
            assert!(
                !monster.catches(Vec2::new(200.0 + dx * outside, 200.0 + dy * outside)),
                "no debería atrapar a {outside} px en dirección ({dx}, {dy})"
            );
        }
    }

    /// Diagnóstico: qué tan lejos queda el monstruo de la meta en cada nivel.
    ///
    ///     cargo test -- --ignored --nocapture distancia_a_la_meta
    #[test]
    #[ignore]
    fn distancia_a_la_meta() {
        use crate::game::{Session, LEVELS};
        use crate::maze::BLOCK_SIZE;

        for (index, level) in LEVELS.iter().enumerate() {
            let session = Session::start(index);

            let mut goal = None;
            for (row, line) in session.maze.iter().enumerate() {
                for (col, &cell) in line.iter().enumerate() {
                    if cell == 'g' || cell == 'G' {
                        goal = Some(Vec2::new(
                            (col * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
                            (row * BLOCK_SIZE + BLOCK_SIZE / 2) as f32,
                        ));
                    }
                }
            }

            match (session.monster.as_ref(), goal) {
                (Some(monster), Some(goal)) => {
                    let to_goal = (monster.pos - goal).norm();
                    let to_player = (monster.pos - session.player.pos).norm();

                    println!(
                        "{}: monstruo a {:.0} px de la meta ({:.1} bloques) y a {:.0} px del inicio",
                        level.name,
                        to_goal,
                        to_goal / BLOCK_SIZE as f32,
                        to_player,
                    );
                }
                _ => println!("{}: falta monstruo o meta", level.name),
            }
        }
    }

    /// Después de patrullar mucho tiempo, el monstruo nunca puede haber quedado
    /// dentro de una pared ni encima de la meta.
    ///
    /// Es la prueba que importa del movimiento: no interesa por dónde anduvo,
    /// interesa que en ningún momento haya atravesado algo. Se simulan 40
    /// segundos de ronda sobre 30 laberintos distintos.
    #[test]
    fn la_ronda_nunca_atraviesa_paredes() {
        use crate::maze::{cell_at, extract_player};
        use crate::mazegen;

        for seed in 1..30u64 {
            let mut maze = mazegen::generate(10, 8, seed);
            let player = extract_player(&mut maze);

            let Some(mut monster) = Monster::spawn(&maze, player.pos) else {
                continue;
            };

            // 40 s a pasos de 1/60 s
            for tick in 0..2400 {
                monster.update(&maze, 1.0 / 60.0, 4);

                let cell = cell_at(&maze, monster.pos.x, monster.pos.y);

                assert_eq!(
                    cell, ' ',
                    "semilla {seed}, paso {tick}: el monstruo quedó en '{cell}' \
                     en ({:.0}, {:.0})",
                    monster.pos.x, monster.pos.y
                );
            }
        }
    }

    /// Y tiene que moverse de verdad: un monstruo que se queda vibrando en el
    /// lugar cumpliría la prueba anterior sin patrullar nada.
    #[test]
    fn la_ronda_recorre_distancia() {
        use crate::maze::extract_player;
        use crate::mazegen;

        let mut quietos = 0;

        for seed in 1..30u64 {
            let mut maze = mazegen::generate(10, 8, seed);
            let player = extract_player(&mut maze);

            let Some(mut monster) = Monster::spawn(&maze, player.pos) else {
                continue;
            };

            let start = monster.pos;

            for _ in 0..1200 {
                monster.update(&maze, 1.0 / 60.0, 4);
            }

            // en 20 s a 45 px/s podría recorrer 900 px; con los rebotes de un
            // laberinto, exigir un bloque de desplazamiento neto es prudente.
            if (monster.pos - start).norm() < BLOCK_SIZE as f32 {
                quietos += 1;
            }
        }

        assert!(
            quietos <= 3,
            "{quietos} monstruos de 29 se quedaron casi en el lugar"
        );
    }

    /// El monstruo no puede quedar pegado a la meta: bloquearía la salida y
    /// ganar exigiría caminar hacia él.
    ///
    /// Se prueba sobre 60 laberintos generados, no sobre los cuatro niveles: el
    /// modo infinito produce uno distinto cada partida, y el caso malo aparecía
    /// justamente ahí.
    #[test]
    fn nunca_aparece_pegado_a_la_meta() {
        use crate::maze::extract_player;
        use crate::mazegen;

        let mut checked = 0;

        for seed in 1..60u64 {
            let mut maze = mazegen::generate(10 + seed as usize % 10, 8 + seed as usize % 6, seed);
            let player = extract_player(&mut maze);

            let Some(monster) = Monster::spawn(&maze, player.pos) else {
                continue;
            };

            let Some(goal) = find_goal(&maze) else {
                continue;
            };

            let distance = (monster.pos - goal).norm();

            assert!(
                distance >= MIN_GOAL_DISTANCE,
                "semilla {seed}: monstruo a {distance:.0} px de la meta, se pedían {MIN_GOAL_DISTANCE:.0}"
            );

            checked += 1;
        }

        assert!(checked > 40, "sólo se comprobaron {checked} laberintos");
    }

    /// Al empezar el nivel el monstruo no puede estar ya encima: eso perdería la
    /// partida en el primer cuadro, sin que el jugador toque una tecla.
    #[test]
    fn no_atrapa_al_arrancar_ningun_nivel() {
        use crate::game::{Session, LEVELS};

        for (index, level) in LEVELS.iter().enumerate() {
            let session = Session::start(index);

            if let Some(monster) = session.monster.as_ref() {
                assert!(
                    !monster.catches(session.player.pos),
                    "{}: el monstruo atrapa al jugador en el cuadro cero",
                    level.name
                );
            }
        }
    }
}
