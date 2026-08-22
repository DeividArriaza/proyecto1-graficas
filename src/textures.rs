//! Texturas de las paredes.
//!
//! Cada carácter del laberinto tiene su propia textura, que es lo que pide la
//! rúbrica: paredes distintas se tienen que ver distintas.
//!
//! # Archivos y respaldo procedural
//!
//! Las texturas se cargan de `assets/<nombre>.png`. Si el archivo no está, se
//! **genera una en código** con el mismo aspecto industrial. Eso permite dos
//! cosas: que el proyecto compile y corra sin depender de archivos binarios en
//! el repositorio, y que reemplazar una textura sea copiar un PNG a `assets/`
//! sin tocar una línea.
//!
//! Los PNG deben ser cuadrados y potencia de dos —64x64 o 128x128—. Más grande
//! es desperdicio: a las alturas de estaca de este juego el detalle extra no se
//! alcanza a ver y encarece cada muestreo.

/// Lado de las texturas generadas en código.
const GENERATED_SIZE: usize = 64;

/// Color para las celdas cuyo carácter no tiene textura asignada.
///
/// Magenta a propósito: es el color universal de "aquí falta una textura". Si
/// aparece en pantalla, hay un carácter en el laberinto que nadie mapeó, y se
/// nota de inmediato en vez de pasar por una pared cualquiera.
const MISSING_TEXTURE: u32 = 0xFF00FF;

pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// Píxeles en 0xAARRGGBB, fila por fila.
    ///
    /// El alfa va en el byte alto. Las paredes lo ignoran —son opacas por
    /// definición— pero los sprites lo necesitan: un monstruo sin transparencia
    /// se ve como un rectángulo de fondo alrededor del bicho.
    pixels: Vec<u32>,
}

impl Texture {
    /// Carga un PNG sin respaldo procedural.
    ///
    /// La usan los sprites, donde no tiene sentido inventar una imagen: una
    /// pared generada en código se ve razonable, un monstruo no.
    pub fn load(path: &str) -> Option<Self> {
        let image = image::open(path).ok()?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let pixels = rgba
            .pixels()
            .map(|p| {
                ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32
            })
            .collect();

        Some(Texture {
            width: width as usize,
            height: height as usize,
            pixels,
        })
    }

    /// Carga un PNG, o genera la textura de respaldo si el archivo no existe o
    /// no se puede leer.
    fn load_or_generate(path: &str, kind: Kind) -> Self {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();

                let pixels = rgba
                    .pixels()
                    .map(|p| {
                        ((p[3] as u32) << 24)
                            | ((p[0] as u32) << 16)
                            | ((p[1] as u32) << 8)
                            | p[2] as u32
                    })
                    .collect();

                Texture {
                    width: width as usize,
                    height: height as usize,
                    pixels,
                }
            }
            Err(_) => {
                println!("textura {path} no encontrada; se genera en código");
                generate(kind)
            }
        }
    }

    /// Color del píxel (x, y), en 0xAARRGGBB. Fuera de rango devuelve
    /// transparente en vez de entrar en pánico: una textura mal formada no
    /// debería tumbar el juego.
    pub fn texel(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }

        self.pixels[y * self.width + x]
    }

    /// Color en coordenadas normalizadas, las dos de 0.0 a 1.0.
    ///
    /// Lo usa el minimapa, que necesita encoger la textura completa dentro de
    /// una casilla de pocos píxeles en vez de recorrer una columna.
    pub fn texel_uv(&self, u: f32, v: f32) -> u32 {
        let x = ((u.clamp(0.0, 0.999_999) * self.width as f32) as usize).min(self.width - 1);
        let y = ((v.clamp(0.0, 0.999_999) * self.height as f32) as usize).min(self.height - 1);

        self.texel(x, y)
    }

    /// Convierte una coordenada de 0.0 a 1.0 en índice de columna.
    pub fn column_of(&self, u: f32) -> usize {
        ((u * self.width as f32) as usize).min(self.width - 1)
    }

    /// Color de la columna `x` a la altura relativa `v`, de 0.0 (arriba de la
    /// pared) a 1.0 (abajo).
    pub fn column_texel(&self, x: usize, v: f32) -> u32 {
        let y = ((v.clamp(0.0, 0.999_999) * self.height as f32) as usize).min(self.height - 1);

        self.texel(x, y)
    }
}

/// Qué aspecto genera el respaldo procedural.
#[derive(Copy, Clone)]
enum Kind {
    /// Concreto sucio: la pared común de la instalación.
    Concrete,
    /// Panel de acero remachado, con juntas.
    SteelPanel,
    /// Franjas diagonales amarillas y negras.
    Hazard,
    /// Terminal verde: marca la salida.
    Terminal,
}

/// Las texturas cargadas, indexadas por el carácter de la celda.
pub struct TextureSet {
    concrete: Texture,
    steel: Texture,
    hazard: Texture,
    terminal: Texture,
    /// Para cualquier carácter desconocido: un color plano del tamaño mínimo.
    fallback: Texture,
}

