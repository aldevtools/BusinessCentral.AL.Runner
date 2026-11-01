//! al-runner issue 6629: a Worksheet row edited with SetValue, then a page-variable field whose OnValidate reads the row back
//! with Get. The fixture is the reporter's repro, with the test procedure in a plain codeunit that returns the outcome. A page
//! variable in a header group outside the repeater saves nothing first (pages/lt-y17-journal-batch/Y17JournalBatchSwitch.al:522).

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

page 50501 "Repro Header Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Repro Contact";
    InsertAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(Activate; Activate)
                {
                    trigger OnValidate()
                    var
                        ReproContact: Record "Repro Contact";
                    begin
                        ReproContact.Get(Rec."Code");
                        Error('stored %1', ReproContact.Name);
                    end;
                }
            }
            repeater(Lines)
            {
                field("Code"; Rec."Code") { }
                field(Name; Rec.Name) { }
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

    procedure Header(): Text
    var
        ReproContact: Record "Repro Contact";
        Worksheet: TestPage "Repro Header Worksheet";
    begin
        ReproContact."Code" := 'A';
        ReproContact.Insert();

        Worksheet.OpenEdit();
        Worksheet.GoToRecord(ReproContact);
        Worksheet.Name.SetValue('Alice');
        asserterror Worksheet.Activate.SetValue(true);
        exit(GetLastErrorText());
    end;
}
"#;

#[test]
fn an_edited_worksheet_row_is_saved_before_the_next_field_validates() {
    assert_eq!(text(run_in(SRC, "C", "Repro")), "ok");
}

#[test]
fn a_page_variable_outside_the_repeater_saves_nothing_first() {
    assert_eq!(
        text(run_in(SRC, "C", "Header")),
        "Validation error for Field: Activate,  Message = 'stored '"
    );
}
