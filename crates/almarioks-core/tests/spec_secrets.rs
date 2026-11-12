//! Independent validation from the spec: secrets never leave the core except by "show one field" (RF-43, RF-44,
//! RF-61, RF-76; "Campo protegido"). The vault with protected title, user name, URL, notes and custom fields is
//! made with `keepass`, as another KeePass app would write it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::{collections::BTreeSet, fs};

use almarioks_core::{
    CoreError,
    kdbx::{self, KeyFile, MasterPassword, OpenedVault, VaultKey},
    search::SearchResult,
    vault::{EntryId, EntryPatch, NewEntry},
};
use keepass::{Database, DatabaseKey};
use secrecy::{ExposeSecret, SecretString};
use spec_support::{PW, Rig, at, kdf, key, status_open};

const C_PW: &str = "CANARIO-PW-7f3a";
const C_NOTES: &str = "CANARIO-NOTAS-91bd palabraunicasecreta";
const C_PIN: &str = "CANARIO-PIN-55e0";
const C_TITLE: &str = "CANARIO-TITULO-2c4e";
const C_USER: &str = "CANARIO-USUARIO-0aa1";
const C_URL: &str = "https://canario-url-77b2.example.com";
const C_PW2: &str = "CANARIO-PW2-e1e1";
const C_CORE: &str = "CANARIO-CORE-d00d";
const ALL: [&str; 8] = [C_PW, C_NOTES, C_PIN, C_TITLE, C_USER, C_URL, C_PW2, C_CORE];

fn assert_clean(what: &str, text: &str) {
    for canary in ALL {
        assert!(
            !text.contains(canary),
            "{what} carries the protected value {canary:?}: {text}"
        );
    }
    assert!(
        !text.contains("palabraunicasecreta"),
        "{what} carries a word of protected notes"
    );
    assert!(!text.contains(PW), "{what} carries the master password");
}

/// Bytes of a vault with: "Correo canario" (protected password, notes and custom field PIN; visible custom field),
/// an entry whose title, user name and URL are protected, and an entry made by the core.
fn canary_vault_bytes() -> Vec<u8> {
    let made_here = kdbx::create_vault(&key(PW), "Secretos", kdf()).expect("create");
    let mut db = Database::parse(&made_here, DatabaseKey::new().with_password(PW)).expect("parse");
    {
        let mut root = db.root_mut();
        let mut first = root.add_entry();
        first.set_unprotected("Title", "Correo canario");
        first.set_unprotected("UserName", "usuario-visible");
        first.set_protected("Password", C_PW);
        first.set_protected("Notes", C_NOTES);
        first.set_protected("PIN", C_PIN);
        first.set_unprotected("Visible", "valor-visible");
    }
    {
        let mut root = db.root_mut();
        let mut second = root.add_entry();
        second.set_protected("Title", C_TITLE);
        second.set_protected("UserName", C_USER);
        second.set_protected("URL", C_URL);
        second.set_protected("Password", C_PW2);
        second.set_unprotected("Notes", "notas visibles de la segunda");
    }
    let mut out = Vec::new();
    db.save(&mut out, DatabaseKey::new().with_password(PW))
        .expect("save");
    let mut vault = kdbx::open(&out, &key(PW)).expect("core opens it");
    let root = vault.root_id();
    let entry = NewEntry {
        title: "Hecha aquí".to_owned(),
        password: Some(SecretString::from(C_CORE.to_owned())),
        ..NewEntry::default()
    };
    vault
        .create_entry(root, entry, at(1_794_391_200))
        .expect("core entry");
    vault.to_bytes(&key(PW)).expect("bytes")
}

fn ids_by_title(vault: &OpenedVault) -> (EntryId, EntryId, EntryId) {
    let list = vault.list_entries(vault.root_id()).expect("list");
    let find = |title: Option<&str>| {
        list.iter()
            .find(|e| e.title.as_deref() == title)
            .map(|e| e.id)
            .expect("entry")
    };
    (
        find(Some("Correo canario")),
        find(None),
        find(Some("Hecha aquí")),
    )
}

#[test]
fn spec_rf_43_rf_76_tree_lists_detail_and_debug_carry_no_protected_value() {
    let vault = kdbx::open(&canary_vault_bytes(), &key(PW)).expect("open");
    let (first, second, core) = ids_by_title(&vault);
    assert_clean("group_tree", &format!("{:?}", vault.group_tree()));
    assert_clean(
        "list_entries",
        &format!("{:?}", vault.list_entries(vault.root_id())),
    );
    assert_clean("OpenedVault Debug", &format!("{vault:?}"));
    for id in [first, second, core] {
        assert_clean("entry detail", &format!("{:?}", vault.entry(id)));
    }
}

