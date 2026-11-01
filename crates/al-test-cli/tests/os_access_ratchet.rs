//! OS access ratchet (ADR-006: core crates do no OS access). Counts the places in the source of
//! the core crates that name an OS API: the environment (`std::env`, `env::var`, `env::var_os`),
//! files (`std::fs`, `fs::`, `File::create`, `File::open`, `OpenOptions`), processes
//! (`std::process`) and the network (`std::net`). A file under `crates/<core>/src` may hold none,
//! except the files listed in `metrics/os-access.toml`, each of which may not hold more than its
//! count. A count only goes down: move the access to a front end (`al-testrun`, `al-test-cli`,
//! `alrunner-cli`) or behind a capability in `al-host`, then lower or delete the row. Raising a
//! count or adding a row needs the owner.
//!
//! Comments and string literals are not code and are not counted; `env!` and `option_env!` read
//! at compile time and are not OS access. Overlapping names count once (`std::env::var` is one
//! place). Exempt: test code in the source (a `#[cfg(test)]` item and a file named `tests.rs`
//! or under a `tests/` folder) and generated files.

mod rust_scan;

use rust_scan::{blank, is_generated, matching_brace, parse_table};
use std::collections::BTreeMap;

const TABLE: &str = "metrics/os-access.toml";

/// The core crates: the list of ADR-006, then the crates that say in their own documentation
/// that they do no OS access (`al-host`, `al-version`, `al-bcdb`, `al-cache`) and the link step
/// between `al-sema` and `al-vm` (`al-link`).
const CORE: &[&str] = &[
    "al-syntax",
    "al-sema",
    "al-builtins",
    "al-ir",
    "al-vm",
    "al-data",
    "al-host",
    "al-version",
    "al-bcdb",
    "al-cache",
    "al-link",
];

