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

    /// Pinta la columna vertical `x` entre las filas [y0, y1).
    ///
    /// Los píxeles de una columna no son contiguos (van separados por `width`),
    /// así que no se puede usar `fill`, pero al menos los límites se verifican
    /// una sola vez y no en cada píxel.
    pub fn column(&mut self, x: usize, y0: usize, y1: usize, color: u32) {
        if x >= self.width {
            return;
        }

        let y1 = y1.min(self.height);

        for y in y0..y1 {
            self.buffer[y * self.width + x] = color;
        }
    }

    pub fn set_background_color(&mut self, color: u32) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }
}
