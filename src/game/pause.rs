//! Menú de pausa.
//!
//! A diferencia del menú principal y de la pantalla de victoria, esta pantalla
//! **no destruye la sesión**: el nivel sigue vivo detrás, congelado, y se dibuja
//! atenuado como fondo. Eso es justamente lo que la hace una pausa y no una
//! salida.
//!
//! Como el nivel de fondo se vuelve a dibujar en cada cuadro, la atenuación no
//! se acumula.
//!
//! Mezcla acciones (continuar, reintentar, salir) con ajustes de música. Es lo
//! que hace cualquier menú de pausa, y aquí tiene una razón concreta: la música
//! sigue sonando mientras esta pantalla está abierta, así que el volumen se
//! ajusta oyendo el resultado.

use minifb::{Key, KeyRepeat, Window};

use crate::framebuffer::Framebuffer;
use crate::render::text;

const PANEL_COLOR: u32 = 0x0E1116;
const BORDER_COLOR: u32 = 0x3A4652;
const TITLE_COLOR: u32 = 0xE8E4D8;
const SELECTED_COLOR: u32 = 0xFFDD33;
const OPTION_COLOR: u32 = 0x8A8F96;
const VALUE_COLOR: u32 = 0xC8CDD4;
const HIGHLIGHT_COLOR: u32 = 0x1C2128;
const HINT_COLOR: u32 = 0x5A6169;

const BAR_BACKGROUND: u32 = 0x252B33;
const BAR_FILL: u32 = 0xFFDD33;

/// Cuánta luz conserva el nivel de fondo.
///
/// No se apaga del todo a propósito: ver el laberinto detrás recuerda que la
/// partida sigue ahí esperando, no que se perdió.
const BACKGROUND_DIM: f32 = 0.28;

const TITLE_SCALE: usize = 5;
const OPTION_SCALE: usize = 3;
const HINT_SCALE: usize = 2;

const OPTION_SPACING: usize = 48;
const BORDER_THICKNESS: usize = 2;

/// Separación entre el borde del panel y su contenido.
const PADDING: usize = 40;

const BAR_WIDTH: usize = 200;
const BAR_HEIGHT: usize = 14;

/// Qué eligió el jugador.
pub enum Choice {
    /// Seguir jugando el mismo nivel donde quedó.
    Resume,
    /// Empezar el nivel de nuevo.
    Retry,
    /// Abandonar y volver al menú principal.
    Menu,
    /// Prender o apagar la música.
    ToggleMusic,
    /// Subir el volumen de la música.
    VolumeUp,
    /// Bajarlo.
    VolumeDown,
}

/// Renglones del menú, en orden.
///
/// El orden no es arbitrario: `CONTINUAR` primero porque es lo más probable, y
/// `MENU PRINCIPAL` último porque es lo que no se quiere presionar por error.
#[derive(Copy, Clone, PartialEq)]
enum Row {
    Resume,
    Music,
    Volume,
    Retry,
    Menu,
}

const ROWS: [Row; 5] = [
    Row::Resume,
    Row::Music,
    Row::Volume,
    Row::Retry,
    Row::Menu,
];

impl Row {
    fn label(self) -> &'static str {
        match self {
            Row::Resume => "CONTINUAR",
            Row::Music => "MUSICA",
            Row::Volume => "VOLUMEN",
            Row::Retry => "REINTENTAR NIVEL",
            Row::Menu => "MENU PRINCIPAL",
        }
    }

    /// Qué hace `Enter` sobre este renglón.
    ///
    /// `VOLUMEN` no responde a `Enter` porque no es una acción: se ajusta con las
    /// flechas laterales.
    fn on_confirm(self) -> Option<Choice> {
        match self {
            Row::Resume => Some(Choice::Resume),
            Row::Music => Some(Choice::ToggleMusic),
            Row::Volume => None,
            Row::Retry => Some(Choice::Retry),
            Row::Menu => Some(Choice::Menu),
        }
    }
}

/// Posición del cursor dentro del menú de pausa.
pub struct Pause {
    selected: usize,
}

impl Pause {
    pub fn new() -> Self {
        Pause { selected: 0 }
    }

    /// Vuelve a dejar el cursor en `CONTINUAR`.
    ///
    /// Se llama al entrar en pausa: la opción más probable es seguir jugando, y
    /// dejarla preseleccionada convierte pausar y reanudar en `Esc` + `Enter` sin
    /// mirar la pantalla.
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn update(&mut self, window: &Window) -> Option<Choice> {
        // Escape reanuda: la misma tecla que pausó, para que sea reversible sin
        // pensar.
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            return Some(Choice::Resume);
        }

        let count = ROWS.len();

