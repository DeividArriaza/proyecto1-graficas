mod audio;
mod caster;
mod discovery;
mod flashlight;
mod fps;
mod framebuffer;
mod game;
mod maze;
mod monster;
mod mazegen;
mod player;
mod render;
mod sprites;
mod textures;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::time::{Duration, Instant};

use crate::audio::{Audio, Effect};
use crate::fps::FpsCounter;
use crate::framebuffer::Framebuffer;
use crate::game::pause::{Choice as PauseChoice, Pause};
use crate::game::outcome::Choice as OutcomeChoice;
use crate::game::welcome::{Action, Menu};
use crate::game::{LevelReport, Outcome, Screen, Session};
use crate::maze::BLOCK_SIZE;
use crate::player::process_events;
use crate::render::lighting;
use crate::sprites::SpriteSheet;
use crate::textures::TextureSet;

/// Duración objetivo de un cuadro: 16 ms ~ 60 cuadros por segundo.
const FRAME_TIME: Duration = Duration::from_millis(16);

fn main() {
    let width = 1300;
    let height = 900;

    let mut framebuffer = Framebuffer::new(width, height);
    let textures = TextureSet::load();

    // La hoja del monstruo: 4 columnas por 2 filas. Si el archivo no está, el
    // juego corre sin monstruo, igual que corre sin audio.
    let monster_sheet = SpriteSheet::load("assets/sprites/eviloogie.png", 4, 2);

    let mut fps_counter = FpsCounter::new();
    let mut audio = Audio::new();

    let mut screen = Screen::Welcome;
    let mut menu = Menu::new();
    let mut pause = Pause::new();

    // La sesión no existe hasta que se elige un nivel. `Option` en vez de una
    // sesión vacía de relleno: así el compilador obliga a comprobar que hay
    // partida antes de tocarla.
    let mut session: Option<Session> = None;

    // El informe del último nivel terminado. Vive aparte de `Screen` por lo mismo
    // que la sesión: así se puede reasignar la pantalla sin chocar con el
    // préstamo de los datos que esa pantalla está mostrando.
    let mut report: Option<LevelReport> = None;

    let mut window = Window::new("Maze Runner", width, height, WindowOptions::default()).unwrap();

    // El cursor se esconde dentro del nivel y reaparece en el menú. Se lleva la
    // cuenta para llamar a `set_cursor_visibility` sólo cuando cambia: es una
    // llamada al servidor de ventanas, no algo para repetir sesenta veces por
    // segundo.
    let mut cursor_hidden = false;

    while window.is_open() {
        let frame_start = Instant::now();
        let fps = fps_counter.tick();
        let delta = fps_counter.delta_seconds();

        let should_hide = matches!(screen, Screen::Playing);

        if should_hide != cursor_hidden {
            window.set_cursor_visibility(!should_hide);
            cursor_hidden = should_hide;
        }

        // ------------------------------------------------------------------
        // FASE DE ACTUALIZACIÓN
        //
        // Sólo decide y cambia estado. Nunca dibuja, y nunca sale del cuadro
        // por su cuenta.
        //
        // El `continue` a mitad del cuadro está prohibido acá, y no por estilo:
        // se saltaría `update_with_buffer`, que es donde `minifb` avanza el
        // contador de duración de cada tecla. `is_key_pressed` devuelve `true`
        // mientras ese contador valga cero, así que sin la llamada la misma
        // pulsación se lee de nuevo en la iteración siguiente. Con Escape eso
        // daba un rebote infinito entre el nivel y la pausa, con la ventana
        // congelada.
        // ------------------------------------------------------------------
        let mut quit = false;

        match screen {
            Screen::Welcome => match menu.update(&window) {
                Some(Action::Play(level)) => {
                    session = Some(Session::start(level));
                    screen = Screen::Playing;
                }
                Some(Action::Quit) => quit = true,
                None => {
                    // Escape también cierra, pero exige una pulsación nueva: si
                    // se leyera la tecla mantenida, salir de la pausa al menú
                    // con Escape cerraría el juego de inmediato.
                    if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                        quit = true;
                    }
                }
            },

            Screen::Playing => {
                if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                    // La sesión no se toca: sigue viva y congelada esperando
                    // que se reanude. Eso es lo que hace que sea una pausa.
                    pause.reset();
                    audio.set_paused(true);
                    screen = Screen::Paused;
                } else if let Some(state) = session.as_mut() {
                    play(&mut window, state, &mut audio, monster_sheet.as_ref(), delta);

                    // Se revisa primero al monstruo: si te atrapa parado
                    // sobre la meta, gana la captura. Llegar y ser atrapado en el
                    // mismo cuadro es un empate, y perder es el desenlace más
                    // informativo de los dos.
                    if let Some(caught) = caught_by_monster(state) {
                        audio.stop_footsteps();
                        audio.play(Effect::Defeat);
                        report = Some(caught);
                        session = None;
                        screen = Screen::Outcome;
                    } else if reached_goal(state) {
                        audio.stop_footsteps();
                        audio.play(Effect::Victory);
                        report = Some(state.report(Outcome::Escaped));
                        session = None;
                        screen = Screen::Outcome;
                    }
                } else {
                    // No debería pasar: pantalla y sesión se asignan juntas.
                    screen = Screen::Welcome;
                }
            }

            Screen::Paused => match pause.update(&window) {
                Some(PauseChoice::Resume) => {
                    // El cursor pudo recorrer media pantalla durante la pausa;
                    // sin olvidar la referencia, todo ese trayecto se aplicaría
                    // de golpe como un giro al reanudar.
                    if let Some(state) = session.as_mut() {
                        state.mouse.reset();
                    }

                    audio.set_paused(false);
                    screen = Screen::Playing;
                }
                Some(PauseChoice::Retry) => {
                    if let Some(level) = session.as_ref().map(|state| state.level) {
                        session = Some(Session::start(level));
                    }

                    audio.set_paused(false);
                    screen = Screen::Playing;
                }
                Some(PauseChoice::Menu) => {
                    audio.stop_footsteps();
                    audio.set_paused(false);
                    session = None;
                    screen = Screen::Welcome;
                }
                // Los ajustes de música no cambian de pantalla: se aplican y el
                // menú sigue abierto para poder seguir oyendo el resultado.
                Some(PauseChoice::ToggleMusic) => audio.toggle_music(),
                Some(PauseChoice::VolumeUp) => audio.change_music_volume(true),
                Some(PauseChoice::VolumeDown) => audio.change_music_volume(false),
                None => {}
            },

            Screen::Outcome => match game::outcome::update(&window) {
                Some(OutcomeChoice::Menu) => {
                    report = None;
                    screen = Screen::Welcome;
                }
                Some(OutcomeChoice::Retry) => {
                    if let Some(level) = report.map(|report| report.level) {
                        session = Some(Session::start(level));
                    }

                    report = None;
                    screen = Screen::Playing;
                }
                None => {}
            },
        }

        if quit {
            break;
        }

        // ------------------------------------------------------------------
        // FASE DE DIBUJO
        //
        // Dibuja la pantalla que quedó activa tras la actualización, así que un
        // cambio de estado se ve en el mismo cuadro y no en el siguiente.
        // ------------------------------------------------------------------
        match screen {
            Screen::Welcome => game::welcome::draw(&mut framebuffer, &menu),

            Screen::Playing => {
                if let Some(state) = session.as_ref() {
                    draw_level(&mut framebuffer, state, &textures, monster_sheet.as_ref(), fps);
                }
            }

            Screen::Paused => {
                if let Some(state) = session.as_ref() {
                    // El nivel se redibuja aunque esté detenido: el atenuado de
                    // la pausa se aplica sobre el búfer, así que sin repintar
                    // debajo la imagen se iría a negro cuadro a cuadro.
                    draw_level(&mut framebuffer, state, &textures, monster_sheet.as_ref(), fps);
                    game::pause::draw(
                        &mut framebuffer,
                        &pause,
                        audio.music_enabled(),
                        audio.music_volume_fraction(),
                    );
                }
            }

            Screen::Outcome => {
                if let Some(report) = report.as_ref() {
                    game::outcome::draw(&mut framebuffer, report);
                }
            }
        }

        // Siempre se presenta el cuadro. Además de mostrarlo, esta llamada es la
        // que hace que `minifb` sondee el teclado y avance el estado de las
        // teclas; saltársela rompe la detección de pulsaciones.
        window
            .update_with_buffer(&framebuffer.buffer, width, height)
            .unwrap();

        // Se duerme solo lo que falte para completar el cuadro, no un tiempo
        // fijo encima del render. Con un `sleep` fijo de 16 ms, un render de
        // 10 ms daba 26 ms por cuadro (38 fps) en vez de los 60 esperados.
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - elapsed);
        }
    }
}

