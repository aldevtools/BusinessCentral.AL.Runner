//! A blank option member declared as `""` is not the one declared as `" "` (the syntax used to keep both as one space).
//! Pinned: a first member `" "` reads the empty text, NUL and a blank text as ordinal 0
//! (format-evaluate/format-matrix/FMEvalOption.al:64-69, :262-267); a last member `""` reads none of them and shows as one
//! character (types/decimal-integer/ScT70062.al:Option_EvaluateByName `empty`, ScT70053.al:Option_Format `blank_len`).
//! Every other place of a blank member is refused until a probe pins it (docs/probes-needed.md, OPTBLANK).

use al_builtins::native::Signal;
use al_testkit::{boolean, text};

fn src(decl: &str) -> String {
    format!(
        r#"
codeunit 69720 C
{{
    procedure Ev(t: Text): Boolean
    var O: Option {decl};
    begin
        exit(Evaluate(O, t));
    end;

    procedure Ord(t: Text): Text
    var O: Option {decl};
    begin
        Evaluate(O, t);
        exit(Format(O, 0, 2));
    end;

    procedure Len(i: Integer): Integer
    var O: Option {decl};
    begin
        O := i;
        exit(StrLen(Format(O)));
    end;
}}
"#
    )
}

fn call(decl: &str, routine: &str, arg: al_builtins::value::Value) -> Result<al_builtins::value::Value, Signal> {
    let mut b = al_testkit::Built::from_source(&src(decl));
    let (oi, ri) = b.routine_in("C", routine);
    b.call(oi, ri, vec![arg]).map(|(v, _)| v)
}

fn t(s: &str) -> al_builtins::value::Value {
    al_builtins::value::Value::Text(s.into())
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_first_space_member_reads_the_blank_texts() {
    for s in ["", " ", "\0"] {
        assert!(boolean(call("\" \",A,B", "Ev", t(s))), "{s:?}");
        assert_eq!(text(call("\" \",A,B", "Ord", t(s))), "0", "{s:?}");
    }
}

#[test]
fn a_last_quoted_empty_member_reads_no_blank_text() {
    for s in ["", " "] {
        assert!(!boolean(call("Alpha,Beta,\"\"", "Ev", t(s))), "{s:?}");
    }
    assert_eq!(
        al_testkit::int(call("Alpha,Beta,\"\"", "Len", al_builtins::value::Value::Integer(2))),
        1
    );
}

#[test]
fn a_named_member_still_reads_beside_a_quoted_empty_member() {
    assert!(boolean(call("Alpha,\"\",Beta", "Ev", t("beta"))));
    assert_eq!(text(call("Alpha,\"\",Beta", "Ord", t("Beta"))), "2");
}

#[test]
fn a_quoted_empty_member_that_is_first_or_in_the_middle_refuses_a_blank_text() {
    for decl in ["\"\",A,B", "A,\"\",B"] {
        for s in ["", " ", "\0"] {
            refused(call(decl, "Ev", t(s)));
        }
    }
}

#[test]
fn a_space_member_in_the_middle_or_last_refuses_a_blank_text() {
    for decl in ["A,\" \",B", "A,B,\" \""] {
        for s in ["", " ", "\0"] {
            refused(call(decl, "Ev", t(s)));
        }
    }
}

#[test]
fn two_blank_members_refuse_a_blank_text() {
    refused(call("\" \",A,\"\"", "Ev", t("")));
}

#[test]
fn a_quoted_empty_member_shows_only_where_it_is_pinned() {
    refused(call("A,\"\",B", "Len", al_builtins::value::Value::Integer(1)));
    refused(call("\"\",A", "Len", al_builtins::value::Value::Integer(0)));
}

/// The marker a `""` member is held as never reaches AL-visible output: each path gives a refusal, or text without it.
#[test]
fn a_quoted_empty_member_never_leaks_its_marker() {
    const SRC: &str = r#"
table 69721 T
{
    fields
    {
        field(1; K; Integer) { }
        field(2; Last; Option) { OptionMembers = A,B,""; }
        field(3; Mid; Option) { OptionMembers = A,"",B; }
    }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 69721 L
{
    var G: Option A,"",B;

    procedure VarFormat(): Text begin G := 1; exit(Format(G)); end;
    procedure VarFormatLast(): Text var O: Option A,B,""; begin O := 2; exit(Format(O)); end;
    procedure VarFormat9(): Text begin G := 1; exit(Format(G, 0, 9)); end;
    procedure VarSubst(): Text begin G := 1; exit(StrSubstNo('%1', G)); end;
    procedure VarVariant(): Text var V: Variant; begin G := 1; V := G; exit(Format(V)); end;
    procedure VarError(): Text begin if Evaluate(G, 'zzz') then; exit(''); end;
    procedure VarErrorUnused(): Text begin Evaluate(G, 'zzz'); exit(''); end;
    procedure FieldFormat(): Text var R: Record T; begin R.Mid := R.Mid::B; R.Mid := 1; exit(Format(R.Mid)); end;
    procedure FieldFormatLast(): Text var R: Record T; begin R.Last := 2; exit(Format(R.Last)); end;
    procedure FieldRefMembers(): Text var R: Record T; F: FieldRef; RR: RecordRef; begin RR.GetTable(R); F := RR.Field(3); exit(F.OptionMembers); end;
    procedure FieldRefCaption(): Text var R: Record T; F: FieldRef; RR: RecordRef; begin RR.GetTable(R); F := RR.Field(3); exit(F.OptionCaption); end;
    procedure FieldRefValue(): Text var R: Record T; F: FieldRef; RR: RecordRef; begin R.Mid := 1; RR.GetTable(R); F := RR.Field(3); exit(Format(F.Value)); end;
    procedure FieldRefValueText(): Text var R: Record T; F: FieldRef; RR: RecordRef; t: Text; begin R.Mid := 1; RR.GetTable(R); F := RR.Field(3); t := F.Value; exit(t); end;
    procedure FieldEvalError(): Text var R: Record T; begin Evaluate(R.Mid, 'zzz'); exit(''); end;
    procedure FieldFilter(): Text var R: Record T; begin R.SetFilter(Mid, ''''''); exit(R.GetFilter(Mid)); end;
    procedure FieldFilterName(): Text var R: Record T; begin R.SetRange(Mid, R.Mid::B); exit(R.GetFilter(Mid)); end;
    procedure Json(): Text var J: JsonObject; V: Variant; t: Text; begin G := 1; V := G; J.Add('o', Format(V)); J.WriteTo(t); exit(t); end;
}
"#;
    let routines = [
        "VarFormat",
        "VarFormatLast",
        "VarFormat9",
        "VarSubst",
        "VarVariant",
        "VarError",
        "VarErrorUnused",
        "FieldFormat",
        "FieldFormatLast",
        "FieldRefMembers",
        "FieldRefCaption",
        "FieldRefValue",
        "FieldRefValueText",
        "FieldEvalError",
        "FieldFilter",
        "FieldFilterName",
        "Json",
    ];
    let marker = al_builtins::optname::QUOTED_EMPTY;
    for r in routines {
        let shown = match al_testkit::run_in(SRC, "L", r) {
            Ok(v) => format!("{v:?}"),
            Err(Signal::Unsupported(m)) => m,
            Err(Signal::Error(e)) => e.message.clone(),
            Err(other) => format!("{other:?}"),
        };
        assert!(!shown.contains(marker) && !shown.contains("\\u{e000}"), "{r}: {shown}");
    }
}
