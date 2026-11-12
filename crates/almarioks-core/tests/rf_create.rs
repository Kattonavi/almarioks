//! Creating vaults through the public API of `almarioks_core::kdbx` (RF-1 to RF-6).
//!
//! Files go to a fresh folder under the system temp dir, removed at the end. Vaults use 2 iterations so the
//! tests stay fast; the calibration is tested with an injected measurement (no sleep, no hardware dependency).

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use almarioks_core::{
    CoreError,
    kdbx::{
        Argon2Variant, KdfParams, KeyFile, MasterPassword, NewKeyFile, NewVaultKdf, VaultKey,
        create_vault_file, open, read_header,
    },
};

const PASSWORD: &str = "Maestra-de-prueba-2026";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!(
            "almarioks-core-test-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn fast() -> NewVaultKdf {
    NewVaultKdf::with_iterations(2).unwrap()
}

fn pw(text: &str) -> Option<MasterPassword> {
    Some(MasterPassword::new(text.to_owned()))
}

fn key_file(bytes: &[u8]) -> KeyFile {
    KeyFile::from_bytes(bytes.to_vec()).unwrap()
}

fn opens(path: &Path, password: Option<&str>, file: Option<&[u8]>) -> Result<(), CoreError> {
    let key = VaultKey::new(password.and_then(pw), file.map(key_file))?;
    open(&fs::read(path).unwrap(), &key).map(|_| ())
}

fn photo() -> Vec<u8> {
    (0u16..2000)
        .map(|i| u8::try_from(i % 239).unwrap())
        .collect()
}

#[test]
fn rf_01_password_vault_reopens_with_that_password_and_no_other_key() {
    let dir = TempDir::new();
    let path = dir.join("cuentas.kdbx");
    let created = create_vault_file(&path, pw(PASSWORD), NewKeyFile::None, fast()).unwrap();
    assert_eq!(created.version().major, 4);
    assert_eq!(created.entry_count(), 0, "empty vault");
    assert_eq!(opens(&path, Some(PASSWORD), None), Ok(()));
    assert_eq!(
        opens(&path, Some("otra"), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, Some(PASSWORD), Some(&photo())),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, None, Some(&photo())),
        Err(CoreError::LlaveIncorrecta)
    );
}

#[test]
fn rf_01_key_file_only_vault_reopens_with_that_file_and_no_other_key() {
    // A photo as key file (spec "Casos límite", RF-87: any file by its SHA-256).
    let dir = TempDir::new();
    let path = dir.join("solo-llave.kdbx");
    create_vault_file(
        &path,
        None,
        NewKeyFile::Existing(key_file(&photo())),
        fast(),
    )
    .unwrap();
    assert_eq!(opens(&path, None, Some(&photo())), Ok(()));
    let mut other = photo();
    other[0] ^= 1;
    assert_eq!(
        opens(&path, None, Some(&other)),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, Some(PASSWORD), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, Some(PASSWORD), Some(&photo())),
        Err(CoreError::LlaveIncorrecta)
    );
}

#[test]
fn rf_01_rf_05_password_and_generated_keyx_need_both() {
    let dir = TempDir::new();
    let path = dir.join("ambas.kdbx");
    let key_path = dir.join("ambas.keyx");
    create_vault_file(
        &path,
        pw(PASSWORD),
        NewKeyFile::Generate(key_path.clone()),
        fast(),
    )
    .unwrap();
    let keyx = fs::read(&key_path).unwrap();
    assert_eq!(opens(&path, Some(PASSWORD), Some(&keyx)), Ok(()));
    assert_eq!(
        opens(&path, Some(PASSWORD), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, None, Some(&keyx)),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        opens(&path, Some("otra"), Some(&keyx)),
        Err(CoreError::LlaveIncorrecta)
    );
}

#[test]
fn rf_01_password_with_enye_euro_and_emoji() {
    let dir = TempDir::new();
    let path = dir.join("unicode.kdbx");
    let password = "ñandú-€-🔑-2026";
    create_vault_file(&path, pw(password), NewKeyFile::None, fast()).unwrap();
    assert_eq!(opens(&path, Some(password), None), Ok(()));
    assert_eq!(
        opens(&path, Some("nandu-€-🔑-2026"), None),
        Err(CoreError::LlaveIncorrecta)
    );
}

