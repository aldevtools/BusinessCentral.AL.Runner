//! The Evaluate matrix of corpus master format-evaluate/format-matrix/FMEval*.al (corpus PRs 1249, 1250 and 1256): every cell
//! of the boolean form (`Ok := Evaluate(V, Txt[, N])`) and the statement form (in a TryFunction, the error code) for each type,
//! with the sentinel value each corpus helper starts from. `fm_eval_cells.tsv` holds the cells, generated from the corpus files
//! (`file:line` of the call, the session language, the form, the text, the number and what BC gave); cells whose result depends
//! on today's date or the clock (`IGNORE`) are skipped, as the corpus does.
//!
//! The rule (hard rules 3 and 4): a cell either gives what BC gave or is refused; never another value. The number of refused
//! cells per file only goes down (`REFUSED_AT_MOST`).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

/// Per corpus file: the variable's type, its sentinel, what the helper observes after the call, and objects the type needs.
struct Ty {
    file: &'static str,
    decl: &'static str,
    init: &'static str,
    observe: &'static str,
    objects: &'static str,
}

const TYPES: &[Ty] = &[
    Ty {
        file: "FMEvalBigInteger.al",
        decl: "BigInteger",
        init: "V := 7;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalBoolean.al",
        decl: "Boolean",
        init: "V := true;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalChar.al",
        decl: "Char",
        init: "V := 90;",
        observe: "Format(V, 0, 9) + '/' + Format(V)",
        objects: "",
    },
    Ty {
        file: "FMEvalCode3.al",
        decl: "Code[3]",
        init: "V := 'SEN';",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalDate.al",
        decl: "Date",
        init: "V := 20000101D;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalDateTime.al",
        decl: "DateTime",
        init: "V := CreateDateTime(20000101D, 070809T);",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalDecimal.al",
        decl: "Decimal",
        init: "V := 7;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalDuration.al",
        decl: "Duration",
        init: "V := 7;",
        observe: "Format(V, 0, 9) + '/' + Format(V)",
        objects: "",
    },
    Ty {
        file: "FMEvalEnum.al",
        decl: "Enum \"Warehouse Activity Type\"",
        init: "V := Enum::\"Warehouse Activity Type\"::Pick;",
        observe: "Format(V, 0, 9) + '/' + Format(V)",
        // Base Application's enum 5767, as the corpus cells use it.
        objects: r#"enum 50991 "Warehouse Activity Type"
{
    Extensible = true;
    value(0; " ") { Caption = ' '; }
    value(1; "Put-away") { Caption = 'Put-away'; }
    value(2; Pick) { Caption = 'Pick'; }
    value(3; Movement) { Caption = 'Movement'; }
    value(4; "Invt. Put-away") { Caption = 'Invt. Put-away'; }
    value(5; "Invt. Pick") { Caption = 'Invt. Pick'; }
    value(6; "Invt. Movement") { Caption = 'Invt. Movement'; }
}"#,
    },
    Ty {
        file: "FMEvalGuid.al",
        decl: "Guid",
        init: "",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalInteger.al",
        decl: "Integer",
        init: "V := 7;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalOption.al",
        decl: "Option \" \",Draft,Active",
        init: "V := 2;",
        observe: "Format(V, 0, 9) + '/' + Format(V)",
        objects: "",
    },
    Ty {
        file: "FMEvalText.al",
        decl: "Text",
        init: "V := 'sentinel';",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
    Ty {
        file: "FMEvalTime.al",
        decl: "Time",
        init: "V := 070809T;",
        observe: "Format(V, 0, 9)",
        objects: "",
    },
];

/// Texts whose asserted result holds the year the corpus CI ran in (2026): `01/xx/26` is 26 January of the current year
/// (FMEvalDateTime.al:319-321, :437-439). The test session's clock is another day, so these cells are skipped here.
const TODAY_DEPENDENT: &[&str] = &["01/xx/26"];

/// The most refused cells per file. Lower a number when a change makes more cells run; never raise one.
const REFUSED_AT_MOST: &[(&str, usize)] = &[
    ("FMEvalBigInteger.al", 0),
    ("FMEvalBoolean.al", 0),
    ("FMEvalChar.al", 0),
    ("FMEvalCode3.al", 0),
    ("FMEvalDate.al", 0),
    ("FMEvalDateTime.al", 0),
    ("FMEvalDecimal.al", 0),
    ("FMEvalDuration.al", 0),
    ("FMEvalEnum.al", 0),
    ("FMEvalGuid.al", 0),
    ("FMEvalInteger.al", 0),
    ("FMEvalOption.al", 0),
    ("FMEvalText.al", 0),
    ("FMEvalTime.al", 0),
];

fn source(t: &Ty) -> String {
    format!(
        r#"{objects}
codeunit 50990 FM
{{
    [TryFunction]
    local procedure TryEv(Txt: Text; var V: {decl})
    begin
        Evaluate(V, Txt);
    end;

    procedure B(Txt: Text; N: Integer; Lang: Integer): Text
    var
        V: {decl};
        Ok: Boolean;
        Saved: Integer;
        R: Text;
    begin
        Saved := GlobalLanguage();
        GlobalLanguage(Lang);
        {init}
        if N = 0 then
            Ok := Evaluate(V, Txt)
        else
            Ok := Evaluate(V, Txt, N);
        R := Format(Ok) + '|' + {observe};
        GlobalLanguage(Saved);
        exit(R);
    end;

    procedure S(Txt: Text; Lang: Integer): Text
    var
        V: {decl};
        Saved: Integer;
        R: Text;
    begin
        Saved := GlobalLanguage();
        GlobalLanguage(Lang);
        {init}
        ClearLastError();
        if TryEv(Txt, V) then
            R := 'ok|' + {observe}
        else
            R := 'err:' + GetLastErrorCode() + '|' + {observe};
        GlobalLanguage(Saved);
        exit(R);
    end;
}}
"#,
        objects = t.objects,
        decl = t.decl,
        init = t.init,
        observe = t.observe,
    )
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('n') => out.push('\n'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

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

struct Cell {
    at: String,
    language: i32,
    statement: bool,
    text: String,
    number: i32,
    expected: String,
}

fn cells(file: &str) -> Vec<Cell> {
    include_str!("fm_eval_cells.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && l.starts_with(file))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Cell {
                at: f[0].to_string(),
                language: f[1].parse().unwrap(),
                statement: f[2] == "st",
                text: unescape(f[3]),
                number: f[4].parse().unwrap(),
                expected: unescape(f[5]),
            }
        })
        .filter(|c| c.expected != "IGNORE" && !TODAY_DEPENDENT.contains(&c.text.as_str()))
        .collect()
}

/// Runs every cell of `t`: the wrong ones (cell, expected, got) and the refused ones (cell, reason).
fn run_type(t: &Ty) -> (Vec<String>, Vec<String>) {
    let mut b = Built::from_source(&source(t));
    let (oi, rb) = b.routine_in("FM", "B");
    let (_, rs) = b.routine_in("FM", "S");
    let (mut wrong, mut refused) = (Vec::new(), Vec::new());
    for c in cells(t.file) {
        let text = Value::Text(c.text.clone().into());
        let lang = Value::Integer(c.language);
        let r = if c.statement {
            b.call(oi, rs, vec![text, lang])
        } else {
            b.call(oi, rb, vec![text, Value::Integer(c.number), lang])
        };
        match r {
            Ok((Value::Text(got), _)) if observed(got.as_ref()) == c.expected => {}
            Ok((got, _)) => wrong.push(format!("{} {:?}: expected {:?}, got {got:?}", c.at, c.text, c.expected)),
            Err(Signal::Unsupported(why)) => refused.push(format!("{} {:?}: {why}", c.at, c.text)),
            Err(other) => wrong.push(format!(
                "{} {:?}: expected {:?}, got {other:?}",
                c.at, c.text, c.expected
            )),
        }
    }
    (wrong, refused)
}

fn check(file: &str) {
    let t = TYPES.iter().find(|t| t.file == file).unwrap();
    let cap = REFUSED_AT_MOST.iter().find(|(f, _)| *f == file).unwrap().1;
    let (wrong, refused) = run_type(t);
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

#[test]
fn big_integer() {
    check("FMEvalBigInteger.al");
}
#[test]
fn boolean() {
    check("FMEvalBoolean.al");
}
#[test]
fn char() {
    check("FMEvalChar.al");
}
#[test]
fn code3() {
    check("FMEvalCode3.al");
}
#[test]
fn date() {
    check("FMEvalDate.al");
}
#[test]
fn date_time() {
    check("FMEvalDateTime.al");
}
#[test]
fn decimal() {
    check("FMEvalDecimal.al");
}
#[test]
fn duration() {
    check("FMEvalDuration.al");
}
#[test]
fn enum_type() {
    check("FMEvalEnum.al");
}
#[test]
fn guid() {
    check("FMEvalGuid.al");
}
#[test]
fn integer() {
    check("FMEvalInteger.al");
}
#[test]
fn option() {
    check("FMEvalOption.al");
}
#[test]
fn text() {
    check("FMEvalText.al");
}
#[test]
fn time() {
    check("FMEvalTime.al");
}
