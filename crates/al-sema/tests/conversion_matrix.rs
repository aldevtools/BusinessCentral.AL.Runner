//! The compile matrix of AL implicit conversions: for every context (assignment, compound assignment, by-value and var
//! argument, exit, comparison, arithmetic, in-set, case label, conditional expression) and every (source, target) type
//! pair, whether the AL compiler accepts one line of AL, or which diagnostic codes it reports on that line.
//!
//! The fixture `fixtures/conversion-matrix/compile-matrix.csv` is our own generated data, not Microsoft code: a copy of
//! `scripts/conversion-matrix/compile-matrix.csv` of the corpus repo (BusinessCentral.AL.Language.Tests, branch
//! `al-runtime-rs-staging-y31`, commit 5dd356d4), measured with the AL compiler 30.0.44.47290-beta; the verdicts are the
//! same for runtime 16.1, 17.1 and 18.0. `compile-messages.csv` next to it holds the compiler's message of the first
//! three cells of every (context, code). The source of every cell is built here exactly as the corpus generator
//! `scripts/conversion-matrix/gen.py` builds it (one codeunit per context; a cell is one line), so a cell's line here is
//! the line alc judged.
//!
//! The test compares our verdict per cell with alc's: accepted (no error on the line; a warning such as AL0603 is not
//! reported by us and not compared), or the set of error codes. The cells we do not match yet are listed in
//! `fixtures/conversion-matrix/known-mismatches.txt` (`context,source,target,alc,ours`). The list only shrinks: a listed
//! cell that now matches fails the test too, so the line is removed with the fix. `UPDATE_CONVERSION_MATRIX=1` rewrites
//! the list; `CONVERSION_MATRIX_REPORT=<file>` writes every mismatch grouped by kind.

use al_sema::workspace::{AppSource, Workspace};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conversion-matrix");

const VARS: &[(&str, &str)] = &[
    ("Integer", "Integer"),
    ("BigInteger", "BigInteger"),
    ("Decimal", "Decimal"),
    ("Boolean", "Boolean"),
    ("Char", "Char"),
    ("Byte", "Byte"),
    ("Option", "Option A,B,C"),
    ("Option2", "Option X,Y,Z"),
    ("Enum", "Enum \"Y31 E\""),
    ("Enum2", "Enum \"Y31 E2\""),
    ("EnumImpl", "Enum \"Y31 EI\""),
    ("Text", "Text"),
    ("Text10", "Text[10]"),
    ("Text3", "Text[3]"),
    ("Code250", "Code[250]"),
    ("Code10", "Code[10]"),
    ("Code3", "Code[3]"),
    ("Label", "Label 'abc'"),
    ("TextConst", "TextConst ENU = 'abc'"),
    ("Date", "Date"),
    ("Time", "Time"),
    ("DateTime", "DateTime"),
    ("Duration", "Duration"),
    ("DateFormula", "DateFormula"),
    ("Guid", "Guid"),
    ("RecordId", "RecordId"),
    ("Variant", "Variant"),
    ("TextBuilder", "TextBuilder"),
    ("Interface", "Interface \"Y31 I\""),
];

const FIELDS: &[&str] = &[
    "F.Integer",
    "F.BigInteger",
    "F.Decimal",
    "F.Boolean",
    "F.Option",
    "F.Enum",
    "F.Text10",
    "F.Code10",
    "F.Date",
    "F.Time",
    "F.DateTime",
    "F.Duration",
    "F.DateFormula",
    "F.Guid",
    "F.RecordId",
    "F.Media",
    "F.MediaSet",
];

const LITERALS: &[(&str, &str)] = &[
    ("lit.Integer", "7"),
    ("lit.BigInteger", "10000000000L"),
    ("lit.Decimal", "1.5"),
    ("lit.Char1", "'a'"),
    ("lit.Text", "'abc'"),
    ("lit.TextEmpty", "''"),
    ("lit.Boolean", "true"),
    ("lit.Date", "20260131D"),
    ("lit.Time", "120000T"),
    ("lit.DateTime", "0DT"),
    ("lit.Enum", "\"Y31 E\"::A"),
    ("lit.OptionMember", "vOption::B"),
];

