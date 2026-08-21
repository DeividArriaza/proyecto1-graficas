# Niveles

Los tres niveles fijos se cargan de este directorio. Si un archivo no está, el
nivel se genera con una semilla fija — el mismo laberinto en cada arranque, así
que sigue siendo un nivel y no una sorpresa distinta cada vez.

| Archivo | Nivel | Tamaño generado si falta |
| --- | --- | --- |
| `level1.txt` | Almacén | 8 x 6 celdas (17 x 13 caracteres) |
| `level2.txt` | Pasillos | 12 x 9 celdas (25 x 19 caracteres) |
| `level3.txt` | Subsuelo | 16 x 12 celdas (33 x 25 caracteres) |

El modo infinito nunca lee de disco: genera semilla y tamaño nuevos en cada
partida.

## Formato

| Carácter | Significado |
| --- | --- |
| `p` | Posición inicial del jugador |
| `g` | Meta |
| `\|` | Pared: textura de concreto |
| `-` | Pared: textura de panel de acero |
| `+` | Pared: textura de franjas de peligro |
| espacio | Piso transitable |

Todas las filas deben tener el mismo largo. El requisito del curso es que el
laberinto sea igual o más grande que el de referencia, que mide 13 x 9
caracteres.

Para ver un laberinto generado y usarlo como punto de partida:

```bash
cargo test -- --nocapture muestra_un_laberinto
```
