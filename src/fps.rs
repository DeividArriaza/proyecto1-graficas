//! Medición de cuadros por segundo.

use std::time::Instant;

/// Peso del cuadro nuevo en el promedio. Con 0.1 el valor mostrado responde en
/// una decena de cuadros y no tiembla: el número instantáneo (`1 / dt`) salta
/// demasiado para poder leerlo.
const SMOOTHING: f32 = 0.1;

/// Duración máxima que se le reporta a un cuadro, en segundos.
///
/// Un cuadro puede durar muchísimo si la ventana se arrastra, si el sistema
/// suspende el proceso o si se abre otra aplicación encima. Sin este tope, al
/// volver se aplicaría todo ese tiempo de golpe: el jugador aparecería metros
/// más adelante —posiblemente del otro lado de una pared— y la batería se
/// vaciaría de un salto. Acotarlo pierde un poco de tiempo real, que es
/// exactamente lo que se quiere.
const MAX_DELTA: f32 = 0.1;

pub struct FpsCounter {
    /// Momento en que empezó el cuadro anterior.
    last_frame: Option<Instant>,
    /// Promedio móvil exponencial de los cuadros por segundo.
    smoothed: f32,
    /// Duración del último cuadro, en segundos.
    delta: f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        FpsCounter {
            last_frame: None,
            smoothed: 0.0,
            delta: 0.0,
        }
    }

    /// Se llama una vez por cuadro, al inicio. Devuelve los cuadros por segundo
    /// suavizados.
    ///
    /// Mide el tiempo entre inicios de cuadro, así que el número incluye el
    /// `sleep` con que el ciclo de render limita la velocidad: es lo que de
    /// verdad ve el jugador, no sólo el costo de dibujar.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();

        if let Some(last) = self.last_frame {
            let dt = now.duration_since(last).as_secs_f32();
            self.delta = dt.min(MAX_DELTA);

            // un cuadro de duración cero daría división por cero; se descarta.
            if dt > 0.0 {
                let instant_fps = 1.0 / dt;

                self.smoothed = if self.smoothed == 0.0 {
                    // primer valor real: se toma tal cual, para no arrancar
                    // subiendo desde cero.
                    instant_fps
                } else {
                    self.smoothed * (1.0 - SMOOTHING) + instant_fps * SMOOTHING
                };
            }
        }

        self.last_frame = Some(now);

        self.smoothed
    }

    /// Cuánto duró el cuadro anterior, en segundos.
    ///
    /// Lo usa todo lo que cambia con el tiempo y no con el cuadro: el desgaste
    /// de la linterna, y más adelante las animaciones. Así el juego se comporta
    /// igual a 60 fps que a 15.
    pub fn delta_seconds(&self) -> f32 {
        self.delta
    }
}