const EXTRA_SOURCES: &[(&str, &str)] = &[("TestPage.Value", "TP.FInt.Value"), ("MediaId", "R.Media.MediaId()")];

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// The targets are l-values: variables, fields and the TestPage control value.
    LValue,
    /// The targets are declarable types (parameters, return values): the variables but Label and TextConst.
    Type,
    /// The targets are l-values used as an expression (comparison, arithmetic, ...).
    Expr,
}

const CONTEXTS: &[(&str, Kind, &str)] = &[
    ("assign", Kind::LValue, "{t} := {s};"),
    ("compound_add", Kind::LValue, "{t} += {s};"),
    ("arg_byvalue", Kind::Type, "PA_{p}({s});"),
    ("arg_var", Kind::Type, "PV_{p}({s});"),
    ("exit", Kind::Type, ""),
    ("eq", Kind::Expr, "if {s} = {t} then;"),
    ("ne", Kind::Expr, "if {s} <> {t} then;"),
    ("lt", Kind::Expr, "if {s} < {t} then;"),
    ("add", Kind::Expr, "Message('%1', {s} + {t});"),
    ("sub", Kind::Expr, "Message('%1', {s} - {t});"),
    ("in_set", Kind::Expr, "if {s} in [{t}] then;"),
    ("case_label", Kind::Expr, "case {s} of {t}: ; end;"),
    ("cond_branches", Kind::Expr, "Message('%1', vBoolCond ? {s} : {t});"),
];

const HEADER_OBJECTS: &str = r#"
interface "Y31 I"
{
    procedure Name(): Text;
}
enum 92400 "Y31 E"
{
    Extensible = true;
    value(0; A) { }
    value(1; B) { }
    value(2; C) { }
}
enum 92401 "Y31 E2"
{
    value(0; X) { }
    value(1; Y) { }
}
enum 92402 "Y31 EI" implements "Y31 I"
{
    value(0; A) { Implementation = "Y31 I" = "Y31 Impl"; }
}
codeunit 92402 "Y31 Impl" implements "Y31 I"
{
    procedure Name(): Text
    begin
        exit('impl');
    end;
}
table 92400 "Y31 Fields"
{
    fields
    {
        field(1; Integer; Integer) { }
        field(2; BigInteger; BigInteger) { }
        field(3; Decimal; Decimal) { }
        field(4; Boolean; Boolean) { }
        field(5; Option; Option) { OptionMembers = A,B,C; }
        field(6; Enum; Enum "Y31 E") { }
        field(7; Text10; Text[10]) { }
        field(8; Code10; Code[10]) { }
        field(9; Date; Date) { }
        field(10; Time; Time) { }
        field(11; DateTime; DateTime) { }
        field(12; Duration; Duration) { }
        field(13; DateFormula; DateFormula) { }
        field(14; Guid; Guid) { }
        field(15; RecordId; RecordId) { }
        field(16; Media; Media) { }
        field(17; MediaSet; MediaSet) { }
    }
    keys { key(PK; Integer) { } }
}
page 92400 "Y31 Card"
{
    PageType = Card;
    SourceTable = "Y31 Fields";
    layout
    {
        area(Content)
        {
            field(FInt; Rec.Integer) { ApplicationArea = All; }
        }
    }
}
"#;

fn var_name(n: &str) -> String {
    format!("v{n}")
}

fn fld_expr(n: &str) -> String {
    format!("R.\"{}\"", n.split_once('.').unwrap().1)
}

fn sources() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = VARS.iter().map(|(n, _)| (n.to_string(), var_name(n))).collect();
    out.extend(FIELDS.iter().map(|n| (n.to_string(), fld_expr(n))));
    out.extend(LITERALS.iter().map(|(n, e)| (n.to_string(), e.to_string())));
    out.extend(EXTRA_SOURCES.iter().map(|(n, e)| (n.to_string(), e.to_string())));
    out
}

