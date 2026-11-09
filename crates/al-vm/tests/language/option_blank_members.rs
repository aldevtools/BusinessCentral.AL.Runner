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
