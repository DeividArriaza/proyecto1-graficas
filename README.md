# 09 — Raycasting: Movimiento del Jugador

Tercera etapa de la fase de **Raycasting** del curso **cc2018 – Gráficas por Computadora** (UVG). En la etapa anterior el jugador aparecía en el laberinto con un rayo estático; ahora se agrega el **control del jugador**: el teclado modifica en cada cuadro su posición y su ángulo de vista. Sobre esa base se lanza un **abanico de rayos** que cubre el campo de visión, y se detecta la llegada a la meta. Con el campo de visión ya resuelto, lo que falta para la vista en primera persona es proyectar cada rayo como una columna en pantalla.

## Objetivo

- Leer el teclado dentro del ciclo de render y actualizar el estado del jugador cuadro a cuadro.
- Avanzar y retroceder al jugador en la dirección de su ángulo de vista, y girar ese ángulo.
- Lanzar un abanico de rayos repartido de forma pareja sobre el campo de visión.
- Detectar que el jugador llegó a la meta y terminar el juego.

## Controles

| Tecla | Acción |
| ----- | ------ |
| `W` | Avanzar en la dirección de vista |
| `S` | Retroceder |
| `A` | Girar a la izquierda |
| `D` | Girar a la derecha |
| `Escape` | Salir |

El movimiento se calcula con el ángulo de vista, de modo que avanzar siempre ocurre hacia donde el jugador está viendo:

```
pos.x += MOVE_SPEED * cos(a)
pos.y += MOVE_SPEED * sin(a)
```

En esta etapa no hay detección de colisiones: el jugador atraviesa las paredes.

## El campo de visión

Hasta ahora se lanzaba un solo rayo, en la dirección exacta de la vista. El **campo de visión** (`FOV`, *field of view*) es el ángulo total que abarca lo que el jugador alcanza a ver, y se cubre repartiendo `NUM_RAYS` rayos de forma pareja dentro de ese ángulo:

```
fracción = i / (NUM_RAYS - 1)        // de 0.0 a 1.0
ángulo   = a - FOV/2 + FOV * fracción
```

El primer rayo apunta a `a - FOV/2`, el último a `a + FOV/2` y el de en medio coincide con la dirección de vista. Con `NUM_RAYS = 5` el abanico se ve como cinco líneas separadas; en la vista en primera persona se lanzará un rayo por cada columna de píxeles de la pantalla, y la distancia recorrida por cada uno definirá la altura de la pared en esa columna.

## La meta

En cada cuadro la posición del jugador en píxeles se traduce a la celda que ocupa. Si esa celda es la marca `g`, el juego termina.

## Estructura

```
.
├── Cargo.toml          # Manifiesto del proyecto (minifb, nalgebra-glm)
├── Cargo.lock          # Versiones exactas de las dependencias
├── maze.txt            # Definición del laberinto en texto
└── src
    ├── main.rs         # Punto de entrada; ciclo de render, entrada y dibujo del mundo
    ├── framebuffer.rs  # Buffer de píxeles en memoria
    ├── maze.rs         # Carga del laberinto y estado inicial del jugador
    ├── player.rs       # Estado del jugador y lectura del teclado
    └── caster.rs       # Lanzamiento de un rayo en un ángulo dado
```

## Cómo correr

1. Clonar el repositorio y cambiar a esta rama:
    ```bash
    git clone https://github.com/menene/cc2018-2026-02-10.git
    cd cc2018-2026-02-10
    git checkout 09-RC-03-MAZE-MOVEMENT
    ```

2. Compilar y ejecutar:
    ```bash
    cargo run
    ```

3. Se abre una ventana con el laberinto y el jugador. Mover con `W`/`A`/`S`/`D` y observar cómo el abanico de rayos gira y se recorta contra las paredes. Al llegar a la celda marcada con `g` el programa avisa en la terminal y termina. Cerrar con `Escape` o con el botón de cerrar de la ventana.

## Recursos

- [Rust Programming Language](https://www.rust-lang.org/)
- [minifb](https://docs.rs/minifb/)
- [Raycasting](https://en.wikipedia.org/wiki/Ray_casting)
- [Lode's Computer Graphics Tutorial — Raycasting](https://lodev.org/cgtutor/raycasting.html)
