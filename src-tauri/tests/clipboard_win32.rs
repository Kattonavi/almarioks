//! The REAL Windows clipboard (RF-71 to RF-73, RF-75): exclusion formats, clear by sequence number, content of
//! another process left alone, and the Win+V history asked through WinRT.
//!
//! WARNING: these tests overwrite the clipboard of whoever runs them and leave it empty. They only write fake
//! canaries (`CANARIO-ALMARIOKS-…`) and delete them from the clipboard history when they finish.
//!
//! The clipboard is one per session and nextest runs every test in its own process, so the tests take a file
//! lock to run one at a time.
//!
//! A CI session may have no usable clipboard. Then, and only when the `CI` variable is set, each test prints
//! `SKIPPED` with the operating system's error and returns without verifying anything. Without `CI` it fails.

#![cfg(windows)]
// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs::File,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use almarioks_lib::clipboard::{
    ClearOutcome, ClipboardService, SystemClipboard, TickOutcome,
    ffi::{
        WindowsClipboard,
        probe::{self, Format},
    },
};
use secrecy::SecretString;
use windows::ApplicationModel::DataTransfer::{
    Clipboard, ClipboardHistoryItemsResultStatus, StandardDataFormats,
};

/// Every canary starts like this, so the history can be cleaned without looking at anything else.
const CANARY: &str = "CANARIO-ALMARIOKS-";

/// One test at a time across processes: the lock is released when the file is dropped.
fn serial() -> File {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("clipboard-win32.lock");
    let file = File::create(path).expect("create the lock file");
    file.lock().expect("lock the clipboard tests");
    file
}

/// The real clipboard, or `None` (with a message) when this is a CI session without one.
fn real_clipboard(test: &str) -> Option<WindowsClipboard> {
    let problem = match probe::availability().and_then(|()| WindowsClipboard::new()) {
        Ok(clipboard) => return Some(clipboard),
        Err(problem) => problem,
    };
    let os_code = problem.os_code();
    assert!(
        std::env::var_os("CI").is_some(),
        "{test}: the clipboard of this session cannot be used (OS error {os_code:#010x})"
    );
    eprintln!(
        "SKIPPED {test}: this CI session has no usable clipboard (OS error {os_code:#010x}). \
         Nothing was verified here."
    );
    None
}

/// A fake value, different on every run.
fn canary(kind: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());
    format!("{CANARY}{kind}-{}-{nanos}", std::process::id())
}

fn secret(text: &str) -> SecretString {
    SecretString::from(text)
}

/// The text in the clipboard. Only called right after this file, or its fake "other app", wrote to it.
fn clipboard_text() -> Option<String> {
    let bytes = probe::bytes(Format::UnicodeText).expect("open the clipboard")?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .take_while(|unit| *unit != 0)
        .collect();
    Some(String::from_utf16(&units).expect("valid UTF-16"))
}

/// A real second process (`clip.exe`) writes to the clipboard, as Notepad would.
fn other_process_copies(text: &str) {
    let status = Command::new("cmd")
        .args(["/C", &format!("echo {text}| clip")])
        .status()
        .expect("run cmd");
    assert!(status.success(), "clip.exe failed");
}

/// There is no unconditional clear in the product: write a canary and clear it by its own sequence number.
fn leave_empty(clipboard: &WindowsClipboard) {
    let sequence = clipboard
        .set_secret(&secret(&canary("LIMPIEZA")))
        .expect("write the cleanup canary");
    assert_eq!(clipboard.clear_if_sequence_is(sequence), Ok(true));
    assert_eq!(clipboard_text(), None);
}

