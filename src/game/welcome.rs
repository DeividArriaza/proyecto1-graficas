//! Pantalla de bienvenida y selección de nivel.
//!
//! Dibuja el menú y traduce el teclado en movimiento del cursor. No arranca la
//! partida: devuelve qué nivel se eligió y `main` decide qué hacer con eso.

use minifb::{Key, KeyRepeat, Window};

use crate::framebuffer::Framebuffer;
use crate::game::LEVELS;
use crate::render::text;

const BACKGROUND: u32 = 0x0B0D10;
const TITLE_COLOR: u32 = 0xE8E4D8;
const SUBTITLE_COLOR: u32 = 0x6E7A85;
const SELECTED_COLOR: u32 = 0xFFDD33;
const OPTION_COLOR: u32 = 0x8A8F96;
const HINT_COLOR: u32 = 0x555B61;

/// Franja detrás de la opción seleccionada, para que el cursor se vea de
/// inmediato sin tener que comparar colores de texto.
const HIGHLIGHT_COLOR: u32 = 0x1C2128;

const TITLE_SCALE: usize = 6;
const SUBTITLE_SCALE: usize = 2;
const OPTION_SCALE: usize = 3;
const HINT_SCALE: usize = 2;

/// Separación vertical entre opciones del menú, en píxeles.
const OPTION_SPACING: usize = 52;

/// Etiqueta de la opción que cierra el juego.
const QUIT_LABEL: &str = "SALIR";

/// Qué eligió el jugador.
pub enum Action {
    /// Jugar el nivel en esa posición de `LEVELS`.
    Play(usize),
    /// Cerrar el juego.
    Quit,
}

/// Cuántas opciones tiene el menú: los niveles más `SALIR`.
fn option_count() -> usize {
    LEVELS.len() + 1
}

/// Texto de la opción en la posición `index`.
fn option_label(index: usize) -> &'static str {
    LEVELS
        .get(index)
        .map(|level| level.name)
        .unwrap_or(QUIT_LABEL)
}

/// Estado del menú: nada más que dónde está el cursor.
pub struct Menu {
    pub selected: usize,
}

impl Menu {
    pub fn new() -> Self {
        Menu { selected: 0 }
    }

    /// Lee el teclado y mueve el cursor. Devuelve la acción si se confirmó con
    /// Enter o Espacio.
    ///
    /// El desplazamiento es circular: bajar en la última opción vuelve a la
    /// primera. En un menú de cinco entradas, chocar contra el borde es más
    /// molesto que útil.
    pub fn update(&mut self, window: &Window) -> Option<Action> {
        let count = option_count();

        if window.is_key_pressed(Key::Down, KeyRepeat::No) || window.is_key_pressed(Key::S, KeyRepeat::No) {
            self.selected = (self.selected + 1) % count;
        }

        if window.is_key_pressed(Key::Up, KeyRepeat::No) || window.is_key_pressed(Key::W, KeyRepeat::No) {
            // se suma `count - 1` en vez de restar 1: con `usize` la resta desde
            // cero desbordaría.
            self.selected = (self.selected + count - 1) % count;
        }

        if window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::Space, KeyRepeat::No)
        {
            // la última opción no es un nivel, es la salida.
            return Some(if self.selected < LEVELS.len() {
                Action::Play(self.selected)
            } else {
                Action::Quit
            });
        }

        None
    }
}

pub fn draw(framebuffer: &mut Framebuffer, menu: &Menu) {
    framebuffer.set_background_color(BACKGROUND);
    framebuffer.clear();

    let width = framebuffer.width;
    let height = framebuffer.height;

    text::draw_text_centered(framebuffer, height / 8, TITLE_SCALE, TITLE_COLOR, "LETHAL MAZE");
    text::draw_text_centered(
        framebuffer,
        height / 8 + text::text_height(TITLE_SCALE) + 24,
        SUBTITLE_SCALE,
        SUBTITLE_COLOR,
        "INSTALACION ABANDONADA - ENCUENTRA LA SALIDA",
    );

    // el bloque de opciones se centra verticalmente en la pantalla.
    let block_height = option_count() * OPTION_SPACING;
    let first_y = (height / 2 + height / 12).saturating_sub(block_height / 2);

    for i in 0..option_count() {
        let y = first_y + i * OPTION_SPACING;
        let selected = i == menu.selected;

        if selected {
            let band_height = text::text_height(OPTION_SCALE) + 16;

            framebuffer.rect(
                width / 6,
                y.saturating_sub(8),
                width - width / 3,
                band_height,
                HIGHLIGHT_COLOR,
            );
        }

        let color = if selected { SELECTED_COLOR } else { OPTION_COLOR };

        text::draw_text_centered(framebuffer, y, OPTION_SCALE, color, option_label(i));
    }

    let hints = [
        "W/S O FLECHAS PARA ELEGIR   ENTER CONFIRMA   ESC SALE",
        "WASD MOVER   MOUSE O AD GIRAR   SHIFT CORRER   M LINTERNA   ESC MENU",
    ];

    for (i, hint) in hints.iter().enumerate() {
        let y = height - height / 8 + i * (text::text_height(HINT_SCALE) + 12);

        text::draw_text_centered(framebuffer, y, HINT_SCALE, HINT_COLOR, hint);
    }
}
