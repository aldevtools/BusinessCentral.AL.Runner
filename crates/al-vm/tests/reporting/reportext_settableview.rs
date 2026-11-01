//! `SetTableView` with a record whose table only a data item a reportextension adds uses: the view reaches that item
//! (report/lt-y17-ext-settableview/Y17ExtSetTableViewTests.al, corpus master; Microsoft's Get Source Documents, whose
//! Serv. Get Source Documents extension adds the Service Header and Service Line items).

use al_testkit::text;

const SRC: &str = r#"
table 69960 "XS H" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 69961 "XS PH"
{
    fields { field(1; Subtype; Integer) { } field(2; "No."; Code[20]) { } field(3; Cust; Code[20]) { } }
    keys { key(PK; Subtype, "No.") { } key(K2; Cust) { } }
}
codeunit 69962 "XS Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
}
report 69963 "XS RB"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(Req; "XS H")
        {
            trigger OnAfterGetRecord() begin Log.Add('Req:' + Req."No."); end;
        }
    }
    var Log: Codeunit "XS Log";
}
reportextension 69964 "XS RX" extends "XS RB"
{
    dataset
    {
        addlast(Req)
        {
            dataitem(XH; "XS PH")
            {
                DataItemLink = "No." = field("No.");
                DataItemTableView = sorting(Subtype, "No.");
                trigger OnPreDataItem() begin Log.Add('pre XH [' + XH.GetFilters() + ']'); end;
                trigger OnAfterGetRecord() begin Log.Add('XH:' + Format(XH.Subtype) + '/' + XH."No." + '/' + XH.Cust); end;
            }
        }
    }
    var Log: Codeunit "XS Log";
}
codeunit 69965 T
{
    local procedure Seed()
    var H: Record "XS H"; PH: Record "XS PH";
    begin
        H."No." := 'B'; H.Insert(); H."No." := 'A'; H.Insert();
        PH.Subtype := 1; PH."No." := 'B'; PH.Cust := 'C1'; PH.Insert();
        PH.Subtype := 2; PH."No." := 'A'; PH.Cust := 'C1'; PH.Insert();
        PH.Subtype := 1; PH."No." := 'A'; PH.Cust := 'C2'; PH.Insert();
    end;
    procedure Header(): Text
    var PH: Record "XS PH"; Rpt: Report "XS RB"; Log: Codeunit "XS Log";
    begin
        Seed();
        PH.SetFilter(Cust, 'C1');
        Rpt.SetTableView(PH);
        Rpt.Run();
        exit(Log.Take());
    end;
    procedure KeyOfView(): Text
    var PH: Record "XS PH"; Rpt: Report "XS RB"; Log: Codeunit "XS Log";
    begin
        Seed();
        PH.SetCurrentKey(Cust);
        PH.SetFilter(Cust, 'C1|C2');
        Rpt.SetTableView(PH);
        Rpt.Run();
        exit(Log.Take());
    end;
}
"#;

// Y17ExtSetTableViewTests.al:83 ExtItem_SetTableView_Header: the view's filter shows in the added item's OnPreDataItem and
// combines with its DataItemLink per parent row.
#[test]
fn the_view_reaches_the_item_a_reportextension_adds() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "T", "Header")),
        "Req:A;pre XH [Cust: C1];XH:2/A/C1;Req:B;pre XH [Cust: C1];XH:1/B/C1;"
    );
}

// Y17ExtSetTableViewTests.al:163 ExtItem_SetTableView_KeyOfView: the item's DataItemTableView sorting wins over the view's key.
#[test]
fn the_sorting_of_the_added_item_wins_over_the_key_of_the_view() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "T", "KeyOfView")),
        "Req:A;pre XH [Cust: C1|C2];XH:1/A/C2;XH:2/A/C1;Req:B;pre XH [Cust: C1|C2];XH:1/B/C1;"
    );
}
