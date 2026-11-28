//! `--convert-citations` (step 3 of the corpus-detach plan, ADR-065 amendment 2) on a runner repository built here:
//! a `corpus` gitlink moved from pin 1 to pin 2, where the cited codeunit is renamed, one test is renamed through the
//! rename map, one file moves to another folder and another gets lines above the cited one. Each old citation is read
//! at the pin of the commit that first wrote it, carried to the checked-out pin, or left for a hand fix with the reason.

use al_test_cli::citation_convert::{self, Class, Row};
use al_test_cli::rename_map::RenameMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn put(dir: &Path, name: &str, text: &str) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

/// A codeunit with two tests; `second` names the second one.
fn one(name: &str, second: &str, tail: &str) -> String {
    format!(
        "codeunit 50100 \"{name}\"\n{{\n    Subtype = Test;\n\n    [Test]\n    procedure First()\n    begin\n        \
         Observer.Expect('one', 1);\n    end;\n\n    [Test]\n    procedure {second}()\n    begin\n    end;\n{tail}}}\n"
    )
}

/// A one-test codeunit; `top` goes above it.
fn single(id: u32, name: &str, test: &str, top: &str) -> String {
    format!(
        "{top}codeunit {id} \"{name}\"\n{{\n    Subtype = Test;\n\n    [Test]\n    procedure {test}()\n    begin\n    end;\n}}\n"
    )
}

/// Commits the corpus files and returns the commit.
fn corpus_commit(c: &Path, files: &[(&str, &str)], gone: &[&str]) -> String {
    for g in gone {
        git(c, &["rm", "-q", g]);
    }
    for (name, text) in files {
        put(c, name, text);
    }
    git(c, &["add", "."]);
    git(c, &["commit", "-qm", "c"]);
    git(c, &["rev-parse", "HEAD"])
}

/// Commits the runner files with the gitlink `corpus` at `pin`.
fn runner_commit(r: &Path, pin: &str, files: &[(&str, &str)]) {
    for (name, text) in files {
        put(r, name, text);
    }
    git(
        r,
        &["update-index", "--add", "--cacheinfo", &format!("160000,{pin},corpus")],
    );
    for (name, _) in files {
        git(r, &["add", name]);
    }
    git(r, &["commit", "-qm", "r"]);
}

const CITES: &str = "// TOne.al:8 is the first test's cell 'one'.
// TOne.al:12 is the second test.
// TOne.al:3 is not in a test.
// TOne.al:4 is blank.
// TMoved.al:6 is a test of a file that moves.
// TShift.al:6 is a test whose file gets lines above it.
// TOne.al names its codeunit.
// TOne.al:12 checks 'one', which is the other test's cell.
// Nowhere.al:1 is no corpus file.
const S: &str = \"TOne.al:8\";
// TOne.al:8 without its literal is the test: a line off by one would cite another cell.
// TOne.al:Second names its test by name, TOne.al:Nothing no member.
";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let c = root.join("corpus");
    std::fs::create_dir_all(&c).unwrap();
    git(&c, &["init", "-q", "-b", "master"]);
    let pin1 = corpus_commit(
        &c,
        &[
            ("tests/app/TOne.al", &one("LT One", "Second", "")),
            ("tests/app/TMoved.al", &single(50200, "LT Moved", "Alpha", "")),
            ("tests/app/TShift.al", &single(50300, "LT Shift", "Beta", "")),
        ],
        &[],
    );
    git(&root, &["init", "-q", "-b", "main"]);
    runner_commit(&root, &pin1, &[("src/a.rs", CITES)]);
    // Pin 2: the codeunit and its second test renamed (one line each), a test appended, a file moved, lines inserted.
    let third = "\n    [Test]\n    procedure Third()\n    begin\n    end;\n";
    let pin2 = corpus_commit(
        &c,
        &[
            ("tests/app/TOne.al", &one("LT One Renamed", "SecondRenamed", third)),
            ("tests/app/sub/TMoved.al", &single(50200, "LT Moved", "Alpha", "")),
            (
                "tests/app/TShift.al",
                &single(50300, "LT Shift", "Beta", "// a\n// b\n// c\n"),
            ),
            (
                "tests/rename-map.tsv",
                "old_key\tnew_key\napp/50100/Second\tapp/50100/SecondRenamed\n",
            ),
        ],
        &["tests/app/TMoved.al"],
    );
    runner_commit(&root, &pin2, &[("src/b.rs", "// unrelated\n")]);
    Fixture { _dir: dir, root }
}

