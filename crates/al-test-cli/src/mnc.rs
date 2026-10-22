//! The must-not-compile corpus: `tests/must-not-compile/cases/NNNN-name.al` of the corpus repo.
//! Each case is a small AL file the real AL compiler rejects. Its header records the diagnostics
//! (`// @expect AL0161 | text`). The runtime has to refuse the case with the same codes
//! (`CLAUDE.md` rule 6, owner decision of 2026-10-14).
//!
//! The format is the corpus's `tests/must-not-compile/README.md`. A case is compiled alone, in a
//! one-app workspace. The dependencies it names (Base Application, System Application) are not
//! loaded: a verdict that needs them is "not checked", never a guess.

use al_sema::alcodes::{self, AppContext, AppTarget};
use al_sema::workspace::{AppSource, Diagnostic, Workspace};
use std::path::{Path, PathBuf};

/// One parsed case.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub name: String,
    pub source: String,
    /// (code, text) per expected diagnostic.
    pub expect: Vec<(String, String)>,
    pub target: AppTarget,
    pub depends: Vec<String>,
    /// Keys of the `@appjson` object and its `runtime`, if given.
    pub appjson_keys: Vec<String>,
    pub appjson_runtime: Option<(u32, u32)>,
    pub from: Option<u32>,
    pub until: Option<u32>,
    pub min_al: Option<u32>,
    /// The source folders of the corpus apps `depends` names, with the corpus apps they depend on, in dependency
    /// order (`read_cases` fills it; empty for a case parsed alone).
    pub corpus_deps: Vec<PathBuf>,
}

/// Parses the header (the leading run of `//` lines) of a case.
pub fn parse_case(name: &str, text: &str) -> Result<Case, String> {
    let mut case = Case {
        name: name.to_string(),
        source: text.to_string(),
        expect: Vec::new(),
        target: AppTarget::Cloud,
        depends: vec!["Base Application".into(), "System Application".into()],
        appjson_keys: Vec::new(),
        appjson_runtime: None,
        from: None,
        until: None,
        min_al: None,
        corpus_deps: Vec::new(),
    };
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let Some(rest) = l.strip_prefix("//") else { break };
        let Some(rest) = rest.trim_start().strip_prefix('@') else {
            continue;
        };
        let (key, value) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let value = value.trim();
        let number = |v: &str| {
            v.parse::<u32>()
                .map_err(|_| format!("{name}: @{key} needs a number, got `{v}`"))
        };
        match key {
            "expect" => {
                let (code, msg) = value.split_once('|').unwrap_or((value, ""));
                let code = code.trim();
                if !(code.len() == 6 && code.starts_with("AL") && code[2..].bytes().all(|b| b.is_ascii_digit())) {
                    return Err(format!("{name}: @expect needs an ALnnnn code, got `{code}`"));
                }
                case.expect.push((code.to_string(), msg.trim().to_string()));
            }
            "target" => {
                case.target = match value.to_ascii_lowercase().as_str() {
                    "cloud" => AppTarget::Cloud,
                    "onprem" => AppTarget::OnPrem,
                    "extension" => AppTarget::Extension,
                    "internal" => AppTarget::Internal,
                    other => return Err(format!("{name}: unknown @target `{other}`")),
                }
            }
            "depends" => case.depends = value.split(',').map(|d| d.trim().to_string()).collect(),
            "appjson" => {
                let v: serde_json::Value =
                    serde_json::from_str(value).map_err(|e| format!("{name}: @appjson is not JSON: {e}"))?;
                let obj = v
                    .as_object()
                    .ok_or_else(|| format!("{name}: @appjson must be an object"))?;
                case.appjson_keys = obj.keys().cloned().collect();
                case.appjson_runtime = obj.get("runtime").and_then(|r| r.as_str()).and_then(|r| {
                    let (a, b) = r.split_once('.')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                });
            }
            "from" => case.from = Some(number(value)?),
            "until" => case.until = Some(number(value)?),
            "min-al" => case.min_al = Some(number(value)?),
            _ => {}
        }
    }
    if case.expect.is_empty() {
        return Err(format!("{name}: no @expect line"));
    }
    Ok(case)
}

