//! `Roots::object_hashes` (ADR-050 decision 3): which edits change an object's hash.

use crate::workspace::{AppKind, AppSource};

const CU: &str =
    "codeunit 50100 \"A\"\n{\n    procedure P(): Integer\n    begin\n        exit(1); // one\n    end;\n}\n";
const TB: &str = "table 50101 \"T\"\n{\n    fields\n    {\n        field(1; F; Integer) { }\n    }\n}\n";

/// The hashes of codeunit 50100 and table 50101 of one app holding `files`, by object id.
fn hashes(files: &[(&str, &str)], on: bool) -> Vec<(u32, String)> {
    let (ws, l) = crate::lower::compile_apps_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect(),
        }],
        vec![AppKind::Workspace],
        &Default::default(),
        None,
        crate::check::Roots {
            object_hashes: on,
            ..Default::default()
        },
    );
    let mut out: Vec<(u32, String)> = l
        .object_hashes
        .iter()
        .enumerate()
        .filter_map(|(oi, h)| Some((ws.objects[oi].id?, h.as_ref()?.hash.clone())))
        .filter(|(id, _)| *id == 50100 || *id == 50101)
        .collect();
    out.sort();
    out
}

fn of(h: &[(u32, String)], id: u32) -> String {
    h.iter()
        .find(|(i, _)| *i == id)
        .map(|(_, h)| h.clone())
        .expect("hashed")
}

#[test]
fn off_hashes_nothing() {
    assert!(hashes(&[("a.al", CU)], false).is_empty());
}

#[test]
fn the_same_source_gives_the_same_hash() {
    assert_eq!(
        hashes(&[("a.al", CU), ("t.al", TB)], true),
        hashes(&[("a.al", CU), ("t.al", TB)], true)
    );
}

#[test]
fn a_changed_body_changes_the_hash() {
    let before = hashes(&[("a.al", CU)], true);
    let after = hashes(&[("a.al", &CU.replace("exit(1)", "exit(2)"))], true);
    assert_ne!(of(&before, 50100), of(&after, 50100));
}

#[test]
fn a_comment_inside_a_line_changes_nothing() {
    let before = hashes(&[("a.al", CU)], true);
    let after = hashes(&[("a.al", &CU.replace("// one", "// two"))], true);
    assert_eq!(before, after);
}

#[test]
fn a_line_shift_changes_the_hash() {
    // A call stack shows lines, which AL code can read.
    let before = hashes(&[("a.al", CU)], true);
    let after = hashes(&[("a.al", &format!("// header\n{CU}"))], true);
    assert_ne!(of(&before, 50100), of(&after, 50100));
}

#[test]
fn an_object_in_another_file_keeps_its_hash() {
    let before = hashes(&[("a.al", CU), ("t.al", TB)], true);
    let after = hashes(&[("a.al", CU), ("t.al", &TB.replace("F; Integer", "F; Decimal"))], true);
    assert_eq!(of(&before, 50100), of(&after, 50100));
    assert_ne!(of(&before, 50101), of(&after, 50101));
}

#[test]
fn the_folder_of_the_app_does_not_matter_but_the_file_name_does() {
    let before = hashes(&[("src/a.al", CU)], true);
    assert_eq!(before, hashes(&[("src/a.al", CU)], true));
    assert_ne!(of(&before, 50100), of(&hashes(&[("src/b.al", CU)], true), 50100));
}
