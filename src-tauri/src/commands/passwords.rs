//! Password strength and the generator (RF-7, RF-65 to RF-70). Both are computed by the core.
//!
//! `generate_password` is one of the two commands allowed to hand a secret to the window (RF-76, ADR-0002): the
//! person asked for a new password to look at. `password_strength` receives one and answers only a number.

use almarioks_core::{
    CoreError,
    generator::{self, PasswordRules},
    strength,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};
use tauri::ipc::Request;

use super::{
    args::{SecretArg, parse_args},
    blocking,
    error::AppError,
};

/// `password_strength{password}`. No `Debug`: it holds what the person is typing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordStrengthArgs {
    password: SecretArg,
}

/// `generate_password{length, lower, upper, digits, symbols, avoid_lookalikes}`, the names `src/ipc/client.ts`
/// sends. Every field is required: the defaults of RF-65 are the form's, not the command's.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePasswordArgs {
    length: u32,
    lower: bool,
    upper: bool,
    digits: bool,
    symbols: bool,
    avoid_lookalikes: bool,
}

impl GeneratePasswordArgs {
    /// The rules for the core. A length that does not fit its type is as impossible as any other out of range.
    fn rules(&self) -> Result<PasswordRules, CoreError> {
        Ok(PasswordRules {
            length: usize::try_from(self.length).map_err(|_| CoreError::ReglasImposibles)?,
            lowercase: self.lower,
            uppercase: self.upper,
            digits: self.digits,
            symbols: self.symbols,
            avoid_lookalikes: self.avoid_lookalikes,
        })
    }
}

/// `{ score }`: 0 to 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StrengthView {
    score: u8,
}

/// `{ password, score }`. The password stays wrapped until it is written into the answer, and this copy is wiped
/// when the command ends. No `Debug` that shows it.
pub struct GeneratedView {
    password: SecretString,
    score: u8,
}

impl Serialize for GeneratedView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = serializer.serialize_struct("GeneratedView", 2)?;
        fields.serialize_field("password", self.password.expose_secret())?;
        fields.serialize_field("score", &self.score)?;
        fields.end()
    }
}

/// The strength of what the person is typing (RF-7, RF-70). Answers only the score.
///
/// # Errors
/// `ArgumentoInvalido` if the arguments are malformed.
#[tauri::command]
pub async fn password_strength(request: Request<'_>) -> Result<StrengthView, AppError> {
    let args: PasswordStrengthArgs = parse_args(request.body())?;
    blocking(move || {
        Ok(StrengthView {
            score: strength::password_strength(args.password.expose()),
        })
    })
    .await
}

/// A new password with the chosen rules, and its strength (RF-65 to RF-70).
///
/// # Errors
/// `ArgumentoInvalido` if the arguments are malformed; `ReglasImposibles` if no type is chosen or the length is
/// outside 8 to 128 (RF-68).
#[tauri::command]
pub async fn generate_password(request: Request<'_>) -> Result<GeneratedView, AppError> {
    let args: GeneratePasswordArgs = parse_args(request.body())?;
    let rules = args.rules()?;
    blocking(move || {
        let password = generator::generate_password(&rules)?;
        let score = strength::password_strength(password.expose_secret());
        Ok(GeneratedView { password, score })
    })
    .await
}
