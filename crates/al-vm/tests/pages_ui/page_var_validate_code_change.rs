//! al-runner issue 6669: a page variable control's OnValidate writes Rec in code; the next page variable SetValue, an action,
//! Next and Close save it (pages/lt-y65-code-change-next-field/Y65CodeChangeNextField.al, writer Y65; assumption
//! page-var-validate-code-change-saved). The fixture is the reporter's repro plus an action that reads the stored row.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50520 "Repro Member"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[50]) { } field(3; "Secret Key"; Guid) { } }
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
codeunit 50521 C
{
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
