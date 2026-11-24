//! Session state outside `Session`'s fields (al-runner issue 8211): every `static` of al-vm, al-builtins and al-data (a
//! thread-local or a process table) is listed, with what it is to a session, in the module comment of
//! `crates/al-vm/src/session_fields.rs` ("Thread-local and process tables"). A new one fails here until it is listed: the
//! field table names every `Session` field, and this list names every owner outside them. The `static`s of
//! al-testrun are listed the same way in `crates/al-testrun/src/login_start/outside.rs`, and so are the inputs of a
//! login that live in neither: every field of al-testrun's `Scratch` and every engine field al-vm's
//! `Engine::reset_run_state` lists as "Set by a run", each written `field NAME` after its file (the login replay keys
//! those by hand).
//!
//! A listed name is written `static NAME` in backticks after its file. The scan fails both ways: for a `static` the list
//! does not name, and for a listed one that is no longer there. Exempt: test code (a `#[cfg(test)]` item; a file whose
//! module is declared under `#[cfg(test)]` or that says `#![cfg(test)]`) and generated files.

mod rust_scan;

use rust_scan::{blank, is_generated, matching_brace};
use std::collections::BTreeSet;

const CRATES: &[&str] = &["al-vm", "al-builtins", "al-data"];
const LIST: &str = "crates/al-vm/src/session_fields.rs";
const TESTRUN_CRATES: &[&str] = &["al-testrun"];
const TESTRUN_LIST: &str = "crates/al-testrun/src/login_start/outside.rs";
/// The file of al-testrun's `Scratch` and the file of al-vm's `Engine::reset_run_state`.
const SCRATCH_FILE: &str = "crates/al-testrun/src/runtime/mod.rs";
const ENGINE_FILE: &str = "crates/al-vm/src/lib.rs";

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

/// Fails for a `static` of `crates` (generated files and test code left out) that the list in `list_file` does not name,
/// and for a listed one that is no longer there. The list is the text after "Thread-local and process tables:".
fn assert_every_static_is_listed(crates: &[&str], list_file: &str, what: &str) {
    let root = rust_scan::root();
    let list = std::fs::read_to_string(root.join(list_file)).expect("the list of statics");
    let list = list
        .split("Thread-local and process tables:")
        .nth(1)
        .and_then(|l| l.split("\nuse ").next())
        .expect("the list of thread-local and process tables");
    let named = listed(list);
    let mut found = BTreeSet::new();
    for krate in crates {
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
        "{what} that {list_file} does not list (say what it is to a session):\n{}",
        missing.join("\n")
    );
    let stale: Vec<String> = (named.difference(&found))
        .map(|(path, name)| format!("{path} `static {name}`"))
        .collect();
    assert!(
        stale.is_empty(),
        "{list_file} lists a static that is no longer there (delete it from the list):\n{}",
        stale.join("\n")
    );
}

#[test]
fn every_owner_of_session_state_outside_session_is_listed() {
    assert_every_static_is_listed(CRATES, LIST, "session state outside `Session`");
}

// Al-testrun keeps session state outside `Session` too (the events hooks, the lane's
// single instances, the carry, the login memo); each is listed with what a login start does with it.
#[test]
fn every_owner_of_session_state_in_al_testrun_is_listed() {
    assert_every_static_is_listed(
        TESTRUN_CRATES,
        TESTRUN_LIST,
        "session state outside `Session` kept by al-testrun",
    );
}

/// The names of the fields of `struct name` in `src`: each `name:` at the top level of its braces, after any attribute
/// and visibility.
fn struct_fields(src: &str, name: &str) -> Vec<String> {
    let code = blank(src);
    let text: String = code.iter().collect();
    let at = text.find(&format!("struct {name} {{")).expect("the struct");
    let open = at + text[at..].find('{').expect("its brace");
    let close = matching_brace(&code, open);
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut field = String::new();
    let body: Vec<char> = code[open + 1..close].to_vec();
    for (i, &c) in body.iter().enumerate() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if i == 0 || body[i - 1] != '-' => depth -= 1,
            ',' if depth == 0 => {
                out.extend(field_name(&field));
                field.clear();
                continue;
            }
            _ => {}
        }
        field.push(c);
    }
    out.extend(field_name(&field));
    out
}

