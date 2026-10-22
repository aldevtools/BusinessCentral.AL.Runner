//! A TestPage path whose outcome no corpus test pins is refused, not run with a picked behavior (hard rule 4;
//! the corpus probes are queued in aldevtools/BusinessCentral.AL.Language.Tests#712).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

/// An `Error('')` out of a table field's OnValidate during `TestPage.SetValue` ends the SetValue quietly on a List and a Card,
/// and the row keeps its value (pages/lt-qi-empty-error/QiEmptyErrorPages.al:342-414); on another page type it is not pinned.
#[test]
fn an_empty_error_out_of_a_table_on_validate_during_set_value_is_quiet_on_list_and_card_only() {
    let src = r#"
table 69965 "EE Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Flag; Boolean) { trigger OnValidate() begin if not Flag then Error(''); end; }
    }
    keys { key(PK; Id) { } }
}
page 69966 "EE List"
{
    PageType = List; SourceTable = "EE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } } } }
}
page 69967 "EE Card"
{
    PageType = Card; SourceTable = "EE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } } }
}
page 69969 "EE Doc"
{
    PageType = Document; SourceTable = "EE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } } }
}
codeunit 69968 C
{
    procedure OnList()
    var R: Record "EE Row"; P: TestPage "EE List";
    begin
        R.Id := 1; R.Flag := true; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Flag.SetValue(false);
        if P.Flag.Value() <> 'Yes' then Error('shown %1', P.Flag.Value());
    end;
    procedure OnCard()
    var R: Record "EE Row"; P: TestPage "EE Card";
    begin
        R.Id := 1; R.Flag := true; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Flag.SetValue(false);
        if P.Flag.Value() <> 'Yes' then Error('shown %1', P.Flag.Value());
        P.Close();
        R.Get(1);
        if not R.Flag then Error('stored No');
        if GetLastErrorCode() <> 'NSValidateField:Dialog' then Error('code %1', GetLastErrorCode());
    end;
    procedure OnDoc()
    var R: Record "EE Row"; P: TestPage "EE Doc";
    begin
        R.Id := 1; R.Flag := true; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Flag.SetValue(false);
    end;
}
"#;
    assert!(al_testkit::run_in(src, "C", "OnList").is_ok());
    assert!(al_testkit::run_in(src, "C", "OnCard").is_ok());
    refused(al_testkit::run_in(src, "C", "OnDoc"), "empty text");
}
