//! Session state outside `Session`'s fields (al-runner issue 8211): every `static` of al-vm, al-builtins and al-data (a
//! thread-local or a process table) is listed, with what it is to a session, in the module comment of
//! `crates/al-vm/src/session_fields.rs` ("Thread-local and process tables"). A new one fails here until it is listed: the
//! field table names every `Session` field, and this list names every owner outside them.
//!
//! A listed name is written `static NAME` in backticks after its file. The scan fails both ways: for a `static` the list
//! does not name, and for a listed one that is no longer there. Exempt: test code (a `#[cfg(test)]` item; a file whose
//! module is declared under `#[cfg(test)]` or that says `#![cfg(test)]`) and generated files.

mod rust_scan;

use rust_scan::{blank, is_generated, matching_brace};
use std::collections::BTreeSet;

const CRATES: &[&str] = &["al-vm", "al-builtins", "al-data"];
const LIST: &str = "crates/al-vm/src/session_fields.rs";

/// Blanks every `#[cfg(test)]` item (as `os_access_ratchet.rs` does).
fn blank_test_items(code: &mut [char]) {
    let attr: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut i = 0;
    while i + attr.len() <= code.len() {
        if code[i..i + attr.len()] != attr[..] {
            i += 1;
            continue;
        }
        let mut j = i + attr.len();
        while j < code.len() && code[j] != '{' && code[j] != ';' {
            j += 1;
        }
        let end = if j < code.len() && code[j] == '{' {
            matching_brace(code, j)
        } else {
            j
        };
        let stop = (end + 1).min(code.len());
        for c in &mut code[i..stop] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        i = end + 1;
    }
}

/// The names of the modules declared as test code in `files` (a crate's sources): `mod NAME;` under a `#[cfg(test)]`
/// attribute (other attributes may stand between).
fn test_modules(files: &[(String, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, text) in files {
        let code: String = blank(text).iter().collect();
        for (at, _) in code.match_indices("#[cfg(test)]") {
            let mut rest = code[at + "#[cfg(test)]".len()..].trim_start();
            // Other attributes of the same item.
            while rest.starts_with("#[") {
                let Some(end) = rest.find(']') else { break };
                rest = rest[end + 1..].trim_start();
            }
            let Some(decl) = rest.strip_prefix("mod ") else {
                continue;
            };
            let name: String = decl
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if decl.trim_start()[name.len()..].trim_start().starts_with(';') {
                out.insert(name);
            }
        }
    }
    out
}

/// Whether `path` is test code: it says `#![cfg(test)]`, or its module (or a module folder it sits in) is declared under
/// `#[cfg(test)]` (`test_modules`).
fn is_test_file(path: &str, text: &str, test_mods: &BTreeSet<String>) -> bool {
    if text.contains("#![cfg(test)]") {
        return true;
    }
    let in_src = path.split("/src/").nth(1).unwrap_or(path);
    let mut segments: Vec<&str> = in_src.split('/').collect();
    if let Some(file) = segments.pop() {
        let stem = file.trim_end_matches(".rs");
        if stem != "mod" && test_mods.contains(stem) {
            return true;
        }
    }
    segments.iter().any(|seg| test_mods.contains(*seg))
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The names of the `static` items in `src`, test code left out: `static NAME:`, `static mut NAME:`, a name in any case,
/// the keyword and the name on two lines, and a name a macro makes (`static $n:`, named `$n`).
fn statics(src: &str) -> Vec<String> {
    let mut code = blank(src);
    blank_test_items(&mut code);
    let text: String = code.iter().collect();
    let mut out = Vec::new();
    for (at, _) in text.match_indices("static") {
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| is_ident(c) || c == '\'' || c == '$') {
            continue;
        }
        let after = &text[at + "static".len()..];
        if !after.starts_with(char::is_whitespace) {
            continue;
        }
        let mut rest = after.trim_start();
        if let Some(r) = rest.strip_prefix("mut")
            && r.starts_with(char::is_whitespace)
        {
            rest = r.trim_start();
        }
        let sigil = if rest.starts_with('$') { "$" } else { "" };
        let name: String = rest[sigil.len()..].chars().take_while(|c| is_ident(*c)).collect();
        if !name.is_empty() && rest[sigil.len() + name.len()..].trim_start().starts_with(':') {
            out.push(format!("{sigil}{name}"));
        }
    }
    out
}

/// (file, name) pairs the list names: each backticked `crates/...` path, then the backticked `static NAME`s after it up
/// to the next path.
fn listed(list: &str) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let mut file = None;
    for (k, token) in list.split('`').enumerate() {
        if k % 2 == 0 {
            continue;
        }
        if token.starts_with("crates/") {
            file = Some(token.to_string());
        } else if let (Some(f), Some(name)) = (&file, token.strip_prefix("static ")) {
            out.insert((f.clone(), name.to_string()));
        }
    }
    out
}

