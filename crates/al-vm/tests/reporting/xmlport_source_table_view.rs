//! `SourceTableView` on a table element. On import it changes nothing: a row the view's filter leaves out is imported
//! as any other (corpus platform/lt-ag-contoso/AgXmlPortImport.al:62-80, :298-309); a filtered view on a linked element is
//! not pinned on import. On export the rows are read in the view's order and through its filter
//! (xmlport/lt-ra-xmlport/RaXmlPortExport.al:622-805); a filter on an element that is not linked is not pinned there.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(view: &str, link: bool, export: bool) -> String {
    let link_props = if link {
        "LinkTable = Head; LinkFields = HeadId = field(Id);"
    } else {
        ""
    };
    let call = if export {
        "B.Data.CreateOutStream(OutS); Xmlport.Export(Xmlport::\"SV Port\", OutS);"
    } else {
        "B.Data.CreateOutStream(OutS); OutS.WriteText('<Root><Head><Id>3</Id><Row><Id>3</Id><Active>1</Active></Row><Row><Id>4</Id><Active>0</Active></Row></Head></Root>'); \
         B.Data.CreateInStream(InS); Xmlport.Import(Xmlport::\"SV Port\", InS);"
    };
    format!(
        r#"
table 69990 "SV Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "SV Head" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69992 "SV Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Active; Boolean) {{ }} field(3; HeadId; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69993 "SV Port"
{{
    Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Head; "SV Head")
            {{
                fieldelement(Id; Head.Id) {{ }}
                tableelement(Row; "SV Row")
                {{
                    {link_props}
                    SourceTableView = {view};
                    fieldelement(Id; Row.Id) {{ }}
                    fieldelement(Active; Row.Active) {{ }}
                }}
            }}
        }}
    }}
}}
codeunit 69994 C
{{
    procedure F(): Text
    var B: Record "SV Blob" temporary; R: Record "SV Row"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Code := 'X'; B.Insert();
        {call}
        if R.FindSet() then repeat T += Format(R.Id) + ':' + Format(R.Active) + ','; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

const FILTERED: &str = "sorting(Id) order(descending) where(Active = const(true))";

#[test]
fn an_import_ignores_the_source_table_view() {
    // AgXmlPortImport.al:303: the inactive row is imported too.
    assert_eq!(al_testkit::text(run(&src(FILTERED, false, false), "F")), "3:Yes,4:No,");
    // A view of sorting only, on a linked element (Base App xmlport 1225 "Imp / Exp Data Exch Def & Map").
    assert_eq!(
        al_testkit::text(run(&src("sorting(Id)", true, false), "F")),
        "3:Yes,4:No,"
    );
}

#[test]
fn a_filter_on_an_element_that_is_not_linked_on_export_and_on_a_linked_one_on_import_stay_refused() {
    for (view, link, export) in [(FILTERED, false, true), (FILTERED, true, false)] {
        let r = run(&src(view, link, export), "F");
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("sourcetableview")),
            "{view} {link} {export}: {r:?}"
        );
    }
}