/// Avanza un cuadro de partida: entrada, linterna y descubrimiento.
fn play(
    window: &mut Window,
    state: &mut Session,
    audio: &mut Audio,
    sprites: Option<&SpriteSheet>,
    delta: f32,
) {
    state.elapsed += delta;

    // La posición se guarda antes de mover para saber si el jugador avanzó de
    // verdad. Preguntar por las teclas no alcanza: empujando contra una pared se
    // presiona W sin desplazarse, y los pasos sonarían igual.
    let previous = state.player.pos;

    process_events(
        window,
        &mut state.player,
        &state.maze,
        &mut state.mouse,
        delta,
    );

    let moved = (state.player.pos - previous).norm() > 0.5;
    audio.footsteps(delta, moved, player::is_running(window));

    // `M` prende y apaga la linterna. El desgaste corre aparte: gasta mientras
    // está encendida y recarga, más despacio, mientras no.
    if window.is_key_pressed(Key::M, KeyRepeat::No) {
        state.flashlight.toggle();
        audio.play(Effect::Flashlight);
    }

    state.flashlight.update(delta);

    // La animación avanza con el tiempo, no con los cuadros: el monstruo respira
    // al mismo ritmo a 15 o a 60 fps.
    if let (Some(monster), Some(sheet)) = (state.monster.as_mut(), sprites) {
        monster.update(delta, sheet.frames());
    }

    // La celda propia se recuerda siempre; el resto sólo si hay luz que lo
    // alcance. El alcance se escala con la intensidad, así que la batería baja
    // descubre menos.
    state.discovered.mark_player(&state.player);

    if state.flashlight.on {
        state.discovered.reveal_from(
            &state.maze,
            &state.player,
            lighting::beam_reach() * state.flashlight.intensity(),
        );
    }
}