/// The names counted. Assembled at run time so this file does not hold them itself.
fn patterns() -> Vec<String> {
    let s = "std::";
    vec![
        format!("{s}env"),
        "env::var".into(),
        format!("{s}fs"),
        "fs::".into(),
        "File::create".into(),
        "File::open".into(),
        "OpenOptions".into(),
        format!("{s}process"),
        format!("{s}net"),
    ]
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Blanks every `#[cfg(test)]` item: the attribute up to the end of the item's braces, or up to
/// its `;` when it has none (`#[cfg(test)] mod tests;`).
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

/// The number of places in `src` that name an OS API.
fn os_sites(src: &str) -> usize {
    let mut code = blank(src);
    blank_test_items(&mut code);
    let text: String = code.iter().collect();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for p in patterns() {
        for (at, _) in text.match_indices(p.as_str()) {
            let before = text[..at].chars().next_back();
            // `xfs::` is not `fs::`; `std::env` must not be the start of `std::environment`
            if before.is_some_and(is_ident) {
                continue;
            }
            let after = text[at + p.len()..].chars().next();
            if p.starts_with("std::") && after.is_some_and(is_ident) {
                continue;
            }
            spans.push((at, at + p.len()));
        }
    }
    spans.sort();
    let mut count = 0;
    let mut end = 0;
    for (a, b) in spans {
        if count == 0 || a >= end {
            count += 1;
            end = b;
        } else {
            end = end.max(b);
        }
    }
    count
}

fn is_test_file(path: &str) -> bool {
    path.ends_with("/tests.rs") || path.split('/').skip(3).any(|seg| seg == "tests")
}

/// (path, count) of every core source file that names an OS API.
fn all_sites() -> Vec<(String, usize)> {
    let root = rust_scan::root();
    let mut out = Vec::new();
    for krate in CORE {
        let dir = root.join("crates").join(krate);
        assert!(dir.is_dir(), "core crate {krate} is not in crates/; update CORE");
        for (path, text) in rust_scan::rust_files(&dir.join("src"), &root) {
            if is_generated(&text) || is_test_file(&path) {
                continue;
            }
            let n = os_sites(&text);
            if n > 0 {
                out.push((path, n));
            }
        }
    }
    out
}

fn violations(found: &[(String, usize)], caps: &BTreeMap<String, usize>) -> Vec<String> {
    let mut out = Vec::new();
    for (path, n) in found {
        match caps.get(path) {
            None => out.push(format!(
                "{path}: names an OS API in {n} place(s); core crates do no OS access (ADR-006). \
                 Take the value as a parameter or through al-host instead; adding a row to \
                 {TABLE} needs the owner."
            )),
            Some(cap) if n > cap => out.push(format!(
                "{path}: names an OS API in {n} places, cap {cap} (ADR-006). Take the value as a \
                 parameter or through al-host instead of raising the cap; raising a cap needs \
                 the owner."
            )),
            _ => {}
        }
    }
    for path in caps.keys() {
        if !found.iter().any(|(p, _)| p == path) {
            out.push(format!(
                "{path}: listed in {TABLE} but names no OS API any more (or is gone); delete the row."
            ));
        }
    }
    out
}

fn lowerable(found: &[(String, usize)], caps: &BTreeMap<String, usize>) -> Vec<String> {
    found
        .iter()
        .filter_map(|(path, n)| {
            let cap = caps.get(path)?;
            (n < cap).then(|| format!("{path}: now {n} places; lower its count from {cap} to {n} in {TABLE}"))
        })
        .collect()
}

#[test]
fn no_core_file_gains_os_access() {
    let root = rust_scan::root();
    let table = std::fs::read_to_string(root.join(TABLE)).unwrap_or_else(|e| panic!("{TABLE}: {e}"));
    let caps = parse_table(TABLE, &table);
    let found = all_sites();
    if std::env::var_os("PRINT_OS_ACCESS").is_some() {
        for (path, n) in &found {
            println!("\"{path}\" = {n}");
        }
    }
    for note in lowerable(&found, &caps) {
        eprintln!("note: {note}");
    }
    let bad = violations(&found, &caps);
    assert!(bad.is_empty(), "OS access ratchet:\n{}", bad.join("\n"));
}

#[test]
fn each_kind_of_access_is_counted_once_per_place() {
    let s = "std::";
    let src = format!(
        "use {s}fs;\nfn f() {{\n    let a = {s}env::var(\"X\");\n    let b = env::var_os(\"Y\");\n    \
         fs::write(p, b);\n    let f = File::create(p);\n    let g = File::open(p);\n    \
         let o = OpenOptions::new();\n    {s}process::exit(0);\n    let n = {s}net::TcpStream::connect(a);\n}}\n"
    );
    assert_eq!(os_sites(&src), 9);
}

#[test]
fn comments_strings_compile_time_env_and_lookalikes_are_not_counted() {
    let s = "std::";
    let src = format!(
        "//! reads {s}env::var\n/* {s}fs::write */\nfn f() {{\n    let a = \"{s}fs::read\";\n    \
         let b = env!(\"CARGO_MANIFEST_DIR\");\n    let c = option_env!(\"X\");\n    \
         let d = xfs::read();\n    let e = self.refs::x;\n    let g = {s}environment;\n}}\n"
    );
    assert_eq!(os_sites(&src), 0);
}

#[test]
fn test_code_is_exempt() {
    let s = "std::";
    let src = format!(
        "fn f() {{}}\n#[cfg(test)]\nmod tests {{\n    fn t() {{ {s}fs::read(\"x\"); }}\n}}\n\
         #[cfg(test)]\nfn helper() {{ {s}env::var(\"X\"); }}\n#[cfg(test)]\nmod more;\n"
    );
    assert_eq!(os_sites(&src), 0);
    assert!(is_test_file("crates/al-vm/src/runtime/tests.rs"));
    assert!(is_test_file("crates/al-vm/src/runtime/tests/a.rs"));
    assert!(!is_test_file("crates/al-vm/src/runtime/run.rs"));
}

#[test]
fn a_listed_file_may_not_gain_but_may_lose_sites() {
    let caps = parse_table(TABLE, "\"crates/a/src/x.rs\" = 2 # why\n");
    let bad = violations(&[("crates/a/src/x.rs".to_string(), 3)], &caps);
    assert_eq!(bad.len(), 1);
    assert!(bad[0].contains("3 places, cap 2"), "{bad:?}");
    let fewer = [("crates/a/src/x.rs".to_string(), 1)];
    assert!(violations(&fewer, &caps).is_empty());
    assert!(lowerable(&fewer, &caps)[0].contains("lower its count from 2 to 1"));
    assert_eq!(violations(&[], &caps).len(), 1, "a row for a clean file fails");
    let new = violations(&[("crates/a/src/y.rs".to_string(), 1)], &BTreeMap::new());
    assert!(new[0].contains("ADR-006"), "{new:?}");
}
