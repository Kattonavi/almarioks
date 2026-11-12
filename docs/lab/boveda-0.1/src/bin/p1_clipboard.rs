//! P1: write text to the Windows clipboard excluded from history (Win+V) and cloud, detect whether it is
//! still ours without reading it back, and clear it only if it is (RF-71..RF-75).
//!
//! WARNING: this overwrites the clipboard of whoever runs it. It never prints what was there before.
//! Every canary is fake data. The positive-control canary (allowed into history) is deleted from the
//! history at the end and is always marked CanUploadToCloudClipboard=0, so it never leaves this PC.

use std::{process::Command, thread::sleep, time::Duration};

use windows::{
    ApplicationModel::DataTransfer::{Clipboard, ClipboardHistoryItemsResultStatus, StandardDataFormats},
    Win32::{
        Foundation::{GlobalFree, HANDLE, HWND},
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner, GetClipboardSequenceNumber,
                IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            Ole::CF_UNICODETEXT,
        },
        UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE},
    },
    core::w,
};
use zeroize::Zeroize;

const CF_TEXT_ID: u32 = CF_UNICODETEXT.0 as u32;

struct Formats {
    exclude: u32,
    history: u32,
    cloud: u32,
}

#[derive(Clone, Copy)]
enum Marks {
    /// Only CanUploadToCloudClipboard=0: may enter the local history, never the cloud.
    CloudOnly,
    /// The three formats: ExcludeClipboardContentFromMonitorProcessing + CanInclude...=0 + CanUpload...=0.
    All,
}

fn formats() -> Formats {
    // SAFETY: RegisterClipboardFormatW takes a NUL-terminated wide string literal.
    unsafe {
        Formats {
            exclude: RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
            history: RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
            cloud: RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
        }
    }
}

/// Puts `bytes` in a movable global block and hands it to the clipboard (which then owns it).
///
/// # Safety
/// The clipboard must be open by this thread and emptied with an owner window.
unsafe fn set_bytes(format: u32, bytes: &[u8]) -> windows::core::Result<()> {
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))?;
        let ptr = GlobalLock(mem).cast::<u8>();
        if ptr.is_null() {
            let _ = GlobalFree(Some(mem));
            return Err(windows::core::Error::from_thread());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(mem); // returns Err(NO_ERROR) when the lock count reaches 0
        match SetClipboardData(format, Some(HANDLE(mem.0))) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Ownership did not pass to the system: free it ourselves.
                let _ = GlobalFree(Some(mem));
                Err(e)
            }
        }
    }
}

/// Copies `text` and returns the clipboard sequence number read right after `CloseClipboard`.
fn copy(owner: Option<HWND>, f: &Formats, text: &str, marks: Marks) -> windows::core::Result<u32> {
    let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let mut bytes: Vec<u8> = wide.iter().flat_map(|c| c.to_le_bytes()).collect();
    // SAFETY: open/empty/set/close on this thread, always closed on every path.
    let result = unsafe {
        OpenClipboard(owner)?;
        let r = (|| {
            EmptyClipboard()?;
            match marks {
                Marks::CloudOnly => set_bytes(f.cloud, &0u32.to_le_bytes())?,
                Marks::All => {
                    set_bytes(f.exclude, &0u32.to_le_bytes())?;
                    set_bytes(f.history, &0u32.to_le_bytes())?;
                    set_bytes(f.cloud, &0u32.to_le_bytes())?;
                }
            }
            set_bytes(CF_TEXT_ID, &bytes)
        })();
        let _ = CloseClipboard();
        r
    };
    wide.zeroize();
    bytes.zeroize();
    result?;
    // SAFETY: no preconditions.
    Ok(unsafe { GetClipboardSequenceNumber() })
}

#[derive(Debug)]
#[allow(dead_code)] // read through Debug
enum ClearOutcome {
    Cleared,
    NotOurs { now: u32 },
}

/// RF-73/RF-75: empties the clipboard only if nobody wrote to it after us. The sequence is compared while
/// the clipboard is OPEN, so no other process can change it between the check and EmptyClipboard.
fn clear_if_ours(owner: HWND, expected: u32) -> windows::core::Result<ClearOutcome> {
    // SAFETY: open/close pair on this thread.
    unsafe {
        OpenClipboard(Some(owner))?;
        let now = GetClipboardSequenceNumber();
        let r = if now == expected { EmptyClipboard().map(|()| ClearOutcome::Cleared) } else { Ok(ClearOutcome::NotOurs { now }) };
        let _ = CloseClipboard();
        r
    }
}

