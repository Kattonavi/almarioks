//! Search through the public API (RF-59 to RF-63, RF-92, example 2 of the spec). Expected values come from the
//! spec; the time over 5 000 entries is measured and printed (the 100 ms target is not pass/fail; 1 s is).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use almarioks_core::{
    kdbx::{MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open},
    search::{SearchHit, SearchResult},
    vault::{EntryId, GroupId, NewEntry, Timestamp},
};
use secrecy::SecretString;

const PASSWORD: &str = "Maestra-busqueda-2026";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn vault() -> OpenedVault {
    let bytes = create_vault(&key(), "Raíz", NewVaultKdf::with_iterations(2).unwrap()).unwrap();
    open(&bytes, &key()).unwrap()
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200 + seconds).unwrap()
}

fn add(vault: &mut OpenedVault, group: GroupId, entry: NewEntry) -> EntryId {
    vault.create_entry(group, entry, at(0)).unwrap()
}

fn titled(title: &str) -> NewEntry {
    NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    }
}

fn hits(vault: &OpenedVault, query: &str) -> Vec<SearchHit> {
    match vault.search(query) {
        SearchResult::Matches(hits) => hits,
        SearchResult::NoFilter => panic!("{query:?} should filter"),
    }
}

fn titles(vault: &OpenedVault, query: &str) -> Vec<String> {
    hits(vault, query)
        .into_iter()
        .map(|hit| hit.summary.title.unwrap_or_default())
        .collect()
}

/// Example 2 of the spec, plus the two entries it names ("Años", "Banana").
fn example_2() -> OpenedVault {
    let mut vault = vault();
    let root = vault.root_id();
    add(
        &mut vault,
        root,
        NewEntry {
            username: "ana".into(),
            ..titled("Canción favorita")
        },
    );
    add(
        &mut vault,
        root,
        NewEntry {
            notes: "cancion de cuna".into(),
            ..titled("Banco")
        },
    );
    add(
        &mut vault,
        root,
        NewEntry {
            username: "ana@example.com".into(),
            ..titled("GitHub")
        },
    );
    add(&mut vault, root, titled("Años"));
    add(&mut vault, root, titled("Banana"));
    vault
}

#[test]
fn rf_59_rf_60_rf_63_example_2() {
    let vault = example_2();
    assert_eq!(titles(&vault, "CANCION"), ["Canción favorita", "Banco"]);
    assert_eq!(titles(&vault, "ana git"), ["GitHub"]);
    assert_eq!(titles(&vault, "  banco  "), ["Banco"]);
    assert_eq!(titles(&vault, "anos"), ["Años"]);
    // "ana" is part of "Banana" (title block first), and of two user names (then by title).
    assert_eq!(
        titles(&vault, "ana"),
        ["Banana", "Canción favorita", "GitHub"]
    );
}

#[test]
fn rf_59_every_word_must_match_some_field_and_tags_and_urls_count() {
    let mut vault = vault();
    let root = vault.root_id();
    add(
        &mut vault,
        root,
        NewEntry {
            url: "https://correo.example".into(),
            tags: vec!["Trabajo".into()],
            ..titled("Correo")
        },
    );
    add(&mut vault, root, titled("Trabajo pendiente"));
    assert_eq!(titles(&vault, "example trabajo"), ["Correo"]);
    assert_eq!(titles(&vault, "TRABAJO"), ["Trabajo pendiente", "Correo"]);
    assert!(titles(&vault, "trabajo nada").is_empty());
}

#[test]
fn rf_61_the_password_is_never_searched() {
    let mut vault = vault();
    let root = vault.root_id();
    add(
        &mut vault,
        root,
        NewEntry {
            password: Some(SecretString::from("zorro-azul".to_owned())),
            ..titled("Banco")
        },
    );
    assert!(hits(&vault, "zorro").is_empty());
}

#[test]
fn rf_62_entries_in_the_recycle_bin_are_excluded_at_any_depth() {
    let mut vault = vault();
    let root = vault.root_id();
    let trabajo = vault.create_group(root, "Trabajo", at(0)).unwrap();
    let clientes = vault.create_group(trabajo, "Clientes", at(0)).unwrap();
    add(&mut vault, clientes, titled("Cliente hondo"));
    let suelta = add(&mut vault, root, titled("Cliente suelto"));
    let viva = add(&mut vault, root, titled("Cliente vivo"));
    vault.delete_entry(suelta, at(1)).unwrap();
    // Bin > Trabajo > Clientes > entry: two levels below the bin.
    vault.delete_group(trabajo, at(1)).unwrap();
    let found = hits(&vault, "cliente");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].summary.id, viva);
}

#[test]
fn rf_63_group_path_includes_the_root_and_ties_go_by_uuid() {
    let mut vault = vault();
    let root = vault.root_id();
    let trabajo = vault.create_group(root, "Trabajo", at(0)).unwrap();
    let clientes = vault.create_group(trabajo, "Clientes", at(0)).unwrap();
    let a = add(&mut vault, clientes, titled("Gemelo"));
    let b = add(&mut vault, root, titled("gemelo"));
    let found = hits(&vault, "gemelo");
    let mut expected = [a, b];
    expected.sort();
    assert_eq!(
        found.iter().map(|h| h.summary.id).collect::<Vec<_>>(),
        expected
    );
    let path_of = |id: EntryId| {
        found
            .iter()
            .find(|h| h.summary.id == id)
            .unwrap()
            .group_path
            .clone()
    };
    assert_eq!(path_of(a), ["Raíz", "Trabajo", "Clientes"]);
    assert_eq!(path_of(b), ["Raíz"]);
}

#[test]
fn rf_92_an_empty_query_does_not_filter_and_rf_64_no_results_is_an_empty_list() {
    let vault = example_2();
    assert_eq!(vault.search(""), SearchResult::NoFilter);
    assert_eq!(vault.search(" \t\u{3000} "), SearchResult::NoFilter);
    assert_eq!(vault.search("xyz"), SearchResult::Matches(Vec::new()));
}

#[test]
fn rf_59_search_over_5000_entries_is_measured() {
    let mut vault = vault();
    let root = vault.root_id();
    let groups: Vec<GroupId> = (0..10)
        .map(|g| {
            vault
                .create_group(root, &format!("Grupo {g}"), at(0))
                .unwrap()
        })
        .collect();
    for i in 0..5_000_usize {
        let entry = NewEntry {
            username: format!("usuario{i}@example.com"),
            url: format!("https://sitio{i}.example"),
            notes: format!("Notas de la cuenta número {i}, con acentos: canción, año."),
            tags: vec![format!("etiqueta{}", i % 7)],
            ..titled(&format!("Cuenta {i} Señal"))
        };
        add(&mut vault, groups[i % groups.len()], entry);
    }
    let start = Instant::now();
    let found = hits(&vault, "cuenta senal 49");
    let elapsed = start.elapsed();
    println!(
        "RF-59 search over 5000 entries: {} hits in {:.1} ms (target 100 ms, measured, not pass/fail)",
        found.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    // 49, 490..499, 1490..1499, ... 4900..4999 and so on: every hit has "49" and the title words.
    assert!(!found.is_empty());
    assert!(
        found
            .iter()
            .all(|h| h.summary.title.as_deref().unwrap().contains("49"))
    );
    assert!(elapsed.as_secs() < 1, "search took {elapsed:?}");
}
