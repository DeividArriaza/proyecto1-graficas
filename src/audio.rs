//! Música y efectos de sonido.
//!
//! Sigue el mismo criterio que las texturas: **si un archivo no está, se salta
//! en silencio**. El juego nunca depende de que el audio exista, así que se
//! puede agregar de a un archivo, y quien clone el repositorio sin los assets
//! igual puede jugar.
//!
//! Lo mismo con el dispositivo de sonido: si no hay tarjeta, o el sistema no
//! deja abrirla, `Audio` queda inerte y todos sus métodos no hacen nada. Un
//! juego no debería negarse a arrancar porque no encontró bocinas.
//!
//! # Cómo se reproducen las dos clases de sonido
//!
//! - **Pistas largas** (música, ambiente) van en un `Player` cada una, que
//!   permite controlar el volumen y repetir en bucle.
//! - **Efectos cortos** se mandan directo al mezclador con `Mixer::add`. Eso los
//!   dispara en paralelo sin encolarlos: dos pasos seguidos pueden solaparse, que
//!   es como suena de verdad. Con un `Player` se pondrían en fila y el segundo
//!   esperaría a que terminara el primero.

use std::io::Cursor;
use std::sync::Arc;

use rodio::source::Source;
use rodio::{Decoder, MixerDeviceSink, Player};

/// Carpeta donde se buscan los archivos de audio.
const AUDIO_DIR: &str = "assets/audio";

/// Volumen inicial de la música. Bien por debajo de 1.0: acompaña, no tapa.
const MUSIC_VOLUME: f32 = 0.35;

/// Cuánto sube o baja el volumen por pulsación en el menú de pausa.
const VOLUME_STEP: f32 = 0.05;

/// Techo del volumen de música.
///
/// Se corta antes de 1.0 porque a todo volumen la música tapa los efectos, y los
/// pasos y el clic de la linterna son información de juego, no decoración.
const MUSIC_VOLUME_MAX: f32 = 0.8;

/// Volumen del ambiente de la instalación. Más bajo todavía; tiene que sentirse
/// sin que se note.
const AMBIENCE_VOLUME: f32 = 0.25;

/// Volumen de los efectos.
const EFFECT_VOLUME: f32 = 0.7;

/// Segundos entre pasos al caminar.
const STEP_INTERVAL_WALK: f32 = 0.45;

/// Segundos entre pasos al correr.
///
/// No es una elección estética: la cadencia es la señal más clara de que estás
/// corriendo, más que la velocidad en pantalla.
const STEP_INTERVAL_RUN: f32 = 0.28;

/// Duración máxima, en segundos, para considerar que `step` es un paso suelto.
///
/// Los bancos de sonido casi nunca ofrecen un paso aislado: lo que se descarga
/// suele ser una caminata de varios segundos. Un archivo así disparado con
/// cadencia se solaparía diez veces y sonaría a estampida, así que según su
/// duración se trata de una de dos maneras distintas. El umbral es generoso:
/// un paso real dura entre 0.2 y 0.5 s.
const SINGLE_STEP_MAX: f32 = 1.2;

/// Cuánto se acelera la caminata en bucle al correr.
///
/// Sólo aplica al modo bucle. Cambiar la velocidad de reproducción también sube
/// el tono, y por eso no se exagera: 1.35 se lee como apuro, 2.0 como ardilla.
const LOOP_RUN_SPEED: f32 = 1.35;

/// Efectos cortos que el juego dispara.
#[derive(Copy, Clone)]
pub enum Effect {
    /// Un paso.
    Step,
    /// Clic de la linterna, al prender o apagar.
    Flashlight,
    /// Al alcanzar la salida.
    Victory,
    /// Al ser atrapado por el monstruo.
    Defeat,
}

impl Effect {
    /// Nombre del archivo, sin extensión.
    fn name(self) -> &'static str {
        match self {
            Effect::Step => "step",
            Effect::Flashlight => "flashlight",
            Effect::Victory => "victory",
            Effect::Defeat => "defeat",
        }
    }

    fn index(self) -> usize {
        match self {
            Effect::Step => 0,
            Effect::Flashlight => 1,
            Effect::Victory => 2,
            Effect::Defeat => 3,
        }
    }
}

