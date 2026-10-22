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

/// An `Error('')` out of a table field's OnValidate during `TestPage.SetValue`: what the test client does with an error
/// with an empty text is pinned only for a card control's own OnValidate (pages/lt-qi-empty-error/QiEmptyErrorPages.al:253).
#[test]
fn an_empty_error_out_of_a_table_on_validate_during_set_value_is_refused() {
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
codeunit 69968 C
{
    procedure OnList()
    var R: Record "EE Row"; P: TestPage "EE List";
    begin
        R.Id := 1; R.Flag := true; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Flag.SetValue(false);
    end;
    procedure OnCard()
    var R: Record "EE Row"; P: TestPage "EE Card";
    begin
        R.Id := 1; R.Flag := true; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Flag.SetValue(false);
    end;
}
"#;
    refused(al_testkit::run_in(src, "C", "OnList"), "empty text");
    refused(al_testkit::run_in(src, "C", "OnCard"), "empty text");
}

/// A list whose repeater (not the page) says `Editable = false`: whether it shows the blank new-row line is not pinned (the
/// corpus pins only page-level `Editable = false`, handlers/TestPageNewRowLine_Tests.al). Tests-ERM 139846
/// ShouldOpenJobQueueEntriesForCreatedJobQueuesIfOptionSelected walks Job Queue Entries (repeater Editable = false) and
/// expects no blank line; landing on it is refused. Walking the data rows still works.
#[test]
fn the_blank_line_of_a_list_whose_repeater_is_read_only_is_refused() {
    let src = r#"
table 69969 "RR Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 69970 "RR List"
{
    PageType = List; SourceTable = "RR Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { Editable = false; field(Id; Rec.Id) { } } } }
}
codeunit 69971 C
{
    procedure Walk(): Text
    var R: Record "RR Row"; P: TestPage "RR List"; T: Text;
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
        P.OpenEdit();
        P.First();
        T := P.Id.Value();
        P.Next();
        T += P.Id.Value();
        exit(T);
    end;
    procedure PastTheEnd(): Boolean
    var R: Record "RR Row"; P: TestPage "RR List";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.First();
        exit(P.Next());
    end;
}
"#;
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "Walk")), "12");
    refused(al_testkit::run_in(src, "C", "PastTheEnd"), "repeater");
}
