//! An unlinked ListPart over a temporary source that the host's OnOpenPage filled opens on the row its Rec stands on, as a
//! list whose OnOpenPage left Rec on a later row does (pages/lt-y15/Y15WorksheetPartRefresh.al:287
//! Y15_PartRefresh_TwoRowsPartOnFirst `open log` `AGR2;AGR1;AGR2;AGR2;AGCR2`, `at open` `2/B`; al-runner issue 6465).

const SRC: &str = r#"
codeunit 69970 "PT Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin if Entries <> '' then Entries += ';'; Entries += T; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69971 "PT Row" { fields { field(1; "No."; Integer) { } field(2; Description; Text[30]) { } } keys { key(PK; "No.") { } } }
page 69972 "PT Rows"
{
    PageType = ListPart; SourceTable = "PT Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "PT Log"; begin Log.Add('AGR' + Format(Rec."No.")); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "PT Log"; begin Log.Add('AGCR' + Format(Rec."No.")); end;
    procedure AddRow(No: Integer) begin Rec.Init(); Rec."No." := No; Rec.Insert(); end;
}
page 69973 "PT Worksheet"
{
    PageType = Worksheet; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { part(Rows; "PT Rows") { } } }
    trigger OnOpenPage() begin CurrPage.Rows.Page.AddRow(1); CurrPage.Rows.Page.AddRow(2); end;
}
codeunit 69974 C
{
    procedure Open(): Text
    var P: TestPage "PT Worksheet"; Log: Codeunit "PT Log"; R: Text;
    begin
        Log.Take();
        P.OpenView();
        R := Log.Take() + '|' + P.Rows."No.".Value();
        P.Close();
        exit(R);
    end;
}
"#;

#[test]
fn a_part_the_host_filled_opens_on_the_row_its_rec_stands_on() {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "Open");
    let (v, _) = b.call(oi, ri, vec![]).unwrap();
    assert_eq!(al_testkit::text(Ok(v)), "AGR2;AGR1;AGR2;AGR2;AGCR2|2");
}
