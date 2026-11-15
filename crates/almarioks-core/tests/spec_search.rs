//! Independent validation from the spec: search (RF-59 to RF-63, RF-92; example 2).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use almarioks_core::{
    kdbx::OpenedVault,
    search::SearchResult,
    vault::{EntryId, NewEntry},
};
use spec_support::{T0, TempDir, at, create_vault_at};

fn put(v: &mut OpenedVault, title: &str, username: &str, notes: &str) -> EntryId {
    let root = v.root_id();
    let entry = NewEntry {
        title: title.to_owned(),
        username: username.to_owned(),
        notes: notes.to_owned(),
        ..NewEntry::default()
    };
    v.create_entry(root, entry, at(T0)).expect("entry")
}

/// Example 2, plus "Años", "Banana" and an entry in the bin that would match.
fn example_2(tag: &str) -> (TempDir, OpenedVault) {
    let dir = TempDir::new(tag);
    let mut v = create_vault_at(&dir.join("v.kdbx"), "Raíz");
    put(&mut v, "Canción favorita", "ana", "");
    put(&mut v, "Banco", "", "cancion de cuna");
    put(&mut v, "GitHub", "ana@example.com", "");
    put(&mut v, "Años", "", "");
    put(&mut v, "Banana", "", "");
    let gone = put(&mut v, "Canción borrada", "ana", "");
    v.delete_entry(gone, at(T0)).expect("to the bin");
    (dir, v)
}

fn titles(result: SearchResult) -> Vec<String> {
    match result {
        SearchResult::Matches(hits) => hits
            .into_iter()
            .map(|h| h.summary.title.unwrap_or_default())
            .collect(),
        SearchResult::NoFilter => panic!("expected matches, got NoFilter"),
    }
}

#[test]
fn spec_rf_59_rf_60_rf_63_example_2_cancion_title_block_first() {
    let (_dir, v) = example_2("ex2-cancion");
    assert_eq!(titles(v.search("CANCION")), ["Canción favorita", "Banco"]);
}

#[test]
fn spec_rf_59_example_2_every_word_must_appear() {
    let (_dir, v) = example_2("ex2-words");
    assert_eq!(titles(v.search("ana git")), ["GitHub"]);
    assert!(
        titles(v.search("ana cuna")).is_empty(),
        "no entry has both words"
    );
}

#[test]
fn spec_rf_59_example_2_query_is_trimmed() {
    let (_dir, v) = example_2("ex2-trim");
    assert_eq!(titles(v.search("  banco  ")), ["Banco"]);
}

#[test]
fn spec_rf_60_example_2_anos_finds_anos_with_tilde() {
    let (_dir, v) = example_2("ex2-anos");
    assert_eq!(titles(v.search("anos")), ["Años"]);
    assert_eq!(
        titles(v.search("AÑOS")),
        ["Años"],
        "the query is folded too"
    );
}

#[test]
fn spec_rf_59_rf_63_example_2_ana_matches_as_part_of_the_text_and_orders_title_hits_first() {
    let (_dir, v) = example_2("ex2-ana");
    assert_eq!(
        titles(v.search("ana")),
        ["Banana", "Canción favorita", "GitHub"]
    );
}

#[test]
fn spec_rf_62_entries_in_the_bin_are_excluded() {
    let (_dir, v) = example_2("ex2-bin");
    assert!(
        !titles(v.search("borrada"))
            .iter()
            .any(|t| t == "Canción borrada")
    );
    assert!(titles(v.search("borrada")).is_empty());
}

#[test]
fn spec_rf_92_a_query_empty_after_trimming_does_not_filter() {
    let (_dir, v) = example_2("ex2-empty");
    for query in ["", "   ", "\u{3000}\t\n", "\u{A0}"] {
        assert_eq!(v.search(query), SearchResult::NoFilter, "{query:?}");
    }
}

#[test]
fn spec_rf_59_searches_url_and_tags_and_tells_the_group_path() {
    let dir = TempDir::new("rf59-fields");
    let mut v = create_vault_at(&dir.join("v.kdbx"), "Raíz");
    let root = v.root_id();
    let work = v.create_group(root, "Trabajo", at(T0)).expect("group");
    let entry = NewEntry {
        title: "Servidor".to_owned(),
        url: "https://intranet.example.org".to_owned(),
        tags: vec!["Producción".to_owned()],
        ..NewEntry::default()
    };
    v.create_entry(work, entry, at(T0)).expect("entry");
    assert_eq!(titles(v.search("INTRANET")), ["Servidor"]);
    assert_eq!(titles(v.search("produccion")), ["Servidor"], "tags, folded");
    let SearchResult::Matches(hits) = v.search("servidor") else {
        panic!("matches")
    };
    assert_eq!(
        hits.first().map(|h| h.group_path.clone()),
        Some(vec!["Raíz".to_owned(), "Trabajo".to_owned()])
    );
}

#[test]
fn spec_rf_63_equal_titles_inside_a_block_go_by_uuid() {
    let dir = TempDir::new("rf63-tie");
    let mut v = create_vault_at(&dir.join("v.kdbx"), "Raíz");
    put(&mut v, "Duplicado", "", "");
    put(&mut v, "Duplicado", "", "");
    put(&mut v, "Otro", "duplicado", "");
    let SearchResult::Matches(hits) = v.search("duplicado") else {
        panic!("matches")
    };
    let order: Vec<_> = hits
        .iter()
        .map(|h| {
            (
                h.summary.title.clone().unwrap_or_default(),
                h.summary.id.uuid(),
            )
        })
        .collect();
    assert_eq!(order.len(), 3);
    assert_eq!(
        order.get(2).map(|o| o.0.as_str()),
        Some("Otro"),
        "title hits first"
    );
    assert!(
        order.first().map(|o| o.1) < order.get(1).map(|o| o.1),
        "ties by UUID: {order:?}"
    );
}
