# Audio

Los archivos van en esta carpeta, con estos nombres exactos. Se acepta `.ogg`,
`.mp3` o `.wav` — se prueban en ese orden, así que no hay que renombrar nada.

Lo que falte se omite en silencio: se pueden agregar de a uno.

| Archivo | Qué es | Cuándo suena | Duración ideal |
| --- | --- | --- | --- |
| `music.mp3` | Música de fondo, en bucle | Todo el tiempo | Una canción completa |
| `ambience.ogg` | Zumbido de la instalación, en bucle | Todo el tiempo | 10-30 s, que empalme bien |
| `step.wav` | Un paso | Al desplazarse | 0.2-0.5 s |
| `flashlight.wav` | Clic del interruptor | Al presionar M | 0.1-0.3 s |
| `victory.wav` | Al alcanzar la salida | Una vez, al ganar | 1-3 s |

Los efectos deben ser **cortos y sin silencio al inicio**. Un wav con 300 ms de
silencio adelante hace que el paso suene tarde respecto a la pisada.

## Volúmenes

Se ajustan en `src/audio.rs`:

- `MUSIC_VOLUME` = 0.35
- `AMBIENCE_VOLUME` = 0.25
- `EFFECT_VOLUME` = 0.7

## Cadencia de los pasos

`STEP_INTERVAL_WALK` = 0.45 s, `STEP_INTERVAL_RUN` = 0.28 s. Ajustar según cuán
largo suene el archivo de paso: si se solapan demasiado, subir los intervalos.

## Licencias

Guardar la procedencia de cada archivo. Para la entrega conviene poder decir de
dónde salió cada sonido.