/// (name, expression, declared type); the declared type of a field target is not needed.
fn targets(kind: Kind) -> Vec<(String, String, String)> {
    match kind {
        Kind::Type => VARS
            .iter()
            .filter(|(n, _)| !matches!(*n, "Label" | "TextConst"))
            .map(|(n, d)| (n.to_string(), var_name(n), d.to_string()))
            .collect(),
        Kind::LValue | Kind::Expr => {
            let mut out: Vec<_> = VARS
                .iter()
                .map(|(n, d)| (n.to_string(), var_name(n), d.to_string()))
                .collect();
            out.extend(FIELDS.iter().map(|n| (n.to_string(), fld_expr(n), String::new())));
            out.push(("TestPage.Value".into(), "TP.FInt.Value".into(), String::new()));
            out
        }
    }
}

fn safe(n: &str) -> String {
    n.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// The codeunit of one context and the (source, target) of each cell by its 1-based line.
fn build_context(ctx: &str, kind: Kind, tmpl: &str) -> (String, BTreeMap<usize, (String, String)>) {
    let mut lines: Vec<String> = vec![
        "codeunit 92401 \"Y31 Matrix\"".into(),
        "{".into(),
        "    Subtype = Test;".into(),
        "    var".into(),
    ];
    for (n, d) in VARS {
        lines.push(format!("        {}: {};", var_name(n), d));
    }
    lines.push("        vBoolCond: Boolean;".into());
    lines.push("        R: Record \"Y31 Fields\";".into());
    lines.push("        TP: TestPage \"Y31 Card\";".into());
    let mut cells = BTreeMap::new();
    let ts = targets(kind);
    let srcs = sources();
    if ctx == "exit" {
        let mut k = 0;
        for (tn, _, td) in &ts {
            for (sn, se) in &srcs {
                k += 1;
                lines.push(format!("    local procedure E{k}(): {td} begin exit({se}); end;"));
                cells.insert(lines.len(), (sn.clone(), tn.clone()));
            }
        }
    } else {
        lines.push("    procedure Cells()".into());
        lines.push("    begin".into());
        for (tn, te, _) in &ts {
            for (sn, se) in &srcs {
                let stmt = tmpl.replace("{s}", se).replace("{t}", te).replace("{p}", &safe(tn));
                lines.push(format!("        {stmt}"));
                cells.insert(lines.len(), (sn.clone(), tn.clone()));
            }
        }
        lines.push("    end;".into());
        if kind == Kind::Type {
            for (tn, _, td) in &ts {
                lines.push(format!("    local procedure PA_{}(p: {td}) begin end;", safe(tn)));
                lines.push(format!("    local procedure PV_{}(var p: {td}) begin end;", safe(tn)));
            }
        }
    }
    lines.push("}".into());
    (lines.join("\n") + "\n", cells)
}

/// alc's verdict per (context, source, target): `ok`, or the error codes joined by `+` (a warning is dropped).
fn expected() -> BTreeMap<(String, String, String), String> {
    let text = std::fs::read_to_string(format!("{FIXTURES}/compile-matrix.csv")).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let mut out = BTreeMap::new();
    for line in lines {
        let cols: Vec<&str> = line.split(',').collect();
        for (i, cell) in cols.iter().enumerate().skip(2) {
            if cell.is_empty() {
                continue;
            }
            let verdict = if cell.starts_with("ok") {
                "ok".to_string()
            } else {
                cell.to_string()
            };
            out.insert(
                (cols[0].to_string(), cols[1].to_string(), header[i].to_string()),
                verdict,
            );
        }
    }
    out
}

/// Our verdict per cell of one context: `ok`, or the codes of our diagnostics on the cell's line joined by `+`.
fn ours(ctx: &str, kind: Kind, tmpl: &str) -> BTreeMap<(String, String, String), String> {
    let (src, cells) = build_context(ctx, kind, tmpl);
    let app = AppSource {
        id: "y31".into(),
        name: "Y31 matrix".into(),
        dependencies: vec![],
        files: vec![
            ("src/objects.al".into(), HEADER_OBJECTS.to_string()),
            ("src/matrix.al".into(), src.clone()),
        ],
    };
    let ws = Workspace::load(vec![app], &al_version::corpus_symbols(29));
    let members = al_sema::members::build(&ws);
    let catalog = al_builtins::catalog::Catalog::for_target(
        &al_version::Target::for_bc(al_version::BcVersion::new(29, 0))
            .with_runtime(al_version::Runtime { major: 18, minor: 0 }),
    );
    let checked = al_sema::check::check(&ws, &members, &catalog);
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(src.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let mut per_line: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for d in ws.diagnostics.iter().chain(checked.diagnostics.iter()) {
        if !d.file.ends_with("matrix.al") {
            continue;
        }
        let line = line_starts.partition_point(|&s| s <= d.offset);
        per_line.entry(line).or_default().insert(d.code.to_string());
    }
    cells
        .iter()
        .map(|(line, (s, t))| {
            let v = per_line
                .get(line)
                .map(|c| c.iter().cloned().collect::<Vec<_>>().join("+"))
                .unwrap_or_else(|| "ok".to_string());
            ((ctx.to_string(), s.clone(), t.clone()), v)
        })
        .collect()
}

#[test]
fn the_runner_judges_every_cell_of_the_compile_matrix_as_alc_does() {
    let want = expected();
    let mut got = BTreeMap::new();
    for (ctx, kind, tmpl) in CONTEXTS {
        got.extend(ours(ctx, *kind, tmpl));
    }
    assert_eq!(want.len(), 33_060, "cells of the fixture");
    assert_eq!(
        got.keys().collect::<BTreeSet<_>>(),
        want.keys().collect::<BTreeSet<_>>(),
        "the generated cells are the fixture's"
    );
    let mut mismatches = BTreeMap::new();
    for (k, w) in &want {
        let g = &got[k];
        if g != w {
            mismatches.insert(k.clone(), (w.clone(), g.clone()));
        }
    }
    if let Ok(path) = std::env::var("CONVERSION_MATRIX_REPORT") {
        std::fs::write(path, report(&mismatches)).unwrap();
    }
    let lines: BTreeSet<String> = mismatches
        .iter()
        .map(|((c, s, t), (w, g))| format!("{c},{s},{t},{w},{g}"))
        .collect();
    let known_path = format!("{FIXTURES}/known-mismatches.txt");
    if std::env::var_os("UPDATE_CONVERSION_MATRIX").is_some() {
        let mut text = String::new();
        for l in &lines {
            text.push_str(l);
            text.push('\n');
        }
        std::fs::write(&known_path, text).unwrap();
        return;
    }
    let known: BTreeSet<String> = std::fs::read_to_string(&known_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let new: Vec<&String> = lines.difference(&known).collect();
    let fixed: Vec<&String> = known.difference(&lines).collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "{} cells differ from alc and are not in known-mismatches.txt, first: {:?}\n{} listed cells now match or changed, first: {:?}\n(context,source,target,alc,ours; UPDATE_CONVERSION_MATRIX=1 rewrites the list)",
        new.len(),
        new.iter().take(20).collect::<Vec<_>>(),
        fixed.len(),
        fixed.iter().take(20).collect::<Vec<_>>()
    );
}

/// The mismatches grouped by kind (we accept what alc rejects, we reject what alc accepts, another code), then by
/// (context, alc's verdict, ours), with the cells.
fn report(m: &BTreeMap<(String, String, String), (String, String)>) -> String {
    let mut groups: BTreeMap<(&str, String, String, String), Vec<String>> = BTreeMap::new();
    for ((c, s, t), (w, g)) in m {
        let kind = if g == "ok" {
            "1 runner accepts, alc rejects"
        } else if w == "ok" {
            "2 runner rejects, alc accepts"
        } else {
            "3 wrong code"
        };
        groups
            .entry((kind, c.clone(), w.clone(), g.clone()))
            .or_default()
            .push(format!("{s}->{t}"));
    }
    let mut out = String::new();
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for ((k, _, _, _), cells) in &groups {
        *totals.entry(k).or_default() += cells.len();
    }
    for (k, n) in &totals {
        let _ = writeln!(out, "{k}: {n}");
    }
    for ((k, c, w, g), cells) in &groups {
        let _ = writeln!(out, "\n[{k}] {c}: alc {w}, ours {g} ({} cells)", cells.len());
        let _ = writeln!(out, "  {}", cells.join(" "));
    }
    out
}