/// The cases of `dir` (the `cases` folder, or the folder that holds it), in file order.
pub fn read_cases(dir: &Path) -> Result<Vec<Case>, String> {
    let cases = if dir.join("cases").is_dir() {
        dir.join("cases")
    } else {
        dir.to_path_buf()
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&cases)
        .map_err(|e| format!("{}: {e}", cases.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "al"))
        .collect();
    files.sort();
    // The corpus apps sit next to the must-not-compile folder (`tests/al-language-xapp-a`, ...).
    let tests = cases.parent().and_then(Path::parent);
    files
        .iter()
        .map(|p| {
            let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let mut case = parse_case(&name, &text)?;
            if let Some(tests) = tests {
                case.corpus_deps = crate::deps::corpus_apps_named(tests, &case.depends)?;
            }
            Ok(case)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// We report exactly the expected codes.
    Match,
    /// We report other codes than the expected ones.
    Mismatch(String),
    /// A code of the case is one the front end cannot decide (`alcodes::NOT_CHECKED`), or is not
    /// implemented yet. The reason says which.
    NotChecked(String),
    /// The case does not apply to this BC major.
    Skipped(String),
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub name: String,
    pub verdict: Verdict,
    /// Our diagnostics as (code, text).
    pub got: Vec<(String, String)>,
    /// Same code, different text: reported, never a failure (the README: the text is a reference).
    pub text_notes: Vec<String>,
}

/// Compiles `case` alone and compares the AL diagnostics with the expected ones.
pub fn run_case(case: &Case, bc_major: u16) -> Outcome {
    let major = u32::from(bc_major);
    let mut out = Outcome {
        name: case.name.clone(),
        verdict: Verdict::Match,
        got: Vec::new(),
        text_notes: Vec::new(),
    };
    if case.from.is_some_and(|f| major < f) || case.until.is_some_and(|u| major > u) {
        let range = match (case.from, case.until) {
            (Some(f), Some(u)) => format!("BC {f} to {u}"),
            (Some(f), None) => format!("BC {f} and later"),
            (None, Some(u)) => format!("BC {u} and earlier"),
            (None, None) => "every BC".to_string(),
        };
        out.verdict = Verdict::Skipped(format!("the verdict holds for {range}, this is BC {major}"));
        return out;
    }
    let (_, _, diags) = compile(case, bc_major);
    let mut got: Vec<(String, String)> = diags.iter().map(|d| (d.code.to_string(), d.message.clone())).collect();
    got.sort();
    out.got = got.clone();
    let mut want = case.expect.clone();
    want.sort();
    let codes = |v: &[(String, String)]| v.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>();
    if codes(&got) != codes(&want) {
        // A code we cannot decide in general (AL0175 is reported only for Char - Text) makes the case "not
        // checked" when it was not found, even when another code of it is found: the verdict is about all of
        // the expected diagnostics.
        for (code, _) in &want {
            if let Some(why) = alcodes::not_checked_reason(code) {
                out.verdict = Verdict::NotChecked(format!("{code}: {why}"));
                return out;
            }
        }
        let list = |v: &[(String, String)]| v.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>().join(", ");
        out.verdict = Verdict::Mismatch(format!("expected [{}], got [{}]", list(&want), list(&got)));
        return out;
    }
    for ((_, w), (c, g)) in want.iter().zip(&got) {
        if !w.is_empty() && !g.starts_with(w.as_str()) {
            out.text_notes.push(format!("{c}: expected text `{w}`, ours `{g}`"));
        }
    }
    out
}

/// Compiles `case` alone at `bc_major`: the workspace, the check, and the AL diagnostics of the
/// checker and of the app context (unsorted).
fn compile(case: &Case, bc_major: u16) -> (Workspace, al_sema::check::CheckResult, Vec<Diagnostic>) {
    let major = u32::from(bc_major);
    // The case's app comes first (file 0 is the case); the corpus apps of `@depends` follow, from source. It depends
    // on the named ones only: an app sees what it declares (`al_sema::workspace`).
    let corpus_apps: Vec<AppSource> = case
        .corpus_deps
        .iter()
        .filter_map(|d| crate::runtime::load_app(d).ok())
        .collect();
    let dependencies = corpus_apps
        .iter()
        .filter(|a| case.depends.iter().any(|n| n.trim().eq_ignore_ascii_case(&a.name)))
        .map(|a| a.id.clone())
        .collect();
    let mut apps = vec![AppSource {
        id: "mnc".into(),
        name: "Must Not Compile by AL Language".into(),
        dependencies,
        files: vec![("src/case.al".into(), case.source.clone())],
    }];
    apps.extend(corpus_apps);
    let ws = Workspace::load(apps, &al_version::corpus_symbols(bc_major));
    let members = al_sema::members::build(&ws);
    // The corpus apps' runtime on this leg (`oracle.toml` app_runtime): 16.1, 17.1, 18.0, unless `@appjson` names one.
    let runtime = case.appjson_runtime.or_else(|| {
        let r = al_version::Runtime::max_for_bc(bc_major);
        Some((u32::from(r.major), u32::from(r.minor)))
    });
    // The builtins the app sees are those of its runtime: a newer one is AL0666.
    let catalog = match runtime.and_then(|(ma, mi)| Some((u16::try_from(ma).ok()?, u16::try_from(mi).ok()?))) {
        Some((major, minor)) => al_builtins::catalog::Catalog::for_target(
            &al_version::Target::for_bc(al_version::BcVersion::new(bc_major, 0))
                .with_runtime(al_version::Runtime { major, minor }),
        ),
        None => al_builtins::catalog::Catalog::load(),
    };
    let checked = al_sema::check::check(&ws, &members, &catalog);
    let ctx = AppContext {
        target: case.target,
        runtime,
        app_json_keys: case.appjson_keys.clone(),
        bc_major: Some(major),
    };
    let diags: Vec<Diagnostic> = checked
        .diagnostics
        .iter()
        .cloned()
        .chain(alcodes::check_context(&ws, 0, &ctx))
        // The case's own diagnostics: what the compiler says about the corpus apps is not the case's verdict.
        .filter(|d| d.code.starts_with("AL") && d.app == 0)
        .collect();
    (ws, checked, diags)
}

/// The builtin overloads a case that holds for every BC major pins as not compiling:
/// a call whose method the text of an expected diagnostic names in quotes (`'Prev' is not available
/// in runtime version` names `Prev`), and, when our codes match the case's, every call on a line
/// where we report one of them. Other calls of the file (a `Message` that only completes the code)
/// pin nothing. Site: `must-not-compile/cases/<name>.al`. 0561-testpage-prev.al pins `TestPage.Prev`.
pub fn pinned_calls(case: &Case, catalog: &al_builtins::catalog::Catalog) -> Vec<crate::coverage::Call> {
    // A case for some majors only (`@from`, `@until`: a member a later runtime has) leaves the
    // item's behavior on the other majors open, so it pins nothing.
    if case.from.is_some() || case.until.is_some() {
        return Vec::new();
    }
    let (ws, _, diags) = compile(case, al_version::MIN_BC_MAJOR);
    // Which overload a call means does not depend on the runtime: the newest catalog resolves a call the case's
    // runtime refuses (AL0666), so the refused overload is the one pinned.
    let checked = al_sema::check::check(&ws, &al_sema::members::build(&ws), catalog);
    let mut got: Vec<&str> = diags.iter().map(|d| d.code).collect();
    let mut want: Vec<&str> = case.expect.iter().map(|(c, _)| c.as_str()).collect();
    got.sort_unstable();
    want.sort_unstable();
    let text = ws.text(0);
    let line = |off: usize| text[..off.min(text.len())].matches('\n').count() + 1;
    let lines: std::collections::BTreeSet<usize> = if got == want {
        diags.iter().map(|d| line(d.offset)).collect()
    } else {
        Default::default()
    };
    // The quoted names of the expected texts: every second piece between single quotes.
    let named: Vec<String> = case
        .expect
        .iter()
        .flat_map(|(_, t)| t.split('\'').skip(1).step_by(2).map(str::to_ascii_lowercase))
        .collect();
    crate::coverage::collect_calls(&ws, &checked, catalog)
        .into_iter()
        .filter(|c| {
            lines.contains(&c.line)
                || catalog
                    .by_id(&c.id)
                    .is_some_and(|s| named.contains(&s.name.to_ascii_lowercase()))
        })
        .map(|c| crate::coverage::Call {
            file: format!("must-not-compile/cases/{}.al", case.name),
            ..c
        })
        .collect()
}

/// Totals over a run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub matched: usize,
    pub mismatched: usize,
    pub not_checked: usize,
    pub skipped: usize,
}

impl Summary {
    pub fn of(outcomes: &[Outcome]) -> Summary {
        let mut s = Summary::default();
        for o in outcomes {
            match o.verdict {
                Verdict::Match => s.matched += 1,
                Verdict::Mismatch(_) => s.mismatched += 1,
                Verdict::NotChecked(_) => s.not_checked += 1,
                Verdict::Skipped(_) => s.skipped += 1,
            }
        }
        s
    }
}

/// The report: one line per case, then the totals.
pub fn render(outcomes: &[Outcome], bc_major: u16) -> String {
    let mut s = String::new();
    for o in outcomes {
        let (tag, why) = match &o.verdict {
            Verdict::Match => ("match", String::new()),
            Verdict::Mismatch(w) => ("MISMATCH", w.clone()),
            Verdict::NotChecked(w) => ("not checked", w.clone()),
            Verdict::Skipped(w) => ("skipped", w.clone()),
        };
        s.push_str(&format!(
            "{:<13} {}{}\n",
            tag,
            o.name,
            if why.is_empty() {
                String::new()
            } else {
                format!(": {why}")
            }
        ));
        for n in &o.text_notes {
            s.push_str(&format!("              text differs, {n}\n"));
        }
    }
    let t = Summary::of(outcomes);
    s.push_str(&format!(
        "Must-not-compile, BC {bc_major}: {} of {} cases match, {} mismatch, {} not checked, {} skipped\n",
        t.matched,
        outcomes.len(),
        t.mismatched,
        t.not_checked,
        t.skipped
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASE: &str = "// @expect AL0162 | 'OnInit' is not a valid trigger\n// A human line.\n// @target OnPrem\n// @from 27\n\ntable 1 T { }\n";

    #[test]
    fn the_header_is_parsed() {
        let c = parse_case("0004-x", CASE).unwrap();
        assert_eq!(
            c.expect,
            vec![("AL0162".to_string(), "'OnInit' is not a valid trigger".to_string())]
        );
        assert_eq!(c.target, AppTarget::OnPrem);
        assert_eq!(c.from, Some(27));
        assert_eq!(c.until, None);
    }

    #[test]
    fn a_case_without_expect_is_an_error() {
        assert!(parse_case("x", "// nothing\ntable 1 T { }").is_err());
        assert!(parse_case("x", "// @expect BAD | t\n").is_err());
    }

    #[test]
    fn appjson_gives_keys_and_runtime() {
        let c = parse_case(
            "x",
            "// @expect AL0124 | t\n// @appjson {\"runtime\": \"15.2\", \"assemblyProbingPaths\": [\"./x\"]}\n",
        )
        .unwrap();
        assert_eq!(c.appjson_runtime, Some((15, 2)));
        assert!(c.appjson_keys.contains(&"assemblyProbingPaths".to_string()));
    }

    #[test]
    fn from_and_until_skip_a_major() {
        let c = parse_case(
            "x",
            "// @expect AL0162 | t\n// @from 28\n// @until 28\ncodeunit 1 C { }",
        )
        .unwrap();
        assert!(matches!(run_case(&c, 27).verdict, Verdict::Skipped(_)));
        assert!(matches!(run_case(&c, 29).verdict, Verdict::Skipped(_)));
        assert!(!matches!(run_case(&c, 28).verdict, Verdict::Skipped(_)));
    }

    #[test]
    fn a_missing_diagnostic_is_a_mismatch_and_an_undecidable_code_is_not_checked() {
        let c = parse_case("x", "// @expect AL0162 | t\ncodeunit 50000 C { }").unwrap();
        assert!(matches!(run_case(&c, 27).verdict, Verdict::Mismatch(_)));
        let c = parse_case("x", "// @expect AL0175 | t\ncodeunit 50000 C { }").unwrap();
        assert!(matches!(run_case(&c, 27).verdict, Verdict::NotChecked(_)));
        let c = parse_case("x", "// @expect AL9999 | t\ncodeunit 50000 C { }").unwrap();
        assert!(matches!(run_case(&c, 27).verdict, Verdict::NotChecked(_)));
    }

    #[test]
    fn a_found_diagnostic_matches_and_a_wording_difference_is_only_a_note() {
        let src = "// @expect AL0197 | An application object of type 'Codeunit' with name 'C' is already declared by the extension\n// @expect AL0197 | other wording\ncodeunit 50000 C { }\ncodeunit 50001 C { }";
        let o = run_case(&parse_case("x", src).unwrap(), 27);
        assert_eq!(o.verdict, Verdict::Match);
        assert_eq!(o.text_notes.len(), 1, "{:?}", o.text_notes);
    }

    /// `@depends` may name corpus apps (the cross-app fixtures: corpus CONTRIBUTING.md "Multi-app fixtures",
    /// must-not-compile 1580 and 1581). They are loaded from their source folders next to the must-not-compile folder,
    /// with the corpus apps they depend on, and the case's app depends on the named ones only.
    #[test]
    fn a_case_that_depends_on_corpus_apps_compiles_against_their_source() {
        let tmp = tempfile::tempdir().unwrap();
        let tests = tmp.path().join("tests");
        let app = |dir: &str, json: &str, al: &str| {
            let d = tests.join(dir);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("app.json"), json).unwrap();
            std::fs::write(d.join("o.al"), al).unwrap();
        };
        app(
            "xapp-a",
            r#"{"id":"aaaaaaaa-0000-0000-0000-000000000001","name":"XApp A","publisher":"P","version":"1.0.0.0","dependencies":[]}"#,
            r#"table 61101 "XA Base" { fields { field(1; "No."; Code[20]) { } } }"#,
        );
        app(
            "xapp-b",
            r#"{"id":"bbbbbbbb-0000-0000-0000-000000000002","name":"XApp B","publisher":"P","version":"1.0.0.0","dependencies":[{"id":"aaaaaaaa-0000-0000-0000-000000000001","name":"XApp A","publisher":"P","version":"1.0.0.0"}]}"#,
            r#"codeunit 61110 "XB C" { }"#,
        );
        let cases = tests.join("must-not-compile/cases");
        std::fs::create_dir_all(&cases).unwrap();
        let case = |depends: &str| {
            format!(
                "// @expect AL0132 | 'Record \"XA Base\"' does not contain a definition for 'NoSuch'\n// @depends {depends}\n\
                 codeunit 51580 C {{ procedure P() var R: Record \"XA Base\"; begin R.NoSuch := 1; end; }}\n"
            )
        };
        std::fs::write(cases.join("1580-x.al"), case("XApp B")).unwrap();
        let read = read_cases(&tests.join("must-not-compile")).unwrap();
        assert_eq!(read[0].corpus_deps, [tests.join("xapp-a"), tests.join("xapp-b")]);
        std::fs::write(cases.join("1580-x.al"), case("XApp A,XApp B")).unwrap();
        let read = read_cases(&cases).unwrap();
        let o = run_case(&read[0], 29);
        assert_eq!(o.verdict, Verdict::Match, "{:?}", o.got);
        assert!(o.text_notes.is_empty(), "{:?}", o.text_notes);
    }

    /// A case pins the builtin call on the line of the diagnostic it expects (corpus
    /// must-not-compile/cases/0561-testpage-prev.al), not the other calls of the file.
    #[test]
    fn a_case_pins_the_call_on_the_line_of_its_expected_diagnostic() {
        let case = parse_case(
            "0561-testpage-prev",
            "// @expect AL0666 | 'Prev' is not available in runtime version '
codeunit 50561 \"MNC O 0561\"
{
    procedure Probe()
    var Pg: TestPage \"Customer List\";
    begin
        Pg.Prev();
        Message('x');
    end;
}
",
        )
        .unwrap();
        let catalog = al_builtins::catalog::Catalog::load();
        let calls = pinned_calls(&case, &catalog);
        let ids: Vec<&str> = calls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["TestPage.Prev#1"]);
        assert_eq!(calls[0].file, "must-not-compile/cases/0561-testpage-prev.al");
        assert_eq!(calls[0].line, 7);
    }
}
