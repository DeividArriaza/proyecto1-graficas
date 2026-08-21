# Texturas

Los PNG de este directorio reemplazan a las texturas generadas en código.
Cualquier archivo que falte se genera solo, así que se pueden ir agregando de a
una.

| Archivo | Celda del laberinto | Aspecto esperado |
| --- | --- | --- |
| `concrete.png` | `\|` | Concreto industrial sucio |
| `steel.png` | `-` | Panel de acero remachado |
| `hazard.png` | `+` | Franjas diagonales de peligro |
| `terminal.png` | `g` | Pantalla o terminal: marca la salida |

Requisitos: PNG cuadrado, potencia de dos, 64x64 o 128x128. Más grande es
desperdicio — a las alturas de estaca del juego el detalle extra no se alcanza a
ver y encarece cada muestreo.

Fuentes con licencia libre: ambientCG, Kenney.nl, OpenGameArt (filtrar por CC0).