fn has_text() -> bool {
    // SAFETY: no preconditions.
    unsafe { IsClipboardFormatAvailable(CF_TEXT_ID).is_ok() }
}

/// Reads CF_UNICODETEXT. Used ONLY to show text this lab put there or that the fake "other app" put there.
fn read_text(owner: HWND) -> Option<String> {
    // SAFETY: open/close pair; the handle is only read while locked.
    unsafe {
        OpenClipboard(Some(owner)).ok()?;
        let out = GetClipboardData(CF_TEXT_ID).ok().and_then(|h| {
            let mem = windows::Win32::Foundation::HGLOBAL(h.0);
            let p = GlobalLock(mem).cast::<u16>();
            if p.is_null() {
                return None;
            }
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            let _ = GlobalUnlock(mem);
            Some(s)
        });
        let _ = CloseClipboard();
        out
    }
}

fn seq() -> u32 {
    // SAFETY: no preconditions.
    unsafe { GetClipboardSequenceNumber() }
}

fn owner_is(hwnd: HWND) -> bool {
    // SAFETY: no preconditions.
    unsafe { GetClipboardOwner().map(|o| o == hwnd).unwrap_or(false) }
}

fn other_process_copies(text: &str) {
    // A real second process (clip.exe), as Notepad would be.
    let status = Command::new("cmd").args(["/C", &format!("echo {text}| clip")]).status();
    println!("    otro proceso (cmd + clip.exe) copio \"{text}\": {:?}", status.map(|s| s.success()));
}