/// A linked element with a view, under a parent (RaXmlPortExport.al:328-470, xmlports 87923 to 87926).
fn export_src(root_view: &str, line_view: &str) -> String {
    format!(
        r#"
table 69995 "SE Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69996 "SE Head" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Rank; Integer) {{ }} }} keys {{ key(PK; Code) {{ }} key(ByRank; Rank) {{ }} }} }}
table 69997 "SE Line"
{{
    fields {{ field(1; HeadCode; Code[10]) {{ }} field(2; Code; Code[10]) {{ }} field(3; Seq; Integer) {{ }} field(4; Show; Boolean) {{ }} }}
    keys {{ key(PK; HeadCode, Code) {{ }} key(BySeq; HeadCode, Seq) {{ }} }}
}}
xmlport 69998 "SE Port"
{{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Head; "SE Head")
            {{
                MinOccurs = Zero; {root_view}
                fieldattribute(Code; Head.Code) {{ }}
                tableelement(Line; "SE Line")
                {{
                    LinkTable = Head; LinkFields = HeadCode = field(Code); MinOccurs = Zero; {line_view}
                    fieldattribute(Code; Line.Code) {{ }}
                }}
            }}
        }}
    }}
}}
codeunit 69999 C
{{
    local procedure Seed()
    var H: Record "SE Head"; L: Record "SE Line";
    begin
        H.Code := 'A'; H.Rank := 2; H.Insert(); H.Code := 'B'; H.Rank := 1; H.Insert();
        L.HeadCode := 'A'; L.Code := 'L1'; L.Seq := 3; L.Show := true; L.Insert();
        L.HeadCode := 'A'; L.Code := 'L2'; L.Seq := 1; L.Show := false; L.Insert();
        L.HeadCode := 'A'; L.Code := 'L3'; L.Seq := 2; L.Show := true; L.Insert();
        L.HeadCode := 'B'; L.Code := 'L1'; L.Seq := 1; L.Show := true; L.Insert();
    end;

    procedure G(): Text
    var B: Record "SE Blob" temporary; H: Record "SE Head"; P: XmlPort "SE Port"; OutS: OutStream; InS: InStream; T: Text; Line: Text;
    begin
        Seed();
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

    procedure F(): Text
    var B: Record "SE Blob" temporary; OutS: OutStream; InS: InStream; T: Text; Line: Text;
    begin
        Seed();
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"SE Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T.Replace('<?xml version="1.0" encoding="UTF-16" standalone="no"?>', ''));
    end;
}}
"#
    )
}

#[test]
fn an_export_reads_a_linked_element_in_the_order_and_through_the_filter_of_its_view() {
    // RaXmlPortExport.al:781 (sorting), :793 (sorting and where).
    let r = run(&export_src("", "SourceTableView = sorting(HeadCode, Seq);"), "F");
    assert_eq!(
        al_testkit::text(r),
        r#"<Root><Head Code="A"><Line Code="L2" /><Line Code="L3" /><Line Code="L1" /></Head><Head Code="B"><Line Code="L1" /></Head></Root>"#
    );
    let r = run(&export_src("", "SourceTableView = sorting(HeadCode, Seq) where(Show = const(true));"), "F");
    assert_eq!(
        al_testkit::text(r),
        r#"<Root><Head Code="A"><Line Code="L3" /><Line Code="L1" /></Head><Head Code="B"><Line Code="L1" /></Head></Root>"#
    );
    // :793 'where on the link field': the view's filter and the link both hold.
    let r = run(&export_src("", "SourceTableView = where(HeadCode = const('A'));"), "F");
    assert_eq!(
        al_testkit::text(r),
        r#"<Root><Head Code="A"><Line Code="L1" /><Line Code="L2" /><Line Code="L3" /></Head><Head Code="B" /></Root>"#
    );
}

#[test]
fn an_export_reads_an_element_that_is_not_linked_in_the_order_of_its_view() {
    // RaXmlPortExport.al:622 (sorting, order(descending)).
    let r = run(&export_src("SourceTableView = sorting(Rank);", ""), "F");
    assert_eq!(
        al_testkit::text(r),
        r#"<Root><Head Code="B"><Line Code="L1" /></Head><Head Code="A"><Line Code="L1" /><Line Code="L2" /><Line Code="L3" /></Head></Root>"#
    );
    let r = run(&export_src("SourceTableView = sorting(Rank) order(descending);", ""), "F");
    assert!(al_testkit::text(r).starts_with(r#"<Root><Head Code="A">"#));
}

#[test]
fn the_sorting_of_the_view_replaces_the_key_and_direction_of_a_set_table_view() {
    // RaXmlPortExport.al:654 (`SetTableView sorting(Seq) descending` exports in the view's ascending order).
    let r = run(&export_src("SourceTableView = sorting(Rank);", ""), "G");
    assert!(al_testkit::text(r).starts_with(r#"<Root><Head Code="B">"#));
}