/// Cuántos efectos distintos hay.
const EFFECT_COUNT: usize = 4;

/// Cómo se reproducen los pasos, según lo que traiga el archivo.
enum Footsteps {
    /// No hay archivo, o no se pudo decodificar.
    Silent,
    /// Un paso aislado: se dispara con cadencia, un disparo por zancada.
    OneShot,
    /// Una caminata ya grabada: suena en bucle mientras el jugador se desplaza y
    /// se pausa al detenerse. La cadencia la trae el propio archivo, así que
    /// correr se expresa acelerando la reproducción.
    Loop { player: Player, running: bool },
}

pub struct Audio {
    /// El dispositivo tiene que seguir vivo mientras se quiera oír algo: al
    /// soltarlo, el sonido se corta.
    device: Option<MixerDeviceSink>,

    /// Pistas largas en bucle. Se conservan por la misma razón que el
    /// dispositivo: si se sueltan, dejan de sonar.
    music: Option<Player>,
    ambience: Option<Player>,

    /// Los efectos se guardan **codificados**, tal como vinieron del disco, y se
    /// decodifican en cada disparo. Decodificar un wav corto es despreciable, y
    /// así no hace falta lidiar con formatos ni con tasas de muestreo distintas.
    ///
    /// Es `Arc<[u8]>` y no `Vec<u8>` para que reproducir no copie los bytes:
    /// `Cursor` acepta cualquier cosa que sea `AsRef<[u8]>`, y clonar un `Arc`
    /// sólo incrementa un contador.
    effects: [Option<Arc<[u8]>>; EFFECT_COUNT],

    /// ¿La música está sonando? Es estado propio porque `Player` no expone si
    /// fue pausado por el jugador o por la pausa del juego.
    music_enabled: bool,

    /// Volumen actual de la música, para poder mostrarlo en el menú.
    music_volume: f32,

    /// Cuánto falta para el próximo paso. Sólo lo usa el modo `OneShot`.
    step_timer: f32,

    footsteps: Footsteps,
}

impl Audio {
    pub fn new() -> Self {
        let device = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(device) => Some(device),
            Err(error) => {
                println!("sin dispositivo de audio ({error}); el juego corre en silencio");
                None
            }
        };

        let mut audio = Audio {
            device,
            music: None,
            ambience: None,
            effects: Default::default(),
            music_enabled: true,
            music_volume: MUSIC_VOLUME,
            step_timer: 0.0,
            footsteps: Footsteps::Silent,
        };

        audio.load_effects();
        audio.start_loops();
        audio.footsteps = audio.prepare_footsteps();

