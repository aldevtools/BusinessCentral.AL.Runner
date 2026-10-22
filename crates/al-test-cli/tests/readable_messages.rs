//! Refusal reasons and internal errors read as AL, not as Rust. A message that formats a value with
//! `{:?}` prints internal structure (`Record(RefCell { .. })`) into the text a user reads, and the
//! reasons stop grouping in `--top-refusals`. Describe a value with `al_builtins::describe::Describe`.
//!
//! This scan covers the production code of every crate (the part of a file before its
//! `#[cfg(test)]` module). A debug format that stays must be listed in `KEPT` with the reason.

use std::path::{Path, PathBuf};

/// `(file suffix, the trimmed source line, why the debug format is fine here)`.
const KEPT: &[(&str, &str, &str)] = &[
    (
        "alrunner-cli/src/affected.rs",
        r#""{:?}","#,
        "the run options of `--affected` as the input of a hash, never shown",
    ),
    (
        "alrunner-cli/src/affected.rs",
        r#"Err(_) => sha(format!("{:?} {:?} {:?}", i.json, i.manifest, i.symbols).as_bytes()),"#,
        "the manifest of a package as the input of a hash, never shown",
    ),
    (
        "alrunner-cli/src/affected.rs",
        r#"let key = sha(format!("{apps:?}").as_bytes());"#,
        "the workspace apps as the input of the stored result's file name, never shown",
    ),
    (
        "al-test-cli/src/behavior_coverage.rs",
        r#"format!("{k:?}")"#,
        "the grammar kind name as the catalog key of the behavior coverage map (SyntaxKind names are the item names)",
    ),
    (
        "al-test-cli/src/behavior_coverage.rs",
        r#"Some(t) => format!("Literal:{:?}", t.kind()),"#,
        "the literal kind as a catalog key of the behavior coverage map",
    ),
    (
        "al-vm/src/exec/mod.rs",
        r#"Some(c.code.iter().map(|op| format!("{op:?}")).collect())"#,
        "the listing of the instructions of a routine (tests of instruction selection): for the developers of the compiler",
    ),
    (
        "al-link/src/lib.rs",
        r#""source position {at}: {:?} against {:?} ({} and {} positions)","#,
        "the report of a defect of the link step (AL_CHECK_LINK, tests): for the developers of the compiler",
    ),
    (
        "al-link/src/lib.rs",
        r#"let (x, y) = (format!("{a:?}"), format!("{b:?}"));"#,
        "the report of a defect of the link step (AL_CHECK_LINK, tests): for the developers of the compiler",
    ),
    (
        "al-vm/src/exec/compile.rs",
        r#"eprintln!("  {i:3} {op:?}");"#,
        "the bytecode listing of AL_EXEC_DUMP, for the developers of the VM",
    ),
    (
        "al-vm/src/exec/compile.rs",
        r#"other => unreachable!("not a jump: {other:?}"),"#,
        "an internal error of the bytecode compiler, never shown to an AL user",
    ),
    (
        "al-testrun/src/runtime/compiled.rs",
        r#""the program differs (first object that differs: {:?}; positions equal: {}; counts {} and {})","#,
        "the report of a defect of the incremental update (ALRUNNER_CHECK_INCREMENTAL, tests): for the developers of the compiler",
    ),
    (
        "al-testrun/src/runtime/compiled.rs",
        r#""the diagnostics differ: {:?} and {:?}","#,
        "the report of a defect of the incremental update (ALRUNNER_CHECK_INCREMENTAL, tests): for the developers of the compiler",
    ),
    (
        "al-testrun/src/runtime/compiled.rs",
        r#""the baseline decision differs: {:?} {:?} and {:?} {:?}","#,
        "the report of a defect of the incremental update (ALRUNNER_CHECK_INCREMENTAL, tests): for the developers of the compiler",
    ),
    (
        "alrunner-cli/src/run.rs",
        r#"let target_key = format!("{:?} {}", chosen.target, a.execute);"#,
        "part of a hash key for the warm slot, never shown",
    ),
    (
        "al-data/src/memory.rs",
        r#".filter(|&i| format!("{:?}", n[i]) != format!("{:?}", old[i]))"#,
        "equality of two values (Value has no PartialEq) for the record of changes to restored rows, never shown",
    ),
    (
        "al-data/src/contract.rs",
        r#"other => panic!("{other:?}"),"#,
        "the conformance suite asserts on a store outcome; a failing test prints it",
    ),
    (
        "al-host/src/testdata.rs",
        r#"let key = format!("{}|{:?}|{}", table.name.to_lowercase(), table.app_id, table.per_company);"#,
        "a cache key, never shown",
    ),
    (
        "al-sema/src/queries.rs",
        r#""column {name}: a {class:?} field in this position (or with Method) is not supported yet""#,
        "`class` is a unit enum: the debug text is the variant name (Normal, FlowField)",
    ),
    (
        "al-sema/src/lower/stmt.rs",
        r#"other => Err(format!("statement {other:?} is not lowered yet")),"#,
        "`other` is a syntax kind: the debug text is its name",
    ),
    (
        "al-sema/src/lower/expr.rs",
        r#"other => Err(format!("expression {other:?} `{}` is not lowered yet", n.text())),"#,
        "`other` is a syntax kind: the debug text is its name",
    ),
    (
        "al-syntax/src/parser.rs",
        r#"self.error(&format!("expected {kind:?}"));"#,
        "a syntax kind: the debug text is its name",
    ),
    (
        "al-syntax/src/parser.rs",
        r#"s.push_str(&format!("{:?}", node.kind()));"#,
        "the tree dump of the parser tests: a syntax kind name",
    ),
    (
        "al-testrun/src/runtime/handlers.rs",
        r#""{} and {} are both {kind:?} handlers; which one BC would bind is not pinned","#,
        "`kind` is a unit enum (UiKind): the debug text is the variant name",
    ),
    (
        "al-vm/src/globaltriggers.rs",
        r#""{op:?} of a temporary record of {}, which has Global Triggers events enabled (whether BC raises them is not pinned)","#,
        "`op` is a unit enum (Insert, Modify, Delete, Rename)",
    ),
    (
        "al-vm/src/partial.rs",
        r#"format!("{:?}", t.primary_key(row))"#,
        "a comparison key, never shown",
    ),
    (
        "al-vm/src/partial.rs",
        r#"if format!("{:?}", fresh) != format!("{:?}", r.fields) {"#,
        "a comparison of two buffers, never shown",
    ),
    (
        "al-vm/src/platform/metadata.rs",
        r#""page '{}': the action '{}' is a {:?}{}, whose row is not pinned","#,
        "the argument is a control kind: a unit enum, the debug text is the variant name",
    ),
    (
        "al-vm/src/reports.rs",
        r#""CurrReport.{flow:?} outside the triggers where it is pinned (what it does there is not pinned)""#,
        "`flow` is a unit enum (Skip, Break, Quit): the debug text is the variant name",
    ),
    (
        "al-builtins/src/value.rs",
        r#""PageState(page {}, open {}, mode {:?})","#,
        "the Debug impl of PageState prints facts only; `mode` is a unit enum",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_message_formats_a_value_with_the_debug_format() {
    let mut files = Vec::new();
    for k in std::fs::read_dir(root().join("crates")).unwrap().flatten() {
        let src = k.path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.len() > 100, "found {} source files", files.len());
    let mut found = Vec::new();
    for f in &files {
        let rel = f.strip_prefix(root()).unwrap().to_string_lossy().replace('\\', "/");
        // `al-testkit` is a dev-dependency: its messages are what a failing test prints about a value.
        if rel.ends_with("tests.rs") || rel.starts_with("crates/al-testkit/") {
            continue;
        }
        let text = std::fs::read_to_string(f).unwrap();
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("#[cfg(test)]") {
                break;
            }
            let t = line.trim();
            if t.starts_with("//") || !(t.contains("?}") && (t.contains("{:") || t.contains(":?}"))) {
                continue;
            }
            if !(t.contains(":?}") || t.contains(":#?}")) {
                continue;
            }
            if KEPT.iter().any(|(file, l, _)| rel.ends_with(file) && t == *l) {
                continue;
            }
            found.push(format!("{rel}:{}: {t}", i + 1));
        }
    }
    assert!(
        found.is_empty(),
        "{} message(s) use the debug format; use `Describe::describe` or list the line in KEPT:\n{}",
        found.len(),
        found.join("\n")
    );
}
