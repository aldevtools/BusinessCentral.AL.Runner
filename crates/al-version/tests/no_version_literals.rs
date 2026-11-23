//! All version logic is data or a gate (ADR-015 3.4 point 6). Outside `al-version`, library
//! code must not spell a BC version: no `BC27`-style symbol, no `27.5`, and no comparison of a
//! `major` with a number. Comments, `#[cfg(test)]` modules (at the end of a file) and test module files
//! (`tests.rs`, `*_tests.rs`, and files under a `tests/` folder of `src`) are skipped.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Why `line` spells a version, if it does.
fn offence(line: &str) -> Option<&'static str> {
    let b = line.as_bytes();
    let mut i = 0;
    while let Some(at) = line[i..].find("BC") {
        let rest = line[i + at + 2..].trim_start_matches(' ');
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits >= 2 && !line[..i + at].ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            return Some("a BC version name");
        }
        i += at + 2;
        if i >= b.len() {
            break;
        }
    }
    if line.contains("27.5") {
        return Some("27.5");
    }
    let mut i = 0;
    while let Some(at) = line[i..].find("major") {
        let rest = line[i + at + 5..].trim_start();
        for op in ["==", ">=", "<=", "!=", "<", ">"] {
            if let Some(r) = rest.strip_prefix(op) {
                if r.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
                    return Some("a major compared with a number");
                }
            }
        }
        i += at + 5;
    }
    None
}

/// Whether `f` under the crate folder `src` holds a test module: `tests.rs`, `*_tests.rs`, or a file under a `tests/`
/// folder of `src`.
fn is_test_module_file(src: &Path, f: &Path) -> bool {
    let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let in_tests_folder = f
        .strip_prefix(src)
        .ok()
        .and_then(Path::parent)
        .is_some_and(|p| p.components().any(|c| c.as_os_str() == "tests"));
    stem == "tests" || stem.ends_with("_tests") || in_tests_folder
}

#[test]
fn no_crate_outside_al_version_spells_a_bc_version() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut bad = Vec::new();
    for c in std::fs::read_dir(&crates).unwrap() {
        let c = c.unwrap().path();
        if c.file_name().is_some_and(|n| n == "al-version") || !c.join("src").is_dir() {
            continue;
        }
        let mut files = Vec::new();
        rust_files(&c.join("src"), &mut files);
        for f in files.into_iter().filter(|f| !is_test_module_file(&c.join("src"), f)) {
            let text = std::fs::read_to_string(&f).unwrap();
            let live = text.split("#[cfg(test)]").next().unwrap();
            for (n, line) in live.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if let Some(why) = offence(line) {
                    bad.push(format!("{}:{}: {why}: {}", f.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "version literals outside al-version:\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_scanner_finds_what_it_should() {
    assert!(offence("let s = \"BC27PLUS\";").is_some());
    // A minor symbol (ADR-078 point 3) spells a version too.
    assert!(offence("let s = \"BC29_1PLUS\";").is_some());
    assert!(offence("x == \"BC 29\"").is_some());
    assert!(offence("if v.major >= 29 {").is_some());
    assert!(offence("let v = \"27.5\";").is_some());
    assert!(offence("let a = v.major;").is_none());
    assert!(offence("wanted.major < al_version::MIN_BC_MAJOR").is_none());
    assert!(offence("fn abc12() {}").is_none());
}
