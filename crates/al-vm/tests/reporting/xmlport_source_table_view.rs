//! `SourceTableView` on a table element. On import it changes nothing: a row the view's filter leaves out is imported
//! as any other (corpus platform/lt-ag-contoso/AgXmlPortImport.al:62-80, :298-309). On export which rows and in which
//! order is not pinned (:312 sees one row), so an export stays refused; so does a filtered view on a linked element.

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
fn an_export_and_a_filtered_view_on_a_linked_element_stay_refused() {
    for (view, link, export) in [
        (FILTERED, false, true),
        ("sorting(Id)", true, true),
        (FILTERED, true, false),
    ] {
        let r = run(&src(view, link, export), "F");
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("sourcetableview")),
            "{view} {link} {export}: {r:?}"
        );
    }
}