impl TextureSet {
    pub fn load() -> Self {
        TextureSet {
            concrete: Texture::load_or_generate("assets/concrete.png", Kind::Concrete),
            steel: Texture::load_or_generate("assets/steel.png", Kind::SteelPanel),
            hazard: Texture::load_or_generate("assets/hazard.png", Kind::Hazard),
            terminal: Texture::load_or_generate("assets/terminal.png", Kind::Terminal),
            fallback: flat(MISSING_TEXTURE),
        }
    }

    /// Qué textura le toca a cada tipo de celda.
    pub fn for_cell(&self, cell: char) -> &Texture {
        match cell {
            '|' => &self.concrete,
            '-' => &self.steel,
            '+' => &self.hazard,
            'g' | 'G' => &self.terminal,
            _ => &self.fallback,
        }
    }
}

/// Textura de un solo píxel, de color plano y opaca.
fn flat(color: u32) -> Texture {
    Texture {
        width: 1,
        height: 1,
        pixels: vec![color | OPAQUE],
    }
}

/// Bits de alfa de un píxel completamente opaco.
pub const OPAQUE: u32 = 0xFF00_0000;

/// ¿Este píxel se dibuja?
///
/// El umbral está a mitad de camino y no en cero: los bordes de un sprite suelen
/// traer píxeles semitransparentes del antialias, y dibujarlos sobre un fondo
/// que no se mezcla deja un halo. Descartarlos da un contorno duro, que es lo
/// correcto sin composición alfa real.
pub fn is_visible(pixel: u32) -> bool {
    (pixel >> 24) >= 128
}

/// Ruido entero determinista.
///
/// No usa un generador de aleatorios a propósito: la misma coordenada tiene que
/// dar siempre el mismo valor, o la textura cambiaría en cada arranque. Es la
/// mezcla de multiplicaciones y desplazamientos típica de una función hash: no
/// tiene significado geométrico, sólo reparte bits.
fn noise(x: usize, y: usize, seed: u32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(2_246_822_519));

    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    h ^= h >> 16;

    h
}

/// Aclara u oscurece un color por un factor multiplicativo.
fn scale(color: u32, factor: f32) -> u32 {
    let channel = |shift: u32| {
        let value = ((color >> shift) & 0xFF) as f32 * factor;
        (value.clamp(0.0, 255.0) as u32) << shift
    };

    channel(16) | channel(8) | channel(0)
}

fn generate(kind: Kind) -> Texture {
    let size = GENERATED_SIZE;
    let mut pixels = Vec::with_capacity(size * size);

    for y in 0..size {
        for x in 0..size {
            pixels.push(OPAQUE | match kind {
                Kind::Concrete => concrete_texel(x, y),
                Kind::SteelPanel => steel_texel(x, y, size),
                Kind::Hazard => hazard_texel(x, y),
                Kind::Terminal => terminal_texel(x, y, size),
            });
        }
    }

    Texture {
        width: size,
        height: size,
        pixels,
    }
}

fn concrete_texel(x: usize, y: usize) -> u32 {
    const BASE: u32 = 0x4A5248;

    // dos escalas de ruido: grano fino píxel a píxel, y manchones de 8x8 que
    // rompen la uniformidad para que no se vea como televisión sin señal.
    let grain = (noise(x, y, 1) % 20) as f32 / 100.0;
    let blotch = (noise(x / 8, y / 8, 2) % 18) as f32 / 100.0;

    scale(BASE, 0.82 + grain + blotch)
}

fn steel_texel(x: usize, y: usize, size: usize) -> u32 {
    const BASE: u32 = 0x6B6B70;

    let half = size / 2;

    // juntas: el panel se divide en cuatro cuadrantes con una ranura oscura.
    if x % half == 0 || y % half == 0 {
        return scale(BASE, 0.45);
    }

    // remaches: un punto claro cerca de cada esquina de cuadrante.
    let near_x = (x % half).min(half - x % half);
    let near_y = (y % half).min(half - y % half);
    if near_x <= 2 && near_y <= 2 {
        return scale(BASE, 1.35);
    }

    let grain = (noise(x, y, 3) % 12) as f32 / 100.0;

    scale(BASE, 0.94 + grain)
}

fn hazard_texel(x: usize, y: usize) -> u32 {
    const YELLOW: u32 = 0xC8A020;
    const BLACK: u32 = 0x1A1A1A;

    // franjas a 45 grados: la diagonal es constante sobre x + y, así que
    // dividirla en bandas de 8 da las rayas inclinadas.
    let stripe = ((x + y) / 8) % 2 == 0;

    let base = if stripe { YELLOW } else { BLACK };

    // desgaste, para que no se vea como un vector recién impreso.
    let wear = (noise(x, y, 4) % 16) as f32 / 100.0;

    scale(base, 0.88 + wear)
}

fn terminal_texel(x: usize, y: usize, size: usize) -> u32 {
    const PANEL: u32 = 0x101410;
    const GLOW: u32 = 0x40FF80;

    let margin = size / 8;

    // marco oscuro alrededor de una pantalla verde.
    if x < margin || y < margin || x >= size - margin || y >= size - margin {
        return PANEL;
    }

    // líneas de barrido horizontales, como un monitor viejo.
    if y % 4 == 0 {
        return scale(GLOW, 0.35);
    }

    let flickerless = (noise(x, y, 5) % 10) as f32 / 100.0;

    scale(GLOW, 0.55 + flickerless)
}

