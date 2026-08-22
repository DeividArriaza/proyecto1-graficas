pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    background_color: u32,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            background_color: 0x000000,
            current_color: 0xFFFFFF,
        }
    }

    /// Multiplica todos los píxeles por `factor`, entre 0.0 y 1.0.
    ///
    /// Lo usa la pantalla de pausa para atenuar el nivel que quedó dibujado
    /// detrás. Va aquí y no en `render::lighting` porque no es iluminación: es
    /// una operación sobre el búfer entero, sin noción de distancia ni de luz.
    ///
    /// Cuidado al usarla: es acumulativa. Aplicarla cuadro tras cuadro sobre el
    /// mismo contenido lo va llevando a negro, así que lo de abajo tiene que
    /// redibujarse cada cuadro.
    pub fn darken(&mut self, factor: f32) {
        let factor = factor.clamp(0.0, 1.0);

        for pixel in self.buffer.iter_mut() {
            let r = ((*pixel >> 16) & 0xFF) as f32 * factor;
            let g = ((*pixel >> 8) & 0xFF) as f32 * factor;
            let b = (*pixel & 0xFF) as f32 * factor;

            *pixel = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(self.background_color);
    }

    pub fn point(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = self.current_color;
        }
    }

    /// Pinta de golpe las filas [y0, y1) completas.
    ///
    /// Las filas son contiguas en memoria, así que se resuelven con un solo
    /// `fill` sobre el slice en vez de un `point()` por píxel: se ahorra la
    /// verificación de límites en cada uno de los ~1.17 millones de píxeles.
    pub fn fill_rows(&mut self, y0: usize, y1: usize, color: u32) {
        let y1 = y1.min(self.height);

        if y0 >= y1 {
            return;
        }

        self.buffer[y0 * self.width..y1 * self.width].fill(color);
    }

    /// Pinta un solo píxel del color dado.
    ///
    /// Distinto de `point()`, que usa el color actual del framebuffer: cuando
    /// cada píxel tiene su propio color —una columna texturizada— arrastrar el
    /// estado del color no sirve de nada.
    pub fn pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    /// Pinta la columna vertical `x` entre las filas [y0, y1).
    ///
    /// Sin uso desde que las paredes se texturizan: cada píxel de la columna
    /// tiene su propio color. Se conserva porque sigue siendo la forma correcta
    /// de pintar una columna de un solo color.
    ///
    /// Los píxeles de una columna no son contiguos (van separados por `width`),
    /// así que no se puede usar `fill`, pero al menos los límites se verifican
    /// una sola vez y no en cada píxel.
    #[allow(dead_code)]
    pub fn column(&mut self, x: usize, y0: usize, y1: usize, color: u32) {
        if x >= self.width {
            return;
        }

        let y1 = y1.min(self.height);

        for y in y0..y1 {
            self.buffer[y * self.width + x] = color;
        }
    }

    /// Pinta el rectángulo sólido de `width` x `height` con esquina superior
    /// izquierda en (x, y), recortado contra los límites del framebuffer.
    ///
    /// Cada fila del rectángulo sí es contigua en memoria, así que se resuelve
    /// con un `fill` por fila en vez de un `point()` por píxel.
    pub fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        let x1 = (x + width).min(self.width);
        let y1 = (y + height).min(self.height);

        if x >= x1 || y >= y1 {
            return;
        }

        for row in y..y1 {
            let start = row * self.width + x;
            let end = row * self.width + x1;
            self.buffer[start..end].fill(color);
        }
    }

    pub fn set_background_color(&mut self, color: u32) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dibujar_fuera_de_los_limites_no_entra_en_panico() {
        let mut fb = Framebuffer::new(10, 10);

        // cada uno de estos se sale por un lado distinto
        fb.pixel(100, 5, 0xFF0000);
        fb.pixel(5, 100, 0xFF0000);
        fb.point(usize::MAX, 0);
        fb.rect(8, 8, 100, 100, 0xFF0000);
        fb.rect(100, 100, 10, 10, 0xFF0000);
        fb.column(100, 0, 5, 0xFF0000);
        fb.column(5, 0, 100, 0xFF0000);
        fb.fill_rows(0, 100, 0xFF0000);
        fb.fill_rows(50, 60, 0xFF0000);
    }

    #[test]
    fn el_rectangulo_se_recorta_al_borde() {
        let mut fb = Framebuffer::new(4, 4);

        // pedir 3x3 desde (2,2) sólo puede pintar 2x2
        fb.rect(2, 2, 3, 3, 0xABCDEF);

        assert_eq!(fb.buffer[2 * 4 + 2], 0xABCDEF, "dentro");
        assert_eq!(fb.buffer[3 * 4 + 3], 0xABCDEF, "esquina");
        assert_eq!(fb.buffer[1 * 4 + 2], 0, "una fila más arriba, intacta");
        assert_eq!(fb.buffer[2 * 4 + 1], 0, "una columna a la izquierda, intacta");
    }

    /// Un rectángulo no puede desbordarse a la fila siguiente. Si el recorte
    /// horizontal se hiciera mal, pintar de ancho excesivo escribiría en el
    /// principio de la fila de abajo y se vería como una banda diagonal.
    #[test]
    fn el_rectangulo_no_se_derrama_a_la_fila_siguiente() {
        let mut fb = Framebuffer::new(4, 4);

        fb.rect(2, 0, 10, 1, 0xABCDEF);

        assert_eq!(fb.buffer[0], 0, "columna 0 de la fila 0 intacta");
        assert_eq!(fb.buffer[2], 0xABCDEF);
        assert_eq!(fb.buffer[3], 0xABCDEF);
        assert_eq!(fb.buffer[4], 0, "la fila 1 no debe haberse tocado");
        assert_eq!(fb.buffer[5], 0);
    }

    #[test]
    fn un_rectangulo_vacio_no_pinta_nada() {
        let mut fb = Framebuffer::new(4, 4);

        fb.rect(1, 1, 0, 5, 0xFF0000);
        fb.rect(1, 1, 5, 0, 0xFF0000);

        assert!(fb.buffer.iter().all(|&p| p == 0));
    }

    #[test]
    fn atenuar_en_los_extremos() {
        let mut fb = Framebuffer::new(2, 1);

        fb.buffer[0] = 0xFFFFFF;
        fb.buffer[1] = 0x804020;

        fb.darken(1.0);
        assert_eq!(fb.buffer[0], 0xFFFFFF, "factor 1 no cambia nada");
        assert_eq!(fb.buffer[1], 0x804020);

        fb.darken(0.0);
        assert_eq!(fb.buffer[0], 0x000000, "factor 0 lleva a negro");
        assert_eq!(fb.buffer[1], 0x000000);
    }

    #[test]
    fn atenuar_no_mezcla_canales() {
        let mut fb = Framebuffer::new(1, 1);

        fb.buffer[0] = 0xFF0000;
        fb.darken(0.5);

        assert_eq!(fb.buffer[0], 0x7F0000, "el rojo baja y los otros siguen en cero");
    }

    #[test]
    fn limpiar_usa_el_color_de_fondo() {
        let mut fb = Framebuffer::new(3, 3);

        fb.set_background_color(0x123456);
        fb.clear();

        assert!(fb.buffer.iter().all(|&p| p == 0x123456));
    }

    #[test]
    fn las_filas_se_pintan_completas() {
        let mut fb = Framebuffer::new(3, 3);

        fb.fill_rows(1, 2, 0xAAAAAA);

        assert!(fb.buffer[0..3].iter().all(|&p| p == 0), "fila 0 intacta");
        assert!(fb.buffer[3..6].iter().all(|&p| p == 0xAAAAAA), "fila 1 pintada");
        assert!(fb.buffer[6..9].iter().all(|&p| p == 0), "fila 2 intacta");
    }
}
