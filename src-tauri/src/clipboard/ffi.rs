//! The Windows clipboard behind [`SystemClipboard`]: the one module of the project with `unsafe`.
//!
//! How (measured in `docs/lab/boveda-0.1`, P1):
//! - A hidden message-only window (`HWND_MESSAGE`) owns what we copy. It lives on its own thread, which runs a
//!   `GetMessageW` loop and does every clipboard operation itself: a window belongs to the thread that creates
//!   it, Tauri commands run on whatever pool thread is free, and this way the handle never leaves its thread.
//! - A copy sets, in one opening of the clipboard and BEFORE the text, the three registered formats that keep
//!   it out of the clipboard history (Win+V) and of the cloud (RF-72), then `CF_UNICODETEXT`.
//! - The sequence number is read right after `CloseClipboard`. A clear opens the clipboard, compares the number
//!   while it is OPEN (nobody else can write meanwhile) and only then empties it (RF-73).
//!
//! The product never reads the clipboard. [`probe`] does, for the integration tests only.

// The only file of the workspace allowed to use `unsafe` (workspace lint `unsafe_code = "deny"`, constitution
// rule 9): Win32 has no safe API for the clipboard. Every `unsafe` block says why it is sound.
#![allow(unsafe_code)]

use std::{
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use secrecy::{ExposeSecret, SecretString};
use windows::{
    Win32::{
        Foundation::{
            ERROR_ACCESS_DENIED, ERROR_BUSY, ERROR_INVALID_DATA, ERROR_INVALID_WINDOW_HANDLE,
            ERROR_NOT_ENOUGH_MEMORY, ERROR_NOT_OWNER, ERROR_TIMEOUT, GlobalFree, HANDLE, HWND,
            LPARAM, WIN32_ERROR, WPARAM,
        },
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardOwner, GetClipboardSequenceNumber,
                OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            Ole::CF_UNICODETEXT,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
            PostMessageW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
        },
    },
    core::{HRESULT, PCWSTR, w},
};
use zeroize::Zeroizing;

use super::{SequenceNumber, SystemClipboard, SystemClipboardError};

/// Posted to the owner window to say "there are requests in the queue".
const WAKE: u32 = WM_APP;
/// Another app may hold the clipboard open for an instant: try a few times, far below the 1 s of RF-30.
const OPEN_ATTEMPTS: u32 = 10;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(5);
/// The owner thread answers in about `OPEN_ATTEMPTS * OPEN_RETRY_DELAY` at worst; past this it is taken for dead.
/// Short on purpose: a lock waits for a copy in flight and then for its own clear, and both must fit in the
/// 1 s of RF-30. A false timeout fails safe: the owner thread takes back a copy nobody waited for.
const REPLY_TIMEOUT: Duration = Duration::from_millis(250);
/// Requests waiting for the owner thread. The service sends one at a time; a full queue is an error, not a wait.
const QUEUE: usize = 8;

const EXCLUDE_FROM_MONITORS: PCWSTR = w!("ExcludeClipboardContentFromMonitorProcessing");
const CAN_INCLUDE_IN_HISTORY: PCWSTR = w!("CanIncludeInClipboardHistory");
const CAN_UPLOAD_TO_CLOUD: PCWSTR = w!("CanUploadToCloudClipboard");

type Reply<T> = SyncSender<Result<T, SystemClipboardError>>;

/// What the owner thread is asked to do. No `Debug`: `Set` carries the value being copied.
enum Request {
    Set {
        /// The value as NUL-terminated UTF-16, wiped when dropped.
        text: Zeroizing<Vec<u16>>,
        reply: Reply<SequenceNumber>,
    },
    ClearIf {
        expected: SequenceNumber,
        reply: Reply<bool>,
    },
    Stop,
}

/// The real Windows clipboard. Dropping it destroys the owner window and stops its thread; what is in the
/// clipboard stays, so call the service's `on_exit` first.
pub struct WindowsClipboard {
    requests: SyncSender<Request>,
    /// The owner window's handle as an integer: `HWND` is a raw pointer and is not `Send`. Only used here to
    /// post the wake-up message, which Windows allows from any thread.
    window: isize,
    worker: Option<JoinHandle<()>>,
}

