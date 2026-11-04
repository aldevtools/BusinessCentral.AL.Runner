//! `ImportFile()` in OnPreXmlPort and OnPostXmlPort: true while the port imports, false while it exports (corpus
//! xmlports/import/XmlPortFixtures.al:265, :275; XpTImportOptions.al:268 Import_TextElementsAndAttributes).

use al_testkit::run;

const SRC: &str = r#"
table 69950 "XI Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
codeunit 69951 "XI Log" { SingleInstance = true; var T: Text; procedure Add(E: Text) begin T += E; end; procedure Get(): Text begin exit(T); end; }
xmlport 69952 "XI Port"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Doc) { textelement(Title) { } } }
    trigger OnPreXmlPort() var L: Codeunit "XI Log"; begin L.Add('pre=' + Format(currXMLport.ImportFile()) + ';'); end;
    trigger OnPostXmlPort() var L: Codeunit "XI Log"; begin L.Add('post=' + Format(currXMLport.ImportFile()) + ';'); end;
}
codeunit 69953 C
{
    procedure F(): Text
    var B: Record "XI Blob" temporary; L: Codeunit "XI Log"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XI Port", OutS);
        Clear(B.Data);
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Doc><Title>t</Title></Doc>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XI Port", InS);
        exit(L.Get());
    end;
}
"#;

#[test]
fn import_file_tells_the_direction_of_the_run() {
    assert_eq!(al_testkit::text(run(SRC, "F")), "pre=No;post=No;pre=Yes;post=Yes;");
}

// xmlports/shapes-properties/AyXmlPortFlowTriggers.al:352-389 Import_pre_*: Break, BreakUnbound and Quit in OnPreXmlPort end the
// import there; the code after the call, the document and OnPostXmlPort do not run.
const FLOW: &str = r#"
table 69955 "XQ Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69956 "XQ Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
xmlport 69957 "XQ Port"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; "XQ Row") { fieldelement(Id; R.Id) { } } } }
    var Log: Text; Action: Integer;
    procedure SetAction(A: Integer) begin Action := A; end;
    procedure GetLog(): Text begin exit(Log); end;
    local procedure Act()
    begin
        case Action of
            1: currXMLport.Break();
            2: currXMLport.BreakUnbound();
            3: currXMLport.Quit();
        end;
        Log += 'after-call;';
    end;
    trigger OnPreXmlPort() begin Log += 'pre;'; Act(); Log += 'after-pre;'; end;
    trigger OnPostXmlPort() begin Log += 'post;'; end;
}
codeunit 69958 C
{
    procedure F(A: Integer): Text
    var B: Record "XQ Blob" temporary; Row: Record "XQ Row"; P: XmlPort "XQ Port"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><R><Id>1</Id></R></Root>');
        B.Data.CreateInStream(InS);
        P.SetAction(A); P.SetSource(InS); P.Import();
        exit(P.GetLog() + Format(Row.Count()));
    end;
    procedure Break(): Text begin exit(F(1)); end;
    procedure Unbound(): Text begin exit(F(2)); end;
    procedure Quit(): Text begin exit(F(3)); end;
}
"#;

#[test]
fn break_and_quit_in_onprexmlport_end_an_import() {
    for routine in ["Break", "Unbound", "Quit"] {
        assert_eq!(al_testkit::text(run(FLOW, routine)), "pre;0", "{routine}");
    }
}
