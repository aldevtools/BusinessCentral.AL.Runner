//! `--coverage`: the Cobertura file and the JSON of the statements each test ran.

use al_testrun::coverage::CoverageStatement;
use al_testrun::runtime::TestResult;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// A source file as the outputs name it: absolute with forward slashes (JSON) and relative to the working directory
/// when it lies below it (Cobertura).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label {
    pub absolute: String,
    pub relative: String,
}

fn slashes(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// The label of `file` of the app at `app_path`: a folder app holds the file at `app_path/file`; a package holds it
/// inside, written `package!/file`.
pub fn label(app_path: &std::path::Path, file: &str, cwd: &std::path::Path) -> Label {
    let app_abs = std::path::absolute(app_path).unwrap_or_else(|_| app_path.to_path_buf());
    let absolute = if app_path.is_dir() {
        slashes(&app_abs.join(file))
    } else {
        format!("{}!/{}", slashes(&app_abs), file)
    };
    let cwd_text = slashes(cwd);
    let relative = absolute
        .strip_prefix(&format!("{}/", cwd_text.trim_end_matches('/')))
        .map_or_else(|| absolute.clone(), String::from);
    Label { absolute, relative }
}

/// What the results ran, per statement: how often.
fn totals(results: &[&TestResult]) -> BTreeMap<u32, u64> {
    let mut m = BTreeMap::new();
    for r in results {
        for (id, n) in &r.coverage {
            *m.entry(*id).or_insert(0) += u64::from(*n);
        }
    }
    m
}

/// Cobertura XML: a package per app, a class per source file, a line per source line that starts a statement. The hits of
/// a line are the runs of the statements that start on it.
pub fn cobertura(
    statements: &[CoverageStatement],
    results: &[&TestResult],
    app_names: &[String],
    labels: &dyn Fn(usize, &str) -> Label,
    cwd: &str,
) -> String {
    let hits = totals(results);
    // package (app) -> file label -> line -> hits
    let mut tree: BTreeMap<usize, BTreeMap<Label, BTreeMap<u32, u64>>> = BTreeMap::new();
    for s in statements {
        let n = hits.get(&s.pos).copied().unwrap_or(0);
        *tree
            .entry(s.app)
            .or_default()
            .entry(labels(s.app, &s.file))
            .or_default()
            .entry(s.line)
            .or_insert(0) += n;
    }
    let (mut valid, mut covered) = (0usize, 0usize);
    let mut body = String::new();
    for (app, files) in &tree {
        let (mut pv, mut pc) = (0usize, 0usize);
        let mut classes = String::new();
        for (label, lines) in files {
            let v = lines.len();
            let c = lines.values().filter(|n| **n > 0).count();
            pv += v;
            pc += c;
            classes.push_str(&format!(
                "        <class name=\"{}\" filename=\"{}\" line-rate=\"{}\" branch-rate=\"0\" complexity=\"0\">\n          <methods />\n          <lines>\n",
                esc(label.relative.rsplit('/').next().unwrap_or("")),
                esc(&label.relative),
                rate(c, v)
            ));
            for (line, n) in lines {
                classes.push_str(&format!(
                    "            <line number=\"{line}\" hits=\"{n}\" branch=\"false\" />\n"
                ));
            }
            classes.push_str("          </lines>\n        </class>\n");
        }
        valid += pv;
        covered += pc;
        body.push_str(&format!(
            "    <package name=\"{}\" line-rate=\"{}\" branch-rate=\"0\" complexity=\"0\">\n      <classes>\n{classes}      </classes>\n    </package>\n",
            esc(app_names.get(*app).map_or("", String::as_str)),
            rate(pc, pv)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<coverage line-rate=\"{}\" branch-rate=\"0\" lines-covered=\"{covered}\" lines-valid=\"{valid}\" branches-covered=\"0\" branches-valid=\"0\" complexity=\"0\" version=\"al-runner\" timestamp=\"0\">\n  <sources>\n    <source>{}</source>\n  </sources>\n  <packages>\n{body}  </packages>\n</coverage>\n",
        rate(covered, valid),
        esc(cwd)
    )
}

fn rate(covered: usize, valid: usize) -> String {
    if valid == 0 {
        "1".to_string()
    } else {
        format!("{:.4}", covered as f64 / valid as f64)
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn statement_json(s: &CoverageStatement, hits: u64) -> Value {
    json!({
        "id": s.pos,
        "scope": s.scope,
        "line": s.line,
        "column": s.column,
        "endLine": s.end_line,
        "hits": hits,
    })
}

/// Statements of `statements` grouped by file, as the `coverage` array of AL Runner v2's summary: `[{file, statements}]`.
fn by_file(
    statements: &[&CoverageStatement],
    hits: &dyn Fn(u32) -> u64,
    labels: &dyn Fn(usize, &str) -> Label,
) -> Vec<Value> {
    let mut files: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for s in statements {
        files
            .entry(labels(s.app, &s.file).absolute)
            .or_default()
            .push(statement_json(s, hits(s.pos)));
    }
    files
        .into_iter()
        .map(|(file, statements)| json!({ "file": file, "statements": statements }))
        .collect()
}

/// `{ "coverage": [...every statement with its runs...], "perTestCoverage": [{test, coverage: [...statements it ran...]}] }`,
/// the shapes of AL Runner v2's `runTests` summary. File names are absolute with forward slashes. v2 also sent `endColumn`;
/// v3 does not know it.
pub fn json(
    statements: &[CoverageStatement],
    results: &[&TestResult],
    labels: &dyn Fn(usize, &str) -> Label,
) -> String {
    let total = totals(results);
    let all: Vec<&CoverageStatement> = statements.iter().collect();
    let coverage = by_file(&all, &|id| total.get(&id).copied().unwrap_or(0), labels);
    let per_test: Vec<Value> = results
        .iter()
        .filter(|r| !r.coverage.is_empty())
        .map(|r| {
            let ran: BTreeMap<u32, u32> = r.coverage.iter().copied().collect();
            let own: Vec<&CoverageStatement> = statements.iter().filter(|s| ran.contains_key(&s.pos)).collect();
            json!({
                "test": format!("Codeunit{}.{}", r.codeunit, r.test),
                "coverage": by_file(&own, &|id| u64::from(ran.get(&id).copied().unwrap_or(0)), labels),
            })
        })
        .collect();
    serde_json::to_string_pretty(&json!({ "coverage": coverage, "perTestCoverage": per_test }))
        .expect("a JSON value serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_testrun::Outcome;

    fn st(pos: u32, line: u32, scope: &str) -> CoverageStatement {
        CoverageStatement {
            pos,
            app: 0,
            file: "src/T.al".into(),
            line,
            column: 9,
            end_line: line,
            scope: scope.into(),
            object: "T".into(),
        }
    }

    fn res(test: &str, coverage: Vec<(u32, u32)>) -> TestResult {
        TestResult {
            suite: 0,
            app: 0,
            codeunit: 7,
            codeunit_name: "T".into(),
            test: test.into(),
            outcome: Outcome::Pass,
            call_stack: String::new(),
            assumptions: Vec::new(),
            time: std::time::Duration::ZERO,
            coverage,
            messages: Vec::new(),
            captured: None,
            impact: Vec::new(),
            stubs: Vec::new(),
            slept_ms: 0,
            entered: Vec::new(),
            dotnet: false,
        }
    }

    fn lab(_: usize, f: &str) -> Label {
        Label {
            absolute: format!("/w/app/{f}"),
            relative: format!("app/{f}"),
        }
    }

    #[test]
    fn cobertura_has_a_class_per_file_and_a_line_per_source_line_with_its_runs() {
        let statements = [st(0, 3, "A"), st(1, 4, "A"), st(2, 4, "A"), st(3, 9, "B")];
        let a = res("A", vec![(0, 2), (2, 1)]);
        let b = res("B", vec![(0, 1)]);
        let x = cobertura(&statements, &[&a, &b], &["App".into()], &lab, "/w");
        assert!(
            x.contains("<class name=\"T.al\" filename=\"app/src/T.al\" line-rate=\"0.6667\""),
            "{x}"
        );
        assert!(x.contains("<line number=\"3\" hits=\"3\" branch=\"false\" />"), "{x}");
        assert!(
            x.contains("<line number=\"4\" hits=\"1\""),
            "two statements on a line add up: {x}"
        );
        assert!(x.contains("<line number=\"9\" hits=\"0\""), "{x}");
        assert!(
            x.contains("lines-valid=\"3\"") && x.contains("lines-covered=\"2\""),
            "{x}"
        );
        assert!(x.contains("<package name=\"App\""), "{x}");
    }

    #[test]
    fn json_has_the_statements_with_their_runs_and_each_tests_own() {
        let statements = [st(0, 3, "A"), st(1, 4, "B")];
        let a = res("TestA", vec![(0, 2)]);
        let b = res("TestB", vec![(0, 1), (1, 1)]);
        let v: Value = serde_json::from_str(&json(&statements, &[&a, &b], &lab)).unwrap();
        let all = &v["coverage"][0];
        assert_eq!(all["file"], "/w/app/src/T.al");
        assert_eq!(all["statements"][0]["hits"], 3);
        assert_eq!(all["statements"][1]["hits"], 1);
        assert_eq!(all["statements"][1]["scope"], "B");
        let t = &v["perTestCoverage"];
        assert_eq!(t[0]["test"], "Codeunit7.TestA");
        assert_eq!(t[0]["coverage"][0]["statements"].as_array().unwrap().len(), 1);
        assert_eq!(t[1]["coverage"][0]["statements"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn labels_are_absolute_with_forward_slashes_and_relative_below_the_working_directory() {
        let dir = tempfile::tempdir().unwrap();
        let l = label(dir.path(), "src/T.al", dir.path());
        assert!(l.absolute.ends_with("/src/T.al") && !l.absolute.contains('\\'), "{l:?}");
        assert_eq!(l.relative, "src/T.al");
        let elsewhere = label(dir.path(), "src/T.al", std::path::Path::new("/nowhere"));
        assert_eq!(elsewhere.relative, elsewhere.absolute);
        let pkg = dir.path().join("x.zip");
        std::fs::write(&pkg, b"").unwrap();
        assert!(label(&pkg, "a/B.al", dir.path()).absolute.ends_with("x.zip!/a/B.al"));
    }
}
