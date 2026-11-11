//! The XmlPort shapes the corpus pins next to ones it does not, from the review of the WINDOWS VariableText import, the childless
//! root table element export and LinkTableForceInsert (al-runner issues 7775, 7396, 7441). A pinned shape runs; a neighbor that is
//! not pinned is refused (NotRun) or keeps the behavior it had before those pins.
//! - spaces around a text element value: only a quoted value with one trailing space in a WINDOWS port loses it
//!   (xmlports/import/XpVarTextWindowsImport.al:409 `1/3=[say {}]`); a port without TextEncoding keeps the spaces as before;
//! - the encoding a WINDOWS port decodes with: declared, or set on the variable before Import, or set in OnPreXmlPort
//!   (XpVarTextWindowsImport.al:337, :349, :385, :457); set in OnInitXmlPort or after the source was decoded, MSDos or UTF16 set,
//!   a byte order mark, and the five bytes code page 1252 leaves undefined are refused;
//! - a guarded static Import with a pending write is false before any trigger only for an Xml port
//!   (XpRootTableNoChildren.al:438); a VariableText port with an empty source fails as before;
//! - LinkTableForceInsert in an import: true on an unlinked table element with one linked child that has LinkFields runs
//!   (XpLinkTableForceInsertUnlinked.al:586); every other import shape is refused;
//! - a childless root table element exports for a real table with Encoding = UTF8 (XpRootTableNoChildren.al:13-160); a temporary
//!   one or one of a UTF-16 port is refused as before.

use al_testrun::Outcome;

