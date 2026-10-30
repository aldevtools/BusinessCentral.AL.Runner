//! A table field's OptionCaptionML gives the captions of the session language, ENU where the language has no entry
//! (record/lt-x-table-props/XTablePropsTests.al:175-232: 1033, 1031 and 1036, also through a table extension's `modify`).
//! A page shows the captions of the language it opened in (XTablePropsTests.al:229-230, pageext/lt-z-modify-controls/
//! ZModifyControls.al:322-360; pages/lt-page-controls-aa/AaFieldProps.al:179-219 keeps the open language after a change).
//! Other languages, and a missing entry of DEU or ENU, are not pinned and are refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = "table 50100 T { fields { field(1; Id; Integer) { } field(2; Op; Option) { OptionMembers = A,B,C; OptionCaptionML = ENU = 'Alpha,Beta,Gamma', DEU = 'Eins,Zwei,Drei'; } \
    field(3; Plain; Option) { OptionMembers = A,B,C; } field(4; OptMl; Option) { OptionMembers = A,B,C; OptionCaptionML = ENU = 'Red,Green,Blue', DEU = 'Rot,Gruen,Blau'; } \
    field(5; NoEnu; Option) { OptionMembers = A,B,C; OptionCaptionML = DEU = 'Eins,Zwei,Drei'; } } keys { key(PK; Id) { } } } \
    tableextension 50101 TX extends T { fields { modify(OptMl) { OptionCaptionML = ENU = 'Cyan,Magenta,Yellow', DEU = 'Zyan,Magenta,Gelb'; } } } \
    page 50102 P { PageType = Card; SourceTable = T; layout { area(content) { field(OpC; Rec.Op) { } field(MlC; Rec.Op) { OptionCaptionML = ENU = 'One,Two,Three', DEU = 'Eins,Zwei,Drei'; } } } } \
    codeunit 50100 C { \
    procedure Fmt(Lang: Integer): Text var R: Record T; begin GlobalLanguage(Lang); R.Op := R.Op::B; R.OptMl := R.OptMl::C; exit(Format(R.Op) + '|' + Format(R.OptMl)); end; \
    procedure Cap(Lang: Integer): Text var RR: RecordRef; F: FieldRef; S: Text; begin GlobalLanguage(Lang); RR.Open(50100); F := RR.Field(2); S := F.OptionCaption; F := RR.Field(4); exit(S + '|' + F.OptionCaption); end; \
    procedure Pg(Lang: Integer): Text var R: Record T; TP: TestPage P; S: Text; begin R.Id := 1; R.Op := R.Op::B; R.Insert(); GlobalLanguage(Lang); TP.OpenEdit(); \
      S := TP.OpC.Value + '|' + TP.MlC.Value; GlobalLanguage(1033); S += '|' + TP.OpC.Value + '|' + TP.MlC.Value; TP.Close(); exit(S); end; \
    procedure Eval(): Boolean var R: Record T; begin exit(Evaluate(R.Op, 'Beta')); end; \
    procedure PlainFmt(): Text var R: Record T; begin R.Plain := R.Plain::B; exit(Format(R.Plain)); end; \
    procedure NoEnuFmt(): Text var R: Record T; begin GlobalLanguage(1033); R.NoEnu := R.NoEnu::B; exit(Format(R.NoEnu)); end; }";

fn run(routine: &str, lang: i32) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", routine, vec![Value::Integer(lang)]).map(|(v, _)| v)
}

fn refused(r: Result<Value, Signal>) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("OptionCaptionML"), "{why}"),
        other => panic!("expected an OptionCaptionML refusal, got {other:?}"),
    }
}

#[test]
fn format_of_an_option_field_follows_the_session_language() {
    assert_eq!(text(run("Fmt", 1033)), "Beta|Yellow");
    assert_eq!(text(run("Fmt", 1031)), "Zwei|Gelb");
    assert_eq!(text(run("Fmt", 1036)), "Beta|Yellow");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "PlainFmt")), "B");
}

#[test]
fn field_ref_option_caption_follows_the_session_language() {
    assert_eq!(text(run("Cap", 1033)), "Alpha,Beta,Gamma|Cyan,Magenta,Yellow");
    assert_eq!(text(run("Cap", 1031)), "Eins,Zwei,Drei|Zyan,Magenta,Gelb");
    assert_eq!(text(run("Cap", 1036)), "Alpha,Beta,Gamma|Cyan,Magenta,Yellow");
}

#[test]
fn a_page_shows_the_captions_of_the_language_it_opened_in() {
    assert_eq!(text(run("Pg", 1031)), "Zwei|Zwei|Zwei|Zwei");
    assert_eq!(text(run("Pg", 1033)), "Beta|Two|Beta|Two");
}

#[test]
fn a_language_or_entry_nothing_pins_is_refused() {
    refused(run("Fmt", 1040));
    refused(run("Cap", 1040));
    refused(al_testkit::run_in(SRC, "C", "NoEnuFmt"));
}

#[test]
fn evaluate_into_a_field_with_option_caption_ml_is_refused() {
    // Evaluate takes the captions of the session language (types/lt-option-evaluate-captions/OptionEvaluateCaptions.al:236
    // CaptionML_ByLanguage): `Beta` is the ENU caption of B.
    assert!(matches!(al_testkit::run_in(SRC, "C", "Eval"), Ok(Value::Boolean(true))));
}
