//! RunPageLink `field()` of a host FlowFilter: the target takes the host's filter as it is (a Code FlowFilter with a value or an
//! expression), and no filter when the host has none (corpus testpage/Y72RunPageLinkFlowFilter.al).

use al_testkit::text;

const SRC: &str = r#"
table 69975 "RF Host"
{
    fields { field(1; Code; Code[10]) { } field(2; "Dim Filter"; Code[10]) { FieldClass = FlowFilter; } }
    keys { key(PK; Code) { } }
}
codeunit 69976 Seen
{
    SingleInstance = true;
    var Filters: Text;
    procedure Put(F: Text) begin Filters := F; end;
    procedure Get(): Text begin exit(Filters); end;
}
page 69977 "RF Target"
{
    PageType = Card; SourceTable = "RF Host"; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } } }
    trigger OnOpenPage() var S: Codeunit Seen; begin S.Put(Rec.GetFilters() + ' ' + Format(Rec.HasFilter())); end;
}
page 69978 "RF Card"
{
    PageType = Card; SourceTable = "RF Host"; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } } }
    actions
    {
        area(Processing)
        {
            action(Go) { RunObject = Page "RF Target"; RunPageLink = Code = field(Code), "Dim Filter" = field("Dim Filter"); }
        }
    }
}
codeunit 69979 C
{
    local procedure Run(DimFilter: Text): Text
    var H: Record "RF Host"; P: TestPage "RF Card"; T: TestPage "RF Target"; S: Codeunit Seen;
    begin
        H.Get('P1');
        if DimFilter <> '' then
            H.SetFilter("Dim Filter", DimFilter);
        P.Trap();
        Page.Run(Page::"RF Card", H);
        T.Trap();
        P.Go.Invoke();
        T.Close();
        P.Close();
        exit(S.Get());
    end;

    procedure Links(): Text
    var H: Record "RF Host";
    begin
        H.Code := 'P1'; H.Insert();
        exit(Run('') + '|' + Run('D1|D2'));
    end;
}
"#;

#[test]
fn a_host_flowfilter_link_copies_the_host_filter_or_adds_none() {
    // Y72RunPageLinkFlowFilter.al Link_CodeFlowFilter_HostUnfiltered `Code: P1`, Link_CodeFlowFilter_HostExpression `D1|D2`.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Links")),
        "Code: P1 Yes|Code: P1, Dim Filter: D1|D2 Yes"
    );
}
