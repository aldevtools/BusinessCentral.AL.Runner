//! `Unbound = true` on a text element (corpus xmlport/lt-y60-xmlport-unbound-text/Y60XpUnboundText.al, xmlports 93220-93223):
//! on a VariableText import the unbound text element takes every further column of the line, OnAfterAssignVariable once per
//! column (empty ones included), all in one record; without Unbound the columns after the next are left out without an error.
//! In an Xml port it imports like a plain text element; BC's export of it does not end, so the export is refused.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(colx_props: &str, doc: &str) -> String {
    format!(
        r#"
table 69960 "XU Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69961 "XU Line" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 69962 "XU Log" {{ SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text begin exit(Log); end; }}
xmlport 69963 "XU Port"
{{
    Direction = Import; Format = VariableText; FieldSeparator = ','; FieldDelimiter = '"'; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            MinOccurs = Zero;
            tableelement(Hdr; "XU Line")
            {{
                AutoSave = false;
                textelement(col1) {{ MaxOccurs = Once; MinOccurs = Zero; trigger OnAfterAssignVariable() begin Log.Add('c1=' + col1); end; }}
                textelement(colx) {{ MinOccurs = Zero; {colx_props} trigger OnAfterAssignVariable() begin Log.Add('cx=' + colx); colx := ''; end; }}
                trigger OnAfterInitRecord() begin Log.Add('init'); end;
                trigger OnBeforeInsertRecord() begin Log.Add('ins'); end;
            }}
        }}
    }}
    trigger OnPostXmlPort() begin Log.Add('post col1=[' + col1 + '] colx=[' + colx + ']'); end;
    var Log: Codeunit "XU Log";
}}
codeunit 69964 C
{{
    procedure F(): Text
    var B: Record "XU Blob" temporary; OutS: OutStream; InS: InStream; Crlf: Text; Ch: Char; L: Codeunit "XU Log";
    begin
        Ch := 13; Crlf += Ch; Ch := 10; Crlf += Ch;
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('{doc}'.Replace('#', Crlf));
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XU Port", InS);
        exit(L.Take());
    end;
}}
"#
    )
}

const UNBOUND: &str = "Unbound = true;";

#[test]
fn an_unbound_text_element_takes_every_further_column() {
    // Y60XpUnboundText.al Unbound_Text_Import_ManyColumns.
    assert_eq!(
        al_testkit::text(run(&src(UNBOUND, "a,b,c,d#e,f,g,h#"), "F")),
        "init;c1=a;cx=b;cx=c;cx=d;ins;init;c1=e;cx=f;cx=g;cx=h;ins;post col1=[e] colx=[];"
    );
    assert_eq!(
        al_testkit::text(run(&src(UNBOUND, "a,b,c"), "F")),
        "init;c1=a;cx=b;cx=c;ins;post col1=[a] colx=[];"
    );
    // Unbound_Text_MaxOccursUnbounded: MaxOccurs = Unbounded changes nothing.
    assert_eq!(
        al_testkit::text(run(&src("Unbound = true; MaxOccurs = Unbounded;", "a,b,c,d#e,f,g,h#"), "F")),
        "init;c1=a;cx=b;cx=c;cx=d;ins;init;c1=e;cx=f;cx=g;cx=h;ins;post col1=[e] colx=[];"
    );
}

#[test]
fn an_unbound_text_element_reads_short_empty_and_quoted_columns() {
    // Unbound_Text_Import_ShortAndEmptyColumns, Unbound_Text_Import_TrailingSeparator_QuotedValue.
    assert_eq!(
        al_testkit::text(run(&src(UNBOUND, "a#b,,d#e,f#"), "F")),
        "init;c1=a;ins;init;c1=b;cx=;cx=d;ins;init;c1=e;cx=f;ins;post col1=[e] colx=[];"
    );
    assert_eq!(
        al_testkit::text(run(&src(UNBOUND, "a,b,#"), "F")),
        "init;c1=a;cx=b;cx=;ins;post col1=[a] colx=[];"
    );
    assert_eq!(
        al_testkit::text(run(&src(UNBOUND, "a,\"b,x\",c,#"), "F")),
        "init;c1=a;cx=b,x;cx=c;cx=;ins;post col1=[a] colx=[];"
    );
}

#[test]
fn without_unbound_the_further_columns_are_left_out() {
    // Unbound_Text_Import_WithoutUnbound_SameFile.
    assert_eq!(
        al_testkit::text(run(&src("", "a,b,c,d#e,f,g,h#"), "F")),
        "init;c1=a;cx=b;ins;init;c1=e;cx=f;ins;post col1=[e] colx=[];"
    );
}

fn xml_src(body: &str) -> String {
    format!(
        r#"
table 69965 "XW Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 69966 "XW Log" {{ SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text begin exit(Log); end; }}
xmlport 69967 "XW Port"
{{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            textelement(Plain) {{ MinOccurs = Zero; trigger OnAfterAssignVariable() begin Log.Add('plain=' + Plain); end; }}
            textelement(Free) {{ MinOccurs = Zero; Unbound = true; trigger OnAfterAssignVariable() begin Log.Add('free=' + Free); end; }}
        }}
    }}
    trigger OnPostXmlPort() begin Log.Add('post plain=[' + Plain + '] free=[' + Free + ']'); end;
    var Log: Codeunit "XW Log";
}}
codeunit 69968 C
{{
    procedure F(): Text
    var B: Record "XW Blob" temporary; OutS: OutStream; InS: InStream; L: Codeunit "XW Log";
    begin
        B.Code := 'X'; B.Insert();
        {body}
        exit(L.Take());
    end;
}}
"#
    )
}

#[test]
fn an_unbound_xml_text_element_imports_as_a_plain_one_and_its_export_is_refused() {
    // Unbound_Text_Xml_Import.
    let import = "B.Data.CreateOutStream(OutS); OutS.WriteText('<Root><Free>f</Free></Root>'); B.Data.CreateInStream(InS); Xmlport.Import(Xmlport::\"XW Port\", InS);";
    assert_eq!(al_testkit::text(run(&xml_src(import), "F")), "free=f;post plain=[] free=[f];");
    // Unbound_Text_Xml_Export (not a [Test]): BC's export does not end.
    let export = "B.Data.CreateOutStream(OutS); Xmlport.Export(Xmlport::\"XW Port\", OutS);";
    assert!(matches!(run(&xml_src(export), "F"), Err(Signal::Unsupported(r)) if r.contains("unbound")));
}