/// Looks in the Win+V history for items whose text contains `needle` and optionally deletes them. Never
/// returns or prints any other item. Needs a single-threaded apartment.
fn history_items_with(needle: &str, delete: bool) -> windows::core::Result<usize> {
    let result = Clipboard::GetHistoryItemsAsync()?.join()?;
    if result.Status()? != ClipboardHistoryItemsResultStatus::Success {
        return Ok(0);
    }
    let text_format = StandardDataFormats::Text()?;
    let mut found = 0;
    for item in result.Items()? {
        let content = item.Content()?;
        if !content.Contains(&text_format)? {
            continue;
        }
        let Ok(text) = content.GetTextAsync().and_then(|text| text.join()) else {
            continue;
        };
        if text.to_string().contains(needle) {
            found += 1;
            if delete {
                Clipboard::DeleteItemFromHistory(&item)?;
            }
        }
    }
    Ok(found)
}

/// Deletes from the history the canaries `clip.exe` left there (ours never get in). Best effort.
fn forget_canaries_in_history() {
    if probe::enter_single_threaded_apartment().is_err() {
        return;
    }
    if Clipboard::IsHistoryEnabled().unwrap_or(false)
        && let Err(error) = history_items_with(CANARY, true)
    {
        eprintln!("could not clean the canaries from the clipboard history: {error}");
    }
}

#[test]
fn rf_72_a_copy_carries_the_three_exclusion_formats_and_the_text() {
    let _serial = serial();
    let name = "rf_72_a_copy_carries_the_three_exclusion_formats_and_the_text";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    // Accents and a character outside the basic plane: the text must survive UTF-16 intact.
    let value = format!("{}-ñá-\u{1D11E}", canary("FORMATOS"));

    let sequence = clipboard.set_secret(&secret(&value)).unwrap();

    assert_eq!(clipboard_text().as_deref(), Some(value.as_str()));
    assert!(
        probe::bytes(Format::ExcludeFromMonitors).unwrap().is_some(),
        "ExcludeClipboardContentFromMonitorProcessing is missing"
    );
    for format in [Format::CanIncludeInHistory, Format::CanUploadToCloud] {
        let bytes = probe::bytes(format).unwrap();
        let dword = bytes.as_deref().and_then(|bytes| bytes.get(..4));
        assert_eq!(dword, Some(&[0u8; 4][..]), "{format:?} must be DWORD 0");
    }
    assert_eq!(clipboard.sequence_number(), sequence);

    assert_eq!(clipboard.clear_if_sequence_is(sequence), Ok(true));
    assert_eq!(clipboard_text(), None);
}

#[test]
fn rf_73_clear_by_sequence_number_empties_the_clipboard_even_after_a_paste() {
    let _serial = serial();
    let name = "rf_73_clear_by_sequence_number_empties_the_clipboard_even_after_a_paste";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    let value = canary("PEGADO");
    let sequence = clipboard.set_secret(&secret(&value)).unwrap();

    // A paste reads the clipboard; an old app asks for the ANSI text Windows makes on demand. Neither is a
    // write: the content must still count as ours.
    assert_eq!(clipboard_text().as_deref(), Some(value.as_str()));
    let ansi = probe::bytes(Format::AnsiText).unwrap().expect("CF_TEXT");
    assert!(ansi.starts_with(value.as_bytes()));
    assert_eq!(clipboard.sequence_number(), sequence, "reading moved it");

    assert_eq!(clipboard.clear_if_sequence_is(sequence), Ok(true));
    assert_eq!(clipboard_text(), None);
    assert_eq!(
        clipboard.clear_if_sequence_is(sequence),
        Ok(false),
        "the same number never clears twice"
    );
}

#[test]
fn rf_73_what_another_process_copied_afterwards_is_left_alone() {
    let _serial = serial();
    let name = "rf_73_what_another_process_copied_afterwards_is_left_alone";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    let ours = clipboard.set_secret(&secret(&canary("NUESTRO"))).unwrap();
    let foreign = canary("AJENO");

    other_process_copies(&foreign);

    assert_ne!(clipboard.sequence_number(), ours);
    assert_eq!(clipboard.clear_if_sequence_is(ours), Ok(false));
    assert_eq!(
        clipboard_text().as_deref().map(str::trim_end),
        Some(foreign.as_str()),
        "the other process's text must still be there"
    );

    leave_empty(&clipboard);
    forget_canaries_in_history();
}