/// ¿El monstruo atrapó al jugador? Devuelve el informe si sí.
fn caught_by_monster(state: &Session) -> Option<LevelReport> {
    let monster = state.monster.as_ref()?;

    if !monster.catches(state.player.pos) {
        return None;
    }

    Some(state.report(Outcome::Caught))
}

/// ¿El jugador llegó a la meta?
///
/// Se traduce su posición en píxeles a la celda que ocupa y se revisa si esa
/// celda es la marca de meta.
fn reached_goal(state: &Session) -> bool {
    let i = state.player.pos.x as usize / BLOCK_SIZE;
    let j = state.player.pos.y as usize / BLOCK_SIZE;

    matches!(
        state.maze.get(j).and_then(|row| row.get(i)),
        Some(&('g' | 'G'))
    )
}

fn draw_level(
    framebuffer: &mut Framebuffer,
    state: &Session,
    textures: &TextureSet,
    sprites: Option<&SpriteSheet>,
    fps: f32,
) {
    // La vista pinta techo y piso sobre la pantalla completa, así que no
    // necesita `clear()` previo: serían 1.17 millones de píxeles sobrescritos
    // para nada.
    // La vista devuelve la distancia de la pared en cada columna. Ese búfer de
    // profundidad es lo que permite que los sprites queden ocultos detrás de las
    // paredes en vez de pintarse encima.
    let depth = render::world::render(
        framebuffer,
        &state.maze,
        &state.player,
        &state.flashlight,
        textures,
    );

    // Los sprites van entre el mundo y el HUD: se ocluyen con las paredes, pero
    // nada del juego tapa el minimapa ni el contador.
    if let (Some(monster), Some(sheet)) = (state.monster.as_ref(), sprites) {
        render::billboard::draw(
            framebuffer,
            &depth,
            &state.player,
            monster,
            sheet,
            &state.flashlight,
        );
    }

    // Los sobreimpresos van al final, para quedar encima de la vista.
    render::minimap::draw(
        framebuffer,
        &state.maze,
        &state.player,
        &state.discovered,
        &state.flashlight,
        textures,
    );
    render::hud::draw_fps(framebuffer, fps);
    render::hud::draw_flashlight(framebuffer, &state.flashlight);
}
