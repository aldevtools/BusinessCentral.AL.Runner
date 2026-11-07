//! A VariableText port whose table element holds another table element (corpus streams/DfXmlPortLeads.al:
//! VariableText_Nested_Export and _Import; the staged probe xmlports/shapes-properties/Y83XpMiscTests.al:VariableText_Nested).
//! Export: the first child record follows the parent's values on its line, the other child records follow on lines of their own,
//! the table separator ends the children of a parent, and a record separator joins the parents. Import: every line is a record of
//! the parent table; a required child table is then missing, an empty line is an unexpected child.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(child_props: &str, port_props: &str, seed: &str, doc: &str) -> String {
    format!(
        r#"
table 69870 "XN Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69871 "XN Hdr" {{ fields {{ field(1; No; Code[10]) {{ }} field(2; Descr; Text[30]) {{ }} }} keys {{ key(PK; No) {{ }} }} }}
table 69872 "XN Line" {{ fields {{ field(1; DocNo; Code[10]) {{ }} field(2; LineNo; Integer) {{ }} field(3; Descr; Text[30]) {{ }} }} keys {{ key(PK; DocNo, LineNo) {{ }} }} }}
xmlport 69873 "XN Port"
{{
    Format = VariableText; Direction = Both; FieldDelimiter = '"'; FieldSeparator = ','; UseRequestPage = false; {port_props}
    schema
    {{
        textelement(ROOT)
        {{
            tableelement(PH; "XN Hdr")
            {{
                fieldelement(No; PH.No) {{ }}
                fieldelement(HDescr; PH.Descr) {{ }}
                tableelement(PL; "XN Line")
                {{
                    LinkTable = PH; LinkFields = DocNo = field(No); {child_props}
                    fieldelement(LineNo; PL.LineNo) {{ }}
                    fieldelement(LDescr; PL.Descr) {{ }}
                }}
            }}
        }}
    }}
}}
codeunit 69874 C
{{
    procedure Seed()
    var H: Record "XN Hdr"; L: Record "XN Line";
    begin
        {seed}
    end;

    procedure Export(): Text
    var B: Record "XN Blob" temporary; OutS: OutStream; InS: InStream; Big: BigText; R: Text;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XN Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Big.Read(InS);
        Big.GetSubText(R, 1, Big.Length());
        exit(R);
    end;

    procedure Import(): Text
    var B: Record "XN Blob" temporary; OutS: OutStream; InS: InStream; H: Record "XN Hdr"; R: Text; Ok: Boolean; Cr: Text; Lf: Text; Ch: Char;
    begin
        Ch := 13; Cr += Ch; Ch := 10; Lf += Ch;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText({doc});
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        ClearLastError();
        Ok := Xmlport.Import(Xmlport::"XN Port", InS);
        R := Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|';
        if H.FindSet() then repeat R += H.No + '/' + H.Descr + ';'; until H.Next() = 0;
        exit(R);
    end;
}}
"#
    )
}

const SEED: &str = "H.No := 'H1'; H.Descr := 'First'; H.Insert(); H.No := 'H2'; H.Descr := 'Second'; H.Insert(); L.DocNo := 'H1'; L.LineNo := 10; L.Descr := 'A'; L.Insert(); L.LineNo := 20; L.Descr := 'B'; L.Insert(); L.DocNo := 'H2'; L.LineNo := 10; L.Descr := 'C'; L.Insert();";

#[test]
fn the_export_writes_the_first_child_on_the_parent_line() {
    // DfXmlPortLeads.al:VariableText_Nested_Export (TableSeparator <NewLine><NewLine>: 4 bytes, here the default CR/LF twice).
    assert_eq!(
        al_testkit::text(run(&src("", "", SEED, "''"), "Export")),
        "\"H1\",\"First\",\"10\",\"A\"\r\n\"20\",\"B\"\r\n\r\n\r\n\"H2\",\"Second\",\"10\",\"C\"\r\n\r\n"
    );
}

#[test]
fn a_parent_without_a_child_is_refused() {
    let seed = "H.No := 'H1'; H.Descr := 'First'; H.Insert();";
    assert!(matches!(run(&src("", "", seed, "''"), "Export"), Err(Signal::Unsupported(r)) if r.contains("not pinned")));
}

#[test]
fn an_optional_child_table_reads_every_line_as_a_parent() {
    let opt = "MinOccurs = Zero;";
    // VariableText_Nested_Import `header_only`, `two_headers`.
    assert_eq!(
        al_testkit::text(run(&src(opt, "", "", "'\"H1\",\"First\"' + Cr + Lf"), "Import")),
        "Yes|||H1/First;"
    );
    assert_eq!(
        al_testkit::text(run(
            &src(
                opt,
                "",
                "",
                "'\"H1\",\"First\"' + Cr + Lf + '\"H2\",\"Second\"' + Cr + Lf"
            ),
            "Import"
        )),
        "Yes|||H1/First;H2/Second;"
    );
    // `header_then_lines`: the empty line is an unexpected child; nothing is stored.
    assert_eq!(
        al_testkit::text(run(
            &src(
                opt,
                "",
                "",
                "'\"H1\",\"First\"' + Cr + Lf + Cr + Lf + '\"10\",\"A\"' + Cr + Lf"
            ),
            "Import"
        )),
        "No|The Element <PL> is unexpected.|XmlPortUnexpectedElement|"
    );
}

#[test]
fn a_required_child_table_is_missing_after_the_first_parent() {
    // Y83XpMiscTests.al:VariableText_Nested `nested doc two`, `nested doc three`: nothing is stored.
    for doc in ["'7,seven,c7a' + Cr + Lf", "'7,seven' + Cr + Lf + 'c7a' + Cr + Lf"] {
        assert_eq!(
            al_testkit::text(run(&src("", "", "", doc), "Import")),
            "No|The Element <PL> is expected by Min Occurs value: Once. Element received: <PH>.|XmlPortMetadata|"
        );
    }
}
