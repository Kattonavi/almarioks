//! The core's error type: one variant per stable code of spec 001 ("Definiciones" → "Códigos de error").
//!
//! Variants carry no data on purpose: no library type leaves `kdbx` (ADR-0001) and no secret, path or internal
//! detail can reach a message (RF-83, constitution rule 6). The window shows [`CoreError::message`]; the code is
//! for programs and tests.

use thiserror::Error;

/// Every error the core can return. The `Display` text is the Spanish message for the person (RF-83).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// The vault is locked and the operation needs it open.
    #[error("La bóveda está bloqueada. Desbloquéala para continuar.")]
    Bloqueada,
    /// Another unlock is already running (RF-88).
    #[error("Ya se está abriendo la bóveda. Espera a que termine.")]
    Ocupada,
    /// A command argument has the wrong type or unknown fields (RF-80).
    #[error("La petición no es válida. Vuelve a intentarlo desde la ventana.")]
    ArgumentoInvalido,
    /// The vault is open read-only (KDBX 3.x, RF-14).
    #[error(
        "Esta bóveda está en un formato antiguo y solo se puede leer. Conviértela para editarla."
    )]
    SoloLectura,
    /// The entry or group does not exist (RF-93).
    #[error("No se encontró lo que buscabas. Puede que ya no exista.")]
    NoEncontrado,
    /// The key does not open the vault (RF-11). Same text whatever part of the key is wrong.
    #[error(
        "La contraseña o el archivo llave no abren esta bóveda. Revisa los dos y vuelve a intentarlo."
    )]
    LlaveIncorrecta,
    /// Neither a master password nor a key file was given (RF-3).
    #[error("Escribe una contraseña maestra, elige un archivo llave o usa los dos.")]
    LlaveVacia,
    /// The file is not a vault, is truncated or does not verify (RF-13, RF-94).
    #[error("El archivo no es una bóveda o está dañado. Prueba con una copia de respaldo.")]
    ArchivoDanado,
    /// The file is a KeePass format this app does not open (RF-13).
    #[error(
        "Este formato de bóveda no es compatible. Ábrela en KeePass y guárdala en formato KDBX 4."
    )]
    FormatoNoSoportado,
    /// A `.keyx` version 2 key file whose `Hash` does not match its content (RF-12).
    #[error("El archivo llave está dañado. Usa una copia de respaldo del archivo llave.")]
    ArchivoLlaveDanado,
    /// The header asks for more KDF work or memory than the app allows (RF-16).
    #[error(
        "La bóveda pide más memoria o más tiempo del que la app permite para abrirla. No es segura de abrir."
    )]
    ParametrosKdfExcesivos,
    /// A file already exists at the chosen path (RF-4, RF-6).
    #[error("Ya existe un archivo en esa ubicación. Elige otro nombre o carpeta.")]
    ArchivoExiste,
    /// The title is empty after trimming (RF-39).
    #[error("El título no puede quedar vacío.")]
    TituloVacio,
    /// A group name is empty after trimming.
    #[error("El nombre no puede quedar vacío.")]
    NombreVacio,
    /// A field is longer than its limit (RF-40).
    #[error("El texto es demasiado largo. Acórtalo e inténtalo de nuevo.")]
    CampoMuyLargo,
    /// The text has a control character (spec "Definiciones").
    #[error("El texto tiene un carácter que no se permite. Quítalo e inténtalo de nuevo.")]
    CaracterNoPermitido,
    /// A tag is not valid.
    #[error("La etiqueta no es válida.")]
    EtiquetaInvalida,
    /// The URL cannot be opened (RF-49).
    #[error("Esta dirección no se puede abrir desde la app.")]
    UrlNoPermitida,
    /// The move would put a group inside itself or move a fixed group (RF-55).
    #[error("No se puede mover ahí.")]
    MovimientoInvalido,
    /// The group cannot be changed or deleted (RF-57).
    #[error("Este grupo no se puede cambiar ni borrar.")]
    GrupoProtegido,
    /// The generator options cannot be satisfied (RF-67).
    #[error(
        "Con esas opciones no se puede generar una contraseña. Cambia la longitud o los tipos de carácter."
    )]
    ReglasImposibles,
    /// Writing to disk failed; the original file is intact (RF-23).
    #[error(
        "No se pudo guardar. Tu archivo original no se tocó. Revisa el espacio y los permisos de la carpeta."
    )]
    GuardadoFallido,
    /// Another app changed the vault file since it was opened (RF-24).
    #[error("Otra app cambió el archivo de la bóveda. Recárgala antes de seguir.")]
    CambioExterno,
}

