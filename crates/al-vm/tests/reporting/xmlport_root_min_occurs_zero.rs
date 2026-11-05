//! MinOccurs = Zero on the root text element of an Xml port (AL Runner issue 6963; corpus
//! xmlports/shapes-properties/XpRootMinOccursZero.al on branch al-runtime-rs-staging-r6963).

const SRC: &str = r#"
table 69960 "XR Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69961 "XR Row" { fields { field(1; Code; Code[20]) { } field(2; Url; Text[100]) { } } keys { key(PK; Code) { } } }
xmlport 69962 "XR Port"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Cfg)
        {
            MaxOccurs = Once;
            MinOccurs = Zero;
            textelement(Namespaces)
            {
                MaxOccurs = Once;
                MinOccurs = Zero;
                tableelement(Row; "XR Row")
                {
                    MinOccurs = Zero;
                    XmlName = 'Item';
                    fieldelement(Code; Row.Code) { MinOccurs = Zero; }
                    fieldelement(Url; Row.Url) { MinOccurs = Zero; }
                }
            }
        }
    }
}
codeunit 69963 C
{
    procedure Import(): Text
    var B: Record "XR Blob" temporary; R: Record "XR Row"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Cfg><Namespaces><Item><Code>A</Code><Url>urn:a</Url></Item><Item><Code>B</Code></Item></Namespaces></Cfg>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XR Port", InS);
        if R.FindSet() then
            repeat T += R.Code + '=' + R.Url + ','; until R.Next() = 0;
        exit(T);
    end;

    procedure Export(): Text
    var B: Record "XR Blob" temporary; R: Record "XR Row"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Code := 'A'; R.Url := 'urn:a'; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XR Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;
}
"#;

#[test]
fn import_reads_the_rows_below_an_optional_root() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Import")), "A=urn:a,B=,");
}

