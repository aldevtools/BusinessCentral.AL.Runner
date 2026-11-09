//! Truncate on a table whose OnDatabaseDelete is enabled through `GetDatabaseTableTriggerSetup` (al-runner issue 7385).
//! Real BC deletes every row and raises no database or global delete event: corpus
//! events/W4GlobalTriggers.al:397 Truncate_WithDatabaseDeleteEnabled passes on BC 27.5, 28.5 and 29 (oracle/*/junit.xml).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69795 "TDD Index"
{
    fields { field(1; Id; Integer) { } field(2; Info; Text[20]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69796 "TDD Log"
{
    SingleInstance = true;
    var Text: Text;
    procedure Add(Line: Text) begin Text += Line + ';'; end;
    procedure Take() Result: Text begin Result := Text; Text := ''; end;
}
codeunit 69797 "TDD Subs"
{
    var Log: Codeunit "TDD Log";

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetGlobalTableTriggerMask', '', false, false)]
    local procedure Mask(TableId: Integer; var TableTriggerMask: Integer)
    begin
        if TableId = Database::"TDD Index" then TableTriggerMask := 15;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId <> Database::"TDD Index" then exit;
        OnDatabaseInsert := true;
        OnDatabaseModify := true;
        OnDatabaseDelete := true;
        OnDatabaseRename := true;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnGlobalDelete', '', false, false)]
    local procedure GD(RecRef: RecordRef) begin if RecRef.Number = Database::"TDD Index" then Log.Add('gD'); end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseDelete', '', false, false)]
    local procedure DD(RecRef: RecordRef) begin if RecRef.Number = Database::"TDD Index" then Log.Add('dD'); end;
}
codeunit 69798 C
{
    var Log: Codeunit "TDD Log";

    procedure Plain(): Text
    var MyIndex: Record "TDD Index"; i: Integer;
    begin
        Seed(MyIndex);
        if not MyIndex.IsEmpty() then
            MyIndex.Truncate();
        exit('[' + Log.Take() + '] ' + Format(MyIndex.Count()));
    end;

    procedure Filtered(): Text
    var MyIndex: Record "TDD Index";
    begin
        Seed(MyIndex);
        MyIndex.SetRange(Id, 1);
        MyIndex.Truncate();
        exit(Format(MyIndex.Count()));
    end;

    local procedure Seed(var MyIndex: Record "TDD Index")
    var i: Integer;
    begin
        for i := 1 to 3 do begin
            MyIndex.Id := i;
            MyIndex.Insert(true);
        end;
        Log.Take();
    end;
}
"#;

#[test]
fn truncate_runs_without_delete_events_when_database_delete_is_enabled() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Plain")), "[] 0");
}

/// Only the shape the corpus pins runs: Truncate with a filter on such a table is still refused.
#[test]
fn truncate_with_a_filter_stays_refused_when_database_delete_is_enabled() {
    match al_testkit::run_in(SRC, "C", "Filtered") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("OnDatabaseDelete"), "{why}"),
        other => panic!("{other:?}"),
    }
}
