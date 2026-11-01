//! al-runner issue 6669: a page variable control's OnValidate writes Rec in code; the next page variable SetValue, an action,
//! Next and Close save it (pages/lt-y65-code-change-next-field/Y65CodeChangeNextField.al, writer Y65; assumption
//! page-var-validate-code-change-saved). The fixture is the reporter's repro plus an action that reads the stored row, and a
//! Card without row triggers whose action used to drop the edit.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50520 "Repro Member"
{
    fields
    {
        field(1; "Code"; Code[10]) { } field(2; Name; Text[50]) { } field(3; "Secret Key"; Guid) { }
        field(4; Note; Text[30]) { trigger OnValidate() begin Counter := 7; end; }
        field(5; Counter; Integer) { }
        field(6; Plain; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
page 50520 "Repro Member Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Repro Member";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Code"; Rec."Code") { }
                field(Name; Rec.Name) { }
                field(Secret; Secret) { trigger OnValidate() begin Rec."Secret Key" := CreateGuid(); end; }
                field(Activate; Activate)
                {
                    trigger OnValidate()
                    var M: Record "Repro Member";
                    begin
                        M.Get(Rec."Code");
                        if M.Name = '' then Error('Name is required before the member can be activated.');
                        if IsNullGuid(M."Secret Key") then Error('A secret is required before the member can be activated.');
                    end;
                }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Peek)
            {
                trigger OnAction()
                var M: Record "Repro Member";
                begin
                    M.Get(Rec."Code");
                    Seen := Format(not IsNullGuid(M."Secret Key"));
                end;
            }
        }
    }
    procedure GetSeen(): Text begin exit(Seen); end;
    var Secret: Text[50]; Activate: Boolean; Seen: Text;
}
page 50521 "Repro Member Card"
{
    PageType = Card;
    SourceTable = "Repro Member";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Note; Rec.Note) { } field(Plain; Rec.Plain) { } } }
    actions
    {
        area(Processing)
        {
            action(Peek)
            {
                trigger OnAction()
                var M: Record "Repro Member";
                begin
                    M.Get(Rec."Code");
                    Seen := Format(M.Counter);
                end;
            }
        }
    }
    procedure GetSeen(): Text begin exit(Seen); end;
    var Seen: Text;
}
codeunit 50521 C
{
    procedure SecretThenAction(): Text
    var M: Record "Repro Member"; W: TestPage "Repro Member Worksheet"; R: Text;
    begin
        M."Code" := 'A'; M.Insert();
        W.OpenEdit();
        W.GoToRecord(M);
        W.Secret.SetValue('x');
        W.Peek.Invoke();
        W.Close();
        M.Get('A');
        exit(Format(not IsNullGuid(M."Secret Key")));
    end;

    procedure CardPlainThenAction(): Text
    var M: Record "Repro Member"; C: TestPage "Repro Member Card"; R: Text;
    begin
        M."Code" := 'A'; M.Insert();
        C.OpenEdit();
        C.GoToRecord(M);
        C.Plain.SetValue('p1');
        R := C.Plain.Value + '/' + C."Code".Value + '|';
        C.Peek.Invoke();
        M.Get('A');
        R += M.Plain + '|' + C.Plain.Value + '|';
        C.Close();
        M.Get('A');
        exit(R + M.Plain);
    end;

    procedure CardTableTriggerThenAction(): Text
    var M: Record "Repro Member"; C: TestPage "Repro Member Card";
    begin
        M."Code" := 'A'; M.Insert();
        C.OpenEdit();
        C.GoToRecord(M);
        C.Note.SetValue('n1');
        C.Peek.Invoke();
        C.Close();
        M.Get('A');
        exit(Format(M.Counter) + M.Note);
    end;

    procedure Repro(): Text
    var M: Record "Repro Member"; W: TestPage "Repro Member Worksheet";
    begin
        M."Code" := 'A'; M.Insert();
        W.OpenEdit();
        W.GoToRecord(M);
        W.Name.SetValue('Alice');
        W.Secret.SetValue('x');
        W.Activate.SetValue(true);
        W.Close();
        exit('ok');
    end;

    procedure SecretThenClose(): Text
    var M: Record "Repro Member"; W: TestPage "Repro Member Worksheet";
    begin
        M."Code" := 'A'; M.Insert();
        W.OpenEdit();
        W.GoToRecord(M);
        W.Secret.SetValue('x');
        W.Close();
        M.Get('A');
        exit(Format(not IsNullGuid(M."Secret Key")));
    end;
}
"#;

#[test]
fn a_rec_change_in_a_page_variable_onvalidate_is_saved_before_the_next_field_validates() {
    assert_eq!(text(run_in(SRC, "C", "Repro")), "ok");
}

#[test]
fn a_rec_change_in_a_page_variable_onvalidate_is_saved_by_close() {
    assert_eq!(text(run_in(SRC, "C", "SecretThenClose")), "Yes");
}

#[test]
fn a_rec_change_in_a_page_variable_onvalidate_is_saved_before_an_action() {
    assert_eq!(text(run_in(SRC, "C", "SecretThenAction")), "Yes");
}

// Y65CodeChangeNextField.al:594 Y65_Card_TableTriggerChangeThenAction: the action saves what the table trigger wrote.
#[test]
fn a_table_trigger_change_is_saved_before_a_card_action() {
    assert_eq!(text(run_in(SRC, "C", "CardTableTriggerThenAction")), "7n1");
}

// A Card without row triggers saves a typed edit before an action reads the row again (it was lost before).
#[test]
fn a_card_edit_is_saved_before_an_action() {
    assert_eq!(text(run_in(SRC, "C", "CardPlainThenAction")), "p1/A|p1|p1|p1");
}