#[test]
fn rf_71_a_value_with_a_nul_is_refused_instead_of_being_cut_short() {
    let _serial = serial();
    let name = "rf_71_a_value_with_a_nul_is_refused_instead_of_being_cut_short";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    let before = clipboard.sequence_number();

    let refused = clipboard.set_secret(&secret(&format!("{}\0resto", canary("NUL"))));

    assert!(refused.is_err());
    assert_eq!(clipboard.sequence_number(), before, "nothing was written");
}

#[test]
fn rf_72_the_clipboard_history_does_not_keep_the_canary() {
    let _serial = serial();
    let name = "rf_72_the_clipboard_history_does_not_keep_the_canary";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    probe::enter_single_threaded_apartment().expect("single-threaded apartment for WinRT");
    if !Clipboard::IsHistoryEnabled().unwrap_or(false) {
        eprintln!(
            "NOT VERIFIED {name}: the clipboard history (Win+V) is turned off on this machine."
        );
        return;
    }

    // Positive control: an UNMARKED copy from another process does reach the history, so the question below
    // can tell "kept" from "not kept". Only when cloud sync is off: an unmarked canary would leave this PC.
    if Clipboard::IsRoamingEnabled().unwrap_or(true) {
        eprintln!("{name}: positive control skipped, the cloud clipboard is on.");
    } else {
        let control = canary("CONTROL");
        other_process_copies(&control);
        let asked = Instant::now();
        while history_items_with(&control, false).unwrap() == 0 {
            assert!(
                asked.elapsed() < Duration::from_secs(10),
                "control: the history never showed an unmarked copy"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    let marked = canary("MARCADO");
    let sequence = clipboard.set_secret(&secret(&marked)).unwrap();
    // The history service reads the clipboard a moment after every change: give it time to get it wrong.
    thread::sleep(Duration::from_secs(2));

    assert_eq!(
        history_items_with(&marked, false).unwrap(),
        0,
        "the marked copy is in the Win+V history"
    );
    assert_eq!(
        clipboard.clear_if_sequence_is(sequence),
        Ok(true),
        "the history service must not make the copy look foreign"
    );
    forget_canaries_in_history();
    assert_eq!(history_items_with(CANARY, false).unwrap(), 0);
}

#[test]
fn rf_75_the_system_service_copies_clears_on_lock_and_respects_foreign_content_on_exit() {
    let _serial = serial();
    let name =
        "rf_75_the_system_service_copies_clears_on_lock_and_respects_foreign_content_on_exit";
    let Some(clipboard) = real_clipboard(name) else {
        return;
    };
    let service = ClipboardService::system().expect("the system clipboard service");
    let value = canary("SERVICIO");

    // RF-71, RF-74: the value reaches the real clipboard and the count starts.
    let receipt = service.copy(secret(&value), 20).unwrap();
    assert_eq!(receipt.clears_in_seconds, 20);
    assert_eq!(clipboard_text().as_deref(), Some(value.as_str()));
    assert!(matches!(service.tick(), Ok(TickOutcome::NotDue { .. })));
    assert!(service.clears_in().is_some());

    // RF-31: lock clears what is ours.
    assert_eq!(service.on_lock(), Ok(ClearOutcome::Cleared));
    assert_eq!(clipboard_text(), None);

    // RF-75: exit leaves alone what another process copied after us.
    service.copy(secret(&canary("SERVICIO-2")), 20).unwrap();
    let foreign = canary("AJENO-SALIDA");
    other_process_copies(&foreign);
    assert_eq!(service.clears_in(), None);
    assert_eq!(service.on_exit(), Ok(ClearOutcome::NotOurs));
    assert_eq!(
        clipboard_text().as_deref().map(str::trim_end),
        Some(foreign.as_str())
    );

    leave_empty(&clipboard);
    forget_canaries_in_history();
}
