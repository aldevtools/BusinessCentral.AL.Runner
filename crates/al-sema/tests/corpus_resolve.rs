//! The corpus as a workspace: the test app plus the internals fixture it depends on, without
//! the Microsoft apps. The references that resolve only with them are checked in
//! `al-test-cli/tests/corpus_resolve_deps.rs`.

use al_sema::{AppSource, Workspace};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn al_files(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p: PathBuf = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "al") {
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
                out.push((rel, std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn app(dir: &Path) -> AppSource {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("app.json")).unwrap()).unwrap();
    AppSource {
        id: manifest["id"].as_str().unwrap().into(),
        name: manifest["name"].as_str().unwrap().into(),
        dependencies: manifest["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["id"].as_str().unwrap().to_string())
            .collect(),
        files: al_files(dir),
    }
}

pub fn corpus() -> Workspace {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests");
    assert!(
        root.join("al-language/app.json").exists(),
        "corpus submodule not checked out"
    );
    Workspace::load(corpus_apps(&root), &al_version::corpus_symbols(27))
}

/// `al-language` and the corpus apps it depends on (the internals fixture and the cross-app fixtures), dependencies
/// first, as CI publishes them (the same rule as `al_testrun::deps::corpus_source_dirs`, which al-sema cannot use).
fn corpus_apps(root: &Path) -> Vec<AppSource> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("app.json").is_file())
        .collect();
    dirs.sort();
    let neighbors: Vec<AppSource> = dirs.iter().map(|d| app(d)).collect();
    fn visit(at: usize, all: &[AppSource], done: &mut Vec<usize>) {
        if done.contains(&at) {
            return;
        }
        for d in &all[at].dependencies {
            if let Some(next) = all.iter().position(|a| a.id.eq_ignore_ascii_case(d)) {
                visit(next, all, done);
            }
        }
        done.push(at);
    }
    let main = dirs.iter().position(|d| d.ends_with("al-language")).unwrap();
    let mut order = Vec::new();
    visit(main, &neighbors, &mut order);
    let mut neighbors: Vec<Option<AppSource>> = neighbors.into_iter().map(Some).collect();
    order.into_iter().map(|i| neighbors[i].take().unwrap()).collect()
}

#[test]
fn every_corpus_table_field_has_a_known_type() {
    let ws = corpus();
    let members = al_sema::members::build(&ws);
    let mut unknown = Vec::new();
    let mut fields = 0;
    for (i, m) in members.iter().enumerate() {
        for f in m.fields.iter().filter(|f| f.declared_in == Some(i)) {
            fields += 1;
            if f.ty == al_sema::types::Ty::Unknown {
                unknown.push(format!("{} :: {}", ws.objects[i].name, f.name));
            }
        }
    }
    assert!(fields > 300, "only {fields} fields found");
    assert!(
        unknown.is_empty(),
        "{} field(s) with unknown type:\n{}",
        unknown.len(),
        unknown.join("\n")
    );
}