        audio
    }

    /// Decide cómo se van a reproducir los pasos, midiendo el archivo.
    fn prepare_footsteps(&self) -> Footsteps {
        let Some(bytes) = self.effects[Effect::Step.index()].as_ref() else {
            return Footsteps::Silent;
        };

        let Some(seconds) = measure_duration(bytes) else {
            return Footsteps::Silent;
        };

        if seconds <= SINGLE_STEP_MAX {
            println!("pasos: {seconds:.2} s, se dispara uno por zancada");
            return Footsteps::OneShot;
        }

        println!("pasos: {seconds:.2} s, es una caminata; suena en bucle");

        let Some(device) = self.device.as_ref() else {
            return Footsteps::Silent;
        };

        let Ok(source) = Decoder::new(Cursor::new(Arc::clone(bytes))) else {
            return Footsteps::Silent;
        };

        let player = Player::connect_new(device.mixer());
        player.set_volume(EFFECT_VOLUME);
        player.append(source.repeat_infinite());
        // arranca en silencio: el jugador todavía no se movió.
        player.pause();

        Footsteps::Loop {
            player,
            running: false,
        }
    }

    fn load_effects(&mut self) {
        for effect in [
            Effect::Step,
            Effect::Flashlight,
            Effect::Victory,
            Effect::Defeat,
        ] {
            self.effects[effect.index()] = read_asset(effect.name());
        }
    }

    fn start_loops(&mut self) {
        self.music = self.start_loop("music", self.music_volume);
        self.ambience = self.start_loop("ambience", AMBIENCE_VOLUME);
    }

    /// Arranca una pista en bucle infinito, si existe y hay dispositivo.
    fn start_loop(&self, name: &str, volume: f32) -> Option<Player> {
        let device = self.device.as_ref()?;
        let bytes = read_asset(name)?;

        let source = Decoder::new(Cursor::new(bytes)).ok()?;

        let player = Player::connect_new(device.mixer());
        player.set_volume(volume);
        player.append(source.repeat_infinite());

        Some(player)
    }

    /// Dispara un efecto. Si falta el archivo o el dispositivo, no pasa nada.
    pub fn play(&self, effect: Effect) {
        let Some(device) = self.device.as_ref() else {
            return;
        };

        let Some(bytes) = self.effects[effect.index()].as_ref() else {
            return;
        };

        // Un decodificador nuevo por disparo. Si falla, se ignora en silencio:
        // un efecto que no suena no es motivo para interrumpir la partida.
        if let Ok(source) = Decoder::new(Cursor::new(Arc::clone(bytes))) {
            device.mixer().add(source.amplify(EFFECT_VOLUME));
        }
    }

    /// Lleva la cadencia de los pasos.
    ///
    /// Se llama en cada cuadro con si el jugador se está moviendo y si corre. Al
    /// detenerse, el temporizador se deja en cero para que el próximo paso suene
    /// en cuanto se vuelva a caminar: esperar media zancada al arrancar se siente
    /// como que el juego no responde.
    pub fn footsteps(&mut self, delta: f32, moving: bool, running: bool) {
        match &mut self.footsteps {
            Footsteps::Silent => {}

            Footsteps::OneShot => {
                if !moving {
                    self.step_timer = 0.0;
                    return;
                }

                self.step_timer -= delta;

                if self.step_timer > 0.0 {
                    return;
                }

                self.play(Effect::Step);

                self.step_timer = if running {
                    STEP_INTERVAL_RUN
                } else {
                    STEP_INTERVAL_WALK
                };
            }

            Footsteps::Loop { player, running: was_running } => {
                if moving {
                    player.play();
                } else {
                    player.pause();
                }

                // La velocidad sólo se toca cuando cambia: es una escritura
                // atómica compartida con el hilo de audio, no algo para repetir
                // sesenta veces por segundo.
                if running != *was_running {
                    player.set_speed(if running { LOOP_RUN_SPEED } else { 1.0 });
                    *was_running = running;
                }
            }
        }
    }

    /// ¿Está sonando la música?
    pub fn music_enabled(&self) -> bool {
        self.music_enabled
    }

    /// Volumen de la música como fracción de su máximo, para dibujar la barra.
    pub fn music_volume_fraction(&self) -> f32 {
        self.music_volume / MUSIC_VOLUME_MAX
    }

    /// Prende o apaga la música.
    ///
    /// Se pausa en vez de bajar el volumen a cero, así que al volver a prenderla
    /// la canción sigue donde estaba en lugar de arrancar de nuevo.
    pub fn toggle_music(&mut self) {
        self.music_enabled = !self.music_enabled;

        if let Some(player) = self.music.as_ref() {
            if self.music_enabled {
                player.play();
            } else {
                player.pause();
            }
        }
    }

    /// Sube o baja el volumen de la música un paso.
    pub fn change_music_volume(&mut self, up: bool) {
        let delta = if up { VOLUME_STEP } else { -VOLUME_STEP };

        self.music_volume = (self.music_volume + delta).clamp(0.0, MUSIC_VOLUME_MAX);

        if let Some(player) = self.music.as_ref() {
            player.set_volume(self.music_volume);
        }
    }

    /// Corta el sonido de pasos.
    ///
    /// Hay que llamarla al salir del nivel por cualquier vía. El bucle de
    /// caminata sólo se detiene cuando alguien se lo pide, y `footsteps` deja de
    /// llamarse en cuanto la pantalla cambia: sin esto, llegar a la meta
    /// caminando dejaba los pasos sonando durante el informe de victoria.
    pub fn stop_footsteps(&mut self) {
        self.step_timer = 0.0;

        if let Footsteps::Loop { player, .. } = &self.footsteps {
            player.pause();
        }
    }

    /// Detiene o restaura los sonidos del mundo. Lo usa la pausa.
    ///
    /// **La música no se toca a propósito.** El menú de pausa es donde se ajusta,
    /// y no se puede ajustar lo que no se oye: sin la canción sonando, mover el
    /// volumen sería a ciegas. Lo que se calla es lo que pertenece al nivel — el
    /// ambiente de la instalación y los pasos.
    pub fn set_paused(&self, paused: bool) {
        let walking = match &self.footsteps {
            Footsteps::Loop { player, .. } => Some(player),
            _ => None,
        };

        for track in [self.ambience.as_ref(), walking].into_iter().flatten() {
            if paused {
                track.pause();
            } else {
                track.play();
            }
        }

        // Al reanudar, la caminata quedaría sonando aunque el jugador esté
        // quieto. Se deja en silencio y el primer cuadro de movimiento la
        // vuelve a arrancar.
        if !paused {
            if let Footsteps::Loop { player, .. } = &self.footsteps {
                player.pause();
            }
        }
    }
}

