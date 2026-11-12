import { z } from "./zod";

/**
 * Stable error codes. The first block comes from spec 001; the second block is proposed to the core (not in the spec
 * yet); the last two are produced only by this window and never sent by the core.
 */
export const ERROR_CODES = [
  // Spec 001.
  "Bloqueada",
  "ArgumentoInvalido",
  "SoloLectura",
  "NoEncontrado",
  "LlaveVacia",
  "ArchivoExiste",
  "LlaveIncorrecta",
  "ArchivoLlaveDanado",
  "ArchivoDanado",
  "FormatoNoSoportado",
  "ParametrosKdfExcesivos",
  "Ocupada",
  "GuardadoFallido",
  "CambioExterno",
  "TituloVacio",
  "CampoMuyLargo",
  "CaracterNoPermitido",
  "UrlNoPermitida",
  "EtiquetaInvalida",
  "NombreVacio",
  "MovimientoInvalido",
  "GrupoProtegido",
  "ReglasImposibles",
  // Proposed contract change: docs/UX.md P3 needs "archivo llave no encontrado" and P1 a vault that is gone.
  "ArchivoNoEncontrado",
  "ArchivoLlaveNoEncontrado",
  // Window only.
  "RespuestaInvalida",
  "Inesperado",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

/** One variant per code, so a `switch (error.code)` is checked for exhaustiveness. */
export type AppError = { [C in ErrorCode]: { readonly code: C } }[ErrorCode];

export type Result<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: AppError };

const coreErrorSchema = z.object({ code: z.enum(ERROR_CODES), message: z.string() });

/**
 * Turns whatever `invoke` rejected with into an `AppError`. The core's `message` is dropped on purpose: the window
 * shows its own text for each code (RF-83) and never echoes text it did not write.
 */
export function toAppError(rejection: unknown): AppError {
  const parsed = coreErrorSchema.safeParse(rejection);
  if (!parsed.success) return { code: "Inesperado" };
  const { code } = parsed.data;
  // The window-only codes cannot come from the core.
  if (code === "RespuestaInvalida" || code === "Inesperado") return { code: "Inesperado" };
  return { code };
}

const RETRY = "Vuelve a intentarlo; si se repite, cierra y abre AlmarioKS.";

/** What happened and what to do, in Spanish, without the code (RF-83). */
export const ERROR_MESSAGES: Readonly<Record<ErrorCode, string>> = {
  Bloqueada: "La bóveda está bloqueada. Desbloquéala para seguir.",
  ArgumentoInvalido: `La app no entendió esa petición y no cambió nada. ${RETRY}`,
  SoloLectura:
    "Esta bóveda usa un formato antiguo y solo se puede leer. Conviértela a un archivo nuevo para poder editarla.",
  NoEncontrado: "Ese elemento ya no está en la bóveda. Vuelve a la lista y elige otro.",
  LlaveVacia: "Falta la llave. Escribe una contraseña maestra, elige un archivo llave o usa los dos.",
  ArchivoExiste:
    "Ya hay un archivo con ese nombre en esa carpeta y no se tocó. Elige otro nombre u otra carpeta.",
  LlaveIncorrecta:
    "La contraseña o el archivo llave no abren esta bóveda. Revisa los dos y vuelve a intentarlo.",
  ArchivoLlaveDanado:
    "El archivo llave está dañado y no se usó. Elige una copia sana de tu archivo llave.",
  ArchivoDanado:
    "El archivo de la bóveda está dañado o no es una bóveda. Prueba con una copia, por ejemplo el archivo .bak que está a su lado.",
  FormatoNoSoportado:
    "Esta versión del formato no se puede abrir aquí. Ábrela en KeePass 2 o KeePassXC y guárdala como KDBX 4.",
  ParametrosKdfExcesivos:
    "La bóveda pide más memoria o tiempo del que es seguro usar y no se intentó abrir. Ábrela en KeePass 2 o KeePassXC y baja sus ajustes de cifrado.",
  Ocupada: "La bóveda ya se está abriendo. Espera a que termine.",
  GuardadoFallido:
    "No se pudo guardar. Tus cambios siguen aquí: vuelve a intentarlo o guárdalos en otra ubicación.",
  CambioExterno:
    "Otra app cambió el archivo de la bóveda y no se sobrescribió. Guarda tus cambios en otra ubicación o recarga el archivo descartando lo pendiente.",
  TituloVacio: "La entrada necesita un título. Escribe uno.",
  CampoMuyLargo: "Un campo es más largo de lo permitido. Acórtalo y vuelve a intentarlo.",
  CaracterNoPermitido:
    "Hay un carácter de control que no se puede guardar. Bórralo y vuelve a intentarlo.",
  UrlNoPermitida:
    "Esa dirección no se abre: solo se abren las que empiezan por http:// o https://. Corrígela en la entrada.",
  EtiquetaInvalida: "Una etiqueta está vacía o tiene ; o ,. Corrígela y vuelve a intentarlo.",
  NombreVacio: "El grupo necesita un nombre. Escribe uno.",
  MovimientoInvalido:
    "Un grupo no puede ir dentro de sí mismo ni de uno de sus subgrupos. Elige otro destino.",
  GrupoProtegido: "Ese grupo no se puede mover, renombrar ni borrar. Elige otro.",
  ReglasImposibles:
    "Con estas opciones no se puede generar una contraseña. Elige al menos un tipo de carácter y una longitud entre 8 y 128.",
  ArchivoNoEncontrado: "El archivo de la bóveda ya no está en esa ubicación. Búscalo o elige otro.",
  ArchivoLlaveNoEncontrado: "No se encontró el archivo llave. Vuelve a elegirlo.",
  RespuestaInvalida: `La app recibió una respuesta que no esperaba y no la usó. ${RETRY}`,
  Inesperado: `Algo falló sin una causa conocida. ${RETRY}`,
};

export function errorMessage(error: AppError): string {
  return ERROR_MESSAGES[error.code];
}
