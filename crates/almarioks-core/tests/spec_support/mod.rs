//! Helpers for the `spec_*` tests: independent validation written from `docs/specs/001-boveda/spec.md`, using only
//! the public API of `almarioks-core` (plus `keepass` to play "another KeePass app").
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use almarioks_core::{
    CoreResult,
    kdbx::{self, MasterPassword, NewKeyFile, NewVaultKdf, OpenedVault, VaultKey},
    session::{Applied, Clock, OpenStatus, Ring, SaveState, Session, SessionEvents, Status},
    storage::{Disk, StdDisk},
    vault::{EntryId, GroupId, NewEntry, Timestamp},
};

/// Master password of every test vault: non-ASCII on purpose (spec "Casos límite").
pub const PW: &str = "maestra-de-prueba-ñ€";
/// 2026-11-11T10:00:00Z.
pub const T0: i64 = 1_794_391_200;

pub fn password(text: &str) -> Option<MasterPassword> {
    Some(MasterPassword::new(text.to_owned()))
}

pub fn key(text: &str) -> VaultKey {
    VaultKey::new(password(text), None).expect("key")
}

pub fn kdf() -> NewVaultKdf {
    NewVaultKdf::with_iterations(2).expect("kdf")
}

pub fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(seconds).expect("timestamp")
}

pub fn hold<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A folder under the system temp folder, removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "almarioks-spec-{tag}-{}-{nanos}-{n}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn create_vault_at(path: &Path, name: &str) -> OpenedVault {
    kdbx::create_named_vault_file(path, name, password(PW), NewKeyFile::None, kdf())
        .expect("create vault")
}

pub fn open_path(path: &Path) -> CoreResult<OpenedVault> {
    kdbx::open(&fs::read(path).expect("read vault file"), &key(PW))
}

pub fn count(path: &Path) -> usize {
    open_path(path).expect("open vault file").entry_count()
}

pub fn new_entry(title: &str) -> NewEntry {
    NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    }
}

pub fn titles_in(vault: &OpenedVault, group: GroupId) -> Vec<String> {
    vault
        .list_entries(group)
        .expect("list")
        .into_iter()
        .map(|e| e.title.unwrap_or_default())
        .collect()
}

pub fn root_titles(path: &Path) -> Vec<String> {
    let vault = open_path(path).expect("open");
    titles_in(&vault, vault.root_id())
}

pub fn recovery_files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(".recuperacion-"))
        })
        .collect();
    found.sort();
    found
}

