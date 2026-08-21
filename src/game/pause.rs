//! Menú de pausa.
//!
//! A diferencia del menú principal y de la pantalla de victoria, esta pantalla
//! **no destruye la sesión**: el nivel sigue vivo detrás, congelado, y se dibuja
//! atenuado como fondo. Eso es justamente lo que la hace una pausa y no una
//! salida.
//!
//! Como el nivel de fondo se vuelve a dibujar en cada cuadro, la atenuación no
//! se acumula.

use minifb::{Key, KeyRepeat, Window};

use crate::framebuffer::Framebuffer;
use crate::render::text;

const PANEL_COLOR: u32 = 0x0E1116;
const BORDER_COLOR: u32 = 0x3A4652;
const TITLE_COLOR: u32 = 0xE8E4D8;
const SELECTED_COLOR: u32 = 0xFFDD33;
const OPTION_COLOR: u32 = 0x8A8F96;
const HIGHLIGHT_COLOR: u32 = 0x1C2128;
const HINT_COLOR: u32 = 0x5A6169;

/// Cuánta luz conserva el nivel de fondo.
///
/// No se apaga del todo a propósito: ver el laberinto detrás recuerda que la
/// partida sigue ahí esperando, no que se perdió.
const BACKGROUND_DIM: f32 = 0.28;

const TITLE_SCALE: usize = 5;
const OPTION_SCALE: usize = 3;
const HINT_SCALE: usize = 2;

const OPTION_SPACING: usize = 54;
const BORDER_THICKNESS: usize = 2;

/// Qué eligió el jugador.
pub enum Choice {
    /// Seguir jugando el mismo nivel donde quedó.
    Resume,
    /// Empezar el nivel de nuevo.
    Retry,
    /// Abandonar y volver al menú principal.
    Menu,
}

const OPTIONS: [&str; 3] = ["CONTINUAR", "REINTENTAR NIVEL", "MENU PRINCIPAL"];

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

        let count = OPTIONS.len();

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

        if window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::Space, KeyRepeat::No)
        {
            return Some(match self.selected {
                0 => Choice::Resume,
                1 => Choice::Retry,
                _ => Choice::Menu,
            });
        }

        None
    }
}

/// Dibuja el panel encima de lo que ya haya en el framebuffer.
///
/// Espera que el nivel esté dibujado: lo primero que hace es atenuarlo.
pub fn draw(framebuffer: &mut Framebuffer, pause: &Pause) {
    framebuffer.darken(BACKGROUND_DIM);

    let width = framebuffer.width;
    let height = framebuffer.height;

    let panel_width = width / 2;
    let panel_height = height / 2;
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
        panel_y + panel_height / 10,
        TITLE_SCALE,
        TITLE_COLOR,
        "PAUSA",
    );

    let first_y = panel_y + panel_height / 3;

    for (i, option) in OPTIONS.iter().enumerate() {
        let y = first_y + i * OPTION_SPACING;
        let selected = i == pause.selected;

        if selected {
            framebuffer.rect(
                panel_x + 20,
                y.saturating_sub(8),
                panel_width - 40,
                text::text_height(OPTION_SCALE) + 16,
                HIGHLIGHT_COLOR,
            );
        }

        let color = if selected { SELECTED_COLOR } else { OPTION_COLOR };

        text::draw_text_centered(framebuffer, y, OPTION_SCALE, color, option);
    }

    text::draw_text_centered(
        framebuffer,
        panel_y + panel_height - panel_height / 8,
        HINT_SCALE,
        HINT_COLOR,
        "ESC PARA SEGUIR JUGANDO",
    );
}