/// The ports of XpRootMinOccursZero.al (corpus branch al-runtime-rs-staging-r6963): `schema` inside the port, Encoding UTF-8 for
/// Xml. `Export(Rows)` writes no rows (false) or A 'urn:a' and B '' (true) and gives the lines joined by |; `Import(Data)` gives
/// the rows the import made.
fn port(format: &str, schema: &str) -> String {
    format!(
        r#"
table 69964 "XR Item" {{ fields {{ field(1; "No."; Code[20]) {{ }} field(2; Descr; Text[50]) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
table 69965 "XR Buf" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69966 "XR P"
{{
    Direction = Both; {format} UseRequestPage = false;
    schema {{ {schema} }}
}}
codeunit 69967 C
{{
    procedure Export(Rows: Boolean): Text
    var R: Record "XR Item"; B: Record "XR Buf" temporary; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        if Rows then begin
            R."No." := 'A'; R.Descr := 'urn:a'; R.Insert();
            R."No." := 'B'; R.Descr := ''; R.Insert();
        end;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xmlport.Export(Xmlport::"XR P", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(Data: Text): Text
    var R: Record "XR Item"; B: Record "XR Buf" temporary; OutS: OutStream; InS: InStream; T: Text; CrLf: Text[2];
    begin
        CrLf[1] := 13; CrLf[2] := 10;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Data.Replace('|', CrLf));
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XR P", InS);
        if R.FindSet() then
            repeat T += R."No." + '=[' + R.Descr + '];'; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

const XML: &str = "Format = Xml; Encoding = UTF8;";
const ISSUE: &str = r#"textelement(Cfg) { MaxOccurs = Once; MinOccurs = Zero; textelement(Namespaces) { MaxOccurs = Once; MinOccurs = Zero;
    tableelement(Item; "XR Item") { MinOccurs = Zero; fieldelement(Code; Item."No.") { MinOccurs = Zero; } fieldelement(Url; Item.Descr) { MinOccurs = Zero; } } } }"#;
const TABLE: &str = r#"textelement(Cfg) { MinOccurs = Zero; tableelement(Item; "XR Item") { fieldelement(Code; Item."No.") { } fieldelement(Url; Item.Descr) { } } }"#;
const CHILDREN: &str = r#"textelement(Cfg) { MinOccurs = Zero; MaxOccurs = Once;
    textelement(Head) { MinOccurs = Once; MaxOccurs = Once; trigger OnBeforePassVariable() begin Head := 'H'; end; }
    textelement(Opt) { MinOccurs = Zero; MaxOccurs = Once; }
    tableelement(Item; "XR Item") { MinOccurs = Zero; fieldelement(Code; Item."No.") { MinOccurs = Once; } fieldelement(Url; Item.Descr) { MinOccurs = Zero; } } }"#;
const VARTEXT: &str = r#"textelement(Cfg) { MinOccurs = Zero; tableelement(Item; "XR Item") { MinOccurs = Zero; fieldelement(Code; Item."No.") { } fieldelement(Url; Item.Descr) { } } }"#;
const DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#;

fn export(format: &str, schema: &str, rows: bool) -> String {
    let r = al_testkit::run_with(
        &port(format, schema),
        "C",
        "Export",
        vec![al_builtins::value::Value::Boolean(rows)],
    );
    al_testkit::text(r.map(|(v, _)| v))
}

fn import(format: &str, schema: &str, data: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(
        &port(format, schema),
        "C",
        "Import",
        vec![al_builtins::value::Value::Text(data.into())],
    )
    .map(|(v, _)| v)
}

/// RootMinOccursZero_Issue_Export, _Table_Export, _Children_Export, _VarText_Export.
#[test]
fn export_writes_the_optional_root() {
    let d = DECL;
    assert_eq!(export(XML, ISSUE, false), format!("{d}|<Cfg>|  <Namespaces />|</Cfg>|"));
    assert_eq!(
        export(XML, ISSUE, true),
        format!(
            "{d}|<Cfg>|  <Namespaces>|    <Item>|      <Code>A</Code>|      <Url>urn:a</Url>|    </Item>|    <Item>|      <Code>B</Code>|      <Url />|    </Item>|  </Namespaces>|</Cfg>|"
        )
    );
    assert_eq!(
        export(XML, TABLE, false),
        format!("{d}|<Cfg>|  <Item>|    <Code />|    <Url />|  </Item>|</Cfg>|")
    );
    assert_eq!(
        export(XML, CHILDREN, false),
        format!("{d}|<Cfg>|  <Head>H</Head>|  <Opt />|</Cfg>|")
    );
    assert_eq!(
        export(XML, CHILDREN, true),
        format!(
            "{d}|<Cfg>|  <Head>H</Head>|  <Opt />|  <Item>|    <Code>A</Code>|    <Url>urn:a</Url>|  </Item>|  <Item>|    <Code>B</Code>|    <Url />|  </Item>|</Cfg>|"
        )
    );
    let vt = "Format = VariableText; TextEncoding = UTF8;";
    assert_eq!(export(vt, VARTEXT, false), "|");
    assert_eq!(export(vt, VARTEXT, true), "\"A\",\"urn:a\"|\"B\",\"\"|");
}

/// RootMinOccursZero_*_Import and _Import2: the rows, or the error text.
#[test]
fn import_takes_the_optional_root_and_fails_as_for_the_default() {
    let decl = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
    let ok = [
        (
            ISSUE,
            "<Cfg><Namespaces><Item><Code>A</Code><Url>urn:a</Url></Item><Item><Code>B</Code></Item></Namespaces></Cfg>",
            "A=[urn:a];B=[];",
        ),
        (ISSUE, "<Cfg />", ""),
        (ISSUE, "<Cfg><Namespaces /></Cfg>", ""),
        (
            TABLE,
            "<Cfg><Item><Code>A</Code><Url>urn:a</Url></Item><Item><Code>B</Code><Url /></Item></Cfg>",
            "A=[urn:a];B=[];",
        ),
        (
            CHILDREN,
            "<Cfg><Head>h</Head><Opt>o</Opt><Item><Code>A</Code><Url>urn:a</Url></Item></Cfg>",
            "A=[urn:a];",
        ),
        (
            CHILDREN,
            "<Cfg><Head>h</Head><Item><Code>A</Code></Item></Cfg>",
            "A=[];",
        ),
    ];
    for (schema, body, want) in ok {
        assert_eq!(
            al_testkit::text(import(XML, schema, &format!("{decl}{body}"))),
            want,
            "{body}"
        );
    }
    let schema_error = "Imported XML cannot validate with the schema: ";
    let failing = [
        (
            ISSUE,
            String::new(),
            "Cannot import the specified XML document because the file is empty.".to_string(),
        ),
        (ISSUE, decl.to_string(), "Root element is missing.".to_string()),
        (
            ISSUE,
            format!("{decl}<Other><Namespaces><Item><Code>A</Code></Item></Namespaces></Other>"),
            format!("{schema_error}The 'Other' element is not declared."),
        ),
        (
            ISSUE,
            format!("{decl}<Namespaces><Item><Code>A</Code></Item></Namespaces>"),
            format!("{schema_error}The 'Namespaces' element is not declared."),
        ),
        (
            ISSUE,
            format!("{decl}<Item><Code>A</Code></Item>"),
            format!("{schema_error}The 'Item' element is not declared."),
        ),
        (
            TABLE,
            format!("{decl}<Cfg />"),
            format!(
                "{schema_error}The element 'Cfg' has incomplete content. List of possible elements expected: 'Item'."
            ),
        ),
        (
            CHILDREN,
            format!("{decl}<Cfg><Opt>o</Opt><Item><Code>A</Code></Item></Cfg>"),
            format!(
                "{schema_error}The element 'Cfg' has invalid child element 'Opt'. List of possible elements expected: 'Head'."
            ),
        ),
        (
            CHILDREN,
            format!("{decl}<Cfg><Head>h</Head><Item><Url>urn:a</Url></Item></Cfg>"),
            format!(
                "{schema_error}The element 'Item' has invalid child element 'Url'. List of possible elements expected: 'Code'."
            ),
        ),
        (
            CHILDREN,
            format!("{decl}<Cfg />"),
            format!(
                "{schema_error}The element 'Cfg' has incomplete content. List of possible elements expected: 'Head'."
            ),
        ),
    ];
    for (schema, data, want) in failing {
        assert_eq!(al_testkit::error_message(import(XML, schema, &data)), want, "{data}");
    }
    let vt = "Format = VariableText; TextEncoding = UTF8;";
    assert_eq!(al_testkit::text(import(vt, VARTEXT, "A,urn:a|B,|")), "A=[urn:a];B=[];");
}
