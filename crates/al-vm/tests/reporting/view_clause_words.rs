//! Which clauses a view has (SORTING, ORDER, WHERE) is read from the view's grammar, not by searching its text: a field
//! name or a value that holds one of those words (`"Order No."`, `"Sorting Group"`) names no clause (AL Runner issue 7613).

use al_testkit::{run, text};

/// An XmlPort whose table element has `view` and an export after a descending `SetTableView` on the code key.
fn xmlport_src(view: &str) -> String {
    format!(
        r#"
table 69980 "VW Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "VW Head"
{{
    fields {{ field(1; Code; Code[10]) {{ }} field(2; Rank; Integer) {{ }} field(3; "Order No."; Code[10]) {{ }} }}
    keys {{ key(PK; Code) {{ }} key(ByRank; Rank) {{ }} }}
}}
xmlport 69982 "VW Port"
{{
    Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Head; "VW Head")
            {{
                MinOccurs = Zero; SourceTableView = {view};
                fieldattribute(Code; Head.Code) {{ }}
            }}
        }}
    }}
}}
codeunit 69983 C
{{
    procedure G(): Text
    var B: Record "VW Blob" temporary; H: Record "VW Head"; P: XmlPort "VW Port"; OutS: OutStream; InS: InStream; T: Text; Line: Text;
    begin
        H.Code := 'A'; H.Rank := 2; H."Order No." := 'O1'; H.Insert();
        H.Code := 'B'; H.Rank := 1; H."Order No." := 'O2'; H.Insert();
        H.Code := 'C'; H.Rank := 3; H."Order No." := 'O3'; H.Insert();
        H.Reset();
        H.SetCurrentKey(Code); H.Ascending(false);
        P.SetTableView(H);
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        P.SetDestination(OutS);
        P.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T.Replace('<?xml version="1.0" encoding="UTF-16" standalone="no"?>', ''));
    end;
}}
"#
    )
}

#[test]
fn a_field_named_order_does_not_make_a_sorting_keep_the_direction_of_a_set_table_view() {
    // RaXmlPortExport.al:654: a view's sorting without ORDER exports ascending, also after `SetTableView` with
    // Ascending(false). The WHERE clause names the field "Order No.": still no ORDER clause.
    let r = run(&xmlport_src(r#"sorting(Rank) where("Order No." = filter(<>''))"#), "G");
    assert_eq!(
        text(r),
        r#"<Root><Head Code="B" /><Head Code="A" /><Head Code="C" /></Root>"#
    );
    // The same view without the word: the reference.
    let r = run(&xmlport_src(r#"sorting(Rank) where(Code = filter(<>''))"#), "G");
    assert_eq!(
        text(r),
        r#"<Root><Head Code="B" /><Head Code="A" /><Head Code="C" /></Root>"#
    );
}

/// A report whose data item has `view`; `G` runs it after a `SetTableView` with key Txt.
fn report_src(view: &str) -> String {
    format!(
        r#"
table 69985 "VW Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Txt; Text[10]) {{ }} field(3; "Sorting Group"; Integer) {{ }} }}
    keys {{ key(PK; Id) {{ }} key(K1; Txt) {{ }} }}
}}
codeunit 69986 "VW Log"
{{
    SingleInstance = true;
    var Txt: Text;
    procedure Add(Entry: Text) begin if Txt <> '' then Txt += '|'; Txt += Entry; end;
    procedure Get(): Text begin exit(Txt); end;
}}
report 69987 "VW Report"
{{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {{
        dataitem(Rows; "VW Row")
        {{
            DataItemTableView = {view};
            trigger OnPreDataItem()
            var Log: Codeunit "VW Log";
            begin Log.Add('key=' + Rows.CurrentKey()); end;
            trigger OnAfterGetRecord()
            var Log: Codeunit "VW Log";
            begin Log.Add(Format(Rows.Id)); end;
        }}
    }}
}}
codeunit 69988 C
{{
    procedure G(): Text
    var Row: Record "VW Row"; View: Record "VW Row"; Rpt: Report "VW Report"; Log: Codeunit "VW Log";
    begin
        Row.Id := 1; Row.Txt := 'd'; Row."Sorting Group" := 1; Row.Insert();
        Row.Id := 2; Row.Txt := 'c'; Row."Sorting Group" := 1; Row.Insert();
        Row.Id := 3; Row.Txt := 'b'; Row."Sorting Group" := 2; Row.Insert();
        Row.Id := 4; Row.Txt := 'a'; Row."Sorting Group" := 1; Row.Insert();
        View.SetCurrentKey(Txt);
        Rpt.SetTableView(View);
        Rpt.UseRequestPage(false);
        Rpt.Run();
        exit(Log.Get());
    end;
}}
"#
    )
}

#[test]
fn a_field_named_sorting_does_not_make_a_filter_view_drop_the_key_of_a_set_table_view() {
    // reports/dataset/ReportSetTableViewOverWhere.al:190 P163_SetTableView_OverWhere_KeyOfTheView: a view that only filters
    // keeps the key a SetTableView gave the item. The WHERE clause names the field "Sorting Group": still no SORTING clause.
    let r = run(&report_src(r#"where("Sorting Group" = const(1))"#), "G");
    assert_eq!(text(r), "key=Txt|4|2|1");
    // A value that holds the word is no clause either.
    let r = run(
        &report_src("where(Txt = filter(<>'sorting'), \"Sorting Group\" = const(1))"),
        "G",
    );
    assert_eq!(text(r), "key=Txt|4|2|1");
}
