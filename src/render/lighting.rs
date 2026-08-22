//! Cuánta luz recibe un punto del mundo, y cómo eso cambia su color.
//!
//! Hay dos fuentes que se suman:
//!
//! - **Ambiental**: una penumbra que decae con la distancia y nunca llega a
//!   cero. Existe para que sin linterna el juego siga siendo jugable, y para
//!   que la pared de al lado se adivine en vez de desaparecer.
//! - **Linterna**: un cono que sale del jugador, mucho más largo que la
//!   ambiental pero angosto, y que se apaga hacia los bordes.

use crate::render::FOV;

/// Luz ambiental máxima, pegado a una pared.
///
/// Muy por debajo de 1.0 a propósito: si sin linterna se viera casi igual que
/// con ella, la batería no sería una decisión y el jugador la ignoraría. En
/// 0.30 la pared de al lado se distingue, pero los colores quedan apagados y
/// leer el laberinto de lejos es imposible.
const AMBIENT_MAX: f32 = 0.30;

/// Distancia en píxeles a la que la luz ambiental ya cayó a su mínimo.
/// Con bloques de 100 px son unos 2.6 bloques: apenas el pasillo inmediato.
const AMBIENT_REACH: f32 = 260.0;

/// Piso de la luz ambiental: qué tan oscuro puede quedar lo más lejano.
///
/// En 0.0 el fondo sería negro absoluto: sin linterna el nivel se vuelve
/// injugable y el video de entrega, ilegible. 0.03 deja una silueta que se
/// adivina, nada más.
const AMBIENT_FLOOR: f32 = 0.03;

/// Alcance del haz de la linterna, en píxeles. Unos 9 bloques.
const BEAM_REACH: f32 = 900.0;

/// Media altura del cono, en unidades de pantalla (donde el borde es 1.0).
///
/// Más generosa que la anchura porque la pantalla es más ancha que alta: con el
/// mismo valor en los dos ejes el haz se vería como una franja horizontal en vez
/// de un óvalo.
const BEAM_HALF_HEIGHT: f32 = 0.80;

/// Media anchura del cono, en unidades de pantalla (donde el borde es 1.0).
/// En 0.55 el haz cubre poco más de la mitad central de la pantalla.
const BEAM_HALF_WIDTH: f32 = 0.55;

/// Hasta dónde llega el haz, en píxeles.
pub fn beam_reach() -> f32 {
    BEAM_REACH
}

/// Media apertura del haz, en radianes.
///
/// El cono está definido en columnas de pantalla, no en ángulos del mundo, así
/// que su apertura real depende del FOV: la columna `screen_x` corresponde al
/// ángulo `atan(screen_x · tan(FOV/2))`. Con FOV de 90° y medio ancho de 0.55
/// da unos 28.8°, o sea un cono de 57.6°.
///
/// Lo necesita el pase de descubrimiento, para revelar exactamente lo que el
/// haz alumbra y no todo el campo de visión.
pub fn beam_half_angle() -> f32 {
    (BEAM_HALF_WIDTH * (FOV / 2.0).tan()).atan()
}

/// Luz de fondo que llega a un punto a `distance` píxeles del jugador.
///
/// La caída es cuadrática, no lineal: la penumbra se cierra rápido apenas te
/// alejás de la pared, que es lo que obliga a encender la linterna para ver el
/// pasillo completo. Con caída lineal el fondo se aclara demasiado y el juego
/// se puede recorrer entero a oscuras.
pub fn ambient(distance: f32) -> f32 {
    let nearness = (1.0 - distance / AMBIENT_REACH).clamp(0.0, 1.0);

    (AMBIENT_MAX * nearness * nearness).max(AMBIENT_FLOOR)
}

/// Luz que aporta el haz de la linterna a plena intensidad.
///
/// `screen_x` es la posición horizontal en pantalla, de -1.0 (borde izquierdo)
/// a 1.0 (borde derecho): el haz apunta al centro, así que lo que decide si un
/// punto cae dentro del cono es su distancia al centro de la pantalla.
pub fn beam(distance: f32, screen_x: f32) -> f32 {
    let falloff = (1.0 - distance / BEAM_REACH).clamp(0.0, 1.0);

    // el borde del cono se eleva al cuadrado para que la transición sea suave.
    // Lineal deja un corte visible, como un círculo recortado con tijera.
    let edge = (1.0 - screen_x.abs() / BEAM_HALF_WIDTH).clamp(0.0, 1.0);

    falloff * edge * edge
}

