//! `Report.Execute` with a data item view in the parameters, on a report whose request page has `ShowFilter = false`: the
//! view still filters the data item (session/lt-y22-copy-company/Y22CopyCompany.al:301-323, Y22_CopyCompany_Report: report
//! 357 "Copy Company" hides its filters, and the run copies only the company the view names; al-runner issue 6507).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50300 "HF Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
report 50301 "HF Report"
{
    ProcessingOnly = true;
    UseRequestPage = true;
    dataset
    {
        dataitem(Row; "HF Row")
        {
            trigger OnAfterGetRecord()
            begin
                Seen += Format(Row.Id) + ';';
            end;
        }
    }
    requestpage { ShowFilter = false; layout { area(Content) { } } }
    var
        Seen: Text;
    procedure GetSeen(): Text begin exit(Seen); end;
}
codeunit 50302 T
{
    procedure Run(): Text
    var
        Row: Record "HF Row";
        R: Report "HF Report";
        I: Integer;
    begin
        for I := 1 to 3 do begin
            Row.Id := I;
            Row.Insert();
        end;
        R.Execute('<?xml version="1.0" standalone="yes"?><ReportParameters name="HF Report" id="50301"><DataItems><DataItem name="Row">VERSION(1) SORTING(Field1) WHERE(Field1=1(2))</DataItem></DataItems></ReportParameters>');
        exit(R.GetSeen());
    end;
}
"#;

#[test]
fn execute_applies_the_view_of_a_data_item_whose_filters_are_hidden() {
    assert_eq!(text(run_in(SRC, "T", "Run")), "2;");
}