const SRC: &str = r#"
table 50100 "RP Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "RP Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[50]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50102 "RP Par" { fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50103 "RP Chi" { fields { field(1; "Parent No."; Code[10]) { } field(2; "Line No."; Integer) { } } keys { key(PK; "Parent No.", "Line No.") { Clustered = true; } } }

xmlport 50110 "RP Cols Plain"
{
    Direction = Import; Format = VariableText; UseRequestPage = false;
    schema { textelement(root) { MinOccurs = Zero;
        tableelement(Row; "RP Row") { AutoSave = false;
            textelement(col1) { MaxOccurs = Once; MinOccurs = Zero; trigger OnAfterAssignVariable() begin Log += '[' + col1 + ']'; end; }
            textelement(colx) { MinOccurs = Zero; Unbound = true; trigger OnAfterAssignVariable() begin Log += '[' + colx + ']'; end; }
        } } }
    trigger OnPreXmlPort() begin currXMLport.FieldSeparator(';'); end;
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
xmlport 50111 "RP Cols Win"
{
    Direction = Import; Format = VariableText; TextEncoding = WINDOWS; UseRequestPage = false;
    schema { textelement(root) { MinOccurs = Zero;
        tableelement(Row; "RP Row") { AutoSave = false;
            textelement(col1) { MaxOccurs = Once; MinOccurs = Zero; trigger OnAfterAssignVariable() begin Log += '[' + col1 + ']'; end; }
            textelement(colx) { MinOccurs = Zero; Unbound = true; trigger OnAfterAssignVariable() begin Log += '[' + colx + ']'; end; }
            trigger OnAfterInitRecord() begin if Mode = 3 then currXMLport.TextEncoding(TextEncoding::UTF8); end;
        } } }
    trigger OnInitXmlPort() begin if Mode = 6 then currXMLport.TextEncoding(TextEncoding::UTF8); end;
    trigger OnPreXmlPort() begin currXMLport.FieldSeparator(';');
        case Mode of 1: currXMLport.TextEncoding(TextEncoding::UTF8); 2: currXMLport.TextEncoding(TextEncoding::Windows); 4: currXMLport.TextEncoding(TextEncoding::UTF16); 5: currXMLport.TextEncoding(TextEncoding::MSDos); end; end;
    var Log: Text; Mode: Integer;
    procedure Cfg(M: Integer) begin Mode := M; end;
    procedure GetLog(): Text begin exit(Log); end;
}
xmlport 50112 "RP Root Bare Utf8" { Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false; schema { tableelement(MyRequest; "RP Row") { } } }
xmlport 50113 "RP Root Bare Temp" { Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false; schema { tableelement(MyRequest; "RP Row") { UseTemporary = true; } } }
xmlport 50114 "RP Root Bare Utf16" { Direction = Both; Format = Xml; UseRequestPage = false; schema { tableelement(MyRequest; "RP Row") { } } }
xmlport 50120 "RP Link False"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { MinOccurs = Zero; fieldelement(No; P."No.") { }
        tableelement(C; "RP Chi") { MinOccurs = Zero; LinkTable = P; LinkFields = "Parent No." = field("No."); LinkTableForceInsert = false; fieldelement(Line; C."Line No.") { } } } } }
}
xmlport 50121 "RP Link Two"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { MinOccurs = Zero; LinkTableForceInsert = true; fieldelement(No; P."No.") { }
        tableelement(C; "RP Chi") { MinOccurs = Zero; LinkTable = P; LinkFields = "Parent No." = field("No."); fieldelement(Line; C."Line No.") { } }
        tableelement(D; "RP Row") { MinOccurs = Zero; LinkTable = P; fieldelement(Id; D.Id) { } } } } }
}
xmlport 50122 "RP Link No Fields"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { MinOccurs = Zero; LinkTableForceInsert = true; fieldelement(No; P."No.") { }
        tableelement(D; "RP Row") { MinOccurs = Zero; LinkTable = P; fieldelement(Id; D.Id) { } } } } }
}
xmlport 50123 "RP Link Only"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { MinOccurs = Zero; LinkTableForceInsert = true; fieldelement(No; P."No.") { } } } }
}
xmlport 50124 "RP Link Text"
{
    Direction = Both; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { LinkTableForceInsert = true; fieldelement(No; P."No.") { } fieldelement(Name; P.Name) { } } } }
}
xmlport 50125 "RP Link One"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "RP Par") { MinOccurs = Zero; LinkTableForceInsert = true; fieldelement(No; P."No.") { }
        tableelement(C; "RP Chi") { MinOccurs = Zero; LinkTable = P; LinkFields = "Parent No." = field("No."); fieldelement(Line; C."Line No.") { } } } } }
}
xmlport 50126 "RP Fields Plain" { Format = VariableText; UseRequestPage = false; schema { textelement(root) { tableelement(Row; "RP Row") { fieldelement(Id; Row.Id) { } fieldelement(Name; Row.Name) { } } } } }