/// Cuánto del haz llega a una fila de pantalla.
///
/// Es la mitad vertical del cono. Hasta ahora el haz era en realidad una cuña
/// horizontal: iluminaba una columna de pared por igual de arriba abajo, cuando
/// una linterna real deja la parte alta de una pared cercana más oscura. Con
/// color plano no se notaba; con textura sí.
///
/// `screen_y` va de -1.0 (borde superior) a 1.0 (borde inferior), con 0.0 en el
/// horizonte.
pub fn beam_vertical(screen_y: f32) -> f32 {
    let edge = (1.0 - screen_y.abs() / BEAM_HALF_HEIGHT).clamp(0.0, 1.0);

    edge * edge
}

/// Multiplica cada canal del color por `light`, que se espera entre 0.0 y 1.0.
pub fn apply(color: u32, light: f32) -> u32 {
    let light = light.clamp(0.0, 1.0);

    let r = (((color >> 16) & 0xFF) as f32 * light) as u32;
    let g = (((color >> 8) & 0xFF) as f32 * light) as u32;
    let b = ((color & 0xFF) as f32 * light) as u32;

    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ambiental_decae_y_toca_su_piso() {
        assert!((ambient(0.0) - AMBIENT_MAX).abs() < 1e-6, "pegado a la pared");

        // monótona no creciente
        let mut previous = ambient(0.0);
        for step in 1..200 {
            let value = ambient(step as f32 * 5.0);

            assert!(
                value <= previous + 1e-6,
                "a {} px la luz subió: {previous} -> {value}",
                step * 5
            );

            previous = value;
        }

        assert!((ambient(f32::MAX) - AMBIENT_FLOOR).abs() < 1e-6, "el piso");
        assert!(ambient(1e9) >= AMBIENT_FLOOR, "nunca por debajo del piso");
    }

    #[test]
    fn el_haz_se_apaga_fuera_del_cono_y_del_alcance() {
        // centro, pegado: máximo
        assert!((beam(0.0, 0.0) - 1.0).abs() < 1e-6);

        // fuera del cono, a cualquier distancia
        assert_eq!(beam(0.0, 1.0), 0.0, "borde de pantalla");
        assert_eq!(beam(100.0, -0.9), 0.0, "borde opuesto");

        // más allá del alcance
        assert_eq!(beam(BEAM_REACH, 0.0), 0.0);
        assert_eq!(beam(BEAM_REACH * 2.0, 0.0), 0.0);

        // simétrico respecto al centro
        assert!((beam(200.0, 0.3) - beam(200.0, -0.3)).abs() < 1e-6);
    }

    #[test]
    fn la_caida_vertical_es_simetrica_y_acotada() {
        assert!((beam_vertical(0.0) - 1.0).abs() < 1e-6, "en el horizonte");
        assert_eq!(beam_vertical(1.0), 0.0, "borde inferior");
        assert_eq!(beam_vertical(-1.0), 0.0, "borde superior");
        assert!((beam_vertical(0.4) - beam_vertical(-0.4)).abs() < 1e-6);

        for step in -30..=30 {
            let value = beam_vertical(step as f32 / 20.0);
            assert!((0.0..=1.0).contains(&value), "fuera de rango: {value}");
        }
    }

    #[test]
    fn aplicar_luz_respeta_los_extremos() {
        assert_eq!(apply(0xFFFFFF, 1.0), 0xFFFFFF, "luz plena");
        assert_eq!(apply(0xFFFFFF, 0.0), 0x000000, "sin luz");
        assert_eq!(apply(0x000000, 1.0), 0x000000, "el negro sigue negro");

        // una luz mayor que 1 no debe desbordar a otro canal
        assert_eq!(apply(0xFFFFFF, 5.0), 0xFFFFFF, "se recorta arriba");

        // y una negativa tampoco
        assert_eq!(apply(0xFFFFFF, -2.0), 0x000000, "se recorta abajo");

        // los canales no se mezclan entre sí
        assert_eq!(apply(0xFF0000, 0.5), 0x7F0000);
        assert_eq!(apply(0x00FF00, 0.5), 0x007F00);
        assert_eq!(apply(0x0000FF, 0.5), 0x00007F);
    }

    /// El alfa de un sprite vive en el byte alto. `apply` devuelve sólo color, y
    /// no debe dejar basura ahí ni dejarse confundir por él.
    #[test]
    fn aplicar_luz_ignora_el_canal_alfa() {
        assert_eq!(apply(0xFF_FFFFFF, 1.0), 0xFFFFFF);
        assert_eq!(apply(0x00_FF0000, 1.0), 0xFF0000);
    }
}
