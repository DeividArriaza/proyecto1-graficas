//! El monstruo del laberinto: dónde está y en qué cuadro de su animación va.
//!
//! No dibuja nada y no sabe qué imagen lo representa. Sólo lleva su posición en
//! el mundo y el avance de su ciclo de animación; `render::billboard` se encarga
//! de proyectarlo.
//!
//! Está quieto a propósito. La rúbrica pide una animación, no un enemigo, y
//! perseguir al jugador sería otro juego: haría falta búsqueda de caminos,
//! condición de muerte y reinicio. Quieto y a oscuras ya cumple su función:
//! aparece cuando lo alumbrás.

use nalgebra_glm::Vec2;

use crate::maze::{Maze, BLOCK_SIZE};

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

pub struct Monster {
    pub pos: Vec2,
    /// Cuadro actual de la animación.
    frame: usize,
    /// Tiempo acumulado en el cuadro actual.
    timer: f32,
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

        Some(Monster {
            pos,
            frame: 0,
            timer: 0.0,
        })
    }

    /// Avanza la animación. `frames` es cuántos cuadros tiene el ciclo.
    ///
    ///
    /// El temporizador se descuenta en un `while` y no en un `if`: con un cuadro
    /// muy largo —o un ciclo muy rápido— podría haber que saltar más de un cuadro
    /// de golpe, y un `if` dejaría la animación arrastrándose detrás del tiempo
    /// real.
    pub fn update(&mut self, delta: f32, frames: usize) {
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
        let mut monster = Monster {
            pos: Vec2::new(0.0, 0.0),
            frame: 0,
            timer: 0.0,
        };

        // avanzar justo un cuadro
        monster.update(FRAME_SECONDS, 4);
        assert_eq!(monster.frame(), 1);

        // un salto grande no debe dejar la animación atrasada: se consumen todos
        // los cuadros que quepan en ese tiempo.
        monster.update(FRAME_SECONDS * 2.0, 4);
        assert_eq!(monster.frame(), 3);

        // y da la vuelta en vez de desbordar
        monster.update(FRAME_SECONDS, 4);
        assert_eq!(monster.frame(), 0);

        // con cero cuadros no debe entrar en pánico ni dividir por cero
        monster.update(FRAME_SECONDS, 0);
        assert_eq!(monster.frame(), 0);
    }
}

#[cfg(test)]
mod catch_tests {
    use super::*;

    fn monster_at(x: f32, y: f32) -> Monster {
        Monster {
            pos: Vec2::new(x, y),
            frame: 0,
            timer: 0.0,
        }
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
