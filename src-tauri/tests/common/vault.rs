//! Helpers for the tests that need an open vault (T13, T14): the app over a fake clipboard, clocks the test moves
//! by hand, a counter of webview reloads, the events the window would hear, and a vault with a canary in every
//! protected place.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    time::{Duration, Instant, SystemTime},
};

use almarioks_core::kdbx::{MasterPassword, NewVaultKdf, VaultKey, create_vault};
use almarioks_lib::{
    clipboard::{ClipboardService, Clock, SequenceNumber, SystemClipboard, SystemClipboardError},
    lock::{IdleClock, Moment, VAULT_LOCKED_EVENT, VAULT_SAVE_STATE_EVENT},
    state::{AppParts, AppState},
};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use tauri::{App, Listener, Manager, WebviewWindow, test::MockRuntime};

use super::{
    Answer, CountingReloader, FakeDialogs, FakeOpener, TempDir, app_with, fast_kdf, invoke_args,
    invoke_plain, main_window,
};

/// How long a test waits for another thread before failing instead of hanging.
pub const PATIENCE: Duration = Duration::from_secs(30);

/// Fake data, never a real secret. It is in the master password and in every protected value of the canary
/// vault, and nowhere else: no title, user name, group or path contains it.
pub const CANARY: &str = "CANARIO-ALMARIOKS-rf76";

/// The master password of the canary vault.
pub fn master() -> String {
    format!("{CANARY}-maestra")
}

// ---------- The clipboard ----------

#[derive(Default)]
struct ClipboardState {
    text: Option<String>,
    sequence: u32,
    /// When armed, the next `set_secret` reports it is inside and waits to be released.
    hold: Option<(Sender<()>, Receiver<()>)>,
}

/// A clipboard in memory with the semantics of the real one: every write moves the sequence number.
#[derive(Clone, Default)]
pub struct FakeClipboard(Arc<Mutex<ClipboardState>>);

/// The test's side of a held `set_secret`.
pub struct Hold {
    pub inside: Receiver<()>,
    pub release: Sender<()>,
}

impl FakeClipboard {
    pub fn text(&self) -> Option<String> {
        self.0.lock().unwrap().text.clone()
    }

    /// Another app writes to the clipboard.
    pub fn other_app_copies(&self, text: &str) {
        let mut state = self.0.lock().unwrap();
        state.text = Some(text.to_owned());
        state.sequence += 1;
    }

    pub fn hold_next_set(&self) -> Hold {
        let (inside_tx, inside) = channel();
        let (release, release_rx) = channel();
        self.0.lock().unwrap().hold = Some((inside_tx, release_rx));
        Hold { inside, release }
    }
}

impl SystemClipboard for FakeClipboard {
    fn set_secret(&self, secret: &SecretString) -> Result<SequenceNumber, SystemClipboardError> {
        let (sequence, hold) = {
            let mut state = self.0.lock().unwrap();
            state.text = Some(secret.expose_secret().to_owned());
            state.sequence += 8;
            (state.sequence, state.hold.take())
        };
        if let Some((inside, release)) = hold {
            inside.send(()).unwrap();
            release.recv_timeout(PATIENCE).unwrap();
        }
        Ok(SequenceNumber(sequence))
    }

    fn sequence_number(&self) -> SequenceNumber {
        SequenceNumber(self.0.lock().unwrap().sequence)
    }

    fn clear_if_sequence_is(&self, expected: SequenceNumber) -> Result<bool, SystemClipboardError> {
        let mut state = self.0.lock().unwrap();
        if state.sequence != expected.0 {
            return Ok(false);
        }
        state.text = None;
        state.sequence += 1;
        Ok(true)
    }
}

// ---------- The clocks ----------

/// The clipboard's clock: it only moves when the test says so.
#[derive(Clone)]
pub struct FakeTime(Arc<Mutex<Instant>>);

impl FakeTime {
    pub fn advance(&self, seconds: u64) {
        *self.0.lock().unwrap() += Duration::from_secs(seconds);
    }
}

impl Clock for FakeTime {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}

/// The two clocks of the inactivity timer, moved by hand.
#[derive(Clone)]
pub struct FakeIdleClock(Arc<Mutex<Moment>>);

impl Default for FakeIdleClock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Moment {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
        })))
    }
}

impl FakeIdleClock {
    pub fn moment(&self) -> Moment {
        *self.0.lock().unwrap()
    }

    /// Time passes with the machine awake: both clocks move.
    pub fn advance(&self, seconds: u64) {
        let mut now = self.0.lock().unwrap();
        now.monotonic += Duration::from_secs(seconds);
        now.wall += Duration::from_secs(seconds);
    }

