//! A Modify through a buffer whose own last Modify an asserterror rolled back, on a table with OnBeforeModifyEvent and
//! OnAfterModifyEvent subscribers (al-runner issue 7464). Measured on BC 27.5, 28.5 and 29.0, corpus pull request 1279,
//! record/events/Y7464ModifyAfterRolledBackModify.al: the events run once each with Rec and xRec the buffer, the stored row stays as the
//! rollback left it (BufferUnchanged, NoAssignment, OtherFieldReset), and a buffer changed since writes the whole row (FieldChanged).

use al_testkit::text;

const SRC: &str = r#"
table 50901 "MR Line"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Status; Option) { OptionMembers = Open,Registered,Closed; }
        field(3; Other; Text[30]) { trigger OnValidate() begin if Other = 'X' then Error('other is not allowed'); end; }
        field(4; Counter; Integer) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50902 "MR Log"
{
    SingleInstance = true;
    var Text1: Text;
    procedure Add(Value: Text) begin Text1 += Value + ';'; end;
    procedure Take(): Text var R: Text; begin R := Text1; Text1 := ''; exit(R); end;
}
codeunit 50903 "MR Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"MR Line", OnBeforeModifyEvent, '', false, false)]
    local procedure OnBefore(var Rec: Record "MR Line"; var xRec: Record "MR Line"; RunTrigger: Boolean)
    var Log: Codeunit "MR Log";
    begin
        Log.Add('B ' + Format(Rec.Status) + '/' + Format(Rec.Counter) + ' x=' + Format(xRec.Status) + '/' + Format(xRec.Counter) + ' run=' + Format(RunTrigger));
    end;
    [EventSubscriber(ObjectType::Table, Database::"MR Line", OnAfterModifyEvent, '', false, false)]
    local procedure OnAfter(var Rec: Record "MR Line"; var xRec: Record "MR Line"; RunTrigger: Boolean)
    var Log: Codeunit "MR Log";
    begin
        Log.Add('A ' + Format(Rec.Status) + '/' + Format(Rec.Counter) + ' x=' + Format(xRec.Status) + '/' + Format(xRec.Counter) + ' run=' + Format(RunTrigger));
    end;
}
codeunit 50904 C
{
    local procedure Seed(var Line: Record "MR Line")
    var Old: Record "MR Line"; Log: Codeunit "MR Log";
    begin
        Old.DeleteAll();
        Commit();
        Line.Init();
        Line."No." := 'L1';
        Line.Insert();
        Commit();
        Log.Take();
    end;
    local procedure Stored(): Text
    var Check: Record "MR Line";
    begin
        Check.Get('L1');
        exit(Format(Check.Status) + '/' + Format(Check.Counter));
    end;
    procedure Unchanged(): Text
    var Line: Record "MR Line"; Log: Codeunit "MR Log"; First: Text;
    begin
        Seed(Line);
        Line.Validate(Status, Line.Status::Registered);
        Line.Modify();
        First := Log.Take();
        asserterror Line.Validate(Other, 'X');
        Line.Validate(Status, Line.Status::Registered);
        Line.Modify();
        exit(First + '|' + Log.Take() + '|' + Stored());
    end;
    procedure Changed(): Text
    var Line: Record "MR Line"; Log: Codeunit "MR Log"; First: Text;
    begin
        Seed(Line);
        Line.Status := Line.Status::Registered;
        Line.Modify();
        First := Log.Take();
        asserterror Line.Validate(Other, 'X');
        Line.Counter := 7;
        Line.Modify();
        exit(First + '|' + Log.Take() + '|' + Stored());
    end;
}
"#;

#[test]
fn an_unchanged_buffer_runs_the_events_once_and_writes_nothing() {
    let both = "B Registered/0 x=Registered/0 run=No;A Registered/0 x=Registered/0 run=No;";
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Unchanged")),
        format!("{both}|{both}|Open/0")
    );
}

#[test]
fn a_buffer_changed_since_writes_the_whole_row() {
    let first = "B Registered/0 x=Registered/0 run=No;A Registered/0 x=Registered/0 run=No;";
    let second = "B Registered/7 x=Registered/7 run=No;A Registered/7 x=Registered/7 run=No;";
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Changed")),
        format!("{first}|{second}|Registered/7")
    );
}