codeunit 50149 H
{

    procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1', Got); end;

    procedure Crlf(): Text
    var T: Text[2];
    begin T[1] := 13; T[2] := 10; exit(T); end;

    procedure Src(var B: Record "RP Blob" temporary; Doc: Text; Enc: TextEncoding; var InS: InStream)
    var OutS: OutStream;
    begin
        B.Id := 1; B.Data.CreateOutStream(OutS, Enc); OutS.WriteText(Doc); B.Insert(); B.Data.CreateInStream(InS);
    end;

    // Lead: 1 the UTF-8 byte order mark, 2 FF FE, 3 FE FF, 128 and up that one byte; then Doc in Windows-1252.
    procedure SrcBytes(var B: Record "RP Blob" temporary; Lead: Integer; Doc: Text; var InS: InStream)
    var OutS: OutStream; By: Byte;
    begin
        B.Id := 1; B.Data.CreateOutStream(OutS, TextEncoding::Windows);
        case Lead of
            1: begin By := 239; OutS.Write(By); By := 187; OutS.Write(By); By := 191; OutS.Write(By); end;
            2: begin By := 255; OutS.Write(By); By := 254; OutS.Write(By); end;
            3: begin By := 254; OutS.Write(By); By := 255; OutS.Write(By); end;
            else begin By := Lead; OutS.Write(By); end;
        end;
        OutS.WriteText(Doc); B.Insert(); B.Data.CreateInStream(InS);
    end;

    procedure Plain(Doc: Text): Text
    var B: Record "RP Blob" temporary; P: XmlPort "RP Cols Plain"; InS: InStream;
    begin
        Src(B, Doc, TextEncoding::UTF8, InS); P.SetSource(InS); P.Import(); exit(P.GetLog());
    end;

    procedure Win(Doc: Text; Mode: Integer; Lead: Integer): Text
    var B: Record "RP Blob" temporary; P: XmlPort "RP Cols Win"; InS: InStream;
    begin
        if Lead = 0 then Src(B, Doc, TextEncoding::Windows, InS) else SrcBytes(B, Lead, Doc, InS);
        P.Cfg(Mode); P.SetSource(InS); P.Import(); exit(P.GetLog());
    end;


    procedure Exp(Id: Integer): Text
    var B: Record "RP Blob" temporary; R: Record "RP Row"; OutS: OutStream; InS: InStream; T: Text; L: Text;
    begin
        R.DeleteAll(); R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        B.Id := 1; B.Data.CreateOutStream(OutS);
        Xmlport.Export(Id, OutS);
        B.Insert(); B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(L); T += L + '|'; end;
        exit(T);
    end;


    procedure Imp(Id: Integer; Doc: Text; Pending: Boolean): Text
    var B: Record "RP Blob" temporary; R: Record "RP Row"; Pa: Record "RP Par"; Ch: Record "RP Chi"; InS: InStream; Ok: Boolean;
    begin
        Src(B, Doc, TextEncoding::UTF8, InS);
        R.DeleteAll(); Pa.DeleteAll(); Ch.DeleteAll(); Commit();
        if Pending then begin R.Id := 99; R.Insert(); end;
        Ok := Xmlport.Import(Id, InS);
        exit(StrSubstNo('%1 rows=%2 par=%3 chi=%4', Ok, R.Count(), Pa.Count(), Ch.Count()));
    end;

}
codeunit 50150 TP
{
    Subtype = Test;
    var H: Codeunit H;
    [Test] procedure Plain_UnquotedSpacesStay() begin H.Check(H.Plain('a; b ;c ' + H.Crlf() + ' d;e' + H.Crlf()), '[a][ b ][c ][ d][e]'); end;
    [Test] procedure Plain_QuotedSpacesStay() begin H.Check(H.Plain('"a ";" b ";c' + H.Crlf()), '[a ][ b ][c]'); end;
    [Test] procedure Plain_OnlySpacesStay() begin H.Check(H.Plain('a;   ;c' + H.Crlf()), '[a][   ][c]'); end;
    [Test] procedure Win_QuotedTrailingSpace() begin H.Check(H.Win('C1;"say ";x' + H.Crlf(), 0, 0), '[C1][say][x]'); end;
    [Test] procedure Win_SetUtf8InPre() begin H.Check(H.Win('a;b' + H.Crlf(), 1, 0), '[a][b]'); end;
    [Test] procedure Root_Utf8_Export() begin H.Check(H.Exp(Xmlport::"RP Root Bare Utf8"), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<MyRequest />|<MyRequest />|'); end;
    [Test] procedure Link_OneChild_Import() begin H.Check(H.Imp(Xmlport::"RP Link One", '<Root><P><No>P1</No><C><Line>1</Line></C></P></Root>', false), 'Yes rows=0 par=1 chi=1'); end;
    [Test] procedure Pending_Text_EmptySource()
    begin
        asserterror H.Imp(Xmlport::"RP Fields Plain", '', true);
        H.Check(GetLastErrorText(), 'Cannot import the specified XML document because the file is empty.');
    end;
}
"#;

const REFUSED: [(&str, &str); 24] = [
    ("Win_UnquotedSpaces", r#"H.Win('a; b ;c' + H.Crlf(), 0, 0);"#),
    ("Win_QuotedLeadingSpace", r#"H.Win('a;" b";c' + H.Crlf(), 0, 0);"#),
    (
        "Win_SetUtf8AfterDecoding",
        r#"H.Win('a;b' + H.Crlf() + 'c;d' + H.Crlf(), 3, 0);"#,
    ),
    ("Win_SetUtf8InInit", r#"H.Win('a;b' + H.Crlf(), 6, 0);"#),
    ("Win_SetUtf16", r#"H.Win('a;b' + H.Crlf(), 4, 0);"#),
    ("Win_SetMsdos", r#"H.Win('a;b' + H.Crlf(), 5, 0);"#),
    ("Win_Utf8Bom", r#"H.Win('a;b' + H.Crlf(), 0, 1);"#),
    ("Win_Utf16LeBom", r#"H.Win('a;b' + H.Crlf(), 0, 2);"#),
    ("Win_Utf16BeBom", r#"H.Win('a;b' + H.Crlf(), 0, 3);"#),
    ("Win_SetUtf8_Utf8Bom", r#"H.Win('a;b' + H.Crlf(), 1, 1);"#),
    ("Win_SetUtf8_Utf16LeBom", r#"H.Win('a;b' + H.Crlf(), 1, 2);"#),
    ("Win_SetUtf8_Utf16BeBom", r#"H.Win('a;b' + H.Crlf(), 1, 3);"#),
    ("Win_Undefined81", r#"H.Win('a;b' + H.Crlf(), 0, 129);"#),
    ("Win_Undefined8D", r#"H.Win('a;b' + H.Crlf(), 0, 141);"#),
    ("Win_Undefined8F", r#"H.Win('a;b' + H.Crlf(), 0, 143);"#),
    ("Win_Undefined90", r#"H.Win('a;b' + H.Crlf(), 0, 144);"#),
    ("Win_Undefined9D", r#"H.Win('a;b' + H.Crlf(), 0, 157);"#),
    ("Root_Temp_Export", r#"H.Exp(Xmlport::"RP Root Bare Temp");"#),
    ("Root_Utf16_Export", r#"H.Exp(Xmlport::"RP Root Bare Utf16");"#),
    (
        "Link_False_Import",
        r#"H.Imp(Xmlport::"RP Link False", '<Root><P><No>P1</No><C><Line>1</Line></C></P></Root>', false);"#,
    ),
    (
        "Link_TwoChildren_Import",
        r#"H.Imp(Xmlport::"RP Link Two", '<Root><P><No>P1</No><C><Line>1</Line></C><D><Id>5</Id></D></P></Root>', false);"#,
    ),
    (
        "Link_NoLinkFields_Import",
        r#"H.Imp(Xmlport::"RP Link No Fields", '<Root><P><No>P1</No><D><Id>5</Id></D></P></Root>', false);"#,
    ),
    (
        "Link_NoChild_Import",
        r#"H.Imp(Xmlport::"RP Link Only", '<Root><P><No>P1</No></P></Root>', false);"#,
    ),
    (
        "Link_VariableText_Import",
        r#"H.Imp(Xmlport::"RP Link Text", 'P1,one' + H.Crlf(), false);"#,
    ),
];

#[test]
fn xmlport_pinned_shapes_run() {
    let got = al_testkit::outcomes_in(SRC, "TP");
    let wrong: Vec<_> = got.iter().filter(|(_, o)| o != &Outcome::Pass).collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert_eq!(got.len(), 8);
}

/// Each case runs in a codeunit of its own: a case refused while it ran makes the later cases of its codeunit untrusted.
#[test]
fn xmlport_neighbors_of_the_pins_are_refused() {
    let mut wrong = Vec::new();
    for (name, body) in REFUSED {
        let src = format!(
            "{SRC}\ncodeunit 50151 TR {{ Subtype = Test; var H: Codeunit H; [Test] procedure {name}() begin {body} end; }}\n"
        );
        let got = al_testkit::outcomes_in(&src, "TR");
        if !matches!(&got[..], [(_, Outcome::NotRun(_))]) {
            wrong.push(got);
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