    /// The machine is suspended: the time of day moves, and the monotonic clock is taken not to (the worst case
    /// for the timer; what the real one does depends on the system).
    pub fn suspend(&self, seconds: u64) {
        self.0.lock().unwrap().wall += Duration::from_secs(seconds);
    }

    /// Somebody sets the time of day back.
    pub fn set_wall_back(&self, seconds: u64) {
        self.0.lock().unwrap().wall -= Duration::from_secs(seconds);
    }
}

impl IdleClock for FakeIdleClock {
    fn now(&self) -> Moment {
        self.moment()
    }
}

// ---------- The events the window would hear ----------

/// Every `vault-locked` and `vault-save-state` emitted, as `(event, serialized payload)`.
pub struct Events {
    seen: Arc<Mutex<Vec<(String, String)>>>,
    inbox: Receiver<(String, String)>,
}

impl Events {
    fn listen(app: &App<MockRuntime>) -> Self {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (sender, inbox) = channel();
        for name in [VAULT_LOCKED_EVENT, VAULT_SAVE_STATE_EVENT] {
            let (seen, sender) = (Arc::clone(&seen), sender.clone());
            app.listen_any(name, move |event| {
                let heard = (name.to_owned(), event.payload().to_owned());
                seen.lock().unwrap().push(heard.clone());
                // The test may not be waiting for anything.
                _ = sender.send(heard);
            });
        }
        Self { seen, inbox }
    }

    /// Everything heard so far.
    pub fn all(&self) -> Vec<(String, String)> {
        self.seen.lock().unwrap().clone()
    }

    /// How many times `event` was heard so far.
    pub fn count(&self, event: &str) -> usize {
        self.all().iter().filter(|(name, _)| name == event).count()
    }

    /// Waits (without sleeping) until `event` arrives with exactly this serialized payload.
    pub fn wait_for(&self, event: &str, payload: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let (name, body) = self
                .inbox
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("`{event}` with {payload} never arrived"));
            if name == event && body == payload {
                return;
            }
        }
    }
}

// ---------- The app ----------

/// The app of a test, with a handle on every fake it runs on.
pub struct Rig {
    pub dir: TempDir,
    pub app: App<MockRuntime>,
    pub clipboard: FakeClipboard,
    pub clipboard_clock: FakeTime,
    pub idle_clock: FakeIdleClock,
    pub reloader: CountingReloader,
    pub opener: FakeOpener,
    pub events: Events,
}

impl Rig {
    pub fn new(tag: &str) -> Self {
        Self::with_kdf(tag, fast_kdf)
    }

    /// An app whose new vaults get this KDF.
    pub fn with_kdf(tag: &str, kdf: fn() -> almarioks_core::CoreResult<NewVaultKdf>) -> Self {
        let dir = TempDir::new(tag);
        let clipboard = FakeClipboard::default();
        let clipboard_clock = FakeTime(Arc::new(Mutex::new(Instant::now())));
        let idle_clock = FakeIdleClock::default();
        let reloader = CountingReloader::default();
        let opener = FakeOpener::default();
        let app = app_with(AppParts {
            settings_file: dir.settings_file(),
            clipboard: Ok(ClipboardService::new(
                clipboard.clone(),
                clipboard_clock.clone(),
            )),
            dialogs: Arc::new(FakeDialogs::default()),
            url_opener: Arc::new(opener.clone()),
            idle_clock: Arc::new(idle_clock.clone()),
            reloader: Arc::new(reloader.clone()),
            new_vault_kdf: Arc::new(kdf),
        });
        let events = Events::listen(&app);
        Self {
            dir,
            app,
            clipboard,
            clipboard_clock,
            idle_clock,
            reloader,
            opener,
            events,
        }
    }

    pub fn window(&self) -> WebviewWindow<MockRuntime> {
        main_window(&self.app)
    }

