//! Strict command arguments (RF-80; plan, "Precisiones" 1).
//!
//! Tauri deserializes the parameters of a command itself, and when that fails the window gets a line of text, not
//! an [`AppError`]. Worse, serde's text repeats the offending value (`invalid type: string "…"`), which for
//! `password_strength` would be the master password. So no command declares a deserialized parameter: each one
//! takes the raw [`tauri::ipc::Request`], which cannot fail, and reads its arguments here. Whatever is wrong with
//! them, the answer is `{ code: "ArgumentoInvalido", … }` and serde's text is dropped.

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::Value;
use tauri::ipc::InvokeBody;

use super::error::AppError;

/// The only key the window sends: `invoke(command, { args })`.
const ARGS_KEY: &str = "args";

/// Reads the arguments of a command that takes data: the body must be the JSON object `{ "args": { … } }`, with
/// nothing else at the top level, and `args` must be exactly a `T` (the structs carry `deny_unknown_fields`).
///
/// # Errors
/// `ArgumentoInvalido` for a body that is not JSON, a missing or extra top-level key, a wrong type, an unknown
/// field, a missing field or a number outside the range of its type.
pub fn parse_args<T: DeserializeOwned>(body: &InvokeBody) -> Result<T, AppError> {
    let InvokeBody::Json(Value::Object(top)) = body else {
        return Err(AppError::invalid_argument());
    };
    // `args` must be an object: serde would also read a struct from a list of its values in order, which is not
    // the contract. (That check is for the top level only: a struct nested inside `args` still has serde's rule.)
    let (Some(args @ Value::Object(_)), 1) = (top.get(ARGS_KEY), top.len()) else {
        return Err(AppError::invalid_argument());
    };
    // By reference: the body belongs to Tauri and is not cloned. The error is dropped on purpose (see above).
    T::deserialize(args).map_err(|_| AppError::invalid_argument())
}

/// Checks the body of a command that takes no data: the window sends `{}` (or nothing).
///
/// # Errors
/// `ArgumentoInvalido` if anything came with the call.
pub fn expect_no_args(body: &InvokeBody) -> Result<(), AppError> {
    match body {
        InvokeBody::Json(Value::Null) => Ok(()),
        InvokeBody::Json(Value::Object(top)) if top.is_empty() => Ok(()),
        InvokeBody::Json(_) | InvokeBody::Raw(_) => Err(AppError::invalid_argument()),
    }
}

/// A secret text argument (a password being typed). It is moved into a [`SecretString`] while it is being read,
/// so the copy this crate owns is wiped when the command ends; it has no `Debug` that shows it and is never
/// logged. The copy inside Tauri's request is not ours to wipe (ADR-0002; plan, "Precisiones" 14).
pub struct SecretArg(SecretString);

impl SecretArg {
    /// The text, for the one call that needs it.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl<'de> Deserialize<'de> for SecretArg {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|text| Self(SecretString::from(text)))
    }
}