impl WindowsClipboard {
    /// Starts the owner thread and its hidden window.
    ///
    /// # Errors
    /// The thread or the window cannot be created (for example, a session with no window station).
    pub fn new() -> Result<Self, SystemClipboardError> {
        let (requests, inbox) = mpsc::sync_channel(QUEUE);
        let (ready, started) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("almarioks-clipboard".to_owned())
            .spawn(move || owner_thread(&ready, &inbox))
            .map_err(|_| win32(ERROR_NOT_ENOUGH_MEMORY))?;
        match started.recv() {
            Ok(Ok(window)) => Ok(Self {
                requests,
                window,
                worker: Some(worker),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(win32(ERROR_INVALID_WINDOW_HANDLE)),
        }
    }

    fn ask<T>(&self, request: impl FnOnce(Reply<T>) -> Request) -> Result<T, SystemClipboardError> {
        let (reply, answer) = mpsc::sync_channel(1);
        // A request that cannot be queued is dropped here, and its value wiped with it.
        self.requests
            .try_send(request(reply))
            .map_err(|_| win32(ERROR_BUSY))?;
        self.wake()?;
        // If nobody is listening when the owner thread answers a `Set`, it takes the value back.
        answer
            .recv_timeout(REPLY_TIMEOUT)
            .unwrap_or_else(|_| Err(win32(ERROR_TIMEOUT)))
    }

    fn wake(&self) -> Result<(), SystemClipboardError> {
        #[allow(clippy::as_conversions)]
        // integer back to the handle it came from; same width, nothing is lost
        let window = HWND(self.window as *mut core::ffi::c_void);
        // SAFETY: `PostMessageW` only queues a message; it accepts any handle value from any thread and fails
        // with an error if the window no longer exists.
        unsafe { PostMessageW(Some(window), WAKE, WPARAM(0), LPARAM(0)) }.map_err(os_error)
    }
}

impl SystemClipboard for WindowsClipboard {
    fn set_secret(&self, secret: &SecretString) -> Result<SequenceNumber, SystemClipboardError> {
        let value = secret.expose_secret();
        if value.contains('\0') {
            // `CF_UNICODETEXT` ends at the first NUL: the paste would silently lose the rest of the value.
            return Err(win32(ERROR_INVALID_DATA));
        }
        // UTF-16 never has more units than UTF-8 has bytes: the buffer is allocated once and never moves, so
        // the only copy of the value this module makes is the one `Zeroizing` wipes.
        let mut text = Zeroizing::new(Vec::with_capacity(value.len() + 1));
        text.extend(value.encode_utf16());
        text.push(0);
        self.ask(|reply| Request::Set { text, reply })
    }

    fn sequence_number(&self) -> SequenceNumber {
        current_sequence()
    }

    fn clear_if_sequence_is(&self, expected: SequenceNumber) -> Result<bool, SystemClipboardError> {
        self.ask(|reply| Request::ClearIf { expected, reply })
    }
}

impl Drop for WindowsClipboard {
    fn drop(&mut self) {
        // If the thread is already gone there is nobody to stop: both errors mean exactly that.
        let _ = self.requests.try_send(Request::Stop);
        let _ = self.wake();
        if let Some(worker) = self.worker.take() {
            // A panic of the owner thread has nowhere to go from a destructor.
            let _ = worker.join();
        }
    }
}

/// The three registered formats that keep a copy out of the history and the cloud.
struct ExclusionFormats {
    exclude_from_monitors: u32,
    can_include_in_history: u32,
    can_upload_to_cloud: u32,
}

/// Body of the owner thread: creates the window, serves requests when woken and destroys the window at the end.
fn owner_thread(
    ready: &SyncSender<Result<isize, SystemClipboardError>>,
    inbox: &Receiver<Request>,
) {
    let started = create_owner_window()
        .and_then(|window| Ok((window, register_exclusion_formats()?)))
        .map_err(os_error);
    let (window, formats) = match started {
        Ok(started) => started,
        Err(error) => {
            // The creator is waiting for exactly this message.
            let _ = ready.send(Err(error));
            return;
        }
    };
    #[allow(clippy::as_conversions)]
    // the handle as an integer of the same width, to cross to other threads
    let handle = window.0 as isize;
    if ready.send(Ok(handle)).is_ok() {
        let mut message = MSG::default();
        // SAFETY: `message` is a valid, writable `MSG` for the whole call. `GetMessageW` blocks until a message
        // arrives and dispatches by itself the ones other threads SEND to the window.
        while unsafe { GetMessageW(&raw mut message, None, 0, 0) }.0 > 0 {
            if message.hwnd == window && message.message == WAKE {
                if !serve(window, &formats, inbox) {
                    break;
                }
            } else {
                // SAFETY: `message` was just filled by `GetMessageW`.
                unsafe { DispatchMessageW(&raw const message) };
            }
        }
    }
    // SAFETY: the window was created by this thread and is destroyed once, here. Its only failure is a window
    // that no longer exists, which is the state we want.
    let _ = unsafe { DestroyWindow(window) };
}

/// Serves every queued request. Returns `false` when the thread must stop.
fn serve(window: HWND, formats: &ExclusionFormats, inbox: &Receiver<Request>) -> bool {
    loop {
        match inbox.try_recv() {
            Ok(Request::Set { text, reply }) => {
                let result = set_marked_text(window, formats, &text);
                drop(text);
                if reply.send(result).is_err()
                    && let Ok(sequence) = result
                {
                    // Nobody is waiting for the answer, so nobody would ever clear this copy: take it back.
                    // If the clipboard refuses, there is no one left to tell.
                    let _ = clear_if_sequence_is(window, sequence);
                }
            }
            Ok(Request::ClearIf { expected, reply }) => {
                // The requester may have stopped waiting; the clear itself already happened or not.
                let _ = reply.send(clear_if_sequence_is(window, expected));
            }
            Ok(Request::Stop) | Err(TryRecvError::Disconnected) => return false,
            Err(TryRecvError::Empty) => return true,
        }
    }
}

fn create_owner_window() -> windows::core::Result<HWND> {
    // SAFETY: the class and the title are NUL-terminated wide literals. `STATIC` is a system class, so there is
    // no window procedure of ours to keep alive. The parent `HWND_MESSAGE` makes it a message-only window:
    // never visible, never in the taskbar, not reached by broadcasts.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("AlmarioKS clipboard owner"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
}

fn register_clipboard_format(name: PCWSTR) -> windows::core::Result<u32> {
    // SAFETY: `name` is one of the NUL-terminated wide literals of this module.
    match unsafe { RegisterClipboardFormatW(name) } {
        0 => Err(windows::core::Error::from_thread()),
        format => Ok(format),
    }
}

fn register_exclusion_formats() -> windows::core::Result<ExclusionFormats> {
    Ok(ExclusionFormats {
        exclude_from_monitors: register_clipboard_format(EXCLUDE_FROM_MONITORS)?,
        can_include_in_history: register_clipboard_format(CAN_INCLUDE_IN_HISTORY)?,
        can_upload_to_cloud: register_clipboard_format(CAN_UPLOAD_TO_CLOUD)?,
    })
}

fn current_sequence() -> SequenceNumber {
    // SAFETY: no preconditions; it returns 0 when the session has no access to the clipboard.
    SequenceNumber(unsafe { GetClipboardSequenceNumber() })
}

/// Opens the clipboard for `owner`, retrying while another app holds it open.
fn open_clipboard(owner: Option<HWND>) -> windows::core::Result<()> {
    let mut attempt = 1;
    loop {
        // SAFETY: `owner` is `None` or a live window of the calling thread. A successful open is always paired
        // with `CloseClipboard` by the caller, on this same thread.
        match unsafe { OpenClipboard(owner) } {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= OPEN_ATTEMPTS => return Err(error),
            Err(_) => {
                attempt += 1;
                thread::sleep(OPEN_RETRY_DELAY);
            }
        }
    }
}

fn close_clipboard() {
    // SAFETY: only called by the thread that opened the clipboard. It can only fail if the clipboard is not
    // open, and then there is nothing to close.
    let _ = unsafe { CloseClipboard() };
}

/// Hands a copy of `data` to the open clipboard as `format`.
///
/// # Safety
/// The calling thread must have the clipboard open and have emptied it with an owner window.
unsafe fn put<T: Copy>(format: u32, data: &[T]) -> windows::core::Result<()> {
    let size = size_of_val(data);
    // SAFETY: plain allocation; the size is never 0.
    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, size.max(1)) }?;
    // SAFETY: `block` was just allocated and is not shared yet.
    let target = unsafe { GlobalLock(block) }.cast::<u8>();
    if target.is_null() {
        let error = windows::core::Error::from_thread();
        // SAFETY: `block` is ours and unlocked; nobody else has seen it.
        let _ = unsafe { GlobalFree(Some(block)) };
        return Err(error);
    }
    // SAFETY: `target` points to at least `size` writable bytes (the block was allocated with that size),
    // `data` is `size` readable bytes of plain `Copy` values, and the two cannot overlap.
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr().cast::<u8>(), target, size) };
    // SAFETY: pairs with the lock above. It reports an "error" with code 0 when the lock count reaches 0.
    let _ = unsafe { GlobalUnlock(block) };
    // SAFETY: the clipboard is open and owned by this thread (caller's contract); on success the system owns
    // `block` and we never touch it again.
    if let Err(error) = unsafe { SetClipboardData(format, Some(HANDLE(block.0))) } {
        // The block is still ours and may hold the value: wipe it before freeing it.
        // SAFETY: `block` is ours; when the lock succeeds it gives back `size` writable bytes.
        unsafe {
            let target = GlobalLock(block).cast::<u8>();
            if !target.is_null() {
                std::ptr::write_bytes(target, 0, size);
                let _ = GlobalUnlock(block);
            }
            let _ = GlobalFree(Some(block));
        }
        return Err(error);
    }
    Ok(())
}

