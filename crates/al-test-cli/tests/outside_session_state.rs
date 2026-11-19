//! Session state outside `Session`'s fields (al-runner issue 8211, PERF-01 step 2): every `static` of al-vm, al-builtins
//! and al-data (a thread-local or a process table) is listed, with what the session image does with it, in the module
//! comment of `crates/al-vm/src/login_image/fields.rs` ("Thread-local and process tables"). A new one fails here until it
//! is listed: the field table names every `Session` field, and this list names every owner outside them.
//!
//! Exempt, as in the other scans: test code (a `#[cfg(test)]` item, a file named `tests.rs` or under a `tests/` folder,
//! a file ending `_tests.rs`) and generated files.

mod rust_scan;

use rust_scan::{blank, is_generated, matching_brace};
use std::collections::BTreeSet;

const CRATES: &[&str] = &["al-vm", "al-builtins", "al-data"];
const LIST: &str = "crates/al-vm/src/login_image/fields.rs";

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

fn is_test_file(path: &str) -> bool {
    path.ends_with("/tests.rs") || path.ends_with("_tests.rs") || path.split('/').skip(3).any(|seg| seg == "tests")
}

/// The names of the `static` items in `src` (`static NAME:`), test code left out.
fn statics(src: &str) -> Vec<String> {
    let mut code = blank(src);
    blank_test_items(&mut code);
    let text: String = code.iter().collect();
    let mut out = Vec::new();
    for (at, _) in text.match_indices("static ") {
        if text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let name: String = text[at + 7..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let rest = text[at + 7 + name.len()..].trim_start();
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && rest.starts_with(':')
        {
            out.push(name);
        }
    }
    out
}

/// (file, name) pairs the list names: each backticked `crates/...` path, then the backticked names after it up to the next
/// path.
fn listed(list: &str) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let mut file = None;
    for (k, token) in list.split('`').enumerate() {
        if k % 2 == 0 {
            continue;
        }
        if token.starts_with("crates/") {
            file = Some(token.to_string());
        } else if let Some(f) = &file
            && !token.is_empty()
            && token
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            out.insert((f.clone(), token.to_string()));
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
    let mut missing = Vec::new();
    for krate in CRATES {
        let dir = root.join("crates").join(krate);
        for (path, text) in rust_scan::rust_files(&dir.join("src"), &root) {
            if is_generated(&text) || is_test_file(&path) {
                continue;
            }
            for name in statics(&text) {
                if !named.contains(&(path.clone(), name.clone())) {
                    missing.push(format!("{path} `{name}`"));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "session state outside `Session` that {LIST} does not list (say what the image does with it):\n{}",
        missing.join("\n")
    );
}
