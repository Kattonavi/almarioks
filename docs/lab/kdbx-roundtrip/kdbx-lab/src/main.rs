//! Lab 19: does `keepass` 0.17 (`save_kdbx4`) round-trip a KDBX 4 written by KeePass 2?
//! All data here is fake. `unwrap` is fine in a lab, not in the product.

use std::{env, fs::File, io::Write, process::ExitCode};

use keepass::{
    Database, DatabaseKey,
    config::{DatabaseConfig, KdfConfig, OuterCipherConfig},
    db::{Value, fields},
};
use sha2::{Digest, Sha256};

fn key_from(password: &str, keyfile: &str) -> DatabaseKey {
    let mut key = DatabaseKey::new();
    if password != "-" {
        key = key.with_password(password);
    }
    if keyfile != "-" {
        key = key.with_keyfile(&mut File::open(keyfile).unwrap()).unwrap();
    }
    key
}

fn open(path: &str, key: &DatabaseKey) -> Database {
    Database::open(&mut File::open(path).unwrap(), key.clone()).unwrap()
}

fn save(db: &Database, path: &str, key: &DatabaseKey) {
    let mut out = Vec::new();
    db.save(&mut out, key.clone()).unwrap();
    File::create(path).unwrap().write_all(&out).unwrap();
}

/// KeePass 2 key file, format 2.0: 32 random bytes in hex plus the first 4 bytes of their SHA-256.
fn write_keyx(path: &str) {
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).unwrap();
    let hash = Sha256::digest(raw);
    let hex_key = hex::encode_upper(raw);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>2.0</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data Hash=\"{}\">{}</Data>\n\t</Key>\n</KeyFile>\n",
        hex::encode_upper(&hash[..4]),
        hex_key
    );
    File::create(path).unwrap().write_all(xml.as_bytes()).unwrap();
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("-");

    match arg(1) {
        // rewrite <in> <out> <password|-> <keyfile|->
        "rewrite" => {
            let key = key_from(arg(4), arg(5));
            let db = open(arg(2), &key);
            println!("version={:?} kdf={:?}", db.config.version, db.config.kdf_config);
            save(&db, arg(3), &key);
            let again = open(arg(3), &key);
            println!("rust_reopen_equal={}", db == again);
        }
        // edit <in> <out> <password|-> <keyfile|->
        "edit" => {
            let key = key_from(arg(4), arg(5));
            let mut db = open(arg(2), &key);
            {
                let mut root = db.root_mut();
                let mut work = root.group_by_name_mut("Trabajo").unwrap();
                let mut github = work.entry_by_name_mut("GitHub").unwrap();
                github.edit_tracking(|e| e.set_protected(fields::PASSWORD, "cambiada-en-rust-ñ"));
            }
            {
                let mut root = db.root_mut();
                let mut entry = root.add_entry();
                entry.set_unprotected(fields::TITLE, "Creada en Rust");
                entry.set_unprotected(fields::USERNAME, "sara@example.com");
                entry.set_protected(fields::PASSWORD, "pässwörd-日本語-🔑");
                entry.set_unprotected(fields::URL, "https://example.org");
                entry.set_protected("Campo secreto", "valor-secreto");
                entry.add_attachment("rust.txt", Value::unprotected(b"hola desde rust".to_vec()));
            }
            save(&db, arg(3), &key);
        }
        // create <out> <password|-> <keyfile_out|->
        "create" => {
            if arg(4) != "-" {
                write_keyx(arg(4));
            }
            let key = key_from(arg(3), arg(4));
            let mut config = DatabaseConfig::default();
            // The crate re-exports no Argon2 version type: reuse the one in its default config.
            let KdfConfig::Argon2 { version, .. } = config.kdf_config.clone() else {
                panic!("default KDF is no longer Argon2d");
            };
            config.outer_cipher_config = OuterCipherConfig::ChaCha20;
            config.kdf_config = KdfConfig::Argon2id {
                iterations: 2,
                memory: 64 * 1024 * 1024,
                parallelism: 2,
                version,
            };
            let mut db = Database::with_config(config);
            db.meta.database_name = Some("Bóveda de Rust".into());
            {
                let mut root = db.root_mut();
                root.name = "Raíz".into();
                let mut group = root.add_group();
                group.name = "Correo".into();
                let mut entry = group.add_entry();
                entry.set_unprotected(fields::TITLE, "Correo personal");
                entry.set_unprotected(fields::USERNAME, "alguien@example.com");
                entry.set_protected(fields::PASSWORD, "Tr0ub4dor&3-ñandú");
                entry.set_unprotected(fields::URL, "https://mail.example.com");
                entry.add_attachment("codigos.txt", Value::protected(b"1111 2222 3333".to_vec()));
            }
            save(&db, arg(2), &key);
            println!("created version={:?}", db.config.version);
        }
        other => {
            eprintln!("unknown command: {other}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