/// `<file>.recuperacion-AAAAMMDD-HHMMSS.kdbx`, as RF-33 names it.
pub fn is_recovery_name(path: &Path, vault_file: &str) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let prefix = format!("{vault_file}.recuperacion-");
    let Some(rest) = name.strip_prefix(&prefix) else {
        return false;
    };
    let Some(stamp) = rest.strip_suffix(".kdbx") else {
        return false;
    };
    let chars: Vec<char> = stamp.chars().collect();
    chars.len() == 15
        && chars.iter().enumerate().all(|(i, c)| {
            if i == 8 {
                *c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

pub fn status_open(status: Status) -> OpenStatus {
    match status {
        Status::Open(open) => open,
        other => panic!("expected an open vault, got {other:?}"),
    }
}

// ------------------------------------------------------------------------------------------------------------
// A disk that fails, hangs or corrupts where the test decides (spec "Cómo se prueba": 23, 27, 30, 33).
// ------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Read,
    Exists,
    Write,
    WriteNew,
    Copy,
    ClearReadOnly,
    Replace,
    Remove,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Act {
    #[default]
    Pass,
    /// An I/O error.
    Fail,
    /// Blocks until [`FaultDisk::release`], then does the operation.
    Hang,
    /// Writes only the first half of the bytes and says it worked; reads come back with the last byte flipped.
    Corrupt,
}

#[derive(Default)]
struct Rules {
    /// Every operation that writes to a path that is not a recovery file.
    main: Act,
    /// Every operation that writes to a recovery file.
    recovery: Act,
    /// The next N reads (not of a recovery file) hang.
    hang_reads: u32,
    /// `(k, act, then_fail_all)`: the k-th operation after arming does `act`.
    step: Option<(u64, Act, bool)>,
    calls: u64,
    triggered: bool,
    failing_all: bool,
    released: bool,
    log: Vec<(Op, PathBuf)>,
}

pub struct FaultDisk {
    inner: StdDisk,
    rules: Mutex<Rules>,
    wake: Condvar,
    watch: Mutex<Option<Sender<(Op, PathBuf)>>>,
}

fn is_recovery(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains(".recuperacion-"))
}

fn injected() -> io::Error {
    io::Error::other("fallo inyectado por la prueba")
}

impl FaultDisk {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: StdDisk,
            rules: Mutex::new(Rules::default()),
            wake: Condvar::new(),
            watch: Mutex::new(None),
        })
    }

    pub fn set_main(&self, act: Act) {
        let mut r = hold(&self.rules);
        r.main = act;
        if act == Act::Hang {
            r.released = false;
        }
    }

    pub fn set_recovery(&self, act: Act) {
        let mut r = hold(&self.rules);
        r.recovery = act;
        if act == Act::Hang {
            r.released = false;
        }
    }

    pub fn hang_next_reads(&self, n: u32) {
        let mut r = hold(&self.rules);
        r.hang_reads = n;
        r.released = false;
    }

    /// The k-th operation from now (1-based) does `act`; with `then_fail_all`, every later one fails too.
    pub fn arm_step(&self, k: u64, act: Act, then_fail_all: bool) {
        let mut r = hold(&self.rules);
        r.calls = 0;
        r.step = Some((k, act, then_fail_all));
        r.triggered = false;
        r.failing_all = false;
        r.released = act != Act::Hang;
    }

    pub fn reset(&self) {
        let mut r = hold(&self.rules);
        let log = std::mem::take(&mut r.log);
        *r = Rules {
            released: true,
            log,
            ..Rules::default()
        };
        drop(r);
        self.wake.notify_all();
    }

    pub fn triggered(&self) -> bool {
        hold(&self.rules).triggered
    }

    pub fn log(&self) -> Vec<(Op, PathBuf)> {
        hold(&self.rules).log.clone()
    }

    /// Every hang is announced here before it blocks.
    pub fn watch(&self) -> Receiver<(Op, PathBuf)> {
        let (tx, rx) = mpsc::channel();
        *hold(&self.watch) = Some(tx);
        rx
    }

    pub fn release(&self) {
        hold(&self.rules).released = true;
        self.wake.notify_all();
    }

    fn decide(&self, op: Op, path: &Path) -> Act {
        let mut r = hold(&self.rules);
        r.log.push((op, path.to_path_buf()));
        r.calls += 1;
        if r.failing_all {
            return Act::Fail;
        }
        if let Some((k, act, all)) = r.step
            && r.calls == k
        {
            r.triggered = true;
            r.step = None;
            r.failing_all = all;
            return act;
        }
        match op {
            Op::Read if !is_recovery(path) && r.hang_reads > 0 => {
                r.hang_reads -= 1;
                Act::Hang
            }
            Op::Read | Op::Exists => Act::Pass,
            _ if is_recovery(path) => r.recovery,
            _ => r.main,
        }
    }

    fn hang(&self, op: Op, path: &Path) {
        if let Some(tx) = hold(&self.watch).as_ref() {
            let _ = tx.send((op, path.to_path_buf()));
        }
        let mut r = hold(&self.rules);
        while !r.released {
            r = self.wake.wait(r).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Runs the decision for a step without data; `Corrupt` behaves as `Pass`.
    fn simple(&self, op: Op, path: &Path) -> io::Result<()> {
        match self.decide(op, path) {
            Act::Fail => Err(injected()),
            Act::Hang => {
                self.hang(op, path);
                Ok(())
            }
            Act::Pass | Act::Corrupt => Ok(()),
        }
    }
}

impl Disk for FaultDisk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        match self.decide(Op::Read, path) {
            Act::Fail => Err(injected()),
            Act::Hang => {
                self.hang(Op::Read, path);
                self.inner.read(path)
            }
            Act::Corrupt => {
                let mut bytes = self.inner.read(path)?;
                if let Some(last) = bytes.last_mut() {
                    *last ^= 0xFF;
                }
                Ok(bytes)
            }
            Act::Pass => self.inner.read(path),
        }
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        self.simple(Op::Exists, path)?;
        self.inner.exists(path)
    }

    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        match self.decide(Op::Write, path) {
            Act::Fail => Err(injected()),
            Act::Hang => {
                self.hang(Op::Write, path);
                self.inner.write_synced(path, bytes)
            }
            Act::Corrupt => self
                .inner
                .write_synced(path, bytes.get(..bytes.len() / 2).unwrap_or_default()),
            Act::Pass => self.inner.write_synced(path, bytes),
        }
    }

    fn write_new_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        match self.decide(Op::WriteNew, path) {
            Act::Fail => Err(injected()),
            Act::Hang => {
                self.hang(Op::WriteNew, path);
                self.inner.write_new_synced(path, bytes)
            }
            Act::Corrupt => self
                .inner
                .write_new_synced(path, bytes.get(..bytes.len() / 2).unwrap_or_default()),
            Act::Pass => self.inner.write_new_synced(path, bytes),
        }
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.simple(Op::Copy, to)?;
        self.inner.copy(from, to)
    }

    fn clear_read_only(&self, path: &Path) -> io::Result<()> {
        self.simple(Op::ClearReadOnly, path)?;
        self.inner.clear_read_only(path)
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.simple(Op::Replace, to)?;
        self.inner.replace(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.simple(Op::Remove, path)?;
        self.inner.remove(path)
    }
}