#[test]
fn spec_rf_43_detail_names_custom_fields_and_says_which_protected_fields_have_a_value() {
    let vault = kdbx::open(&canary_vault_bytes(), &key(PW)).expect("open");
    let (first, second, _) = ids_by_title(&vault);
    let detail = vault.entry(first).expect("detail");
    assert_eq!(
        detail.notes, None,
        "protected notes do not travel (casos límite)"
    );
    let protected: BTreeSet<&str> = detail.protected_fields.iter().map(String::as_str).collect();
    assert_eq!(protected, BTreeSet::from(["Password", "Notes", "PIN"]));
    let pin = detail
        .custom_fields
        .iter()
        .find(|f| f.name == "PIN")
        .expect("PIN is named");
    assert!(pin.protected);
    assert_eq!(pin.value, None);
    let visible = detail
        .custom_fields
        .iter()
        .find(|f| f.name == "Visible")
        .expect("Visible is named");
    assert!(!visible.protected);
    assert_eq!(
        visible.value.as_deref(),
        Some("valor-visible"),
        "non-protected fields are shown"
    );
    assert_eq!(detail.summary.username.as_deref(), Some("usuario-visible"));

    let detail = vault.entry(second).expect("detail");
    assert_eq!(detail.summary.title, None);
    assert_eq!(detail.summary.username, None);
    assert_eq!(detail.summary.url, None);
    assert_eq!(
        detail.notes.as_deref(),
        Some("notas visibles de la segunda")
    );
    let protected: BTreeSet<&str> = detail.protected_fields.iter().map(String::as_str).collect();
    assert_eq!(
        protected,
        BTreeSet::from(["Title", "UserName", "URL", "Password"])
    );
}

#[test]
fn spec_rf_44_showing_a_field_gives_only_that_value_of_that_entry() {
    let vault = kdbx::open(&canary_vault_bytes(), &key(PW)).expect("open");
    let (first, second, core) = ids_by_title(&vault);
    let shown = |id, field: &str| {
        vault
            .reveal_field(id, field)
            .map(|s| s.expose_secret().to_owned())
    };
    assert_eq!(shown(first, "Password").as_deref(), Ok(C_PW));
    assert_eq!(shown(first, "PIN").as_deref(), Ok(C_PIN));
    assert_eq!(shown(first, "Notes").as_deref(), Ok(C_NOTES));
    assert_eq!(shown(second, "Title").as_deref(), Ok(C_TITLE));
    assert_eq!(shown(second, "Password").as_deref(), Ok(C_PW2));
    assert_eq!(shown(core, "Password").as_deref(), Ok(C_CORE));
    assert_eq!(
        shown(first, "NoExiste"),
        Err(CoreError::NoEncontrado),
        "RF-93"
    );
}

#[test]
fn spec_rf_61_search_never_looks_inside_a_protected_field_and_hits_carry_no_secret() {
    let vault = kdbx::open(&canary_vault_bytes(), &key(PW)).expect("open");
    for query in [
        C_PW,
        "palabraunicasecreta",
        "canario-pin",
        C_TITLE,
        "canario-usuario",
        "canario-url",
        C_CORE,
    ] {
        assert_eq!(
            vault.search(query),
            SearchResult::Matches(Vec::new()),
            "RF-61: {query:?} matched"
        );
    }
    for query in ["correo", "visible", "canario", "segunda", "hecha"] {
        let result = vault.search(query);
        assert!(
            matches!(&result, SearchResult::Matches(h) if !h.is_empty()),
            "{query:?}: {result:?}"
        );
        assert_clean("search hit", &format!("{result:?}"));
    }
}

#[test]
fn spec_rf_76_debug_of_keys_inputs_and_session_never_prints_a_secret() {
    let master = MasterPassword::new(PW.to_owned());
    assert_clean("MasterPassword", &format!("{master:?}"));
    let key_file = KeyFile::from_bytes(C_PIN.as_bytes().to_vec()).expect("any file is a key file");
    assert_clean("KeyFile", &format!("{key_file:?}"));
    let vault_key = VaultKey::new(Some(master), Some(key_file)).expect("key");
    assert_clean("VaultKey", &format!("{vault_key:?}"));
    let entry = NewEntry {
        title: "x".to_owned(),
        password: Some(SecretString::from(C_PW.to_owned())),
        ..NewEntry::default()
    };
    assert_clean("NewEntry", &format!("{entry:?}"));
    let patch = EntryPatch {
        password: Some(SecretString::from(C_PW2.to_owned())),
        ..EntryPatch::default()
    };
    assert_clean("EntryPatch", &format!("{patch:?}"));

    let rig = Rig::closed("rf76-session");
    fs::write(&rig.path, canary_vault_bytes()).expect("write");
    let status = rig.unlock().expect("unlock");
    assert_clean("Status", &format!("{status:?}"));
    assert_clean("Session", &format!("{:?}", rig.session));
    let open = status_open(status);
    assert_clean("OpenStatus", &format!("{open:?}"));
    let tree = rig
        .session
        .read(|v| Ok(format!("{:?}{:?}", v.group_tree(), v.search("canario"))));
    assert_clean("read through the session", &tree.expect("read"));
}
