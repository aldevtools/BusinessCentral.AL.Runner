//! Milestone 1, front end: every corpus file parses with zero errors and the
//! tree's text is the file, byte for byte.

use std::path::{Path, PathBuf};

fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests");
    assert!(
        root.join("al-language/app.json").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "al") {
                out.push(p);
            }
        }
    }
    out.sort();
    assert!(out.len() > 1300, "expected the whole corpus, found {} files", out.len());
    out
}

/// A must-not-compile case is rejected by the real compiler; a case about a syntax error is not
/// expected to parse. Such files must still round-trip byte for byte, but their parse errors are
/// not failures and their blocks are not counted as structure (corpus bump 10 added cases for
/// missing `end`, extra `end`, a variable named `begin`).
fn is_must_not_compile(f: &Path) -> bool {
    f.components().any(|c| c.as_os_str() == "must-not-compile")
}

/// Corpus files the parser does not handle yet. Each is a red corpus test waiting for a fix, not a
/// pass: the runtime reports every case in such a file as not run (the file does not compile).
/// An entry whose file now parses fails the test below, so the list only shrinks (ADR-007).
const KNOWN_UNPARSED: &[(&str, &str)] = &[];

#[test]
fn every_corpus_file_parses_without_errors_and_round_trips() {
    let syms = al_version::corpus_symbols(
        &al_version::BcVersion::new(27, 0),
        &al_version::ReleasedMinors::NotRecorded,
    );
    let mut failures = Vec::new();
    let files = corpus_files();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let parse = al_syntax::parse(&src, &syms);
        if parse.syntax().text() != src.as_str() {
            failures.push(format!("{}: tree text differs from source", f.display()));
        }
        if is_must_not_compile(f) {
            continue;
        }
        if let Some(e) = parse.errors.first() {
            let line = src[..e.offset.min(src.len())].matches('\n').count() + 1;
            failures.push(format!(
                "{}:{line}: {} ({} error(s))",
                f.display(),
                e.message,
                parse.errors.len()
            ));
        }
    }
    let known = |f: &String| {
        KNOWN_UNPARSED
            .iter()
            .any(|(k, _)| f.split(':').next().unwrap_or("").ends_with(k))
    };
    let stale: Vec<_> = KNOWN_UNPARSED
        .iter()
        .filter(|(k, _)| !failures.iter().any(|f| f.split(':').next().unwrap_or("").ends_with(k)))
        .map(|(k, why)| format!("{k} ({why})"))
        .collect();
    assert!(
        stale.is_empty(),
        "these files parse now: remove them from KNOWN_UNPARSED:\n{}",
        stale.join("\n")
    );
    failures.retain(|f| !known(f));
    assert!(
        failures.is_empty(),
        "{} of {} files failed:\n{}",
        failures.len(),
        files.len(),
        failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// Guards against a grammar that "passes" by swallowing code into raw
/// property values or section arguments: every `procedure`/`trigger` keyword
/// must become a routine node, every `begin` a block, every `[Test]` an attribute.
#[test]
fn corpus_code_is_parsed_structurally_not_swallowed() {
    use al_syntax::SyntaxKind::*;
    let syms = al_version::corpus_symbols(
        &al_version::BcVersion::new(27, 0),
        &al_version::ReleasedMinors::NotRecorded,
    );
    let (mut kw_routines, mut kw_begins, mut nodes_routines, mut nodes_blocks, mut test_attrs) = (0, 0, 0, 0, 0);
    for f in corpus_files() {
        if is_must_not_compile(&f) {
            continue;
        }
        let src = std::fs::read_to_string(&f).unwrap();
        let tree = al_syntax::parse(&src, &syms).syntax();
        for t in tree.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if t.kind() == Ident {
                let w = t.text().to_ascii_lowercase();
                kw_routines += (w == "procedure" || w == "trigger") as usize;
                kw_begins += (w == "begin") as usize;
            }
        }
        for n in tree.descendants() {
            match n.kind() {
                Procedure | Trigger => nodes_routines += 1,
                Block => nodes_blocks += 1,
                Attribute if n.text().to_string().eq_ignore_ascii_case("[Test]") => test_attrs += 1,
                _ => {}
            }
        }
    }
    assert_eq!(
        nodes_routines, kw_routines,
        "routine nodes vs procedure/trigger keywords"
    );
    assert_eq!(nodes_blocks, kw_begins, "block nodes vs begin keywords");
    assert!(test_attrs > 4800, "only {test_attrs} [Test] attributes parsed");
}

/// A statement block never holds a type reference, also in a file that does not parse: the
/// unresolved-reference scan of `al-sema` (`Workspace::unresolved_references_of`) does not enter
/// blocks and relies on this.
#[test]
fn no_corpus_block_holds_a_type_reference() {
    use al_syntax::SyntaxKind::*;
    let syms = al_version::corpus_symbols(
        &al_version::BcVersion::new(27, 0),
        &al_version::ReleasedMinors::NotRecorded,
    );
    let mut found = Vec::new();
    for f in corpus_files() {
        let src = std::fs::read_to_string(&f).unwrap();
        let tree = al_syntax::parse(&src, &syms).syntax();
        for b in tree.descendants().filter(|n| n.kind() == Block) {
            if let Some(t) = b.descendants().find(|n| n.kind() == TypeRef) {
                found.push(format!("{}: `{}`", f.display(), t.text()));
            }
        }
    }
    assert!(found.is_empty(), "type references inside blocks:\n{}", found.join("\n"));
}
