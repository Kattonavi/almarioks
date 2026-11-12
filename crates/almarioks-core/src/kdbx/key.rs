//! The key of a vault: master password, key file or both (RF-1, RF-3, RF-11, RF-12, RF-87).
//!
//! Secrets are wiped from memory when dropped (`secrecy`, `zeroize`) and their `Debug` never shows them.
//! The key element of a key file is computed by `keepass` (all formats of RF-87: `.keyx` v2, XML v1, 32 raw
//! bytes, 64 hex characters, any other file by its SHA-256); this module only adds what the library does not do:
//! checking the `Hash` of a `.keyx` v2 (ADR-0001, point 3).

use std::fmt;

use keepass::DatabaseKey;
use quick_xml::{XmlVersion, events::Event, reader::Reader};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::error::{CoreError, CoreResult};

/// A master password. Wiped when dropped; `Debug` prints nothing of it.
pub struct MasterPassword(SecretString);

impl MasterPassword {
    /// Takes ownership of the text so no other copy stays with the caller.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(SecretString::from(text))
    }

    /// True for the empty text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
    }

    pub(crate) fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl fmt::Debug for MasterPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MasterPassword(<redacted>)")
    }
}

/// The content of a key file, already checked (RF-12). Wiped when dropped; `Debug` prints nothing of it.
pub struct KeyFile {
    content: Zeroizing<Vec<u8>>,
}

impl KeyFile {
    /// Accepts the bytes of any key file (RF-87). A `.keyx` version 2 must carry a `Hash` that matches its key.
    ///
    /// # Errors
    /// [`CoreError::ArchivoLlaveDanado`] when the file is a `.keyx` version 2 whose key is not hexadecimal or whose
    /// `Hash` is not the first 4 bytes of the SHA-256 of the key (RF-12). Nothing is opened in that case.
    pub fn from_bytes(content: Vec<u8>) -> CoreResult<Self> {
        let content = Zeroizing::new(content);
        check_keyx_v2(&content)?;
        Ok(Self { content })
    }

    /// The raw bytes of the file, to hand them to `keepass` or write a new key file to disk.
    pub(crate) fn content(&self) -> &[u8] {
        &self.content
    }
}

impl fmt::Debug for KeyFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyFile(<redacted>)")
    }
}

/// The composite key: a master password, a key file or both.
#[derive(Debug)]
pub struct VaultKey {
    password: Option<MasterPassword>,
    key_file: Option<KeyFile>,
}

impl VaultKey {
    /// Builds a key from its parts. An empty password counts as no password.
    ///
    /// # Errors
    /// [`CoreError::LlaveVacia`] when there is neither a password nor a key file (RF-3).
    pub fn new(password: Option<MasterPassword>, key_file: Option<KeyFile>) -> CoreResult<Self> {
        let password = password.filter(|p| !p.is_empty());
        if password.is_none() && key_file.is_none() {
            return Err(CoreError::LlaveVacia);
        }
        Ok(Self { password, key_file })
    }

    /// True if the key has a master password.
    #[must_use]
    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    /// True if the key has a key file.
    #[must_use]
    pub fn has_key_file(&self) -> bool {
        self.key_file.is_some()
    }

    /// The library's key. `DatabaseKey` wipes its copy on drop (`ZeroizeOnDrop`); it never leaves `kdbx`.
    pub(crate) fn to_database_key(&self) -> CoreResult<DatabaseKey> {
        let mut key = DatabaseKey::new();
        if let Some(password) = &self.password {
            key = key.with_password(password.expose());
        }
        if let Some(file) = &self.key_file {
            // Reading from a slice cannot fail; the error is mapped only to avoid a panic path.
            key = key
                .with_keyfile(&mut file.content())
                .map_err(|_| CoreError::ArchivoLlaveDanado)?;
        }
        Ok(key)
    }
}

/// What the library would read from an XML key file. Same reader, same rule (tag path and text) as
/// `keepass` 0.17.0 `key::parse_xml_keyfile`, so we validate exactly the data it will use.
struct XmlKeyFile {
    version: Option<String>,
    data: Option<Zeroizing<String>>,
    hash: Option<String>,
}

fn read_xml_key_file(bytes: &[u8]) -> Option<XmlKeyFile> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut xml_version = XmlVersion::Implicit1_0;
    let mut path: Vec<String> = Vec::new();
    let mut found = XmlKeyFile {
        version: None,
        data: None,
        hash: None,
    };
    loop {
        let event = reader.read_event_into(&mut buf).ok()?;
        match event {
            Event::Eof => break,
            Event::Decl(decl) => xml_version = decl.xml_version().ok()?,
            Event::Start(start) => {
                path.push(start.name().as_ref().to_owned());
                if is_path(&path, &["KeyFile", "Key", "Data"]) {
                    found.hash = match start.try_get_attribute("Hash") {
                        Ok(Some(attr)) => {
                            Some(attr.normalized_value(xml_version).ok()?.into_owned())
                        }
                        Ok(None) => None,
                        Err(_) => return None,
                    };
                }
            }
            Event::End(_) => {
                path.pop();
            }
            Event::Text(text) => {
                let value = text.xml_content(xml_version).into_owned();
                if is_path(&path, &["KeyFile", "Meta", "Version"]) {
                    found.version = Some(value);
                } else if is_path(&path, &["KeyFile", "Key", "Data"]) {
                    found.data = Some(Zeroizing::new(value));
                }
            }
            _ => {}
        }
        buf.clear();
    }
    // Without key data the library does not treat the file as XML (it falls back to the other formats).
    found.data.is_some().then_some(found)
}

fn is_path(path: &[String], expected: &[&str]) -> bool {
    path.len() == expected.len() && path.iter().zip(expected).all(|(a, b)| a == b)
}

/// RF-12: a `.keyx` version 2 is valid only if its `Hash` is the first 4 bytes of the SHA-256 of its key.
fn check_keyx_v2(bytes: &[u8]) -> CoreResult<()> {
    let Some(xml) = read_xml_key_file(bytes) else {
        return Ok(()); // not an XML key file: raw, hex or hashed file (RF-87)
    };
    // The library switches to the version 2 rules on this exact text; so do we.
    if xml.version.as_deref() != Some("2.0") {
        return Ok(());
    }
    let data = xml.data.ok_or(CoreError::ArchivoLlaveDanado)?;
    // Same cleanup as the library: trim, then drop spaces, tabs and line breaks inside.
    let hex_key: Zeroizing<String> = Zeroizing::new(
        data.trim()
            .chars()
            .filter(|c| !matches!(c, ' ' | '\n' | '\t' | '\r'))
            .collect(),
    );
    let key =
        Zeroizing::new(hex::decode(hex_key.as_bytes()).map_err(|_| CoreError::ArchivoLlaveDanado)?);
    let Some(hash) = xml.hash else {
        // KeePass writes the hash since format 2.0; a file without it cannot be checked (decision pending).
        return Ok(());
    };
    let hash_hex: String = hash.chars().filter(|c| !c.is_whitespace()).collect();
    let expected = hex::decode(hash_hex).map_err(|_| CoreError::ArchivoLlaveDanado)?;
    let digest = Sha256::digest(key.as_slice());
    if digest.get(..4) == Some(expected.as_slice()) {
        Ok(())
    } else {
        Err(CoreError::ArchivoLlaveDanado)
    }
}