/// Corpus files with model gaps that wait for a parser or resolver fix. Each is a red corpus test,
/// not a pass. A file with no gap left fails the test below, so the list only shrinks (ADR-007).
const KNOWN_GAP_FILES: &[(&str, &str)] = &[
    (
        "pages/lt-ap-testpage-642/ApTestPageTests.al",
        "the checker takes a part control named `Page` (Host.PartF.\"Page\") for the page of the part (corpus bump 27)",
    ),
    (
        "misc/lt-inferred-behaviors/P69PartPageControl.al",
        "the checker does not resolve a member of a part page's control (corpus bump 11)",
    ),
    (
        "misc/lt-probes-needed-6/PnF18ThisInTable.al",
        "the checker does not resolve `this` inside a table (corpus bump 11)",
    ),
    (
        "codeunit/lt-procedure-assignment/ProcedureAssignmentOneParameter.al",
        "the checker takes an assignment to a one-parameter procedure for a call without arguments (corpus bump 14)",
    ),
    (
        "report/lt-report-this-passed-on/P141ThisEvent.al",
        "the checker does not resolve `this` in a report (corpus bump 14)",
    ),
    (
        "report/lt-report-this-passed-on/P141ThisStored.al",
        "the checker does not resolve `this` in a report (corpus bump 14)",
    ),
    (
        "report/lt-report-this-passed-on/P141ThisValueParam.al",
        "the checker does not resolve `this` in a report (corpus bump 14)",
    ),
    (
        "report/lt-requestpage-savevalues-date-text-setvalue/P147SetValueAssign.al",
        "the checker does not read `SetValue :=` on a request page control as an assignment (corpus bump 14)",
    ),
    (
        "report/lt-report-this-passed-on/P141ThisManual.al",
        "the checker does not resolve `this` in a report (corpus bump 14)",
    ),
    (
        "report/lt-report-this-passed-on/P141ThisReportExt.al",
        "the checker does not resolve `this` in a report extension (corpus bump 14)",
    ),
    (
        "record/lt-bm-builtin-name-procedures/BmBuiltinNameProcedures.al",
        "a table procedure Insert() or Get() hides the built-in overloads with arguments (Insert(true), Get(2)) (corpus bump 28)",
    ),
    (
        "report/lt-ch-report-leads/ChReportPart.al",
        "the checker does not know a part of a report's request page (RequestOptionsPage.ChPart.Page) (corpus bump 29)",
    ),
];

/// Milestone 1: every name, member and call in every corpus body resolves,
/// except references into apps outside the workspace (golden list above).
#[test]
fn corpus_bodies_resolve_with_no_model_gaps() {
    let ws = corpus();
    let members = al_sema::members::build(&ws);
    let r = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let all: Vec<_> = r.diagnostics.iter().filter(|d| d.code.starts_with("AR1")).collect();
    let stale: Vec<_> = KNOWN_GAP_FILES
        .iter()
        .filter(|(k, _)| !all.iter().any(|d| d.file.ends_with(k)))
        .map(|(k, why)| format!("{k} ({why})"))
        .collect();
    assert!(
        stale.is_empty(),
        "these files have no gap now: remove them from KNOWN_GAP_FILES:\n{}",
        stale.join("\n")
    );
    let gaps: Vec<String> = all
        .iter()
        .filter(|d| !KNOWN_GAP_FILES.iter().any(|(k, _)| d.file.ends_with(k)))
        .map(|d| format!("{} {}: {}", d.code, d.file, d.message))
        .collect();
    assert!(
        gaps.is_empty(),
        "{} model gap(s):\n{}",
        gaps.len(),
        gaps.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// Prints the resolver's diagnostics over the corpus (run with --nocapture).
#[test]
#[ignore = "exploration aid, not a gate"]
fn explore_body_resolution() {
    let ws = corpus();
    let members = al_sema::members::build(&ws);
    let r = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let mut by_msg: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_code: BTreeMap<&str, usize> = BTreeMap::new();
    for d in &r.diagnostics {
        *by_code.entry(d.code).or_default() += 1;
        *by_msg.entry(format!("{} {}", d.code, d.message)).or_default() += 1;
    }
    println!("{by_code:?}");
    let mut v: Vec<_> = by_msg.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (m, n) in v.iter().take(80) {
        println!("{n:5} {m}");
    }
    if let Ok(pat) = std::env::var("SHOW") {
        for d in r.diagnostics.iter().filter(|d| d.message.contains(&pat)).take(6) {
            let f = ws.files.iter().find(|f| f.path == d.file).unwrap();
            let src =
                al_syntax::SyntaxNode::new_root(ws.green(ws.files.iter().position(|g| g.path == f.path).unwrap()))
                    .text()
                    .to_string();
            let line = src[..d.offset].matches('\n').count() + 1;
            let text = src.lines().nth(line - 1).unwrap_or("").trim();
            println!("  {}:{line}: {text}", d.file);
        }
    }
}