#[cfg(test)]
mod layout_tests {
    /// Diagnóstico: mapea qué columnas y filas de una hoja de sprites tienen
    /// píxeles opacos. Los huecos totalmente transparentes son las separaciones
    /// entre cuadros, así que el mapa revela la distribución real sin abrir la
    /// imagen en un editor.
    ///
    ///     cargo test -- --ignored --nocapture distribucion_de_sprites
    #[test]
    #[ignore]
    fn distribucion_de_sprites() {
        for name in ["smorficus", "eviloogie", "demonario"] {
            let path = format!("assets/sprites/{name}.png");

            let Ok(image) = image::open(&path) else {
                println!("{path}: ausente");
                continue;
            };

            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();

            println!("\n=== {path}  {width}x{height}");

            let opaque_column = |x: u32| (0..height).any(|y| rgba.get_pixel(x, y)[3] > 8);
            let opaque_row = |y: u32| (0..width).any(|x| rgba.get_pixel(x, y)[3] > 8);

            let cols: String = (0..width)
                .map(|x| if opaque_column(x) { '#' } else { '.' })
                .collect();
            let rows: String = (0..height)
                .map(|y| if opaque_row(y) { '#' } else { '.' })
                .collect();

            println!("columnas: {cols}");
            println!("filas:    {rows}");

            // límites de cada bloque contiguo de columnas ocupadas
            let mut spans = Vec::new();
            let mut start = None;
            for x in 0..width {
                match (opaque_column(x), start) {
                    (true, None) => start = Some(x),
                    (false, Some(s)) => {
                        spans.push((s, x - 1));
                        start = None;
                    }
                    _ => {}
                }
            }
            if let Some(s) = start {
                spans.push((s, width - 1));
            }

            println!("bloques de columnas ({}): {spans:?}", spans.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_respaldo_procedural_tiene_el_tamano_declarado() {
        let texture = Texture::load_or_generate("no/existe/nada.png", Kind::Concrete);

        assert_eq!(texture.width, GENERATED_SIZE);
        assert_eq!(texture.height, GENERATED_SIZE);
    }

    #[test]
    fn las_texturas_generadas_son_opacas() {
        for kind in [
            Kind::Concrete,
            Kind::SteelPanel,
            Kind::Hazard,
            Kind::Terminal,
        ] {
            let texture = generate(kind);

            for y in 0..texture.height {
                for x in 0..texture.width {
                    assert!(
                        is_visible(texture.texel(x, y)),
                        "un píxel de pared salió transparente en ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn fuera_de_rango_devuelve_transparente() {
        let texture = generate(Kind::Concrete);

        assert_eq!(texture.texel(GENERATED_SIZE, 0), 0);
        assert_eq!(texture.texel(0, GENERATED_SIZE), 0);
        assert!(!is_visible(texture.texel(9999, 9999)));
    }

    /// El muestreo tiene que quedar dentro de la textura para cualquier `u` o
    /// `v`, incluidos los valores límite y los que vienen mal.
    #[test]
    fn el_muestreo_nunca_se_sale() {
        let texture = generate(Kind::Hazard);

        for &u in &[0.0, 0.5, 0.999_999, 1.0, 1.5, -0.5, f32::NAN] {
            let column = texture.column_of(u);

            assert!(
                column < texture.width,
                "u = {u} dio la columna {column} de {}",
                texture.width
            );

            for &v in &[0.0, 0.999_999, 1.0, 2.0, -1.0] {
                // basta con que no entre en pánico y devuelva algo visible
                assert!(is_visible(texture.column_texel(column, v)));
            }
        }
    }

    #[test]
    fn el_umbral_de_visibilidad() {
        assert!(!is_visible(0x00_FFFFFF), "alfa 0");
        assert!(!is_visible(0x7F_FFFFFF), "alfa 127, justo por debajo");
        assert!(is_visible(0x80_FFFFFF), "alfa 128, justo en el umbral");
        assert!(is_visible(0xFF_000000), "alfa pleno, aunque el color sea negro");
    }

    #[test]
    fn cada_tipo_de_pared_tiene_su_textura() {
        let set = TextureSet::load();

        // Se comparan por puntero: lo que importa es que no sean la misma, no
        // qué color tienen.
        let concrete = set.for_cell('|') as *const Texture;
        let steel = set.for_cell('-') as *const Texture;
        let hazard = set.for_cell('+') as *const Texture;
        let goal = set.for_cell('g') as *const Texture;
        let unknown = set.for_cell('?') as *const Texture;

        let all = [concrete, steel, hazard, goal];

        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "dos tipos de pared comparten textura");
            }
        }

        assert_ne!(concrete, unknown, "lo desconocido no debe verse como pared");
        assert_eq!(set.for_cell('G') as *const Texture, goal, "g y G son la meta");
    }
}
