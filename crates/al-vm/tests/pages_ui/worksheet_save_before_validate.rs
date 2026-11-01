//! al-runner issue 6629: a Worksheet row edited with SetValue, then a page-variable field whose OnValidate reads the row back
//! with Get. The fixture is the reporter's repro, with the test procedure in a plain codeunit that returns the outcome.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50500 "Repro Contact"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[50]) { }
    }
    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

page 50500 "Repro Contact Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Repro Contact";
    InsertAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Code"; Rec."Code") { }
                field(Name; Rec.Name) { }
                field(Activate; Activate)
                {
                    trigger OnValidate()
                    var
                        ReproContact: Record "Repro Contact";
                    begin
                        // reads the row back from the database
                        ReproContact.Get(Rec."Code");
                        if ReproContact.Name = '' then
                            Error('Name is required before the contact can be activated.');
                    end;
                }
            }
        }
    }

    var
        Activate: Boolean;
}

codeunit 50501 C
{
    procedure Repro(): Text
    var
        ReproContact: Record "Repro Contact";
        Worksheet: TestPage "Repro Contact Worksheet";
    begin
        ReproContact."Code" := 'A';
        ReproContact.Insert();

        Worksheet.OpenEdit();
        Worksheet.GoToRecord(ReproContact);
        Worksheet.Name.SetValue('Alice');
        Worksheet.Activate.SetValue(true);
        Worksheet.Close();
        exit('ok');
    end;
}
"#;

#[test]
fn an_edited_worksheet_row_is_saved_before_the_next_field_validates() {
    assert_eq!(text(run_in(SRC, "C", "Repro")), "ok");
}
