//! The Format matrix of corpus master format-evaluate/format-matrix/FM<Type>.al (corpus PRs 1249, 1250 and 1256): every cell of
//! `Format(V, 0, N)` and `Format(V, 0, S)` (in a TryFunction: `ok:<text>` or `err:<code>`). `fm_format/FM<Type>.tsv` holds the
//! cells, generated from the corpus file (`file:line`, the session language, the variable's type, its assignment, `N` or `S`, the
//! argument as AL code, and what BC gave or `IGNORE`); `fm_format/FM<Type>.al` the file's helper procedures. The test builds one
//! procedure per cell.
//!
//! The rule (hard rules 3 and 4): a cell either gives what BC gave or is refused; never another value. The number of refused
//! cells per file only goes down (`REFUSED_AT_MOST`).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

/// Base Application's enum 5767, as the corpus cells use it.
const WAREHOUSE_ACTIVITY_TYPE: &str = r#"enum 50991 "Warehouse Activity Type"
{
    Extensible = true;
    value(0; " ") { Caption = ' '; }
    value(1; "Put-away") { Caption = 'Put-away'; }
    value(2; Pick) { Caption = 'Pick'; }
    value(3; Movement) { Caption = 'Movement'; }
    value(4; "Invt. Put-away") { Caption = 'Invt. Put-away'; }
    value(5; "Invt. Pick") { Caption = 'Invt. Pick'; }
    value(6; "Invt. Movement") { Caption = 'Invt. Movement'; }
}
"#;

/// The most refused cells per file. Lower a number when a change makes more cells run; never raise one.
const REFUSED_AT_MOST: &[(&str, usize)] = &[
    ("FMBoolean", 0),
    ("FMEnum", 0),
    ("FMOption", 0),
    ("FMGuid", 0),
    ("FMChar", 0),
    ("FMText", 0),
    ("FMCode", 0),
    ("FMNumeric", 0),
    ("FMDateTime", 0),
];

/// The text as the corpus fixture Probe Observer records it (`_fixtures/ProbeObserver.Codeunit.al` Observe, EscapeControl).
fn observed(v: &str) -> String {
    let mut out = String::new();
    for c in v.chars() {
        match c {
            '\u{241E}' | '\u{241F}' => out.push('?'),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 || matches!(c as u32, 0xFFFE | 0xFFFF) => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    s.replace("\\t", "\t").replace("\\\\", "\\")
}

/// One procedure per cell: the value, the session language and the call of the corpus helper.
fn source(file: &str, helpers: &str, tsv: &str) -> String {
    let mut src = String::new();
    if file == "FMEnum" {
        src.push_str(WAREHOUSE_ACTIVITY_TYPE);
    }
    src.push_str("codeunit 50980 FMC\n{\n");
    src.push_str(helpers);
    for (k, line) in tsv.lines().enumerate() {
        let f: Vec<&str> = line.splitn(7, '\t').collect();
        let (lang, decl, assign, kind, arg) = (f[1], f[2], f[3], f[4], f[5]);
        src.push_str(&format!(
            "    procedure C{k}(): Text\n    var\n        R: Text;\n        V: {decl};\n    begin\n        GlobalLanguage({lang});\n        {assign}\n        if Try{kind}(V, {arg}, R) then\n            R := 'ok:' + R\n        else\n            R := 'err:' + GetLastErrorCode();\n        GlobalLanguage(1033);\n        exit(R);\n    end;\n"
        ));
    }
    src.push_str("}\n");
    src
}

fn check(file: &str, helpers: &str, tsv: &str) {
    let mut b = Built::from_source(&source(file, helpers, tsv));
    let (mut wrong, mut refused) = (Vec::new(), Vec::new());
    for (k, line) in tsv.lines().enumerate() {
        let f: Vec<&str> = line.splitn(7, '\t').collect();
        let expected = unescape(f[6]);
        if expected == "IGNORE" {
            continue;
        }
        let (oi, ri) = b.routine_in("FMC", &format!("C{k}"));
        match b.call(oi, ri, vec![]) {
            Ok((Value::Text(got), _)) if observed(got.as_ref()) == expected => {}
            Ok((got, _)) => wrong.push(format!("{}: expected {expected:?}, got {got:?}", f[0])),
            Err(Signal::Unsupported(why)) => refused.push(format!("{}: {why}", f[0])),
            Err(other) => wrong.push(format!("{}: expected {expected:?}, got {other:?}", f[0])),
        }
    }
    let cap = REFUSED_AT_MOST.iter().find(|(f, _)| *f == file).unwrap().1;
    if std::env::var_os("FM_MATRIX_LIST").is_some() {
        for r in &refused {
            eprintln!("refused {r}");
        }
    }
    assert!(
        wrong.is_empty(),
        "{file}: {} cells give another value than BC:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(
        refused.len() <= cap,
        "{file}: {} cells refused, at most {cap}:\n{}",
        refused.len(),
        refused.join("\n")
    );
    if refused.len() < cap {
        eprintln!(
            "{file}: {} cells refused; lower REFUSED_AT_MOST from {cap}",
            refused.len()
        );
    }
}

macro_rules! matrix {
    ($($test:ident => $file:literal),* $(,)?) => {$(
        #[test]
        fn $test() {
            check($file, include_str!(concat!("fm_format/", $file, ".al")), include_str!(concat!("fm_format/", $file, ".tsv")));
        }
    )*};
}

matrix! {
    boolean => "FMBoolean",
    enum_type => "FMEnum",
    option => "FMOption",
    guid => "FMGuid",
    char => "FMChar",
    text => "FMText",
    code => "FMCode",
    numeric => "FMNumeric",
    date_time => "FMDateTime",
}
