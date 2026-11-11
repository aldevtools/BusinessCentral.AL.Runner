//! XmlPort import of a port whose root is a table element (MinOccurs and MaxOccurs Once, no fields of its own) holding a nested
//! table element of headers with leaf text and field elements and a text element that wraps a table element of lines (al-runner
//! issue 6924, the reporter's shape with generic names). Each test is a cell of xmlports/shapes-properties/XpRootTableNested.al
//! (corpus PR 1308, measured on BC 27.5, 28.5 and 29.0); when the corpus bump brings that file into the oracle it is the test.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50300 "RT Root"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50301 "RT Header"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Customer; Code[20]) { }
        field(3; "Ship-to Code"; Code[10]) { }
        field(4; Reference; Code[20]) { }
        field(5; "Location Code"; Code[10]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50302 "RT Line"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Header No."; Integer) { }
        field(3; Item; Code[20]) { }
        field(4; Quantity; Decimal) { }
        field(5; Description; Text[100]) { }
        field(6; "Unit Price"; Decimal) { }
        field(7; "Discount Pct"; Decimal) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50304 "RT Pending"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50303 "RT Blob"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

xmlport 50300 "RT Full"
{
    Format = Xml;
    FormatEvaluate = Xml;

    schema
    {
        tableelement(Msg; "RT Root")
        {
            XmlName = 'Message';
            UseTemporary = true;
            MinOccurs = Once;
            MaxOccurs = Once;

            tableelement(Header; "RT Header")
            {
                UseTemporary = true;
                MinOccurs = Once;
                MaxOccurs = Unbounded;

                textelement(Kind) { MinOccurs = Once; MaxOccurs = Once; }
                fieldelement(Customer; Header.Customer) { MinOccurs = Once; MaxOccurs = Once; }
                fieldelement(ShipTo; Header."Ship-to Code") { MinOccurs = Zero; MaxOccurs = Once; }
                textelement(OrderDate) { MinOccurs = Zero; MaxOccurs = Once; }
                textelement(DueDate) { MinOccurs = Zero; MaxOccurs = Once; }
                fieldelement(Reference; Header.Reference) { MinOccurs = Once; MaxOccurs = Once; }
                fieldelement(Location; Header."Location Code") { MinOccurs = Zero; MaxOccurs = Once; }
                textelement(Lines)
                {
                    MinOccurs = Once;
                    MaxOccurs = Once;

                    tableelement(Line; "RT Line")
                    {
                        UseTemporary = true;

                        fieldelement(Item; Line.Item) { MinOccurs = Once; MaxOccurs = Once; }
                        fieldelement(Qty; Line.Quantity) { MinOccurs = Once; MaxOccurs = Once; }
                        fieldelement(Descr; Line.Description) { MinOccurs = Zero; MaxOccurs = Once; }
                        fieldelement(Price; Line."Unit Price") { MinOccurs = Zero; MaxOccurs = Once; }
                        fieldelement(Disc; Line."Discount Pct") { MinOccurs = Zero; MaxOccurs = Once; }

                        trigger OnBeforeInsertRecord()
                        begin
                            LineNo += 1;
                            Line."Entry No." := LineNo;
                            Line."Header No." := Header."Entry No.";
                            Add('L.bins:' + Line.Item + '/hdr=' + Format(Header."Entry No.") + '/kind=[' + Kind + ']/cust=[' + Header.Customer + ']');
                        end;
                    }
                }

                trigger OnAfterInitRecord()
                begin
                    HeaderNo += 1;
                    Header."Entry No." := HeaderNo;
                    Add('H.init:' + Format(HeaderNo) + '/kind=[' + Kind + ']/od=[' + OrderDate + ']/dd=[' + DueDate + ']/lines=[' + Lines + ']');
                end;

                trigger OnAfterInsertRecord()
                begin
                    Add('H.ains:' + Format(Header."Entry No.") + '/kind=[' + Kind + ']/od=[' + OrderDate + ']/dd=[' + DueDate + ']/lines=[' + Lines + ']');
                end;
            }
        }
    }

    trigger OnInitXmlPort()
    begin
        ClearLastError();
        Add('init');
    end;

    trigger OnPreXmlPort()
    begin
        ClearLastError();
        Add('pre');
    end;

    trigger OnPostXmlPort()
    begin
        Add('post');
    end;

    var
        Log: Text;
        HeaderNo: Integer;
        LineNo: Integer;

    local procedure Add(Entry: Text)
    begin
        Log += Entry + ';';
    end;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;

    procedure Roots(): Text
    var
        Result: Text;
    begin
        Msg.Reset();
        if Msg.FindSet() then
            repeat
                Result += Format(Msg.Id) + ';';
            until Msg.Next() = 0;
        exit(Format(Msg.Count()) + ':' + Result);
    end;

    procedure Headers(): Text
    var
        Result: Text;
    begin
        Header.Reset();
        if Header.FindSet() then
            repeat
                Result += Format(Header."Entry No.") + '/' + Header.Customer + '/[' + Header."Ship-to Code" + ']/' + Header.Reference + '/[' + Header."Location Code" + '];';
            until Header.Next() = 0;
        exit(Format(Header.Count()) + ':' + Result);
    end;

    procedure LinesDump(): Text
    var
        Result: Text;
    begin
        Line.Reset();
        if Line.FindSet() then
            repeat
                Result += Format(Line."Entry No.") + '/' + Format(Line."Header No.") + '/' + Line.Item + '/' + Format(Line.Quantity, 0, 9) + '/' + Line.Description + '/' +
                  Format(Line."Unit Price", 0, 9) + '/' + Format(Line."Discount Pct", 0, 9) + ';';
            until Line.Next() = 0;
        exit(Format(Line.Count()) + ':' + Result);
    end;
}

xmlport 50301 "RT Real"
{
    Format = Xml;
    FormatEvaluate = Xml;

    schema
    {
        tableelement(Msg; "RT Root")
        {
            XmlName = 'Message';
            MinOccurs = Once;
            MaxOccurs = Once;

            tableelement(Header; "RT Header")
            {
                UseTemporary = true;
                MinOccurs = Once;
                MaxOccurs = Unbounded;

                textelement(Kind) { }
                fieldelement(Customer; Header.Customer) { }
                fieldelement(ShipTo; Header."Ship-to Code") { MinOccurs = Zero; }
                textelement(OrderDate) { MinOccurs = Zero; }
                textelement(DueDate) { MinOccurs = Zero; }
                fieldelement(Reference; Header.Reference) { }
                fieldelement(Location; Header."Location Code") { MinOccurs = Zero; }
                textelement(Lines)
                {
                    tableelement(Line; "RT Line")
                    {
                        UseTemporary = true;

                        fieldelement(Item; Line.Item) { }
                        fieldelement(Qty; Line.Quantity) { }
                        fieldelement(Descr; Line.Description) { MinOccurs = Zero; }
                        fieldelement(Price; Line."Unit Price") { MinOccurs = Zero; }
                        fieldelement(Disc; Line."Discount Pct") { MinOccurs = Zero; }

                        trigger OnBeforeInsertRecord()
                        begin
                            LineNo += 1;
                            Line."Entry No." := LineNo;
                            Line."Header No." := Header."Entry No.";
                        end;
                    }
                }

                trigger OnAfterInitRecord()
                begin
                    HeaderNo += 1;
                    Header."Entry No." := HeaderNo;
                end;
            }
        }
    }

    var
        HeaderNo: Integer;
        LineNo: Integer;

    procedure Counts(): Text
    begin
        Header.Reset();
        Line.Reset();
        exit('headers=' + Format(Header.Count()) + '/lines=' + Format(Line.Count()));
    end;
}

xmlport 50302 "RT Min"
{
    Format = Xml;
    FormatEvaluate = Xml;

    schema
    {
        tableelement(Msg; "RT Root")
        {
            XmlName = 'Message';
            UseTemporary = true;
            MinOccurs = Once;
            MaxOccurs = Once;

            tableelement(Header; "RT Header")
            {
                UseTemporary = true;
                MaxOccurs = Unbounded;

                fieldelement(No; Header."Entry No.") { }
                fieldelement(Customer; Header.Customer) { }
                fieldelement(ShipTo; Header."Ship-to Code") { MinOccurs = Zero; }
            }
        }
    }

    procedure Dump(): Text
    var
        Result: Text;
    begin
        Msg.Reset();
        Result := 'roots=' + Format(Msg.Count());
        if Msg.FindFirst() then
            Result += '/' + Format(Msg.Id);
        Result += ' headers=';
        Header.Reset();
        if Header.FindSet() then
            repeat
                Result += Format(Header."Entry No.") + '/' + Header.Customer + '/[' + Header."Ship-to Code" + '];';
            until Header.Next() = 0;
        exit(Result);
    end;
}

codeunit 50300 "RT Tests"
{
    Subtype = Test;

    local procedure Check(What: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1: got [%2], want [%3]', What, Got, Want);
    end;

    local procedure Nl(): Text
    var
        Cr: Char;
        Lf: Char;
    begin
        Cr := 13;
        Lf := 10;
        exit(Format(Cr) + Format(Lf));
    end;

    local procedure Tb(Depth: Integer): Text
    var
        T: Char;
        Result: Text;
        i: Integer;
    begin
        T := 9;
        for i := 1 to Depth do
            Result += Format(T);
        exit(Result);
    end;

    local procedure LineXml(Item: Text; Qty: Text; Descr: Text; Price: Text; Disc: Text): Text
    begin
        exit(Tb(4) + '<Line><Item>' + Item + '</Item><Qty>' + Qty + '</Qty><Descr>' + Descr + '</Descr><Price>' + Price + '</Price><Disc>' + Disc + '</Disc></Line>' + Nl());
    end;

    local procedure HeaderXml(Customer: Text; Reference: Text; LinesXml: Text): Text
    begin
        exit(Tb(1) + '<Header>' + Nl() +
          Tb(2) + '<Kind>Order</Kind>' + Nl() +
          Tb(2) + '<Customer>' + Customer + '</Customer>' + Nl() +
          Tb(2) + '<ShipTo></ShipTo>' + Nl() +
          Tb(2) + '<OrderDate>20200326</OrderDate>' + Nl() +
          Tb(2) + '<DueDate>20200327</DueDate>' + Nl() +
          Tb(2) + '<Reference>' + Reference + '</Reference>' + Nl() +
          Tb(2) + '<Location></Location>' + Nl() +
          Tb(2) + '<Lines>' + Nl() + LinesXml + Tb(2) + '</Lines>' + Nl() +
          Tb(1) + '</Header>' + Nl());
    end;

    local procedure IssueDoc(): Text
    begin
        exit('<?xml version=''1.0'' encoding=''ISO-8859-1''?>' + Nl() + '<Message>' + Nl() +
          HeaderXml('C10000', 'REF-04', LineXml('ITEM001', '2', 'first line', '6.32', '100') + LineXml('ITEM002', '10', 'second line', '1.5', '0')) +
          '</Message>' + Nl());
    end;

    local procedure WriteSource(var Src: Record "RT Blob" temporary; Doc: Text; Enc: TextEncoding; var InS: InStream)
    var
        OutS: OutStream;
    begin
        Src.Init();
        Src.Code := 'SRC';
        Src.Data.CreateOutStream(OutS, Enc);
        OutS.WriteText(Doc);
        Src.Insert();
        Src.Data.CreateInStream(InS);
    end;

    // Imports Doc into "RT Full" with `if not Import() then` and checks every observation of the corpus cell.
    local procedure ImportFull(Doc: Text; Enc: TextEncoding; SetWindows: Boolean; WantImport: Boolean; WantError: Text; WantCode: Text; WantLog: Text; WantRoots: Text; WantHeaders: Text; WantLines: Text)
    var
        Src: Record "RT Blob" temporary;
        Port: XmlPort "RT Full";
        InS: InStream;
        Imported: Boolean;
    begin
        WriteSource(Src, Doc, Enc, InS);
        if SetWindows then
            Port.TextEncoding := TextEncoding::Windows;
        Port.SetSource(InS);
        asserterror Error('an earlier error');
        Imported := Port.Import();
        Check('import', Format(Imported), Format(WantImport));
        if Imported then
            Check('last error after', GetLastErrorText(), '')
        else begin
            Check('error text', GetLastErrorText(), WantError);
            Check('error code', GetLastErrorCode(), WantCode);
        end;
        Check('log', Port.GetLog(), WantLog);
        Check('roots', Port.Roots(), WantRoots);
        Check('headers', Port.Headers(), WantHeaders);
        Check('lines', Port.LinesDump(), WantLines);
    end;

    local procedure IssueLog(): Text
    begin
        exit('init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];L.bins:ITEM001/hdr=1/kind=[Order]/cust=[C10000];L.bins:ITEM002/hdr=1/kind=[Order]/cust=[C10000];H.ains:1/kind=[Order]/od=[20200326]/dd=[20200327]/lines=[];post;');
    end;

    // XpRootTableNested.al:475 RT_Full_IssueDoc_WindowsBlob_Import
    [Test]
    procedure Full_IssueDoc_WindowsBlob()
    begin
        ImportFull(IssueDoc(), TextEncoding::Windows, true, true, '', '', IssueLog(), '1:0;', '1:1/C10000/[]/REF-04/[];',
          '2:1/1/ITEM001/2/first line/6.32/100;2/1/ITEM002/10/second line/1.5/0;');
    end;

    // XpRootTableNested.al:489 RT_Full_IssueDoc_Utf8NoTextEncoding_Import
    [Test]
    procedure Full_IssueDoc_Utf8NoTextEncoding()
    var
        Doc: Text;
    begin
        Doc := IssueDoc();
        Doc := Doc.Replace('<?xml version=''1.0'' encoding=''ISO-8859-1''?>' + Nl(), '');
        ImportFull(Doc, TextEncoding::UTF8, false, true, '', '', IssueLog(), '1:0;', '1:1/C10000/[]/REF-04/[];',
          '2:1/1/ITEM001/2/first line/6.32/100;2/1/ITEM002/10/second line/1.5/0;');
    end;

    // XpRootTableNested.al:507 RT_Full_IssueDoc_Unguarded_Import
    [Test]
    procedure Full_IssueDoc_Unguarded()
    var
        Src: Record "RT Blob" temporary;
        Port: XmlPort "RT Full";
        InS: InStream;
    begin
        WriteSource(Src, IssueDoc(), TextEncoding::Windows, InS);
        Port.TextEncoding := TextEncoding::Windows;
        Port.SetSource(InS);
        Port.Import();
        Check('log', Port.GetLog(), IssueLog());
        Check('headers', Port.Headers(), '1:1/C10000/[]/REF-04/[];');
        Check('lines', Port.LinesDump(), '2:1/1/ITEM001/2/first line/6.32/100;2/1/ITEM002/10/second line/1.5/0;');
    end;

    // XpRootTableNested.al:528 RT_Full_NonAsciiDescription_Windows_Import
    [Test]
    procedure Full_NonAsciiDescription_Windows()
    var
        Doc: Text;
    begin
        Doc := IssueDoc();
        Doc := Doc.Replace('first line', 'Größe ü');
        ImportFull(Doc, TextEncoding::Windows, true, true, '', '', IssueLog(), '1:0;', '1:1/C10000/[]/REF-04/[];',
          '2:1/1/ITEM001/2/Größe ü/6.32/100;2/1/ITEM002/10/second line/1.5/0;');
    end;

    // XpRootTableNested.al:546 RT_Full_TwoHeaders_MissingOptional_Import
    [Test]
    procedure Full_TwoHeaders_MissingOptional()
    var
        Doc: Text;
        Second: Text;
    begin
        Second := Tb(1) + '<Header>' + Nl() +
          Tb(2) + '<Kind>Return</Kind>' + Nl() +
          Tb(2) + '<Customer>C20000</Customer>' + Nl() +
          Tb(2) + '<Reference>REF-05</Reference>' + Nl() +
          Tb(2) + '<Location>BLUE</Location>' + Nl() +
          Tb(2) + '<Lines>' + Nl() + LineXml('ITEM003', '3', 'third line', '2', '5') + Tb(2) + '</Lines>' + Nl() +
          Tb(1) + '</Header>' + Nl();
        Doc := IssueDoc();
        Doc := Doc.Replace('</Message>', Second + '</Message>');
        ImportFull(Doc, TextEncoding::Windows, true, true, '', '',
          'init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];L.bins:ITEM001/hdr=1/kind=[Order]/cust=[C10000];L.bins:ITEM002/hdr=1/kind=[Order]/cust=[C10000];H.ains:1/kind=[Order]/od=[20200326]/dd=[20200327]/lines=[];H.init:2/kind=[Order]/od=[20200326]/dd=[20200327]/lines=[];L.bins:ITEM003/hdr=2/kind=[Return]/cust=[C20000];H.ains:2/kind=[Return]/od=[20200326]/dd=[20200327]/lines=[];post;',
          '1:0;', '2:1/C10000/[]/REF-04/[];2/C20000/[]/REF-05/[BLUE];',
          '3:1/1/ITEM001/2/first line/6.32/100;2/1/ITEM002/10/second line/1.5/0;3/2/ITEM003/3/third line/2/5;');
    end;

    // XpRootTableNested.al:573 RT_Full_Decimals_FormatEvaluateXml_Import
    [Test]
    procedure Full_Decimals_FormatEvaluateXml()
    begin
        ImportFull('<Message><Header><Kind>Order</Kind><Customer>C1</Customer><Reference>R1</Reference><Lines>' +
          '<Line><Item>A</Item><Qty>1234.5678</Qty><Price>-0.001</Price><Disc>0050.50</Disc></Line>' +
          '<Line><Item>B</Item><Qty>7</Qty><Descr></Descr><Price>.5</Price></Line>' +
          '</Lines></Header></Message>', TextEncoding::UTF8, false, true, '', '',
          'init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];L.bins:A/hdr=1/kind=[Order]/cust=[C1];L.bins:B/hdr=1/kind=[Order]/cust=[C1];H.ains:1/kind=[Order]/od=[]/dd=[]/lines=[];post;',
          '1:0;', '1:1/C1/[]/R1/[];', '2:1/1/A/1234.5678//-0.001/50.5;2/1/B/7//0.5/0;');
    end;

    // XpRootTableNested.al:593 RT_Full_DecimalWithComma_Import
    [Test]
    procedure Full_DecimalWithComma()
    begin
        ImportFull('<Message><Header><Kind>Order</Kind><Customer>C1</Customer><Reference>R1</Reference><Lines>' +
          '<Line><Item>A</Item><Qty>1,5</Qty></Line></Lines></Header></Message>', TextEncoding::UTF8, false, false,
          'The value "1,5" can''t be evaluated into type Decimal.', 'Evaluate',
          'init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];', '0:', '0:', '0:');
    end;

    // XpRootTableNested.al:612 RT_Full_HeaderWithoutLines_Import
    [Test]
    procedure Full_HeaderWithoutLines()
    begin
        ImportFull('<Message><Header><Kind>Order</Kind><Customer>C1</Customer><Reference>R1</Reference><Lines></Lines></Header></Message>',
          TextEncoding::UTF8, false, false,
          'Imported XML cannot validate with the schema: The element ''Lines'' has incomplete content. List of possible elements expected: ''Line''.',
          'XmlPortInvalidXml', 'init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];', '0:', '0:', '0:');
    end;

    // XpRootTableNested.al:630 RT_Full_HeaderWithoutReference_Import
    [Test]
    procedure Full_HeaderWithoutReference()
    begin
        ImportFull('<Message><Header><Kind>Order</Kind><Customer>C1</Customer><Lines><Line><Item>A</Item><Qty>1</Qty></Line></Lines></Header></Message>',
          TextEncoding::UTF8, false, false,
          'Imported XML cannot validate with the schema: The element ''Header'' has invalid child element ''Lines''. List of possible elements expected: ''ShipTo, OrderDate, DueDate, Reference''.',
          'XmlPortInvalidXml', 'init;pre;H.init:1/kind=[]/od=[]/dd=[]/lines=[];', '0:', '0:', '0:');
    end;

    // Not pinned: a failure in the second header, after the first header and its lines were stored in their temporary tables.
    [Test]
    procedure Full_FailsAfterStoredRows_Refused()
    begin
        ImportFull('<Message><Header><Kind>Order</Kind><Customer>C1</Customer><Reference>R1</Reference><Lines>' +
          '<Line><Item>A</Item><Qty>1</Qty></Line></Lines></Header>' +
          '<Header><Kind>Order</Kind><Customer>C2</Customer><Reference>R2</Reference><Lines>' +
          '<Line><Item>B</Item><Qty>1,5</Qty></Line></Lines></Header></Message>', TextEncoding::UTF8, false, false,
          '', '', '', '', '', '');
    end;

    // XpRootTableNested.al:648 RT_Real_IssueDoc_Import
    [Test]
    procedure Real_IssueDoc()
    var
        Src: Record "RT Blob" temporary;
        Root: Record "RT Root";
        Port: XmlPort "RT Real";
        InS: InStream;
    begin
        Root.DeleteAll();
        Commit();
        WriteSource(Src, IssueDoc(), TextEncoding::Windows, InS);
        Port.TextEncoding := TextEncoding::Windows;
        Port.SetSource(InS);
        Check('import', Format(Port.Import()), Format(true));
        Check('counts', Port.Counts(), 'headers=1/lines=2');
        Check('real root rows', Format(Root.Count()), '1');
        Root.FindFirst();
        Check('real root id', Format(Root.Id), '0');
    end;

    // XpRootTableNested.al:677 RT_Min_TwoHeaders_LastErrorKept_Import
    [Test]
    procedure Min_TwoHeaders_LastErrorKept()
    var
        Src: Record "RT Blob" temporary;
        Port: XmlPort "RT Min";
        InS: InStream;
    begin
        WriteSource(Src, '<Message>' + Nl() + '  <Header><No>5</No><Customer>c5</Customer><ShipTo></ShipTo></Header>' + Nl() +
          '  <Header><No>6</No><Customer>C6</Customer></Header>' + Nl() + '</Message>', TextEncoding::UTF8, InS);
        Port.SetSource(InS);
        asserterror Error('an earlier error');
        Check('import', Format(Port.Import()), Format(true));
        Check('last error after', GetLastErrorText(), 'an earlier error');
        Check('dump', Port.Dump(), 'roots=1/0 headers=5/C5/[];6/C6/[];');
    end;

    // Not pinned: the static form returns false without reading the document while the caller holds an uncommitted write; whether
    // OnPostXmlPort runs then is not measured (the pin of the skip, XpRootTableNested.al:604, is a failed read). Last: it writes.
    [Test]
    procedure Full_StaticWithPendingWrite_Refused()
    var
        Src: Record "RT Blob" temporary;
        Pending: Record "RT Pending";
        InS: InStream;
    begin
        WriteSource(Src, IssueDoc(), TextEncoding::Windows, InS);
        Pending.Id := 1;
        Pending.Insert();
        if not Xmlport.Import(Xmlport::"RT Full", InS) then;
    end;
}
"#;

const CELLS: [&str; 11] = [
    "Full_IssueDoc_WindowsBlob",
    "Full_IssueDoc_Utf8NoTextEncoding",
    "Full_IssueDoc_Unguarded",
    "Full_NonAsciiDescription_Windows",
    "Full_TwoHeaders_MissingOptional",
    "Full_Decimals_FormatEvaluateXml",
    "Full_DecimalWithComma",
    "Full_HeaderWithoutLines",
    "Full_HeaderWithoutReference",
    "Real_IssueDoc",
    "Min_TwoHeaders_LastErrorKept",
];

#[test]
fn a_root_table_element_holding_a_nested_table_element_imports_as_bc_does() {
    let outcomes = outcomes_in(SRC, "RT Tests");
    let mut want: Vec<(String, al_testrun::Outcome)> = CELLS
        .iter()
        .map(|c| (c.to_string(), al_testrun::Outcome::Pass))
        .collect();
    // A failure after rows were stored in a temporary table is not pinned: refused, not run with a guess.
    want.insert(
        9,
        (
            "Full_FailsAfterStoredRows_Refused".to_string(),
            al_testrun::Outcome::NotRun(
                "XmlPort import of a root table element holding a table element that fails after a row was stored in a temporary table (not pinned)"
                    .to_string(),
            ),
        ),
    );
    want.push((
        "Full_StaticWithPendingWrite_Refused".to_string(),
        al_testrun::Outcome::NotRun("OnPostXmlPort after a guarded run that returned false (not pinned)".to_string()),
    ));
    assert_eq!(outcomes, want);
}