fn rows(f: &Fixture, dry_run: bool) -> Vec<Row> {
    let c = f.root.join("corpus");
    let map = RenameMap::parse(&std::fs::read_to_string(c.join("tests/rename-map.tsv")).unwrap()).unwrap();
    citation_convert::convert(&f.root, &c, &map, &[PathBuf::from("src")], dry_run).unwrap()
}

fn by_line(rows: &[Row]) -> Vec<(usize, Class, Option<&str>)> {
    rows.iter().map(|r| (r.line, r.class, r.new.as_deref())).collect()
}

#[test]
fn each_old_citation_converts_at_its_pin_or_is_left_with_a_reason() {
    let f = fixture();
    let rows = rows(&f, true);
    assert_eq!(
        by_line(&rows),
        vec![
            (1, Class::Cell, Some("corpus:\"LT One Renamed\"/First#'one'")),
            (2, Class::Test, Some("corpus:\"LT One Renamed\"/SecondRenamed")),
            (3, Class::HandFix, None),
            (4, Class::HandFix, None),
            (5, Class::Test, Some("corpus:\"LT Moved\"/Alpha")),
            (6, Class::HandFix, None),
            (7, Class::File, Some("corpus:\"LT One Renamed\"")),
            (8, Class::HandFix, None),
            (9, Class::NotCorpus, None),
            (10, Class::InCode, None),
            (11, Class::Test, Some("corpus:\"LT One Renamed\"/First")),
            (12, Class::Test, Some("corpus:\"LT One Renamed\"/SecondRenamed")),
            (12, Class::HandFix, None),
        ],
        "{rows:#?}"
    );
    let note = |line: usize| rows.iter().find(|r| r.line == line).unwrap().note.clone();
    assert!(
        note(3).contains("outside every member of a test codeunit"),
        "{}",
        note(3)
    );
    assert!(note(4).contains("is blank"), "{}", note(4));
    assert!(note(6).contains("the next pin that changed the file"), "{}", note(6));
    assert!(note(8).contains("quotes 'one'"), "{}", note(8));
    // A converted row names the file and the pin it was read at: TMoved.al before its move, at pin 1.
    let pin1 = git(&f.root, &["rev-parse", "HEAD~1:corpus"]);
    assert_eq!(note(5), format!("read at tests/app/TMoved.al at {}", &pin1[..10]));
    // A dry run writes nothing.
    assert_eq!(std::fs::read_to_string(f.root.join("src/a.rs")).unwrap(), CITES);
}

#[test]
fn a_rewritten_line_is_read_at_the_pin_of_the_commit_that_first_wrote_the_citation() {
    let f = fixture();
    // The line that cites TShift.al:6 is rewrapped at pin 2: blame names pin 2, the first commit pin 1. Pin 1 is read,
    // and the blame pin, which maps the line elsewhere, makes it a hand fix.
    let text = CITES.replace("// TShift.al:6 is a test", "// Note: TShift.al:6 is a test");
    let pin2 = git(&f.root, &["rev-parse", "HEAD:corpus"]);
    runner_commit(&f.root, &pin2, &[("src/a.rs", &text)]);
    let rows = rows(&f, true);
    let r = rows.iter().find(|r| r.line == 6).unwrap();
    assert_eq!(r.class, Class::HandFix);
    assert!(r.note.contains("the blame pin"), "{}", r.note);
}

#[test]
fn a_conversion_writes_the_tokens_and_a_second_run_finds_only_what_was_left() {
    let f = fixture();
    let first = rows(&f, false);
    let text = std::fs::read_to_string(f.root.join("src/a.rs")).unwrap();
    assert!(
        text.starts_with("// corpus:\"LT One Renamed\"/First#'one' is the first test's cell 'one'.\n"),
        "{text}"
    );
    assert!(text.contains("// TOne.al:3 is not in a test."), "{text}");
    let second = rows(&f, false);
    let left = |rows: &[Row]| rows.iter().filter(|r| r.new.is_none()).count();
    assert_eq!(second.len(), left(&first));
    assert_eq!(std::fs::read_to_string(f.root.join("src/a.rs")).unwrap(), text);
}