#[test]
fn every_owner_of_session_state_outside_session_is_listed() {
    let root = rust_scan::root();
    let list = std::fs::read_to_string(root.join(LIST)).expect("the field table");
    let list = list
        .split("Thread-local and process tables:")
        .nth(1)
        .and_then(|l| l.split("\nuse ").next())
        .expect("the list of thread-local and process tables");
    let named = listed(list);
    let mut found = BTreeSet::new();
    for krate in CRATES {
        let dir = root.join("crates").join(krate);
        let files = rust_scan::rust_files(&dir.join("src"), &root);
        let test_mods = test_modules(&files);
        for (path, text) in &files {
            if is_generated(text) || is_test_file(path, text, &test_mods) {
                continue;
            }
            for name in statics(text) {
                found.insert((path.clone(), name));
            }
        }
    }
    let missing: Vec<String> = (found.difference(&named))
        .map(|(path, name)| format!("{path} `{name}`"))
        .collect();
    assert!(
        missing.is_empty(),
        "session state outside `Session` that {LIST} does not list (say what it is to a session):\n{}",
        missing.join("\n")
    );
    let stale: Vec<String> = (named.difference(&found))
        .map(|(path, name)| format!("{path} `static {name}`"))
        .collect();
    assert!(
        stale.is_empty(),
        "{LIST} lists a static that is no longer there (delete it from the list):\n{}",
        stale.join("\n")
    );
}

// The forms the scan must catch, each a planted source.
#[test]
fn the_scan_catches_every_form_of_a_static() {
    let planted = [
        ("static mut COUNT: u32 = 0;", "COUNT"),
        (
            "#[allow(non_upper_case_globals)]\nstatic lower_name: u32 = 0;",
            "lower_name",
        ),
        ("macro_rules! m { ($n:ident) => { static $n: u32 = 0; }; }", "$n"),
        ("static\n    SPLIT: u32 = 0;", "SPLIT"),
        (
            "thread_local! { static TL: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }",
            "TL",
        ),
    ];
    for (src, name) in planted {
        assert_eq!(statics(src), vec![name.to_string()], "{src}");
    }
    // Not a static item: a lifetime, a field type, test code.
    for src in [
        "fn f(x: &'static str) {}",
        "struct S { s: &'static str }",
        "#[cfg(test)]\nmod t { static T: u8 = 0; }",
    ] {
        assert!(statics(src).is_empty(), "{src}");
    }
}

// A file named `..._tests.rs` is test code only when its module is declared under `#[cfg(test)]` (or it says
// `#![cfg(test)]` itself); a module of that name declared without it is scanned.
#[test]
fn a_tests_file_is_exempt_only_when_its_module_is_test_code() {
    let crate_files = [
        (
            "crates/x/src/lib.rs".to_string(),
            "#[cfg(test)]\nmod real_tests;\nmod not_tests;\n".to_string(),
        ),
        (
            "crates/x/src/real_tests.rs".to_string(),
            "static A: u8 = 0;".to_string(),
        ),
        ("crates/x/src/not_tests.rs".to_string(), "static B: u8 = 0;".to_string()),
        (
            "crates/x/src/inner_tests.rs".to_string(),
            "#![cfg(test)]\nstatic C: u8 = 0;".to_string(),
        ),
    ];
    let mods = test_modules(&crate_files);
    let scanned: Vec<&str> = (crate_files.iter())
        .filter(|(path, text)| !is_test_file(path, text, &mods))
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(scanned, vec!["crates/x/src/lib.rs", "crates/x/src/not_tests.rs"]);
}

// A listed name is `static NAME` in backticks; the list's other backticked text (a function, a test) is not a name.
#[test]
fn the_list_names_statics_by_the_word_static() {
    let list = "- `crates/a.rs` `static ONE`, `static two`, `static $n`: session (test `a_test`, `Session::f`).";
    let names: Vec<(String, String)> = listed(list).into_iter().collect();
    assert_eq!(
        names,
        ["$n", "ONE", "two"]
            .map(|n| ("crates/a.rs".to_string(), n.to_string()))
            .to_vec()
    );
}