    /// A path inside the test's folder, as the text the window would send.
    pub fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_str().unwrap().to_owned()
    }

    /// One heartbeat of the app, by hand (production runs it every second).
    pub fn heartbeat(&self) -> bool {
        self.app.state::<AppState>().lock_center().evaluate()
    }

    pub fn args(&self, command: &str, args: Value) -> Answer {
        invoke_args(&self.app, command, args)
    }

    pub fn plain(&self, command: &str) -> Answer {
        invoke_plain(&self.app, command)
    }

    /// `state` of `vault_status`.
    pub fn state(&self) -> String {
        let status = self.plain("vault_status").expect("vault_status answers");
        status["state"].as_str().expect("a state").to_owned()
    }

    /// Writes the canary vault into the test's folder and returns its path.
    pub fn write_canary_vault(&self) -> String {
        let path = self.path("canario.kdbx");
        std::fs::write(&path, canary_vault_bytes()).unwrap();
        path
    }

    /// Writes the canary vault into the test's folder and opens it through the IPC.
    pub fn open_canary_vault(&self) -> CanaryVault {
        let path = self.write_canary_vault();
        let opened = self
            .args("vault_open", json!({ "path": path, "password": master() }))
            .expect("the canary vault opens");
        assert_eq!(opened["state"], "open");
        self.discover(path)
    }

    /// Finds the groups and entries of the canary vault, which must be open, by their names.
    pub fn discover(&self, path: String) -> CanaryVault {
        let tree = self.plain("group_tree").expect("group_tree answers");
        let root = id_of(&tree);
        let work = id_of(
            tree["children"]
                .as_array()
                .unwrap()
                .iter()
                .find(|group| group["name"] == "Trabajo")
                .expect("the group Trabajo"),
        );
        let entries = self
            .args("entry_list", json!({ "group_id": work }))
            .expect("entry_list answers");
        let entries = entries.as_array().unwrap();
        let bank = id_of(
            entries
                .iter()
                .find(|entry| entry["title"] == "Banco")
                .expect("the entry Banco"),
        );
        let all_protected = id_of(
            entries
                .iter()
                .find(|entry| entry["title"].is_null())
                .expect("the entry with a protected title"),
        );
        CanaryVault {
            path,
            root,
            work,
            bank,
            all_protected,
        }
    }
}

fn id_of(value: &Value) -> String {
    value["id"].as_str().expect("an id").to_owned()
}

/// A UUID no vault of the tests has (RF-93).
pub const MISSING_ID: &str = "00000000-0000-4000-8000-00000000dead";

/// What the tests know of the canary vault.
pub struct CanaryVault {
    pub path: String,
    /// The root group, "Canario".
    pub root: String,
    /// The group "Trabajo", with the two entries.
    pub work: String,
    /// "Banco": user `ana`, URL `https://banco.example/login`, password, notes and the custom field `PIN`
    /// protected (all three with the canary), and the plain custom field `Cliente`.
    pub bank: String,
    /// An entry whose five standard fields are protected and hold the canary.
    pub all_protected: String,
}

/// A real KDBX 4 vault with the canary in the master password, in passwords, in protected notes, in a protected
/// custom field and in protected title, user name and URL. The app cannot make protected notes or custom fields
/// (spec, "Fuera de alcance"), so it is filled with the `keepass` crate, as a file that came from KeePass would be.
pub fn canary_vault_bytes() -> Vec<u8> {
    let key = VaultKey::new(Some(MasterPassword::new(master())), None).unwrap();
    let empty = create_vault(&key, "Canario", fast_kdf().unwrap()).unwrap();
    let library_key = || keepass::DatabaseKey::new().with_password(&master());
    let mut db = keepass::Database::open(&mut empty.as_slice(), library_key()).unwrap();
    let mut root = db.root_mut();
    let work = root
        .add_group()
        .edit(|group| group.name = "Trabajo".into())
        .id();
    let mut group = db.group_mut(work).unwrap();
    group.add_entry().edit(|entry| {
        entry.set_unprotected("Title", "Banco");
        entry.set_unprotected("UserName", "ana");
        entry.set_protected("Password", format!("{CANARY}-password"));
        entry.set_unprotected("URL", "https://banco.example/login");
        entry.set_protected("Notes", format!("{CANARY}-notes"));
        entry.set_protected("PIN", format!("{CANARY}-pin"));
        entry.set_unprotected("Cliente", "12345");
        entry.tags = vec!["finanzas".into()];
    });
    group.add_entry().edit(|entry| {
        for name in ["Title", "UserName", "Password", "URL", "Notes"] {
            entry.set_protected(name, format!("{CANARY}-{name}"));
        }
    });
    let mut bytes = Vec::new();
    db.save(&mut bytes, library_key()).unwrap();
    bytes
}

/// A copy of a fixture of the core inside the test's folder; returns its path as text.
pub fn copy_core_fixture(rig: &Rig, relative: &str, name: &str) -> String {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crates/almarioks-core/tests/fixtures")
        .join(relative);
    let target = rig.path(name);
    std::fs::copy(&source, Path::new(&target)).unwrap();
    target
}
