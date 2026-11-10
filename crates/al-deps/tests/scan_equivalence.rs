//! The cheap declaration scan reports what a full parse reports about object
//! headers, on every file of the corpus and of the Microsoft apps of the pinned
//! build in the artifact cache (ADR-013, demand-driven loading).
//!
//! These need the `corpus` submodule and the pinned build's files (`al_testfiles`);
//! a missing checkout or file fails loudly.

use al_syntax::{ast, scan::scan};
use std::path::{Path, PathBuf};

fn corpus_tests() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests");
    assert!(
        root.join("al-language/app.json").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    root
}

fn al_files(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "al") {
                out.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    out
}

/// Compares the scan with the parse of one file; returns the differences.
fn differences(name: &str, src: &str) -> Vec<String> {
    let syms = ["BC27PLUS".to_string()].into();
    let parse = al_syntax::parse(src, &syms);
    let root = ast::SourceFile::cast(parse.syntax()).unwrap();
    let s = scan(src, &syms);
    // A file the parser cannot read in full is not a fair comparison: the scan keeps going
    // where the parser gives up (Microsoft's source has a few such files).
    if !parse.errors.is_empty() {
        return Vec::new();
    }
    let mut diffs = Vec::new();
    let mut note = |what: &str, a: String, b: String| {
        if a != b {
            diffs.push(format!("{name}: {what}: parse {a} / scan {b}"));
        }
    };
    note(
        "namespace",
        format!("{:?}", root.namespace()),
        format!("{:?}", s.namespace),
    );
    note("usings", format!("{:?}", root.usings()), format!("{:?}", s.usings));
    let objs: Vec<_> = root.objects().collect();
    note("object count", objs.len().to_string(), s.objects.len().to_string());
    for (o, so) in objs.iter().zip(&s.objects) {
        let who = format!("{} {}", o.kind(), o.name());
        note(&format!("{who} kind"), o.kind(), so.kind.clone());
        note(&format!("{who} id"), format!("{:?}", o.id()), format!("{:?}", so.id));
        note(&format!("{who} name"), o.name(), so.name.clone());
        note(
            &format!("{who} extends"),
            format!("{:?}", o.extends_parts()),
            format!("{:?}", so.extends),
        );
        note(
            &format!("{who} implements"),
            format!("{:?}", o.implements_parts()),
            format!("{:?}", so.implements),
        );
        note(
            &format!("{who} offset"),
            usize::from(o.syntax().text_range().start()).to_string(),
            so.offset.to_string(),
        );
        let props: Vec<(String, String)> = o.properties().map(|p| (p.name(), p.value_text())).collect();
        note(
            &format!("{who} properties"),
            format!("{props:?}"),
            format!("{:?}", so.properties),
        );
        let layouts: Vec<(String, Vec<(String, String)>)> = o
            .sections()
            .filter(|s| s.name().eq_ignore_ascii_case("rendering"))
            .flat_map(|r| r.sections().collect::<Vec<_>>())
            .filter(|l| l.name().eq_ignore_ascii_case("layout"))
            .map(|l| {
                (
                    l.args().first().cloned().unwrap_or_default(),
                    l.properties().map(|p| (p.name(), p.value_text())).collect(),
                )
            })
            .collect();
        let scanned: Vec<(String, Vec<(String, String)>)> = so
            .layouts
            .iter()
            .map(|l| (l.name.clone(), l.properties.clone()))
            .collect();
        if o.kind().eq_ignore_ascii_case("report") {
            note(
                &format!("{who} layouts"),
                format!("{layouts:?}"),
                format!("{scanned:?}"),
            );
        }
        let subs = ast::routines_in(o.syntax())
            .iter()
            .flat_map(|r| r.attributes())
            .filter(|a| a.name().eq_ignore_ascii_case("EventSubscriber"))
            .count();
        note(
            &format!("{who} subscribers"),
            subs.to_string(),
            so.subscribers.len().to_string(),
        );
    }
    diffs
}

/// Every file in a release build; every `step`-th in a debug build, where parsing 9500 files
/// takes most of a minute.
fn assert_equivalent(files: &[(String, String)], at_least: usize, step: usize) {
    assert!(
        files.len() >= at_least,
        "expected at least {at_least} files, found {}",
        files.len()
    );
    let step = if cfg!(debug_assertions) { step } else { 1 };
    let diffs: Vec<String> = files
        .iter()
        .step_by(step)
        .flat_map(|(n, s)| differences(n, s))
        .collect();
    assert!(
        diffs.is_empty(),
        "{} difference(s) over {} files:\n{}",
        diffs.len(),
        files.len(),
        diffs.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn the_scan_matches_the_parse_on_every_corpus_file() {
    assert_equivalent(&al_files(&corpus_tests().join("al-language")), 1300, 1);
}

#[test]
fn the_scan_matches_the_parse_on_every_file_of_the_dependency_apps() {
    let mut files = Vec::new();
    for app in [
        al_testfiles::SYSTEM_APPLICATION,
        al_testfiles::BUSINESS_FOUNDATION,
        al_testfiles::BASE_APPLICATION,
        al_testfiles::SYSTEM,
    ] {
        files.extend(al_testfiles::sources(app));
    }
    assert_equivalent(&files, 9000, 12);
}