#[test]
fn rf_02_header_of_a_new_vault_says_argon2id_64_mib_and_2_to_50_iterations() {
    let dir = TempDir::new();
    for iterations in [2, 50] {
        let path = dir.join(&format!("kdf-{iterations}.kdbx"));
        let kdf = NewVaultKdf::with_iterations(iterations).unwrap();
        create_vault_file(&path, pw(PASSWORD), NewKeyFile::None, kdf).unwrap();
        let header = read_header(&fs::read(&path).unwrap()).unwrap();
        let KdfParams::Argon2 {
            variant,
            memory_bytes,
            iterations: read,
            parallelism,
        } = header.kdf
        else {
            panic!("not Argon2: {:?}", header.kdf);
        };
        assert_eq!(variant, Argon2Variant::Id);
        assert!(memory_bytes >= 64 * 1024 * 1024, "memory {memory_bytes}");
        assert!((2..=50).contains(&read), "iterations {read}");
        assert_eq!(read, iterations);
        assert_eq!(parallelism, 2);
        assert!(!header.is_kdf_weak());
    }
    // Outside 2 to 50 a KDF cannot even be built.
    for iterations in [0, 1, 51, 1000] {
        assert_eq!(
            NewVaultKdf::with_iterations(iterations).unwrap_err(),
            CoreError::ArgumentoInvalido
        );
    }
}

/// Time of one unlock on a fake machine, by iterations.
type Machine = fn(u64) -> Duration;

/// Runs the calibration on a fake machine; returns the choice and every iteration count it asked for.
fn calibrate_on(machine: impl Fn(u64) -> Duration) -> (u64, Vec<u64>) {
    let asked = RefCell::new(Vec::new());
    let chosen = NewVaultKdf::calibrate(|kdf| {
        asked.borrow_mut().push(kdf.iterations());
        Ok(machine(kdf.iterations()))
    })
    .unwrap();
    (chosen.iterations(), asked.into_inner())
}

/// The spec's answer by brute force: the iterations in 2..=50 whose time is closest to 1 s.
fn closest_to_one_second(machine: impl Fn(u64) -> Duration) -> u64 {
    (2..=50u64)
        .min_by_key(|&i| machine(i).as_micros().abs_diff(1_000_000))
        .unwrap()
}

#[test]
fn rf_02_calibration_picks_the_iterations_closest_to_one_second() {
    let machines: [(&str, Machine); 3] = [
        ("30 ms + 25 ms per pass", |i| {
            Duration::from_millis(30 + 25 * i)
        }),
        ("20 ms per pass, this lab PC in release", |i| {
            Duration::from_millis(20 * i + 17)
        }),
        ("110 ms per pass, a slow laptop", |i| {
            Duration::from_millis(110 * i + 5)
        }),
    ];
    for (label, machine) in machines {
        let (chosen, asked) = calibrate_on(machine);
        assert_eq!(chosen, closest_to_one_second(machine), "{label}");
        assert!(
            asked.iter().all(|i| (2..=50).contains(i)),
            "{label}: asked {asked:?}"
        );
        assert!(asked.len() <= 6, "{label}: {} measurements", asked.len());
    }
}

#[test]
fn rf_02_calibration_floor_is_2_and_ceiling_is_50() {
    // So slow that 2 passes already take 3 s: floor.
    let (slow, asked) = calibrate_on(|i| Duration::from_millis(1_500 * i));
    assert_eq!(slow, 2);
    assert_eq!(asked, vec![2]);
    // So fast that even 50 passes stay far below 1 s: ceiling.
    let (fast, asked) = calibrate_on(Duration::from_micros);
    assert_eq!(fast, 50);
    assert!(
        asked.iter().all(|i| (2..=50).contains(i)),
        "asked {asked:?}"
    );
    // A clock that reads zero does not divide by zero.
    let (zero, _) = calibrate_on(|_| Duration::ZERO);
    assert_eq!(zero, 50);
}