// ------------------------------------------------------------------------------------------------------------
// A clock that never moves on its own and keeps its alarms until the test rings them (spec: 29, 34, 73).
// ------------------------------------------------------------------------------------------------------------

pub struct ManualClock {
    now: Mutex<SystemTime>,
    alarms: Mutex<Vec<Ring>>,
    asked: Mutex<Vec<Duration>>,
}

impl ManualClock {
    pub fn at(unix_seconds: u64) -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(UNIX_EPOCH + Duration::from_secs(unix_seconds)),
            alarms: Mutex::new(Vec::new()),
            asked: Mutex::new(Vec::new()),
        })
    }

    /// Rings every pending alarm, each on its own thread. Returns how many.
    pub fn ring_all(&self) -> usize {
        let rings = std::mem::take(&mut *hold(&self.alarms));
        let n = rings.len();
        for ring in rings {
            thread::spawn(ring);
        }
        n
    }

    pub fn asked(&self) -> Vec<Duration> {
        hold(&self.asked).clone()
    }
}

impl Clock for ManualClock {
    fn now(&self) -> SystemTime {
        *hold(&self.now)
    }

    fn alarm(&self, after: Duration, ring: Ring) {
        hold(&self.asked).push(after);
        hold(&self.alarms).push(ring);
    }
}

// ------------------------------------------------------------------------------------------------------------
// Listener that turns the session's events into channels.
// ------------------------------------------------------------------------------------------------------------

pub struct Recorder {
    locked: Mutex<Sender<()>>,
    states: Mutex<Sender<SaveState>>,
    pub locks: AtomicUsize,
}

impl SessionEvents for Recorder {
    fn locked(&self) {
        self.locks.fetch_add(1, Ordering::SeqCst);
        let _ = hold(&self.locked).send(());
    }

    fn save_state(&self, state: SaveState) {
        let _ = hold(&self.states).send(state);
    }
}

/// A vault file `cuentas.kdbx` in a temp folder and a session over it with the fault disk and the manual clock.
pub struct Rig {
    pub path: PathBuf,
    pub disk: Arc<FaultDisk>,
    pub clock: Arc<ManualClock>,
    pub events: Arc<Recorder>,
    pub session: Arc<Session>,
    pub locked_rx: Receiver<()>,
    pub states_rx: Receiver<SaveState>,
    pub dir: TempDir,
}

pub const WAIT: Duration = Duration::from_secs(30);

impl Rig {
    /// The vault exists; the session is closed.
    pub fn closed(tag: &str) -> Self {
        let dir = TempDir::new(tag);
        let path = dir.join("cuentas.kdbx");
        drop(create_vault_at(&path, "Cuentas"));
        let disk = FaultDisk::new();
        let clock = ManualClock::at(1_794_391_200);
        let (ltx, locked_rx) = mpsc::channel();
        let (stx, states_rx) = mpsc::channel();
        let events = Arc::new(Recorder {
            locked: Mutex::new(ltx),
            states: Mutex::new(stx),
            locks: AtomicUsize::new(0),
        });
        let session = Arc::new(Session::new(disk.clone(), clock.clone(), events.clone()));
        Self {
            path,
            disk,
            clock,
            events,
            session,
            locked_rx,
            states_rx,
            dir,
        }
    }

    pub fn open(tag: &str) -> Self {
        let rig = Self::closed(tag);
        rig.unlock().expect("unlock");
        rig
    }

    pub fn unlock(&self) -> CoreResult<Status> {
        self.session.unlock(&self.path, password(PW), None)
    }

    pub fn add(&self, title: &str) -> CoreResult<EntryId> {
        self.session.mutate(|vault, now| {
            let root = vault.root_id();
            vault
                .create_entry(root, new_entry(title), now)
                .map(Applied::Changed)
        })
    }

    /// Waits for the first save state that is not `Saving`.
    pub fn settle(&self) -> SaveState {
        loop {
            let state = self
                .states_rx
                .recv_timeout(WAIT)
                .expect("a save state within the wait");
            if state != SaveState::Saving {
                return state;
            }
        }
    }

    pub fn drain(&self) {
        while self.states_rx.try_recv().is_ok() {}
    }

    pub fn titles(&self) -> CoreResult<Vec<String>> {
        self.session
            .read(|vault| Ok(titles_in(vault, vault.root_id())))
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("name")
            .to_owned()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.disk.reset();
    }
}