/// RF-71 and RF-72: replaces the clipboard content with `text` (NUL-terminated UTF-16), marked to stay out of the
/// history and the cloud, and returns the sequence number read after closing the clipboard.
fn set_marked_text(
    window: HWND,
    formats: &ExclusionFormats,
    text: &[u16],
) -> Result<SequenceNumber, SystemClipboardError> {
    open_clipboard(Some(window)).map_err(os_error)?;
    let no: [u32; 1] = [0];
    // SAFETY: the clipboard is open by this thread for `window`; `EmptyClipboard` makes `window` the owner
    // before any `put`, which is the contract of `put`.
    let written = unsafe {
        EmptyClipboard()
            .and_then(|()| put(formats.exclude_from_monitors, &no))
            .and_then(|()| put(formats.can_include_in_history, &no))
            .and_then(|()| put(formats.can_upload_to_cloud, &no))
            .and_then(|()| put(u32::from(CF_UNICODETEXT.0), text))
    };
    if written.is_err() {
        // Never leave half a copy: for example the text without its marks.
        // SAFETY: the clipboard is still open by this thread.
        let _ = unsafe { EmptyClipboard() };
    }
    close_clipboard();
    written.map_err(os_error)?;
    let sequence = current_sequence();
    if sequence.0 == 0 {
        // 0 after a write means the session cannot read the number: we could never tell later whether the
        // content is still ours, so the copy is taken back now (0 is what a clear would read as well).
        let _ = clear_if_sequence_is(window, sequence);
        return Err(win32(ERROR_ACCESS_DENIED));
    }
    // The number was read after closing, so another app could have written in between. If it did, it also took
    // the ownership: checking the owner AFTER reading the number proves the number is ours.
    // SAFETY: no preconditions.
    if unsafe { GetClipboardOwner() }.ok() != Some(window) {
        return Err(win32(ERROR_NOT_OWNER));
    }
    Ok(sequence)
}

