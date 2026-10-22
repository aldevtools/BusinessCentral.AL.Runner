//! A report column of an Option field shows the OptionCaptionML caption of the session language: the column's own, else the
//! field's, ENU where the language has no entry (record/lt-x-table-props/XTablePropsTests.al:741-785).

use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = "table 50100 T { fields { field(1; Id; Integer) { } field(2; Op; Option) { OptionMembers = A,B,C; OptionCaptionML = ENU = 'Alpha,Beta,Gamma', DEU = 'Eins,Zwei,Drei'; } \
    field(3; Data; Blob) { } } keys { key(PK; Id) { } } } \
    report 50101 R { ProcessingOnly = false; DefaultLayout = RDLC; UseRequestPage = false; dataset { dataitem(Row; T) { \
      column(Plain; Op) { } \
      column(WithCaptionML; Op) { OptionCaptionML = ENU = 'Col one,Col two,Col three', DEU = 'Spalte eins,Spalte zwei,Spalte drei'; } \
      column(EnuOnly; Op) { OptionCaptionML = ENU = 'Only A,Only B,Only C'; } } } } \
    codeunit 50100 C { procedure F(Lang: Integer): Text var Row: Record T; Buf: Record T; OutStr: OutStream; InStr: InStream; Line: Text; Result: Text; \
      begin Row.Id := 1; Row.Op := Row.Op::B; Row.Insert(); GlobalLanguage(Lang); \
      Buf.Data.CreateOutStream(OutStr); Report.SaveAs(Report::R, '', ReportFormat::Xml, OutStr); Buf.Data.CreateInStream(InStr, TextEncoding::UTF8); \
      while not InStr.EOS() do begin InStr.ReadText(Line); if Line.Trim().StartsWith('<Column ') then Result += '[' + Line.Trim() + ']'; end; \
      exit(Result); end; }";

fn columns(lang: i32) -> Result<Value, al_builtins::native::Signal> {
    al_testkit::run_with(SRC, "C", "F", vec![Value::Integer(lang)]).map(|(v, _)| v)
}

#[test]
fn option_columns_show_the_captions_of_the_session_language() {
    assert_eq!(
        text(columns(1033)),
        "[<Column name=\"Plain\">Beta</Column>][<Column name=\"WithCaptionML\">Col two</Column>][<Column name=\"EnuOnly\">Only B</Column>]"
    );
}

/// A German session's SaveAs runs (report/W3ReportObjectIdPreviewLanguage.al:870) with the DEU captions
/// (XTablePropsTests.al:783); a French session's SaveAs is not pinned and refused.
#[test]
fn save_as_in_a_german_session_shows_deu_and_a_french_one_is_refused() {
    assert_eq!(
        text(columns(1031)),
        "[<Column name=\"Plain\">Zwei</Column>][<Column name=\"WithCaptionML\">Spalte zwei</Column>][<Column name=\"EnuOnly\">Only B</Column>]"
    );
    match columns(1036) {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("session language"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
