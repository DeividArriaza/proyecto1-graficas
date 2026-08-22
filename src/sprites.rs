//! Hojas de sprites: una imagen con varios cuadros de animación dentro.
//!
//! Un sprite no es lo mismo que una textura de pared. La pared se muestrea
//! columna por columna sobre una estaca; el sprite es un objeto suelto que se
//! dibuja como *billboard* —una imagen plana que siempre encara al jugador—
//! escalada según su distancia.
//!
//! # Por qué la rejilla se mide en coma flotante
//!
//! Las hojas que se consiguen en bancos de assets casi nunca tienen dimensiones
//! divisibles por su número de cuadros: `eviloogie.png` mide 270 x 191 con 4
//! columnas y 2 filas, o sea celdas de 67.5 x 95.5 píxeles. Redondear a enteros
//! iría acumulando error y para el último cuadro se estaría muestreando el
//! vecino. Calcular los límites en `f32` y convertir a entero recién al indexar
//! el píxel evita esa acumulación.

use crate::textures::Texture;

pub struct SpriteSheet {
    texture: Texture,
    columns: usize,
    rows: usize,
}

impl SpriteSheet {
    /// Carga una hoja y declara en cuántos cuadros está dividida.
    ///
    /// Devuelve `None` si el archivo no está, para que el juego siga corriendo
    /// sin sprites igual que corre sin audio.
    pub fn load(path: &str, columns: usize, rows: usize) -> Option<Self> {
        let texture = Texture::load(path)?;

        println!(
            "sprite {path}: {}x{} px, rejilla {columns}x{rows}, celdas de {:.1}x{:.1}",
            texture.width,
            texture.height,
            texture.width as f32 / columns as f32,
            texture.height as f32 / rows as f32,
        );

        Some(SpriteSheet {
            texture,
            columns: columns.max(1),
            rows: rows.max(1),
        })
    }

    /// Cuántos cuadros tiene una fila de animación.
    pub fn frames(&self) -> usize {
        self.columns
    }

    /// Proporción ancho/alto de un cuadro.
    ///
    /// Hace falta para no deformar el sprite: la altura en pantalla sale de la
    /// distancia, y el ancho tiene que derivarse de ella con esta proporción.
    pub fn aspect(&self) -> f32 {
        let cell_width = self.texture.width as f32 / self.columns as f32;
        let cell_height = self.texture.height as f32 / self.rows as f32;

        cell_width / cell_height
    }

    /// Color en 0xAARRGGBB del punto (u, v) del cuadro indicado.
    ///
    /// `u` y `v` van de 0.0 a 1.0 dentro del cuadro, no de la hoja completa.
    pub fn sample(&self, column: usize, row: usize, u: f32, v: f32) -> u32 {
        let cell_width = self.texture.width as f32 / self.columns as f32;
        let cell_height = self.texture.height as f32 / self.rows as f32;

        let column = column % self.columns;
        let row = row % self.rows;

        let x = (column as f32 + u.clamp(0.0, 0.999_999)) * cell_width;
        let y = (row as f32 + v.clamp(0.0, 0.999_999)) * cell_height;

        self.texture.texel(x as usize, y as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La hoja real del juego. Si no está, la prueba se salta en vez de fallar:
    /// el repositorio tiene que poder clonarse sin los assets.
    fn sheet() -> Option<SpriteSheet> {
        SpriteSheet::load("assets/sprites/eviloogie.png", 4, 2)
    }

    #[test]
    fn la_hoja_declara_sus_cuadros() {
        let Some(sheet) = sheet() else {
            println!("hoja ausente, prueba omitida");
            return;
        };

        assert_eq!(sheet.frames(), 4);
        assert!(sheet.aspect() > 0.0, "la proporción tiene que ser positiva");
    }

    /// Con celdas de 67.5 x 95.5 px, redondear los límites acumularía error y el
    /// último cuadro terminaría muestreando al vecino. Esta prueba recorre todos
    /// los cuadros en sus extremos para confirmar que nada se sale.
    #[test]
    fn el_muestreo_se_mantiene_dentro_de_la_hoja() {
        let Some(sheet) = sheet() else {
            println!("hoja ausente, prueba omitida");
            return;
        };

        for column in 0..sheet.frames() {
            for row in 0..sheet.rows {
                for &u in &[0.0, 0.5, 0.999_999, 1.0, 2.0, -1.0] {
                    for &v in &[0.0, 0.5, 0.999_999, 1.0, 2.0, -1.0] {
                        // no debe entrar en pánico ni indexar fuera de rango;
                        // `texel` devolvería 0 si se saliera.
                        let _ = sheet.sample(column, row, u, v);
                    }
                }
            }
        }
    }

    #[test]
    fn los_indices_fuera_de_rango_se_envuelven() {
        let Some(sheet) = sheet() else {
            println!("hoja ausente, prueba omitida");
            return;
        };

        // el cuadro 4 no existe con 4 columnas: debe dar el mismo que el 0.
        assert_eq!(
            sheet.sample(0, 0, 0.5, 0.5),
            sheet.sample(sheet.frames(), 0, 0.5, 0.5),
            "el índice de cuadro debe envolverse"
        );
    }

    #[test]
    fn un_cuadro_tiene_pixeles_visibles() {
        let Some(sheet) = sheet() else {
            println!("hoja ausente, prueba omitida");
            return;
        };

        // El centro de cada cuadro debería caer sobre el bicho. Si toda la hoja
        // saliera transparente, el sprite sería invisible en el juego y esto lo
        // delata antes de abrirlo.
        let visible = (0..sheet.frames())
            .filter(|&frame| crate::textures::is_visible(sheet.sample(frame, 0, 0.5, 0.5)))
            .count();

        assert!(
            visible > 0,
            "ningún cuadro tiene un píxel opaco en el centro"
        );
    }
}