/// Cuántos segundos dura una muestra, decodificándola completa.
///
/// Hace falta decodificar porque `Source::total_duration()` devuelve `None` para
/// casi todos los mp3: sólo la informa el que trae cabecera con el conteo de
/// cuadros. Se paga una vez, al arrancar, y sobre archivos cortos.
fn measure_duration(bytes: &Arc<[u8]>) -> Option<f32> {
    let source = Decoder::new(Cursor::new(Arc::clone(bytes))).ok()?;

    let rate = source.sample_rate().get() as f32;
    let channels = source.channels().get() as f32;

    // el iterador entrega una muestra por canal, así que hay que dividir por la
    // cantidad de canales para obtener cuadros.
    let samples = source.count() as f32;

    Some(samples / channels / rate)
}

/// Lee un archivo de audio de `assets/audio`, probando las extensiones
/// soportadas.
///
/// Probar varias extensiones evita tener que decidir el formato por el jugador:
/// los efectos cortos suelen venir en wav y la música en mp3, y así cualquiera
/// de los dos entra sin renombrar nada.
fn read_asset(name: &str) -> Option<Arc<[u8]>> {
    for extension in ["ogg", "mp3", "wav"] {
        let path = format!("{AUDIO_DIR}/{name}.{extension}");

        if let Ok(bytes) = std::fs::read(&path) {
            println!("audio {path} cargado");
            return Some(bytes.into());
        }
    }

    println!("audio {AUDIO_DIR}/{name}.[ogg|mp3|wav] no encontrado; se omite");

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diagnóstico: imprime la duración que reporta el decodificador para cada
    /// archivo presente. Se corre con:
    ///     cargo test -- --ignored --nocapture duraciones
    ///
    /// Está marcado `ignore` porque decodifica la música completa —varios
    /// minutos de audio— y no tiene sentido pagar eso en cada `cargo test`.
    #[test]
    #[ignore]
    fn duraciones() {
        for name in ["step", "flashlight", "victory", "defeat", "ambience", "music"] {
            match read_asset(name) {
                Some(bytes) => match Decoder::new(Cursor::new(bytes)) {
                    Ok(source) => println!("{name}: total_duration = {:?}", source.total_duration()),
                    Err(error) => println!("{name}: no se pudo decodificar ({error})"),
                },
                None => println!("{name}: ausente"),
            }
        }
    }
}
