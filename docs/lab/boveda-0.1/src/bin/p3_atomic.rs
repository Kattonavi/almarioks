//! P3: atomic save on Windows with safe std only (the core forbids `unsafe`).
//! Questions: does `std::fs::rename` replace an existing destination? What happens when another process holds the
//! destination open? Can the previous version be kept as `.bak` without a window where the vault does not exist?

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_WRITE: u32 = 0x2;
const FILE_SHARE_DELETE: u32 = 0x4;

fn read(path: &Path) -> String {
    let mut text = String::new();
    match File::open(path) {
        Ok(mut file) => {
            let _ = file.read_to_string(&mut text);
            text
        }
        Err(error) => format!("<{}>", error.kind()),
    }
}

/// The save sequence proposed for `almarioks-core::storage`.
fn save(vault: &Path, new_content: &[u8]) -> std::io::Result<()> {
    let tmp = vault.with_extension("kdbx.tmp");
    let bak = vault.with_extension("kdbx.bak");
    let bak_tmp = vault.with_extension("kdbx.bak.tmp");

    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp)?;
    file.write_all(new_content)?;
    file.sync_all()?; // FlushFileBuffers: data and metadata on disk before the rename
    drop(file);
    // (here the core re-reads `tmp` and decrypts it with the key)

    let had_previous = vault.exists();
    if had_previous {
        // First run of this lab promoted the copy to `.bak` BEFORE the final rename: when that rename failed, the
        // older backup was already gone. So: copy now, promote only after the vault was replaced.
        fs::copy(vault, &bak_tmp)?; // the vault never stops existing
        // `fs::copy` also copies the read-only attribute; a read-only `.bak` would make the next save fail.
        let mut permissions = fs::metadata(&bak_tmp)?.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&bak_tmp, permissions)?;
    }
    fs::rename(&tmp, vault)?; // replaces the vault in one step
    if had_previous {
        fs::rename(&bak_tmp, &bak)?; // replaces the previous .bak
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let dir: PathBuf = std::env::temp_dir().join(format!("almarioks-lab-p3-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let vault = dir.join("cuentas.kdbx");
    let bak = dir.join("cuentas.kdbx.bak");

    println!("== P3 guardado atomico con std (sin unsafe) ==");
    fs::write(&vault, b"v1")?;
    save(&vault, b"v2")?;
    println!("  tras guardar v2: boveda={} bak={}", read(&vault), read(&bak));
    save(&vault, b"v3")?;
    println!("  tras guardar v3: boveda={} bak={}", read(&vault), read(&bak));
    let leftovers: Vec<_> = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    println!("  archivos en la carpeta: {leftovers:?}");

    println!("== destino abierto por otro (handle sin FILE_SHARE_DELETE, como el Bloc de notas o un antivirus) ==");
    let holder = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&vault)?;
    let result = save(&vault, b"v4");
    println!(
        "  guardar v4 -> {:?} | boveda sigue = {} | bak = {}",
        result.as_ref().map_err(|error| (error.kind(), error.raw_os_error())),
        read(&vault),
        read(&bak)
    );
    println!("  temporal que queda: {}", dir.join("cuentas.kdbx.tmp").exists());
    drop(holder);

    println!("== destino abierto por otro CON FILE_SHARE_DELETE (como KeePass al leer) ==");
    let holder = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(&vault)?;
    let result = save(&vault, b"v5");
    println!(
        "  guardar v5 -> {:?} | boveda = {}",
        result.as_ref().map_err(|error| (error.kind(), error.raw_os_error())),
        read(&vault)
    );
    drop(holder);

    println!("== carpeta de solo lectura simulada: destino con atributo de solo lectura ==");
    let mut permissions = fs::metadata(&vault)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&vault, permissions.clone())?;
    let result = save(&vault, b"v6");
    println!(
        "  guardar v6 -> {:?} | boveda = {}",
        result.as_ref().map_err(|error| (error.kind(), error.raw_os_error())),
        read(&vault)
    );
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&vault, permissions)?;

    println!("== corte entre escribir el temporal y renombrar (se simula no llamando a rename) ==");
    fs::write(dir.join("cuentas.kdbx.tmp"), b"a medias")?;
    println!("  boveda = {} (el temporal sobrante no la afecta y el siguiente guardado lo pisa)", read(&vault));
    save(&vault, b"v7")?;
    println!("  tras guardar v7: boveda={} bak={}", read(&vault), read(&bak));

    fs::remove_dir_all(&dir)?;
    Ok(())
}
