//! Estado de la linterna: prendida o apagada, y cuánta batería le queda.
//!
//! Este módulo no dibuja nada. Sólo lleva la cuenta. Quien traduce ese estado a
//! píxeles es `render::lighting`, y quien muestra la barra es `render::hud`.

/// Segundos de uso continuo hasta agotar una batería llena.
const SECONDS_OF_USE: f32 = 35.0;

/// Segundos apagada para recargar de vacío a lleno.
///
/// Más lento que el gasto a propósito: apagarla tiene que ser una decisión, no
/// un trámite. Pero existe para que agotar la batería nunca deje al jugador
/// ciego de forma permanente, sin manera de terminar el nivel.
const SECONDS_TO_RECHARGE: f32 = 20.0;

/// Nivel de batería bajo el cual la luz empieza a apagarse gradualmente.
///
/// Sin esta rampa la linterna pasaría de luz plena a nada en un cuadro. Con
/// ella el jugador ve venir el apagón y le da tiempo de buscar dónde pararse.
const DIMMING_THRESHOLD: f32 = 0.2;

/// Nivel de batería bajo el cual la luz empieza a parpadear.
const FLICKER_THRESHOLD: f32 = 0.15;

/// Qué tan oscuro llega a ponerse el parpadeo, como fracción de la luz plena.
const FLICKER_DEPTH: f32 = 0.55;

pub struct Flashlight {
    /// ¿Está encendida?
    pub on: bool,
    /// Carga restante, de 0.0 (vacía) a 1.0 (llena).
    pub battery: f32,
    /// Segundos acumulados desde que arrancó el juego. Sólo alimenta el
    /// parpadeo, que necesita una señal que avance con el tiempo.
    elapsed: f32,
}

impl Flashlight {
    pub fn new() -> Self {
        Flashlight {
            on: true,
            battery: 1.0,
            elapsed: 0.0,
        }
    }

    /// Prende o apaga. Prender con la batería vacía no hace nada.
    pub fn toggle(&mut self) {
        if self.battery > 0.0 {
            self.on = !self.on;
        }
    }

    /// Gasta o recarga según el tiempo transcurrido desde el cuadro anterior.
    ///
    /// Se usa `delta_seconds` y no una cantidad fija por cuadro para que el
    /// gasto sea el mismo a 60 fps que a 15: si el desgaste fuera por cuadro,
    /// una máquina lenta tendría el doble de duración de batería.
    pub fn update(&mut self, delta_seconds: f32) {
        self.elapsed += delta_seconds;

        if self.on {
            self.battery -= delta_seconds / SECONDS_OF_USE;

            if self.battery <= 0.0 {
                self.battery = 0.0;
                self.on = false;
            }
        } else {
            self.battery = (self.battery + delta_seconds / SECONDS_TO_RECHARGE).min(1.0);
        }
    }

    /// Qué tan fuerte ilumina, de 0.0 a 1.0. Es lo único que el render necesita
    /// saber de la linterna.
    pub fn intensity(&self) -> f32 {
        if !self.on {
            return 0.0;
        }

        // con poca batería la luz baja de forma proporcional hasta apagarse.
        let level = (self.battery / DIMMING_THRESHOLD).min(1.0);

        if self.battery < FLICKER_THRESHOLD {
            level * self.flicker()
        } else {
            level
        }
    }

    /// Factor de parpadeo, entre `1.0 - FLICKER_DEPTH` y 1.0.
    ///
    /// Se arma sumando dos senos de frecuencias que no son múltiplos entre sí:
    /// el resultado nunca repite un patrón corto, así que se lee como irregular
    /// sin necesitar un generador de aleatorios ni estado adicional. Y al
    /// depender sólo de `elapsed`, el parpadeo va al mismo ritmo sin importar
    /// los cuadros por segundo.
    fn flicker(&self) -> f32 {
        let wobble = (self.elapsed * 37.0).sin() * 0.6 + (self.elapsed * 11.3).sin() * 0.4;

        // wobble va de -1.0 a 1.0; se lleva a [1 - profundidad, 1.0].
        1.0 - FLICKER_DEPTH * (1.0 - wobble) / 2.0
    }
}