        if window.is_key_pressed(Key::Down, KeyRepeat::No)
            || window.is_key_pressed(Key::S, KeyRepeat::No)
        {
            self.selected = (self.selected + 1) % count;
        }

        if window.is_key_pressed(Key::Up, KeyRepeat::No)
            || window.is_key_pressed(Key::W, KeyRepeat::No)
        {
            self.selected = (self.selected + count - 1) % count;
        }

        // El volumen se ajusta con las flechas laterales, y con repetición: hay
        // que poder mantener la tecla en vez de dar dieciséis toques para
        // cruzar la barra.
        if ROWS[self.selected] == Row::Volume {
            if window.is_key_pressed(Key::Right, KeyRepeat::Yes)
                || window.is_key_pressed(Key::D, KeyRepeat::Yes)
            {
                return Some(Choice::VolumeUp);
            }

            if window.is_key_pressed(Key::Left, KeyRepeat::Yes)
                || window.is_key_pressed(Key::A, KeyRepeat::Yes)
            {
                return Some(Choice::VolumeDown);
            }
        }

        if window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::Space, KeyRepeat::No)
        {
            return ROWS[self.selected].on_confirm();
        }

        None
    }
}

/// Dibuja el panel encima de lo que ya haya en el framebuffer.
///
/// Espera que el nivel esté dibujado: lo primero que hace es atenuarlo.
///
/// Recibe el estado de la música en vez de una referencia al módulo de audio: a
/// esta pantalla sólo le hace falta saber qué mostrar, no cómo suena.
pub fn draw(framebuffer: &mut Framebuffer, pause: &Pause, music_on: bool, volume: f32) {
    framebuffer.darken(BACKGROUND_DIM);

    let width = framebuffer.width;
    let height = framebuffer.height;

    let panel_width = width * 3 / 5;
    let panel_height = height * 3 / 5;
    let panel_x = (width - panel_width) / 2;
    let panel_y = (height - panel_height) / 2;

    // borde y panel: dos rectángulos, no cuatro tiras.
    framebuffer.rect(
        panel_x.saturating_sub(BORDER_THICKNESS),
        panel_y.saturating_sub(BORDER_THICKNESS),
        panel_width + BORDER_THICKNESS * 2,
        panel_height + BORDER_THICKNESS * 2,
        BORDER_COLOR,
    );
    framebuffer.rect(panel_x, panel_y, panel_width, panel_height, PANEL_COLOR);

    text::draw_text_centered(
        framebuffer,
        panel_y + panel_height / 12,
        TITLE_SCALE,
        TITLE_COLOR,
        "PAUSA",
    );

    let label_x = panel_x + PADDING;
    let right_edge = panel_x + panel_width - PADDING;
    let first_y = panel_y + panel_height / 4;

    for (i, row) in ROWS.iter().enumerate() {
        let y = first_y + i * OPTION_SPACING;
        let selected = i == pause.selected;

        if selected {
            framebuffer.rect(
                panel_x + PADDING / 2,
                y.saturating_sub(6),
                panel_width - PADDING,
                text::text_height(OPTION_SCALE) + 12,
                HIGHLIGHT_COLOR,
            );
        }

        let color = if selected { SELECTED_COLOR } else { OPTION_COLOR };

        text::draw_text(framebuffer, label_x, y, OPTION_SCALE, color, row.label());

        // Los renglones de ajuste llevan su valor alineado a la derecha, para que
        // el ojo lo encuentre en una columna en vez de buscarlo tras cada
        // etiqueta.
        match row {
            Row::Music => {
                let value = if music_on { "ENCENDIDA" } else { "APAGADA" };
                let x = right_edge.saturating_sub(text::text_width(value, OPTION_SCALE));

                text::draw_text(framebuffer, x, y, OPTION_SCALE, VALUE_COLOR, value);
            }

            Row::Volume => {
                let x = right_edge.saturating_sub(BAR_WIDTH);
                let bar_y = y + text::text_height(OPTION_SCALE) / 2 - BAR_HEIGHT / 2;

                framebuffer.rect(x, bar_y, BAR_WIDTH, BAR_HEIGHT, BAR_BACKGROUND);

                let filled = (BAR_WIDTH as f32 * volume.clamp(0.0, 1.0)) as usize;

                framebuffer.rect(x, bar_y, filled, BAR_HEIGHT, BAR_FILL);
            }

            _ => {}
        }
    }

    let hints = [
        "ESC PARA SEGUIR JUGANDO",
        "FLECHAS LATERALES AJUSTAN EL VOLUMEN",
    ];

    for (i, hint) in hints.iter().enumerate() {
        let y = panel_y + panel_height - panel_height / 6 + i * (text::text_height(HINT_SCALE) + 10);

        text::draw_text_centered(framebuffer, y, HINT_SCALE, HINT_COLOR, hint);
    }
}