/// The name of one field declaration (`#[attr] pub(crate) name: Type`), `None` for blank text.
fn field_name(decl: &str) -> Option<String> {
    let mut rest = decl.trim();
    while rest.starts_with("#[") {
        rest = rest[rest.find(']')? + 1..].trim_start();
    }
    for vis in ["pub(crate)", "pub(super)", "pub"] {
        if let Some(r) = rest.strip_prefix(vis) {
            rest = r.trim_start();
            break;
        }
    }
    let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
    (!name.is_empty() && rest[name.len()..].trim_start().starts_with(':')).then_some(name)
}

/// The fields `Engine::reset_run_state` in `src` lists under `// Set by a run.`: each name up to the next comment.
fn run_set_fields(src: &str) -> Vec<String> {
    let body = src.split("pub fn reset_run_state").nth(1).expect("reset_run_state");
    let section = body
        .split("// Set by a run.")
        .nth(1)
        .expect("its \"Set by a run\" fields");
    (section.lines().skip(1))
        .map(str::trim)
        .take_while(|l| !l.starts_with("//"))
        .map(|l| l.trim_end_matches(',').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// (file, name) pairs the list names as `field NAME` after a backticked `crates/...` path.
fn listed_fields(list: &str) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let mut file = None;
    for (k, token) in list.split('`').enumerate() {
        if k % 2 == 0 {
            continue;
        }
        if token.starts_with("crates/") {
            file = Some(token.to_string());
        } else if let (Some(f), Some(name)) = (&file, token.strip_prefix("field ")) {
            out.insert((f.clone(), name.to_string()));
        }
    }
    out
}

// The inputs of a login outside `Session` and outside a `static`: the run's plan for one test codeunit (`Scratch`) and
// what a run sets on the engine. The login replay key holds them by hand, so each is named in the al-testrun list with
// what the key does with it; a new one fails here until it is, and a listed one that is gone fails too.
#[test]
fn every_field_of_the_scratch_and_every_engine_field_a_run_sets_is_listed() {
    let root = rust_scan::root();
    let read = |file: &str| std::fs::read_to_string(root.join(file)).expect(file);
    let list = read(TESTRUN_LIST);
    let list = list
        .split("Fields a run sets:")
        .nth(1)
        .and_then(|l| l.split("Thread-local and process tables:").next())
        .expect("the list of the fields a run sets");
    let named = listed_fields(list);
    let found: BTreeSet<(String, String)> = (struct_fields(&read(SCRATCH_FILE), "Scratch").into_iter())
        .map(|n| (SCRATCH_FILE.to_string(), n))
        .chain(
            run_set_fields(&read(ENGINE_FILE))
                .into_iter()
                .map(|n| (ENGINE_FILE.to_string(), n)),
        )
        .collect();
    let missing: Vec<String> = (found.difference(&named))
        .map(|(path, name)| format!("{path} `field {name}`"))
        .collect();
    assert!(
        missing.is_empty(),
        "fields a run sets that {TESTRUN_LIST} does not list (say what the login key does with it):\n{}",
        missing.join("\n")
    );
    let stale: Vec<String> = (named.difference(&found))
        .map(|(path, name)| format!("{path} `field {name}`"))
        .collect();
    assert!(
        stale.is_empty(),
        "{TESTRUN_LIST} lists a field that is no longer there (delete it from the list):\n{}",
        stale.join("\n")
    );
}

// The scan of a struct's fields: attributes, visibility, generic types with commas, a function type.
#[test]
fn the_scan_names_each_field_of_a_struct() {
    let src = "/// doc\npub(crate) struct S {\n    /// a: b, c\n    a: u8,\n    #[allow(x)]\n    pub(crate) b: \
               std::collections::BTreeMap<(u32, String), Vec<u8>>,\n    pub c: fn(u8) -> u8,\n    \
               d: Option<Arc<str>>\n}";
    assert_eq!(struct_fields(src, "S"), ["a", "b", "c", "d"]);
    let engine = "pub fn reset_run_state(&mut self) {\n let E {\n // Set by a run.\n one,\n two,\n // Other.\n \
                  three: _,";
    assert_eq!(run_set_fields(engine), ["one", "two"]);
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