#[test]
fn rf_02_calibration_on_this_machine_stays_in_range() {
    // Measured and noted, not pass/fail on the time (spec "Cómo se prueba", RF-2).
    let kdf = NewVaultKdf::calibrate_on_this_machine().unwrap();
    assert!((2..=50).contains(&kdf.iterations()));
    println!(
        "calibrated on this machine: {} iterations",
        kdf.iterations()
    );
}

#[test]
fn rf_03_no_password_and_no_key_file_is_llave_vacia_and_writes_nothing() {
    let dir = TempDir::new();
    let path = dir.join("vacia.kdbx");
    let err = create_vault_file(&path, None, NewKeyFile::None, fast()).unwrap_err();
    assert_eq!(err, CoreError::LlaveVacia);
    let err = create_vault_file(&path, pw(""), NewKeyFile::None, fast()).unwrap_err();
    assert_eq!(err, CoreError::LlaveVacia);
    assert!(!path.exists());
}

#[test]
fn rf_04_existing_vault_path_is_archivo_existe_and_stays_untouched() {
    let dir = TempDir::new();
    let path = dir.join("existente.kdbx");
    let key_path = dir.join("nueva.keyx");
    fs::write(&path, b"no tocar").unwrap();
    let err = create_vault_file(
        &path,
        pw(PASSWORD),
        NewKeyFile::Generate(key_path.clone()),
        fast(),
    )
    .unwrap_err();
    assert_eq!(err, CoreError::ArchivoExiste);
    assert_eq!(fs::read(&path).unwrap(), b"no tocar");
    assert!(!key_path.exists(), "no orphan key file");
}

#[test]
fn rf_06_existing_key_file_path_is_archivo_existe_and_stays_untouched() {
    let dir = TempDir::new();
    let path = dir.join("nueva.kdbx");
    let key_path = dir.join("existente.keyx");
    fs::write(&key_path, b"llave vieja").unwrap();
    let err = create_vault_file(
        &path,
        pw(PASSWORD),
        NewKeyFile::Generate(key_path.clone()),
        fast(),
    )
    .unwrap_err();
    assert_eq!(err, CoreError::ArchivoExiste);
    assert_eq!(fs::read(&key_path).unwrap(), b"llave vieja");
    assert!(!path.exists(), "no orphan vault");
    // Same path for both: the second file cannot be created, and nothing is left behind.
    let same = dir.join("mismo.kdbx");
    let err = create_vault_file(
        &same,
        pw(PASSWORD),
        NewKeyFile::Generate(same.clone()),
        fast(),
    )
    .unwrap_err();
    assert_eq!(err, CoreError::ArchivoExiste);
    assert!(!same.exists());
}

#[test]
fn rf_05_generated_keyx_is_version_2_with_a_valid_hash_and_opens_the_vault() {
    let dir = TempDir::new();
    let mut keys = Vec::new();
    for name in ["a", "b"] {
        let path = dir.join(&format!("{name}.kdbx"));
        let key_path = dir.join(&format!("{name}.keyx"));
        create_vault_file(&path, None, NewKeyFile::Generate(key_path.clone()), fast()).unwrap();
        let keyx = fs::read(&key_path).unwrap();
        let text = String::from_utf8(keyx.clone()).unwrap();
        assert!(text.contains("<Version>2.0</Version>"), "{text}");
        let data = text
            .split("\">")
            .nth(1)
            .and_then(|s| s.split("</Data>").next())
            .unwrap();
        assert_eq!(data.len(), 64, "32 bytes in hexadecimal");
        assert!(data.chars().all(|c| c.is_ascii_hexdigit()));
        // Our validator, which rejects a KeePass `.keyx` with one digit changed (rf_12), accepts it.
        assert!(KeyFile::from_bytes(keyx.clone()).is_ok());
        assert_eq!(opens(&path, None, Some(&keyx)), Ok(()));
        keys.push(data.to_owned());
    }
    assert_ne!(keys[0], keys[1], "each key file has its own random bytes");
}
