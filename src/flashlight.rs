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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arranca_encendida_y_llena() {
        let light = Flashlight::new();

        assert!(light.on);
        assert_eq!(light.battery, 1.0);
    }

    /// Descarga hasta el apagón y **se detiene ahí**.
    ///
    /// Seguir llamando a `update` después recargaría, porque apagada la linterna
    /// se repone. Pasarse de ese punto fue justamente lo que hizo fallar la
    /// primera versión de estas pruebas.
    fn drain(light: &mut Flashlight) {
        let mut ticks = 0;

        while light.on && ticks < 100_000 {
            light.update(0.05);
            ticks += 1;
        }

        assert!(!light.on, "no se apagó en un tiempo razonable");
    }

    #[test]
    fn se_agota_y_se_apaga_sola() {
        let mut light = Flashlight::new();

        drain(&mut light);

        assert_eq!(light.battery, 0.0, "la batería no baja de cero");
        assert_eq!(light.intensity(), 0.0, "apagada no ilumina");
    }

    #[test]
    fn no_se_puede_encender_sin_bateria() {
        let mut light = Flashlight::new();

        drain(&mut light);

        // sin dejar pasar tiempo entre el apagón y el intento: con la batería en
        // cero, la tecla no debe hacer nada.
        light.toggle();

        assert!(!light.on, "con batería vacía no debe encender");
    }

    /// Apenas recarga un poco, ya se puede volver a encender. Es lo que evita
    /// que agotar la batería deje al jugador ciego para siempre.
    #[test]
    fn tras_recargar_un_poco_vuelve_a_encender() {
        let mut light = Flashlight::new();

        drain(&mut light);
        light.update(1.0);

        assert!(light.battery > 0.0, "debería haber recargado algo");

        light.toggle();

        assert!(light.on, "con algo de batería sí debe encender");
    }

    #[test]
    fn recarga_apagada_y_no_pasa_de_llena() {
        let mut light = Flashlight::new();

        light.toggle();
        assert!(!light.on);

        // gastar un poco primero encendiéndola de nuevo
        light.toggle();
        light.update(10.0);
        let after_use = light.battery;
        assert!(after_use < 1.0);

        light.toggle();
        light.update(5.0);
        assert!(light.battery > after_use, "debería haber recargado");

        // mucho tiempo apagada: se llena y se queda ahí
        for _ in 0..1000 {
            light.update(1.0);
        }
        assert_eq!(light.battery, 1.0, "no debe pasar de llena");
    }

    #[test]
    fn la_intensidad_siempre_esta_en_rango() {
        let mut light = Flashlight::new();

        // recorrer toda la descarga, incluida la zona de parpadeo
        for _ in 0..2000 {
            light.update(0.02);

            let intensity = light.intensity();

            assert!(
                (0.0..=1.0).contains(&intensity),
                "intensidad fuera de rango: {intensity} con batería {}",
                light.battery
            );
        }
    }

    /// El desgaste tiene que depender del tiempo, no de los cuadros: la misma
    /// duración total consumida en pasos chicos o grandes debe dejar la misma
    /// batería.
    #[test]
    fn el_desgaste_no_depende_del_tamano_del_paso() {
        let mut coarse = Flashlight::new();
        coarse.update(10.0);

        let mut fine = Flashlight::new();
        for _ in 0..1000 {
            fine.update(0.01);
        }

        assert!(
            (coarse.battery - fine.battery).abs() < 1e-4,
            "grueso {} vs fino {}",
            coarse.battery,
            fine.battery
        );
    }
}
