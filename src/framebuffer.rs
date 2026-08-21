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