/// RF-73: empties the clipboard only if nobody wrote to it after `expected`. The number is compared while the
/// clipboard is open, so no other process can change it between the check and `EmptyClipboard`.
fn clear_if_sequence_is(
    window: HWND,
    expected: SequenceNumber,
) -> Result<bool, SystemClipboardError> {
    open_clipboard(Some(window)).map_err(os_error)?;
    let cleared = if current_sequence() == expected {
        // SAFETY: the clipboard is open by this thread.
        unsafe { EmptyClipboard() }.map(|()| true)
    } else {
        Ok(false)
    };
    close_clipboard();
    cleared.map_err(os_error)
}

fn os_error(error: windows::core::Error) -> SystemClipboardError {
    SystemClipboardError::new(error.code().0)
}

fn win32(code: WIN32_ERROR) -> SystemClipboardError {
    SystemClipboardError::new(HRESULT::from_win32(code.0).0)
}

/// Read-only helpers for the integration tests (`tests/clipboard_win32.rs`), here because this is the only
/// file that may use `unsafe`. The product never calls them: AlmarioKS never reads the clipboard.
#[doc(hidden)]
pub mod probe {
    use windows::Win32::{
        Foundation::HGLOBAL,
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
            DataExchange::GetClipboardData,
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            Ole::{CF_TEXT, CF_UNICODETEXT},
        },
    };

    use super::{
        CAN_INCLUDE_IN_HISTORY, CAN_UPLOAD_TO_CLOUD, EXCLUDE_FROM_MONITORS, close_clipboard,
        open_clipboard, os_error, register_clipboard_format,
    };
    use crate::clipboard::SystemClipboardError;

    /// A clipboard format the tests look at.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Format {
        /// `CF_UNICODETEXT`.
        UnicodeText,
        /// `CF_TEXT`, which Windows makes from the Unicode text when an old app asks for it.
        AnsiText,
        /// `ExcludeClipboardContentFromMonitorProcessing`.
        ExcludeFromMonitors,
        /// `CanIncludeInClipboardHistory`.
        CanIncludeInHistory,
        /// `CanUploadToCloudClipboard`.
        CanUploadToCloud,
    }

    /// Whether this session has a clipboard that can be opened.
    ///
    /// # Errors
    /// The operating system's reason why the clipboard cannot be opened.
    pub fn availability() -> Result<(), SystemClipboardError> {
        open_clipboard(None).map_err(os_error)?;
        close_clipboard();
        Ok(())
    }

    /// The raw bytes the clipboard holds in `format`, or `None` if it does not have that format.
    /// Tests call it only right after writing fake data: it must never be used to look at real content.
    ///
    /// # Errors
    /// The clipboard cannot be opened.
    pub fn bytes(format: Format) -> Result<Option<Vec<u8>>, SystemClipboardError> {
        let id = match format {
            Format::UnicodeText => u32::from(CF_UNICODETEXT.0),
            Format::AnsiText => u32::from(CF_TEXT.0),
            Format::ExcludeFromMonitors => {
                register_clipboard_format(EXCLUDE_FROM_MONITORS).map_err(os_error)?
            }
            Format::CanIncludeInHistory => {
                register_clipboard_format(CAN_INCLUDE_IN_HISTORY).map_err(os_error)?
            }
            Format::CanUploadToCloud => {
                register_clipboard_format(CAN_UPLOAD_TO_CLOUD).map_err(os_error)?
            }
        };
        open_clipboard(None).map_err(os_error)?;
        // SAFETY: the clipboard is open by this thread, so the handle `GetClipboardData` returns stays valid
        // until `close_clipboard` below. The block is only read, while locked, and for exactly the size the
        // system reports for it.
        let bytes = unsafe {
            GetClipboardData(id).ok().and_then(|handle| {
                let block = HGLOBAL(handle.0);
                let start = GlobalLock(block).cast::<u8>();
                if start.is_null() {
                    return None;
                }
                let bytes = std::slice::from_raw_parts(start, GlobalSize(block)).to_vec();
                let _ = GlobalUnlock(block);
                Some(bytes)
            })
        };
        close_clipboard();
        Ok(bytes)
    }

    /// Puts the calling thread in a single-threaded COM apartment: the WinRT clipboard history API the tests
    /// use for RF-72 fails from a multi-threaded one (lab P1).
    ///
    /// # Errors
    /// COM refused, for example because the thread already is in a multi-threaded apartment.
    pub fn enter_single_threaded_apartment() -> Result<(), SystemClipboardError> {
        // SAFETY: no preconditions. The apartment lasts until the thread ends; test threads are not reused.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(os_error)
    }
}
