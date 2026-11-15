//! Independent validation from the spec: entries and groups (RF-26, RF-39 to RF-48, RF-52 to RF-58, RF-91, RF-93;
//! examples 3, 4, 8 and 9). History and deleted objects have no public API: they are read back with `keepass`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use almarioks_core::{
    CoreError, Field,
    kdbx::OpenedVault,
    vault::{
        DeletePreview, Deletion, EntryId, EntryPatch, GroupId, GroupKind, GroupNode, NewEntry,
        validate_new_entry,
    },
};
use keepass::{Database, DatabaseKey};
use secrecy::{ExposeSecret, SecretString};
use spec_support::{PW, T0, TempDir, at, create_vault_at, key, new_entry, titles_in};
use uuid::Uuid;

fn vault(tag: &str) -> (TempDir, OpenedVault) {
    let dir = TempDir::new(tag);
    let vault = create_vault_at(&dir.join("v.kdbx"), "Raíz");
    (dir, vault)
}

fn reparse(vault: &OpenedVault) -> Database {
    let bytes = vault.to_bytes(&key(PW)).expect("bytes");
    Database::parse(&bytes, DatabaseKey::new().with_password(PW))
        .expect("keepass reads what we wrote")
}

fn history_titles(vault: &OpenedVault, id: EntryId) -> Vec<String> {
    let db = reparse(vault);
    let entry = db
        .iter_all_entries()
        .find(|e| e.id().uuid() == id.uuid())
        .expect("entry in file");
    entry
        .history
        .as_ref()
        .map(|h| {
            h.get_entries()
                .iter()
                .map(|v| v.get_title().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn reveal(vault: &OpenedVault, id: EntryId) -> String {
    vault
        .reveal_field(id, "Password")
        .expect("reveal")
        .expose_secret()
        .to_owned()
}

fn secret(text: &str) -> Option<SecretString> {
    Some(SecretString::from(text.to_owned()))
}

fn find<'a>(node: &'a GroupNode, name: &str) -> Option<&'a GroupNode> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|c| find(c, name))
}

fn group(vault: &mut OpenedVault, parent: GroupId, name: &str) -> GroupId {
    vault.create_group(parent, name, at(T0)).expect("group")
}

fn add(vault: &mut OpenedVault, group: GroupId, title: &str) -> EntryId {
    vault
        .create_entry(group, new_entry(title), at(T0))
        .expect("entry")
}

fn missing_entry() -> EntryId {
    EntryId::from_uuid(Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0))
}

fn missing_group() -> GroupId {
    GroupId::from_uuid(Uuid::from_u128(0x0fed_cba9_8765_4321_0fed_cba9_8765_4321))
}

// ----- Entries -----

#[test]
fn spec_rf_45_example_8_editing_only_the_title_keeps_the_password_and_adds_one_version() {
    let (_dir, mut v) = vault("ex8");
    let root = v.root_id();
    let entry = NewEntry {
        title: "GitHub".to_owned(),
        password: secret("tercera-clave"),
        ..NewEntry::default()
    };
    let id = v.create_entry(root, entry, at(T0)).expect("create");

    let patch = EntryPatch {
        title: Some("GitHub personal".to_owned()),
        ..EntryPatch::default()
    };
    assert_eq!(v.update_entry(id, patch, at(T0 + 60)), Ok(true));
    assert_eq!(
        reveal(&v, id),
        "tercera-clave",
        "the request had no password: it stays"
    );
    let detail = v.entry(id).expect("detail");
    assert_eq!(detail.summary.title.as_deref(), Some("GitHub personal"));
    assert_eq!(detail.summary.modified, at(T0 + 60));
    assert_eq!(
        history_titles(&v, id),
        vec!["GitHub".to_owned()],
        "one version with the old title"
    );

    // Confirm again without changing anything: neither date nor history change.
    let same = EntryPatch {
        title: Some("GitHub personal".to_owned()),
        ..EntryPatch::default()
    };
    assert_eq!(v.update_entry(id, same, at(T0 + 120)), Ok(false));
    // Deduced border: a title that is the same once trimmed is no change either.
    let padded = EntryPatch {
        title: Some("  GitHub personal  ".to_owned()),
        ..EntryPatch::default()
    };
    assert_eq!(v.update_entry(id, padded, at(T0 + 180)), Ok(false));
    assert_eq!(
        v.update_entry(id, EntryPatch::default(), at(T0 + 240)),
        Ok(false)
    );
    assert_eq!(v.entry(id).expect("detail").summary.modified, at(T0 + 60));
    assert_eq!(history_titles(&v, id).len(), 1);
}

#[test]
fn spec_rf_45_a_present_field_is_replaced_also_by_empty_and_an_absent_one_stays() {
    let (_dir, mut v) = vault("rf45-empty");
    let root = v.root_id();
    let entry = NewEntry {
        title: "Banco".to_owned(),
        username: "ana".to_owned(),
        password: secret("clave"),
        url: "https://banco.example".to_owned(),
        notes: "notas".to_owned(),
        tags: vec!["dinero".to_owned()],
    };
    let id = v.create_entry(root, entry, at(T0)).expect("create");
    let patch = EntryPatch {
        username: Some(String::new()),
        password: secret(""),
        ..EntryPatch::default()
    };
    assert_eq!(v.update_entry(id, patch, at(T0 + 1)), Ok(true));
    let detail = v.entry(id).expect("detail");
    assert_eq!(detail.summary.username.as_deref(), Some(""));
    assert_eq!(reveal(&v, id), "");
    assert_eq!(detail.summary.url.as_deref(), Some("https://banco.example"));
    assert_eq!(detail.notes.as_deref(), Some("notas"));
    assert_eq!(detail.summary.tags, vec!["dinero".to_owned()]);
}

#[test]
fn spec_rf_39_rf_40_example_9_titles() {
    let (_dir, mut v) = vault("ex9");
    let root = v.root_id();
    let create =
        |v: &mut OpenedVault, title: String| v.create_entry(root, new_entry(&title), at(T0));
    assert_eq!(
        create(&mut v, "   ".to_owned()),
        Err(CoreError::TituloVacio)
    );
    assert_eq!(
        create(&mut v, " ".repeat(300)),
        Err(CoreError::TituloVacio),
        "empty after trimming beats long"
    );
    let banco = create(&mut v, "  Banco  ".to_owned()).expect("Banco");
    assert_eq!(
        v.entry(banco).expect("d").summary.title.as_deref(),
        Some("Banco")
    );
    assert_eq!(
        create(&mut v, "a".repeat(256)),
        Err(CoreError::CampoMuyLargo)
    );
    let emoji = create(&mut v, "😀".repeat(255)).expect("255 emoji are 255 characters");
    assert_eq!(
        v.entry(emoji)
            .expect("d")
            .summary
            .title
            .map(|t| t.chars().count()),
        Some(255)
    );
    // Deduced borders: the limit is checked on the trimmed title; U+3000 is White_Space; U+200B is not.
    assert!(create(&mut v, format!("  {}  ", "b".repeat(255))).is_ok());
    let ideographic = create(&mut v, "\u{3000}Año\u{3000}".to_owned()).expect("ideographic spaces");
    assert_eq!(
        v.entry(ideographic).expect("d").summary.title.as_deref(),
        Some("Año")
    );
    assert!(
        create(&mut v, "\u{200B}".to_owned()).is_ok(),
        "U+200B is not White_Space"
    );
    // The same rules on edit (RF-39 "crear o editar").
    let blank = EntryPatch {
        title: Some("\t \n".to_owned()),
        ..EntryPatch::default()
    };
    assert_eq!(
        v.update_entry(banco, blank, at(T0 + 1)),
        Err(CoreError::TituloVacio)
    );
    let long = EntryPatch {
        title: Some("c".repeat(256)),
        ..EntryPatch::default()
    };
    assert_eq!(
        v.update_entry(banco, long, at(T0 + 1)),
        Err(CoreError::CampoMuyLargo)
    );
    assert_eq!(
        v.entry(banco).expect("d").summary.title.as_deref(),
        Some("Banco"),
        "rejected edits change nothing"
    );
}

#[test]
fn spec_rf_40_each_limit_names_its_field_and_limit_and_counts_characters() {
    let base = || NewEntry {
        title: "Límites".to_owned(),
        ..NewEntry::default()
    };
    let reject = |entry: NewEntry| {
        let error = validate_new_entry(&entry).expect_err("over the limit");
        (error.error(), error.field(), error.limit())
    };
    let long = CoreError::CampoMuyLargo;
    assert_eq!(
        reject(NewEntry {
            title: "t".repeat(256),
            ..base()
        }),
        (long, Field::Title, Some(255))
    );
    assert_eq!(
        reject(NewEntry {
            username: "u".repeat(256),
            ..base()
        }),
        (long, Field::UserName, Some(255))
    );
    assert_eq!(
        reject(NewEntry {
            url: "h".repeat(2_049),
            ..base()
        }),
        (long, Field::Url, Some(2_048))
    );
    assert_eq!(
        reject(NewEntry {
            password: secret(&" ".repeat(1_025)),
            ..base()
        }),
        (long, Field::Password, Some(1_024))
    );
    assert_eq!(
        reject(NewEntry {
            notes: "n".repeat(100_001),
            ..base()
        }),
        (long, Field::Notes, Some(100_000))
    );
    let tags = |n: usize| (0..n).map(|i| format!("t{i}")).collect::<Vec<_>>();
    assert_eq!(
        reject(NewEntry {
            tags: tags(51),
            ..base()
        }),
        (long, Field::Tags, Some(50))
    );
    assert_eq!(
        reject(NewEntry {
            tags: vec!["e".repeat(51)],
            ..base()
        }),
        (long, Field::Tags, Some(50))
    );

    // At the limit, counted in characters (multi-byte), everything is accepted.
    let at_limit = NewEntry {
        title: "ñ".repeat(255),
        username: "€".repeat(255),
        password: secret(&"😀".repeat(1_024)),
        url: "é".repeat(2_048),
        notes: "ü".repeat(100_000),
        tags: (0..50)
            .map(|i| format!("{i:0>2}{}", "á".repeat(48)))
            .collect(),
    };
    assert_eq!(validate_new_entry(&at_limit), Ok(()));
    let (_dir, mut v) = vault("rf40");
    let root = v.root_id();
    assert!(v.create_entry(root, at_limit, at(T0)).is_ok());
    // Definiciones: TituloVacio wins over CampoMuyLargo.
    let both = NewEntry {
        title: "  ".to_owned(),
        username: "u".repeat(300),
        ..NewEntry::default()
    };
    assert_eq!(
        v.create_entry(root, both, at(T0)),
        Err(CoreError::TituloVacio)
    );
}

#[test]
fn spec_rf_41_the_password_is_stored_exactly_as_written() {
    let (_dir, mut v) = vault("rf41");
    let root = v.root_id();
    for typed in [
        "  Pässwörd ñ  ",
        "e\u{301}xacta",
        "\u{3000}MAYÚSCULAS y minúsculas\t",
        "ﬁ ligadura Ⅻ",
    ] {
        let entry = NewEntry {
            title: "Clave".to_owned(),
            password: secret(typed),
            ..NewEntry::default()
        };
        let id = v.create_entry(root, entry, at(T0)).expect("create");
        assert_eq!(
            reveal(&v, id).as_bytes(),
            typed.as_bytes(),
            "RF-41 for {typed:?}"
        );
        let patch = EntryPatch {
            password: secret(typed),
            ..EntryPatch::default()
        };
        assert_eq!(
            v.update_entry(id, patch, at(T0 + 1)),
            Ok(false),
            "same password, no change"
        );
    }
}

#[test]
fn spec_rf_91_tags_with_separators_or_blank_are_rejected_and_others_are_trimmed() {
    let (_dir, mut v) = vault("rf91");
    let root = v.root_id();
    for bad in ["a;b", "a,b", "   ", "\u{3000}"] {
        let entry = NewEntry {
            title: "T".to_owned(),
            tags: vec![bad.to_owned()],
            ..NewEntry::default()
        };
        assert_eq!(
            v.create_entry(root, entry, at(T0)),
            Err(CoreError::EtiquetaInvalida),
            "tag {bad:?}"
        );
    }
    let entry = NewEntry {
        title: "T".to_owned(),
        tags: vec!["  trabajo  ".to_owned()],
        ..NewEntry::default()
    };
    let id = v.create_entry(root, entry, at(T0)).expect("create");
    assert_eq!(
        v.entry(id).expect("d").summary.tags,
        vec!["trabajo".to_owned()]
    );
}

#[test]
fn spec_definiciones_control_characters_are_rejected_but_tab_and_line_feed_are_not() {
    let (_dir, mut v) = vault("control");
    let root = v.root_id();
    let with = |title: &str, notes: &str| NewEntry {
        title: title.to_owned(),
        notes: notes.to_owned(),
        ..NewEntry::default()
    };
    assert_eq!(
        v.create_entry(root, with("Ban\u{7}co", ""), at(T0)),
        Err(CoreError::CaracterNoPermitido)
    );
    assert_eq!(
        v.create_entry(root, with("Banco", "x\u{9F}"), at(T0)),
        Err(CoreError::CaracterNoPermitido)
    );
    assert_eq!(
        v.create_entry(root, with("Banco", "\u{7F}"), at(T0)),
        Err(CoreError::CaracterNoPermitido)
    );
    assert!(
        v.create_entry(root, with("Banco", "línea\nsegunda\tcon tab"), at(T0))
            .is_ok()
    );
}

#[test]
fn spec_rf_93_unknown_ids_answer_no_encontrado_before_any_domain_rule() {
    let (_dir, mut v) = vault("rf93");
    let root = v.root_id();
    assert_eq!(
        v.entry(missing_entry()).map(|_| ()),
        Err(CoreError::NoEncontrado)
    );
    assert_eq!(
        v.list_entries(missing_group()).map(|_| ()),
        Err(CoreError::NoEncontrado)
    );
    let blank = EntryPatch {
        title: Some("  ".to_owned()),
        ..EntryPatch::default()
    };
    assert_eq!(
        v.update_entry(missing_entry(), blank, at(T0)),
        Err(CoreError::NoEncontrado)
    );
    assert_eq!(
        v.create_entry(missing_group(), new_entry(" "), at(T0)),
        Err(CoreError::NoEncontrado)
    );
    let id = add(&mut v, root, "x");
    assert_eq!(
        v.move_entry(id, missing_group(), at(T0)),
        Err(CoreError::NoEncontrado)
    );
    assert_eq!(
        v.create_group(missing_group(), "", at(T0)),
        Err(CoreError::NoEncontrado)
    );
    assert_eq!(
        EntryId::parse("no-es-un-uuid").map(|_| ()),
        Err(CoreError::ArgumentoInvalido),
        "RF-80"
    );
}

#[test]
fn spec_rf_42_lists_are_in_title_order_folded_then_by_uuid() {
    let (_dir, mut v) = vault("rf42");
    let root = v.root_id();
    for title in ["Zeta", "banco", "ñandú", "Árbol", "Duplicado", "Duplicado"] {
        add(&mut v, root, title);
    }
    let list = v.list_entries(root).expect("list");
    let titles: Vec<_> = list
        .iter()
        .map(|e| e.title.clone().unwrap_or_default())
        .collect();
    assert_eq!(
        titles,
        ["Árbol", "banco", "Duplicado", "Duplicado", "ñandú", "Zeta"]
    );
    let dups: Vec<Uuid> = list
        .iter()
        .filter(|e| e.title.as_deref() == Some("Duplicado"))
        .map(|e| e.id.uuid())
        .collect();
    assert!(dups.first() < dups.get(1), "equal titles go by UUID");
}

#[test]
fn spec_rf_26_history_is_oldest_first_and_keeps_within_the_vault_limit() {
    let (_dir, mut v) = vault("rf26");
    let root = v.root_id();
    let id = add(&mut v, root, "V0");
    for n in 1..=12 {
        let patch = EntryPatch {
            title: Some(format!("V{n}")),
            ..EntryPatch::default()
        };
        assert_eq!(v.update_entry(id, patch, at(T0 + n)), Ok(true));
    }
    let limit = reparse(&v)
        .meta
        .history_max_items
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(usize::MAX);
    let all: Vec<String> = (0..12).map(|n| format!("V{n}")).collect();
    let kept = all
        .get(all.len().saturating_sub(limit)..)
        .expect("range")
        .to_vec();
    assert_eq!(
        history_titles(&v, id),
        kept,
        "oldest to newest, the oldest dropped beyond {limit}"
    );
}

#[test]
fn spec_rf_46_moving_an_entry_leaves_it_only_in_the_target_and_adds_no_version() {
    let (_dir, mut v) = vault("rf46");
    let root = v.root_id();
    let target = group(&mut v, root, "Destino");
    let id = add(&mut v, root, "Mover");
    v.move_entry(id, target, at(T0 + 5)).expect("move");
    assert!(titles_in(&v, root).is_empty());
    assert_eq!(titles_in(&v, target), vec!["Mover".to_owned()]);
    assert_eq!(v.entry(id).expect("d").group_id, target);
    assert!(history_titles(&v, id).is_empty(), "RF-46: no version added");
}

#[test]
fn spec_rf_47_rf_48_delete_goes_to_a_new_bin_then_for_good_into_deleted_objects() {
    let (_dir, mut v) = vault("rf47");
    let root = v.root_id();
    let id = add(&mut v, root, "Borrar");
    assert_eq!(
        v.delete_entry(id, at(T0 + 1)),
        Ok(Deletion::MovedToRecycleBin)
    );
    let bin = v.recycle_bin_id().expect("RF-47: the bin is created");
    let tree = v.group_tree();
    assert!(
        tree.children
            .iter()
            .any(|g| g.id == bin && g.kind == GroupKind::RecycleBin)
    );
    assert_eq!(titles_in(&v, bin), vec!["Borrar".to_owned()]);
    assert_eq!(
        reparse(&v).meta.recyclebin_enabled,
        Some(true),
        "RF-47: and enabled"
    );

    assert_eq!(v.delete_entry(id, at(T0 + 2)), Ok(Deletion::Permanent));
    assert_eq!(v.entry(id).map(|_| ()), Err(CoreError::NoEncontrado));
    assert!(
        reparse(&v).deleted_objects.contains_key(&id.uuid()),
        "RF-48: written in deleted objects"
    );
}

// ----- Groups -----

#[test]
fn spec_rf_52_rf_53_group_names_are_trimmed_and_bounded() {
    let (_dir, mut v) = vault("rf52");
    let root = v.root_id();
    assert_eq!(
        v.create_group(root, "   ", at(T0)),
        Err(CoreError::NombreVacio)
    );
    assert_eq!(
        v.create_group(root, &"g".repeat(256), at(T0)),
        Err(CoreError::CampoMuyLargo)
    );
    assert!(v.create_group(root, &"😀".repeat(255), at(T0)).is_ok());
    let id = v.create_group(root, "  Trabajo  ", at(T0)).expect("group");
    assert!(find(&v.group_tree(), "Trabajo").is_some_and(|g| g.id == id));
    v.rename_group(id, "  Oficina  ", at(T0 + 1))
        .expect("rename");
    assert!(find(&v.group_tree(), "Oficina").is_some_and(|g| g.id == id));
    assert_eq!(
        v.rename_group(id, "\u{3000}", at(T0 + 2)),
        Err(CoreError::NombreVacio)
    );
}

#[test]
fn spec_rf_55_rf_54_example_3_moving_groups() {
    let (_dir, mut v) = vault("ex3");
    let root = v.root_id();
    let trabajo = group(&mut v, root, "Trabajo");
    let clientes = group(&mut v, trabajo, "Clientes");
    add(&mut v, clientes, "Cliente A");
    assert_eq!(
        v.move_group(trabajo, clientes, at(T0)),
        Err(CoreError::MovimientoInvalido)
    );
    assert_eq!(
        v.move_group(trabajo, trabajo, at(T0)),
        Err(CoreError::MovimientoInvalido),
        "into itself"
    );
    v.move_group(clientes, root, at(T0))
        .expect("Clientes to the root");
    let tree = v.group_tree();
    let names: Vec<&str> = tree.children.iter().map(|g| g.name.as_str()).collect();
    assert!(
        names.contains(&"Trabajo") && names.contains(&"Clientes"),
        "{names:?}"
    );
    assert!(find(&tree, "Trabajo").is_some_and(|g| g.children.is_empty()));
    assert_eq!(
        titles_in(&v, clientes),
        vec!["Cliente A".to_owned()],
        "RF-54: with its content"
    );
}

#[test]
fn spec_rf_56_rf_58_example_4_deleting_a_group_counts_every_level_then_bin_then_for_good() {
    let (_dir, mut v) = vault("ex4");
    let root = v.root_id();
    let trabajo = group(&mut v, root, "Trabajo");
    let clientes = group(&mut v, trabajo, "Clientes");
    let mut ids = Vec::new();
    for t in ["T1", "T2", "T3"] {
        ids.push(add(&mut v, trabajo, t));
    }
    for t in ["C1", "C2"] {
        ids.push(add(&mut v, clientes, t));
    }
    assert_eq!(
        v.delete_preview(trabajo),
        Ok(DeletePreview {
            entries: 5,
            subgroups: 1
        }),
        "\"5 entradas y 1 subgrupo\""
    );
    assert_eq!(
        v.delete_group(trabajo, at(T0 + 1)),
        Ok(Deletion::MovedToRecycleBin)
    );
    let bin = v.recycle_bin_id().expect("bin");
    let tree = v.group_tree();
    let bin_node = tree.children.iter().find(|g| g.id == bin);
    let in_bin = bin_node
        .and_then(|b| b.children.iter().find(|g| g.id == trabajo))
        .expect("Trabajo inside the bin");
    assert!(in_bin.children.iter().any(|g| g.id == clientes));
    assert_eq!(
        titles_in(&v, trabajo).len() + titles_in(&v, clientes).len(),
        5,
        "the 5 are in the bin"
    );

    assert_eq!(
        v.delete_group(trabajo, at(T0 + 2)),
        Ok(Deletion::Permanent),
        "RF-58: already in the bin"
    );
    let db = reparse(&v);
    for id in &ids {
        assert_eq!(v.entry(*id).map(|_| ()), Err(CoreError::NoEncontrado));
        assert!(
            db.deleted_objects.contains_key(&id.uuid()),
            "RF-58: every entry written down"
        );
    }
    assert!(
        db.deleted_objects.contains_key(&trabajo.uuid())
            && db.deleted_objects.contains_key(&clientes.uuid())
    );
}

#[test]
fn spec_rf_58_emptying_the_bin_removes_everything_in_it_and_keeps_the_bin() {
    let (_dir, mut v) = vault("rf58-empty");
    let root = v.root_id();
    let a = add(&mut v, root, "A");
    let g = group(&mut v, root, "G");
    let b = add(&mut v, g, "B");
    v.delete_entry(a, at(T0)).expect("delete");
    v.delete_group(g, at(T0)).expect("delete");
    let bin = v.recycle_bin_id().expect("bin");
    v.empty_recycle_bin(at(T0 + 1)).expect("empty");
    assert_eq!(v.recycle_bin_id(), Some(bin));
    assert!(titles_in(&v, bin).is_empty());
    let db = reparse(&v);
    for uuid in [a.uuid(), b.uuid(), g.uuid()] {
        assert!(db.deleted_objects.contains_key(&uuid));
    }
    assert_eq!(v.entry(b).map(|_| ()), Err(CoreError::NoEncontrado));
}

#[test]
fn spec_rf_57_root_and_bin_are_protected_and_the_root_may_be_renamed() {
    let (_dir, mut v) = vault("rf57");
    let root = v.root_id();
    let other = group(&mut v, root, "Otro");
    let id = add(&mut v, root, "x");
    v.delete_entry(id, at(T0)).expect("creates the bin");
    let bin = v.recycle_bin_id().expect("bin");
    assert_eq!(
        v.move_group(root, other, at(T0)),
        Err(CoreError::GrupoProtegido)
    );
    assert_eq!(v.delete_group(root, at(T0)), Err(CoreError::GrupoProtegido));
    assert_eq!(v.delete_preview(root), Err(CoreError::GrupoProtegido));
    assert_eq!(
        v.move_group(bin, other, at(T0)),
        Err(CoreError::GrupoProtegido)
    );
    assert_eq!(
        v.rename_group(bin, "Basura", at(T0)),
        Err(CoreError::GrupoProtegido)
    );
    assert_eq!(v.delete_group(bin, at(T0)), Err(CoreError::GrupoProtegido));
    v.rename_group(root, "Mi bóveda", at(T0))
        .expect("renaming the root is allowed");
    assert_eq!(v.name(), "Mi bóveda");
}

#[test]
fn spec_rf_57_a_group_that_holds_the_bin_cannot_be_deleted() {
    // A KeePass vault whose bin lives inside "Contenedor" (casos límite: "la papelera dentro de otro grupo").
    let made =
        almarioks_core::kdbx::create_vault(&key(PW), "Raíz", spec_support::kdf()).expect("create");
    let mut db = Database::parse(&made, DatabaseKey::new().with_password(PW)).expect("parse");
    let bin_uuid = {
        let mut root = db.root_mut();
        let mut holder = root.add_group();
        holder.name = "Contenedor".into();
        let mut bin = holder.add_group();
        bin.name = "Papelera vieja".into();
        bin.id().uuid()
    };
    db.meta.recyclebin_uuid = Some(bin_uuid);
    db.meta.recyclebin_enabled = Some(true);
    let mut out = Vec::new();
    db.save(&mut out, DatabaseKey::new().with_password(PW))
        .expect("save");
    let mut v = almarioks_core::kdbx::open(&out, &key(PW)).expect("open");
    assert_eq!(v.recycle_bin_id().map(GroupId::uuid), Some(bin_uuid));
    let holder = find(&v.group_tree(), "Contenedor")
        .map(|g| g.id)
        .expect("holder");
    assert_eq!(v.delete_preview(holder), Err(CoreError::GrupoProtegido));
    assert_eq!(
        v.delete_group(holder, at(T0)),
        Err(CoreError::GrupoProtegido)
    );
    v.rename_group(holder, "Contenedor renombrado", at(T0))
        .expect("renaming it is allowed");
    // Deleting an entry uses the existing bin, it does not create another one.
    let root = v.root_id();
    let id = add(&mut v, root, "y");
    assert_eq!(v.delete_entry(id, at(T0)), Ok(Deletion::MovedToRecycleBin));
    assert_eq!(v.entry(id).map(|d| d.group_id.uuid()), Ok(bin_uuid));
}