impl CoreError {
    /// The stable code of the spec, in `CódigoAsí` form. Never shown to the person (RF-83).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Bloqueada => "Bloqueada",
            Self::Ocupada => "Ocupada",
            Self::ArgumentoInvalido => "ArgumentoInvalido",
            Self::SoloLectura => "SoloLectura",
            Self::NoEncontrado => "NoEncontrado",
            Self::LlaveIncorrecta => "LlaveIncorrecta",
            Self::LlaveVacia => "LlaveVacia",
            Self::ArchivoDanado => "ArchivoDanado",
            Self::FormatoNoSoportado => "FormatoNoSoportado",
            Self::ArchivoLlaveDanado => "ArchivoLlaveDanado",
            Self::ParametrosKdfExcesivos => "ParametrosKdfExcesivos",
            Self::ArchivoExiste => "ArchivoExiste",
            Self::TituloVacio => "TituloVacio",
            Self::NombreVacio => "NombreVacio",
            Self::CampoMuyLargo => "CampoMuyLargo",
            Self::CaracterNoPermitido => "CaracterNoPermitido",
            Self::EtiquetaInvalida => "EtiquetaInvalida",
            Self::UrlNoPermitida => "UrlNoPermitida",
            Self::MovimientoInvalido => "MovimientoInvalido",
            Self::GrupoProtegido => "GrupoProtegido",
            Self::ReglasImposibles => "ReglasImposibles",
            Self::GuardadoFallido => "GuardadoFallido",
            Self::CambioExterno => "CambioExterno",
        }
    }

    /// The Spanish message for the person: what happened and what to do, without the code (RF-83).
    #[must_use]
    pub fn message(self) -> String {
        self.to_string()
    }
}

/// Result type of the core.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::CoreError;

    const ALL: [CoreError; 23] = [
        CoreError::Bloqueada,
        CoreError::Ocupada,
        CoreError::ArgumentoInvalido,
        CoreError::SoloLectura,
        CoreError::NoEncontrado,
        CoreError::LlaveIncorrecta,
        CoreError::LlaveVacia,
        CoreError::ArchivoDanado,
        CoreError::FormatoNoSoportado,
        CoreError::ArchivoLlaveDanado,
        CoreError::ParametrosKdfExcesivos,
        CoreError::ArchivoExiste,
        CoreError::TituloVacio,
        CoreError::NombreVacio,
        CoreError::CampoMuyLargo,
        CoreError::CaracterNoPermitido,
        CoreError::EtiquetaInvalida,
        CoreError::UrlNoPermitida,
        CoreError::MovimientoInvalido,
        CoreError::GrupoProtegido,
        CoreError::ReglasImposibles,
        CoreError::GuardadoFallido,
        CoreError::CambioExterno,
    ];

    #[test]
    fn rf_83_every_code_has_a_spanish_message_without_the_code() {
        for error in ALL {
            let message = error.message();
            assert!(!message.is_empty(), "{error:?} has no message");
            assert!(
                !message.contains(error.code()),
                "{error:?}: the message shows the code"
            );
        }
    }

    #[test]
    fn codes_are_unique_and_match_the_variant_name() {
        let mut codes: Vec<&str> = ALL.iter().map(|e| e.code()).collect();
        for error in ALL {
            assert_eq!(error.code(), format!("{error:?}"));
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ALL.len());
    }
}
