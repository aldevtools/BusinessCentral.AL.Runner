//! The lexer over every `.al` file in the corpus. The lexer must cover each byte exactly once. BC compiles every file
//! outside `tests/must-not-compile/`, so those produce no `Error` token and no preprocessor diagnostic; a must-not-compile
//! case may be refused by the lexer or the preprocessor (corpus bump 28: 1386 `@abc`, 1575 `&&` and 1576 `==` in `#if`).

use al_syntax::{SyntaxKind, lex};
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

fn must_not_compile(f: &Path) -> bool {
    f.components().any(|c| c.as_os_str() == "must-not-compile")
}

#[test]
fn every_corpus_file_round_trips_and_lexes_without_errors() {
    let mut failures = Vec::new();
    for f in corpus_files() {
        let src = std::fs::read_to_string(&f).unwrap();
        let mut pos = 0usize;
        let mut first_error = None;
        for t in lex(&src) {
            let end = pos + t.len as usize;
            if t.kind == SyntaxKind::Error && first_error.is_none() {
                let line = src[..pos].matches('\n').count() + 1;
                first_error = Some(format!("line {line}: {:?}", &src[pos..end.min(pos + 40)]));
            }
            pos = end;
        }
        if pos != src.len() {
            failures.push(format!("{}: covered {pos} of {} bytes", f.display(), src.len()));
        }
        if let Some(e) = first_error.filter(|_| !must_not_compile(&f)) {
            failures.push(format!("{}: {e}", f.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} file(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_corpus_file_preprocesses_cleanly_for_bc27_5() {
    // ADR-015: the corpus CI symbol set for BC 27.
    let syms = al_version::corpus_symbols(
        &al_version::BcVersion::new(27, 0),
        &al_version::ReleasedMinors::NotRecorded,
    );
    let mut failures = Vec::new();
    let mut disabled_files = 0;
    for f in corpus_files() {
        let src = std::fs::read_to_string(&f).unwrap();
        let p = al_syntax::preprocessor::preprocess(&src, &lex(&src), &syms);
        let covered: usize = p.tokens.iter().map(|t| t.len as usize).sum();
        if covered != src.len() {
            failures.push(format!("{}: covered {covered} of {} bytes", f.display(), src.len()));
        }
        for d in p.diagnostics.into_iter().filter(|_| !must_not_compile(&f)) {
            failures.push(format!("{}: {}", f.display(), d.message));
        }
        disabled_files += p.tokens.iter().any(|t| t.kind == SyntaxKind::DisabledText) as usize;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // The corpus guards a handful of files with `#if BC29PLUS`; if none is
    // disabled the symbol handling is not doing anything.
    assert!(disabled_files > 0, "no #if branch was disabled anywhere in the corpus");
}
