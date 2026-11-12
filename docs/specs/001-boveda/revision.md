# Revisión de la spec 001 (QA con contexto fresco, 2026-11-11)

Un revisor que solo leyó la spec y la constitución encontró 25 huecos antes de escribir código. Qué se hizo con cada uno:

| # | Sev. | Hallazgo | Disposición |
|---|---|---|---|
| 1 | Alto | Llave equivocada y archivo manipulado podían dar el mismo error | RF-11 y RF-13 dicen qué fallo de verificación da cada código |
| 2 | Alto | "Cambió en disco" sin definir | RF-24: por SHA-256 del archivo; la fecha no cuenta |
| 3 | Alto | Validación del esquema de la URL sin definir | RF-49 y D13: se analiza; sin esquema no se abre |
| 4 | Alto | Orden no determinista con títulos iguales | Definiciones: "orden por título" con desempate por UUID |
| 5 | Alto | Qué es carácter, espacio; etiquetas con `;`; caracteres de control | Definiciones, RF-39, RF-91 |
| 6 | Alto | Fortaleza sin algoritmo | Definiciones: puntaje de `zxcvbn` con anclas |
| 7 | Medio | Consulta vacía, `ñ`, qué es "palabra" | RF-59, RF-60, RF-92 y ejemplo 2 |
| 8 | Medio | Historial sin condición ni límite | RF-26, RF-45, RF-46 |
| 9 | **Crítico** | Las notas (y otros campos) pueden venir protegidas: se filtrarían a la ventana y a la búsqueda | Definición de "campo protegido"; RF-43, RF-61, RF-76; D8 corregida |
| 10 | Alto | 1 s para bloquear contra 5 s de recuperación; recuperación que pisa la anterior | RF-33 con nombre fechado; D12 |
| 11 | Alto | La calibración del KDF podía salirse de sus propios topes | RF-2: 2 a 50 iteraciones; el tiempo se mide, no es pasa/falla |
| 12 | Alto | Borrar la papelera o un grupo que la contiene | RF-57, RF-58 |
| 13 | Medio | Dos códigos para la misma entrada; UUID inexistente | Definiciones (precedencia), RF-93 |
| 14 | Medio | "Al menos uno de cada tipo" contra "misma probabilidad"; símbolos sin enumerar | RF-65, RF-69 (descartar y repetir) |
| 15 | **Crítico** | Copiar justo al bloquear dejaba el secreto en el portapapeles sin temporizador | RF-90; RF-73 decide por número de secuencia |
| 16 | **Crítico** | Editar el título podía borrar la contraseña que la ventana nunca tuvo | RF-45: la edición solo cambia lo que trae; ejemplo 8 |
| 17 | Alto | El portátil suspendido 8 h despertaba con la bóveda abierta | RF-29 con tiempo real, no recortable; ejemplo 10 |
| 18 | Alto | Solo se definía `.keyx` v2 como archivo llave | RF-87 |
| 19 | Alto | KDBX 4.0 contra 4.1, y versiones no soportadas | RF-10, RF-13, RF-25 |
| 20 | Alto | Guardados y desbloqueos simultáneos | RF-88, RF-89, RF-23 |
| 21 | **Crítico** | El bloqueo retenía las llaves mientras intentaba la recuperación (regla 7) | RF-33 y D12: primero se sueltan las llaves |
| 22 | Alto | RF que no se pueden probar tal como estaban escritos | Sección "Cómo se prueba lo que no es obvio" |
| 23 | Medio | La regla 6 no nombra al generador | Lista blanca en la prueba de contrato; propuesta al dueño |
| 24 | Medio | RF-84 no cubría descargar e instalar; WebView2 hace conexiones propias | RF-84 reescrito; lo de WebView2 se anota |
| 25 | Medio | `.keyx` propio contra la regla 4; decisiones sin confirmar | Propuesta al dueño; D14. **No se esperó la confirmación:** el dueño pidió avanzar de noche y las decisiones son reversibles |

No se aplicó: cancelar un desbloqueo en curso (sugerido en el 20) queda fuera de alcance.
