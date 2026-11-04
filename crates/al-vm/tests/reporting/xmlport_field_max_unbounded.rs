//! MaxOccurs = Unbounded on a field element (AL Runner issue 7032; corpus xmlports/shapes-properties/XpRootMinOccursZero.al
//! ElementOccurs_* on branch al-runtime-rs-staging-r6963).

const SRC: &str = r#"
table 50100 "Repro Receiver" { fields { field(1; Id; Integer) { } field(2; Flow; Code[20]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "Repro Buf" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
xmlport 50100 "Repro Receivers Export"
{
    Direction = Export; Format = Xml; Encoding = UTF8;
    schema
    {
        textelement(Root)
        {
            tableelement(receivers; "Repro Receiver")
            {
                XmlName = 'Receiver';
                UseTemporary = true;
                fieldelement(Application; receivers.Flow) { MaxOccurs = Unbounded; }

                trigger OnPreXmlItem()
                begin
                    receivers.Id := 1; receivers.Flow := 'FLOW-A'; receivers.Insert();
                    receivers.Id := 2; receivers.Flow := 'FLOW-B'; receivers.Insert();
                end;
            }
        }
    }
}
codeunit 50102 C
{
    procedure Export(): Text
    var Export: XmlPort "Repro Receivers Export"; B: Record "Repro Buf" temporary; OutStr: OutStream; InStr: InStream; Line: Text; Result: Text;
    begin
        B.Data.CreateOutStream(OutStr);
        Export.SetDestination(OutStr);
        Export.Export();
        B.Data.CreateInStream(InStr);
        while not InStr.EOS() do begin InStr.ReadText(Line); Result += Line + '|'; end;
        exit(Result);
    end;
}
"#;

#[test]
fn export_writes_one_element_per_row() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", "Export"));
    assert!(
        got.contains("<Application>FLOW-A</Application>") && got.contains("<Application>FLOW-B</Application>"),
        "{got}"
    );
}

const PORTS: &str = r#"
table 50103 "FM Item" { fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } } keys { key(PK; "No.") { } } }
table 50104 "FM Buf" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
xmlport 50105 "FM Field"
{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Cfg)
        {
            tableelement(Item; "FM Item")
            {
                MinOccurs = Zero;
                fieldelement(Code; Item."No.") { MinOccurs = Once; MaxOccurs = Once; }
                fieldelement(Url; Item.Descr)
                {
                    MinOccurs = Zero; MaxOccurs = Unbounded;
                    trigger OnAfterAssignField() begin Log += 'url ' + Item.Descr + ';'; end;
                }
                trigger OnBeforeInsertRecord() begin Log += 'insert ' + Item."No." + ';'; end;
            }
        }
    }
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
codeunit 50106 F
{
    procedure Export(): Text
    var R: Record "FM Item"; B: Record "FM Buf" temporary; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R."No." := 'A'; R.Descr := 'urn:a'; R.Insert();
        R."No." := 'B'; R.Descr := ''; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xmlport.Export(Xmlport::"FM Field", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(Data: Text): Text
    var R: Record "FM Item"; B: Record "FM Buf" temporary; Xp: XmlPort "FM Field"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xp.SetSource(InS);
        Xp.Import();
        if R.FindSet() then
            repeat T += R."No." + '=[' + R.Descr + '];'; until R.Next() = 0;
        exit(T + ' log=' + Xp.GetLog());
    end;
}
"#;

fn import(body: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let data = format!(r#"<?xml version="1.0" encoding="UTF-8"?>{body}"#);
    al_testkit::run_with(PORTS, "F", "Import", vec![al_builtins::value::Value::Text(data.into())]).map(|(v, _)| v)
}

/// XpRootMinOccursZero.al ElementOccurs_FieldMaxUnbounded_Export (corpus branch al-runtime-rs-staging-r6963).
#[test]
fn export_of_an_unbounded_field_element_is_as_without_it() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PORTS, "F", "Export")),
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Cfg>|  <Item>|    <Code>A</Code>|    <Url>urn:a</Url>|  </Item>|  <Item>|    <Code>B</Code>|    <Url />|  </Item>|</Cfg>|"#
    );
}

/// ElementOccurs_FieldMaxUnbounded_Import and _Import2: each occurrence is assigned, the last one stays; out of order fails.
#[test]
fn import_of_an_unbounded_field_element_keeps_the_last_value() {
    let ok = [
        (
            "<Cfg><Item><Code>A</Code><Url>u1</Url></Item></Cfg>",
            "A=[u1]; log=url u1;insert A;",
        ),
        (
            "<Cfg><Item><Code>A</Code><Url>u1</Url><Url>u2</Url></Item></Cfg>",
            "A=[u2]; log=url u1;url u2;insert A;",
        ),
        (
            "<Cfg><Item><Code>A</Code><Url>u1</Url><Url>u2</Url><Url>u3</Url></Item></Cfg>",
            "A=[u3]; log=url u1;url u2;url u3;insert A;",
        ),
        ("<Cfg><Item><Code>A</Code></Item></Cfg>", "A=[]; log=insert A;"),
    ];
    for (body, want) in ok {
        assert_eq!(al_testkit::text(import(body)), want, "{body}");
    }
    let e = "Imported XML cannot validate with the schema: The element 'Item' has invalid child element";
    let failing = [
        (
            "<Cfg><Item><Code>A</Code><Code>B</Code></Item></Cfg>",
            format!("{e} 'Code'. List of possible elements expected: 'Url'."),
        ),
        (
            "<Cfg><Item><Url>u1</Url></Item></Cfg>",
            format!("{e} 'Url'. List of possible elements expected: 'Code'."),
        ),
        (
            "<Cfg><Item><Url>u1</Url><Code>A</Code></Item></Cfg>",
            format!("{e} 'Url'. List of possible elements expected: 'Code'."),
        ),
    ];
    for (body, want) in failing {
        assert_eq!(al_testkit::error_message(import(body)), want, "{body}");
    }
}
