//! `Query.SaveAsXml` and `Query.SaveAsJson` of a query with Text, Decimal, Date and Boolean columns, and the static
//! `Query.SaveAsXml(Number, OutStream)` of a query. Pinned by queries/read-lifecycle/QuerySaveAs.al:105-163 (raw bytes) and
//! reports/request-page/P105QryXmlContentSM.al:41-150 (the whole text, static form included).

const SRC: &str = r#"
table 71921 "QRow"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; Amt; Decimal) { }
        field(4; D; Date) { }
        field(5; Flag; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 71924 "QBlob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

query 71922 "QSave"
{
    QueryType = Normal;
    OrderBy = ascending(Id);
    elements
    {
        dataitem(Rows; "QRow")
        {
            column(Id; Id) { }
            column(Name; Name) { }
            column(Amt; Amt) { }
            column(D; D) { }
            column(Flag; Flag) { }
        }
    }
}

codeunit 50100 C
{
    local procedure Seed()
    var
        Row: Record "QRow";
    begin
        Row.DeleteAll();
        Row.Init();
        Row.Id := 1;
        Row.Name := 'a,b "q" <x>&y';
        Row.Amt := 12.5;
        Row.D := DMY2Date(5, 3, 2024);
        Row.Flag := true;
        Row.Insert();
        Row.Init();
        Row.Id := 2;
        Row.Name := '';
        Row.Amt := 0;
        Row.D := 0D;
        Row.Flag := false;
        Row.Insert();
    end;

    local procedure Observe(var B: Record "QBlob" temporary): Text
    var
        InS: InStream;
        Line: Text;
        All: Text;
        Byte1: Byte;
        Code: Integer;
        Crs: Integer;
        i: Integer;
        Raw: Text;
    begin
        B.Data.CreateInStream(InS);
        InS.Read(Byte1);
        Code := Byte1;
        B.Data.CreateInStream(InS);
        InS.Read(Raw, B.Data.Length);
        for i := 1 to StrLen(Raw) do
            if Raw[i] = 13 then
                Crs += 1;
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            All += Line + '|';
        end;
        exit(Format(B.Data.Length) + '/' + Format(Code) + '/' + Format(Crs) + '/' + All);
    end;

    procedure Xml(): Text
    var
        B: Record "QBlob" temporary;
        Q: Query "QSave";
        OutS: OutStream;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        Q.SaveAsXml(OutS);
        exit(Observe(B));
    end;

    procedure XmlStatic(): Text
    var
        B: Record "QBlob" temporary;
        OutS: OutStream;
        Ok: Boolean;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        Ok := Query.SaveAsXml(71922, OutS);
        exit(Format(Ok) + '/' + Observe(B));
    end;

    procedure Json(): Text
    var
        B: Record "QBlob" temporary;
        Q: Query "QSave";
        OutS: OutStream;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        Q.SaveAsJson(OutS);
        exit(Observe(B));
    end;
}
"#;

/// The text QuerySaveAs.al:118-120 pins (its `xml text 2`), lines joined with `|` as `Observe` reads them.
const XML_TAIL: &str = "                <xs:element name=\"D\" msdata:DateTimeMode=\"Local\" type=\"xs:dateTime\" minOccurs=\"0\" />|                <xs:element name=\"Flag\" type=\"xs:boolean\" minOccurs=\"0\" />|              </xs:sequence>|            </xs:complexType>|          </xs:element>|        </xs:choice>|      </xs:complexType>|    </xs:element>|  </xs:schema>|  <Result>|    <Id>1</Id>|    <Name>a,b \"q\" &lt;x&gt;&amp;y</Name>|    <Amt>12.5</Amt>|    <D>2024-03-05T00:00:00+00:00</D>|    <Flag>true</Flag>|  </Result>|  <Result>|    <Id>2</Id>|    <Name />|    <Amt>0</Amt>|    <Flag>false</Flag>|  </Result>|</DataSet>|";

#[test]
fn save_as_xml_writes_a_dot_net_data_set_of_text_decimal_date_and_boolean_columns() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", "Xml"));
    // QuerySaveAs.al:116-119: 1330 bytes, a byte order mark (0xEF first), 34 line ends.
    assert!(got.starts_with("1330/239/34/"), "{got}");
    // P105QryXmlContentSM.al:54: the head of the file, `msdata:Locale=""`, a Text column is xs:string, a Decimal xs:decimal.
    assert!(
        got.contains("|                <xs:element name=\"Name\" type=\"xs:string\" minOccurs=\"0\" />|                <xs:element name=\"Amt\" type=\"xs:decimal\" minOccurs=\"0\" />|"),
        "{got}"
    );
    assert!(got.ends_with(XML_TAIL), "{got}");
}

#[test]
fn the_static_save_as_xml_of_a_query_writes_the_same_file() {
    // P105QryXmlContentSM.al:115-131 SaveAsXml_Static_Content: Yes and the file of the instance call.
    let instance = al_testkit::text(al_testkit::run_in(SRC, "C", "Xml"));
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", "XmlStatic"));
    assert_eq!(got, format!("Yes/{instance}"));
}

#[test]
fn save_as_json_writes_text_decimal_date_and_boolean_columns() {
    // QuerySaveAs.al:150-155.
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", "Json"));
    assert_eq!(
        got,
        "125/91/0/[{\"Id\":1,\"Name\":\"a,b \\\"q\\\" <x>&y\",\"Amt\":12.5,\"D\":\"2024-03-05T00:00:00Z\",\"Flag\":true},{\"Id\":2,\"Name\":\"\",\"Amt\":0,\"Flag\":false}]|"
    );
}