/// Looks for our own canaries in the Win+V history. Never prints any other item.
fn history_contains(canary: &str, delete: bool) -> Result<bool, String> {
    let result = Clipboard::GetHistoryItemsAsync().and_then(|op| op.join()).map_err(|e| format!("{e}"))?;
    let status = result.Status().map_err(|e| format!("{e}"))?;
    if status != ClipboardHistoryItemsResultStatus::Success {
        return Err(format!("Status = {status:?}"));
    }
    let items = result.Items().map_err(|e| format!("{e}"))?;
    let text_format = StandardDataFormats::Text().map_err(|e| format!("{e}"))?;
    for item in items {
        let Ok(content) = item.Content() else { continue };
        if !content.Contains(&text_format).unwrap_or(false) {
            continue;
        }
        let Ok(text) = content.GetTextAsync().and_then(|op| op.join()) else { continue };
        if text.to_string().contains(canary) {
            if delete {
                let deleted = Clipboard::DeleteItemFromHistory(&item);
                println!("      (canario de control borrado del historial: {deleted:?})");
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// Removes from the Win+V history every item this lab may have left: text containing "CANARIO-", plus the
/// exact `hola\r\n` an earlier version of the lab copied through clip.exe. Prints only the count.
fn clean_history() -> Result<usize, String> {
    let result = Clipboard::GetHistoryItemsAsync().and_then(|op| op.join()).map_err(|e| format!("{e}"))?;
    if result.Status().map_err(|e| format!("{e}"))? != ClipboardHistoryItemsResultStatus::Success {
        return Ok(0);
    }
    let text_format = StandardDataFormats::Text().map_err(|e| format!("{e}"))?;
    let mut n = 0;
    for item in result.Items().map_err(|e| format!("{e}"))? {
        let Ok(content) = item.Content() else { continue };
        if !content.Contains(&text_format).unwrap_or(false) {
            continue;
        }
        let Ok(text) = content.GetTextAsync().and_then(|op| op.join()) else { continue };
        let text = text.to_string();
        if (text.contains("CANARIO-") || text == "hola\r\n") && Clipboard::DeleteItemFromHistory(&item).unwrap_or(false) {
            n += 1;
        }
    }
    Ok(n)
}

fn main() -> windows::core::Result<()> {
    // SAFETY: COM for the WinRT clipboard history API; the WinRT Clipboard class is single-threaded (from MTA it fails with 0x8000001D, first run).
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    // SAFETY: a message-only window ("STATIC" class) owned by this thread, destroyed at the end.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("boveda-lab-clipboard"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )?
    };
    let f = formats();
    println!("== P1 portapapeles ==");
    println!(
        "  formatos registrados: Exclude...={:#06x} CanIncludeInClipboardHistory={:#06x} CanUploadToCloudClipboard={:#06x}",
        f.exclude, f.history, f.cloud
    );

    println!("\n  [1] OpenClipboard(NULL) + EmptyClipboard + SetClipboardData (la doc dice que falla):");
    let r = copy(None, &f, "CANARIO-NULL-OWNER", Marks::All);
    println!("    resultado: {r:?} | hay texto: {} | dueño NULL: {}", has_text(), unsafe { GetClipboardOwner() }.is_err());

    println!("\n  [2] Copia marcada con dueño = ventana oculta de solo mensajes:");
    let before = seq();
    let s1 = copy(Some(hwnd), &f, "CANARIO-A-0001", Marks::All)?;
    println!("    secuencia antes {before} -> despues de CloseClipboard {s1} (+{} por 1 copia con 4 formatos)", s1.wrapping_sub(before));
    println!("    dueño = nuestra ventana: {} | hay texto: {}", owner_is(hwnd), has_text());

    println!("\n  [3] (a) nadie mas copia: limpieza condicional");
    println!("    clear_if_ours -> {:?} | hay texto despues: {} | secuencia ahora {}", clear_if_ours(hwnd, s1)?, has_text(), seq());

    println!("\n  [4] (b) otra app copia antes de la limpieza");
    let s2 = copy(Some(hwnd), &f, "CANARIO-B-0002", Marks::All)?;
    other_process_copies("CANARIO-AJENO-hola");
    println!("    secuencia nuestra {s2}, ahora {} | dueño = nuestra ventana: {}", seq(), owner_is(hwnd));
    println!("    clear_if_ours -> {:?}", clear_if_ours(hwnd, s2)?);
    println!("    el portapapeles conserva lo ajeno: {:?}", read_text(hwnd).map(|t| t.trim_end().to_owned()));

    println!("\n  [5] (c) copiamos otra contraseña antes de la limpieza: vale solo la ultima secuencia");
    let s3 = copy(Some(hwnd), &f, "CANARIO-C-0003", Marks::All)?;
    let s4 = copy(Some(hwnd), &f, "CANARIO-C-0004", Marks::All)?;
    println!("    limpieza vieja (secuencia {s3}) -> {:?}", clear_if_ours(hwnd, s3)?);
    println!("    limpieza nueva (secuencia {s4}) -> {:?} | hay texto: {}", clear_if_ours(hwnd, s4)?, has_text());

    println!("\n  [6] Historial de Win+V por programa (WinRT Clipboard):");
    let enabled = Clipboard::IsHistoryEnabled();
    let roaming = Clipboard::IsRoamingEnabled();
    println!("    IsHistoryEnabled = {enabled:?} | IsRoamingEnabled = {roaming:?}");
    let tag = format!("{:08x}", seq().wrapping_mul(2_654_435_761));
    let control = format!("CANARIO-CONTROL-{tag}");
    let marked = format!("CANARIO-MARCADO-{tag}");
    let history_only = format!("CANARIO-SOLO-HISTORIAL0-{tag}");
    copy(Some(hwnd), &f, &control, Marks::CloudOnly)?;
    sleep(Duration::from_millis(1500));
    copy(Some(hwnd), &f, &marked, Marks::All)?;
    sleep(Duration::from_millis(1500));
    // Only CanIncludeInClipboardHistory=0 (+ cloud 0 for safety), without the Exclude format.
    {
        // SAFETY: open/close pair on this thread.
        unsafe {
            OpenClipboard(Some(hwnd))?;
            EmptyClipboard()?;
            set_bytes(f.history, &0u32.to_le_bytes())?;
            set_bytes(f.cloud, &0u32.to_le_bytes())?;
            let wide: Vec<u8> = history_only.encode_utf16().chain(std::iter::once(0)).flat_map(u16::to_le_bytes).collect();
            set_bytes(CF_TEXT_ID, &wide)?;
            let _ = CloseClipboard();
        }
    }
    sleep(Duration::from_millis(1500));
    println!("    control (sin marcas de historial) en el historial: {:?}", history_contains(&control, true));
    println!("    marcado (3 formatos) en el historial:              {:?}", history_contains(&marked, true));
    println!("    solo CanIncludeInClipboardHistory=0:              {:?}", history_contains(&history_only, true));

    println!("    limpieza del historial (restos del lab, incluido el canario ajeno): {:?} elementos", clean_history());

    // Leave the clipboard empty (the user's previous content was already overwritten by this lab).
    // SAFETY: open/close pair on this thread; window created above.
    unsafe {
        OpenClipboard(Some(hwnd))?;
        let _ = EmptyClipboard();
        let _ = CloseClipboard();
        let _ = DestroyWindow(hwnd);
    }
    println!("\n  portapapeles vaciado al salir: hay texto = {}", has_text());
    Ok(())
}
