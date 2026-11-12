//! The error every command answers with (plan, "Contrato" and "Precisiones" 15): `{ code, message, field?, limit? }`.
//!
//! `code` is a stable code of the spec; `message` is the Spanish text of the core (RF-83). Neither carries a
//! secret, a path or an internal detail: the conversions below take only codes, a field name and a number.

use almarioks_core::{CoreError, FieldError};
use serde::Serialize;

use crate::clipboard::ClipboardError;

/// Code of a failure inside the app that no rule of the spec explains (a worker that died, a system dialog that
/// could not be shown). Decisión pendiente: the spec has no code for it; the window shows it as `Inesperado`.
pub const INTERNAL_CODE: &str = "ErrorInterno";

/// Codes this crate adds to the ones of [`CoreError`]: the clipboard's own and [`INTERNAL_CODE`].
pub const APP_ONLY_CODES: [&str; 2] = ["PortapapelesNoDisponible", INTERNAL_CODE];

/// What the window receives when a command fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<usize>,
}

impl AppError {
    /// The app failed on its own. Says nothing about why: the cause never reaches the window.
    #[must_use]
    pub fn internal() -> Self {
        Self {
            code: INTERNAL_CODE,
            message:
                "Algo falló dentro de la app. Vuelve a intentarlo; si se repite, cierra y abre AlmarioKS."
                    .to_owned(),
            field: None,
            limit: None,
        }
    }

    /// `ArgumentoInvalido` (RF-80).
    #[must_use]
    pub fn invalid_argument() -> Self {
        CoreError::ArgumentoInvalido.into()
    }

    /// The stable code. Never shown to the person.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// The Spanish message: what happened and what to do, without the code (RF-83).
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The field that broke a rule (`CampoMuyLargo`, `CaracterNoPermitido`, `EtiquetaInvalida`…).
    #[must_use]
    pub const fn field(&self) -> Option<&'static str> {
        self.field
    }

    /// The limit that was passed (`CampoMuyLargo`).
    #[must_use]
    pub const fn limit(&self) -> Option<usize> {
        self.limit
    }
}

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        Self {
            code: error.code(),
            message: error.message(),
            field: None,
            limit: None,
        }
    }
}

impl From<FieldError> for AppError {
    fn from(error: FieldError) -> Self {
        Self {
            code: error.code(),
            message: error.message(),
            field: Some(error.field().name()),
            limit: error.limit(),
        }
    }
}

impl From<ClipboardError> for AppError {
    fn from(error: ClipboardError) -> Self {
        match error {
            ClipboardError::Core(core) => core.into(),
            ClipboardError::Unavailable => Self {
                code: error.code(),
                message: error.to_string(),
                field: None,
                limit: None,
            },
        }
    }
}
