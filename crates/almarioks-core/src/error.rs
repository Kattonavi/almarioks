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
    /// The vault file is not at the given path (RF-95).
    #[error("No se encontró la bóveda en esa ubicación. Puede que se haya movido o borrado.")]
    ArchivoNoEncontrado,
    /// The key file is not at the given path (RF-95).
    #[error("No se encontró el archivo llave en esa ubicación. Elige dónde está ahora.")]
    ArchivoLlaveNoEncontrado,
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
            Self::ArchivoNoEncontrado => "ArchivoNoEncontrado",
            Self::ArchivoLlaveNoEncontrado => "ArchivoLlaveNoEncontrado",
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

/// The field a validation error is about (plan "Precisiones" 15, RF-40). The names are KeePass' keys, plus `Tags`
/// for the tag list and `Name` for a group's (or a new vault's) name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// `Title`.
    Title,
    /// `UserName`.
    UserName,
    /// `Password`.
    Password,
    /// `URL`.
    Url,
    /// `Notes`.
    Notes,
    /// `Tags`: the number of tags or one tag.
    Tags,
    /// `Name`: a group's name.
    Name,
}

impl Field {
    /// The name the window receives (`field` of `AppError`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::UserName => "UserName",
            Self::Password => "Password",
            Self::Url => "URL",
            Self::Notes => "Notes",
            Self::Tags => "Tags",
            Self::Name => "Name",
        }
    }
}

/// A rejected value with its data (RF-40, plan "Precisiones" 15): the stable code, the field and, for
/// [`CoreError::CampoMuyLargo`], the limit in characters (or in tags, for the tag count).
///
/// [`CoreError`] stays a plain code; this type carries the data and converts into it, so `code()` and `message()`
/// are the same as the code's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("{error}")]
pub struct FieldError {
    error: CoreError,
    field: Field,
    limit: Option<usize>,
}

impl FieldError {
    pub(crate) const fn new(error: CoreError, field: Field) -> Self {
        Self {
            error,
            field,
            limit: None,
        }
    }

    pub(crate) const fn too_long(field: Field, limit: usize) -> Self {
        Self {
            error: CoreError::CampoMuyLargo,
            field,
            limit: Some(limit),
        }
    }

    /// The code: `TituloVacio`, `NombreVacio`, `CampoMuyLargo`, `CaracterNoPermitido` or `EtiquetaInvalida`.
    #[must_use]
    pub const fn error(self) -> CoreError {
        self.error
    }

    /// The field that broke the rule.
    #[must_use]
    pub const fn field(self) -> Field {
        self.field
    }

    /// The limit that was passed; only for [`CoreError::CampoMuyLargo`].
    #[must_use]
    pub const fn limit(self) -> Option<usize> {
        self.limit
    }

    /// Same as [`CoreError::code`] of [`Self::error`].
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.error.code()
    }

    /// Same as [`CoreError::message`] of [`Self::error`].
    #[must_use]
    pub fn message(self) -> String {
        self.error.message()
    }
}

impl From<FieldError> for CoreError {
    fn from(error: FieldError) -> Self {
        error.error
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreError, Field, FieldError};

    const ALL: [CoreError; 25] = [
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
        CoreError::ArchivoNoEncontrado,
        CoreError::ArchivoLlaveNoEncontrado,
    ];

    #[test]
    fn rf_40_a_field_error_keeps_the_code_and_message_of_its_code() {
        let error = FieldError::too_long(Field::Url, 2_048);
        assert_eq!(error.code(), "CampoMuyLargo");
        assert_eq!(error.message(), CoreError::CampoMuyLargo.message());
        assert_eq!(CoreError::from(error), CoreError::CampoMuyLargo);
        assert_eq!((error.field().name(), error.limit()), ("URL", Some(2_048)));
        let tag = FieldError::new(CoreError::EtiquetaInvalida, Field::Tags);
        assert_eq!((tag.field().name(), tag.limit()), ("Tags", None));
    }

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
