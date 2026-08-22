# Lethal Maze

Raycaster en Rust: un laberinto en primera persona, a oscuras, con una linterna
que se gasta y algo caminando por los pasillos.

Proyecto 1 del curso **cc2018 – Gráficas por Computadora** (UVG).

## Cómo correr

```bash
cargo run --release
```

Se abre una ventana de 960 x 600. El juego arranca en el menú.

La ventana se puede **redimensionar y maximizar**, y el contenido se escala.
`minifb` no ofrece pantalla completa, así que maximizar es lo más cerca que se
llega. Al maximizar a una proporción distinta de 8:5 la imagen se estira un poco.

No hace falta ningún archivo externo para jugar: si falta una textura se genera
en código, y si falta un archivo de audio se omite en silencio.

## Controles

| Tecla | Acción |
| --- | --- |
| `W` `A` `S` `D` | Moverse |
| Mouse | Girar (horizontal) |
| `A` / `D` | Girar con el teclado |
| `Shift` o `Ctrl` | Correr |
| `M` | Prender y apagar la linterna |
| `Esc` | Pausa dentro del nivel; salir desde el menú |
| `Enter` | Confirmar |

En la pausa: `W`/`S` para elegir, flechas laterales para el volumen de la música.

## Qué tiene

**Motor**

- Raycasting con **DDA**, un rayo por columna de pantalla. Devuelve distancia,
  cara impactada y coordenada de textura.
- Corrección de ojo de pez, y reparto de rayos por posiciones parejas sobre el
  plano de proyección en vez de por ángulos parejos.
- Paredes texturizadas, con las caras norte y sur más oscuras que las este y
  oeste para que las esquinas se lean.
- Piso y techo con degradado por distancia real, no color plano.
- Búfer de profundidad por columna, que es lo que permite ocluir sprites.
- Sprites tipo *billboard* con transparencia y animación por cuadros.

**Juego**

- Tres niveles fijos y un **modo infinito** que genera un laberinto nuevo cada
  partida.
- Generación por recorrido en profundidad, con la meta colocada en la celda más
  lejana medida por recorrido en anchura. La conectividad está garantizada por
  construcción.
- **Linterna con batería**: se gasta encendida, se repone apagada, parpadea
  cuando queda poca. Sin ella se ve el pasillo inmediato y nada más.
- **Niebla de guerra** en el minimapa: sólo se revela lo que la linterna alcanzó,
  y el rastro propio se recuerda siempre.
- Un monstruo que patrulla los pasillos. Atraparte termina el nivel.
- Pantallas de bienvenida, pausa y desenlace, con informe de tiempo, exploración
  y batería restante.
- Música, ambiente y efectos, con la cadencia de los pasos atada al
  desplazamiento real.

## Estructura

```
src/
├── main.rs          Ciclo de juego: fase de actualización y fase de dibujo
├── caster.rs        Lanzamiento de rayos (DDA). Geometría pura
├── maze.rs          Tipo del laberinto, carga y consulta de celdas
├── mazegen.rs       Generación por DFS + colocación de la meta por BFS
├── player.rs        Estado, colisiones y entrada del jugador
├── monster.rs       Posición, patrulla y animación del monstruo
├── flashlight.rs    Batería de la linterna
├── discovery.rs     Qué celdas se han visto (niebla de guerra)
├── framebuffer.rs   Búfer de píxeles
├── textures.rs      Texturas de pared, con respaldo procedural
├── sprites.rs       Hojas de sprites
├── audio.rs         Música y efectos
├── fps.rs           Medición de cuadros por segundo
├── game/            Estados y pantallas
│   ├── mod.rs       Pantalla activa, niveles, sesión, informe
│   ├── welcome.rs   Menú principal
│   ├── pause.rs     Menú de pausa
│   └── outcome.rs   Informe de éxito o derrota
└── render/          Dibujo. Nada de acá conoce la entrada del usuario
    ├── mod.rs       FOV y altura del ojo
    ├── world.rs     Vista en primera persona
    ├── billboard.rs Proyección de sprites
    ├── minimap.rs   Minimapa de la esquina
    ├── lighting.rs  Modelo de luz: ambiental y haz de la linterna
    ├── text.rs      Fuente de mapa de bits 5x7
    └── hud.rs       Contador de FPS y estado de la linterna
```

`main.rs` no dibuja y `render/` no lee el teclado.

## Assets

Todo lo de `assets/` es reemplazable sin tocar código. Cada carpeta tiene su
propio `README.md` con los nombres y formatos esperados.

```
assets/
├── levels/    Los tres niveles fijos. Si falta uno, se genera
├── audio/     Música, ambiente y efectos
└── sprites/   Hojas de sprites
```

Sprites: derivados de trabajos de **surt** y **NMN**, tomados de
[FPS Monster Enemies](https://opengameart.org/content/fps-monster-enemies) (CC0).

## Pruebas

```bash
cargo test
```

72 pruebas. Cubren las colisiones, el caster contra geometría conocida, el modelo
de luz, la batería, los límites del framebuffer, el muestreo de texturas y
sprites, la cobertura de la fuente, la generación de laberintos y la patrulla del
monstruo.

La más completa dibuja el juego entero en memoria —los cuatro niveles, en dos
resoluciones, con todas las capas— y verifica que nada se rompa.

Hay tres pruebas de diagnóstico marcadas `ignore`:

```bash
cargo test -- --ignored --nocapture muestra_un_laberinto      # imprime un laberinto
cargo test -- --ignored --nocapture duraciones                # mide los audios
cargo test -- --ignored --nocapture distancia_a_la_meta       # ubica al monstruo
```

## Video

En `video/`. Ver el `README.md` de esa carpeta.
