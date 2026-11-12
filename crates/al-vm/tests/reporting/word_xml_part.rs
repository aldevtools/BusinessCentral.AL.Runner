//! `Report.WordXmlPart` (al-runner issues 6993, 6994). Every expected text is what BC produced for the same report in the corpus
//! probe reports/WordXmlPartNames.al (staging branch al-runtime-rs-staging-r6993, BC 29.0) and reports/AqTests.al:244-264. The
//! content of `BCReportInformation` is shown as `x`, as the corpus Probe Observer shows it.

use al_testkit::{run, text};

const TABLES: &str = r#"
table 95740 "LT WXP Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Name; Text[50]) { }
        field(3; Amount; Decimal) { }
        field(4; Qty; Integer) { }
        field(5; Flag; Boolean) { }
        field(6; Day; Date) { }
        field(7; At; Time) { }
        field(8; Stamp; DateTime) { }
        field(9; Kind; Option) { OptionMembers = " ",Alpha,Beta; }
        field(10; Status; Enum "LT WXP Status") { }
        field(11; Id; Guid) { }
        field(12; Big; BigInteger) { }
        field(13; Dur; Duration) { }
        field(14; Formula; DateFormula) { }
        field(15; Pic; Blob) { }
        field(16; Photo; Media) { }
        field(18; "Unit Price (LCY)"; Decimal) { }
    }
    keys { key(PK; "Code") { } }
}
table 95741 "LT WXP Line"
{
    fields { field(1; RowCode; Code[20]) { } field(2; LineNo; Integer) { } field(3; Qty; Decimal) { } }
    keys { key(PK; RowCode, LineNo) { } }
}
enum 95742 "LT WXP Status" { value(0; Open) { } value(1; Released) { } }
"#;

/// The text of `Report.WordXmlPart(Id, Extended)` for report `id` of `objects`, with the content of BCReportInformation as `x`.
fn part(objects: &str, id: u32, extended: bool) -> String {
    let src = format!(
        "{TABLES}\n{objects}\ncodeunit 95759 T {{ procedure F(): Text begin exit(Report.WordXmlPart({id}, {extended})); end; }}"
    );
    let got = text(run(&src, "F"));
    let (open, close) = ("<BCReportInformation>", "</BCReportInformation>");
    match (got.find(open), got.find(close)) {
        (Some(a), Some(b)) => format!("{}x{}", &got[..a + open.len()], &got[b..]),
        _ => got,
    }
}

fn lines(parts: &[&str]) -> String {
    parts.join("\r\n")
}

const HEAD_PLAIN: &str = r#"<?xml version="1.0" encoding="utf-16"?>"#;

// WordXmlPartNames.al: WordXmlPart_SpacesDashDot_Plain. A run of spaces, dots, dashes and slashes is one underscore; one
// trailing dot is dropped.
#[test]
fn spaces_dashes_dots_and_slashes_are_one_underscore() {
    let objects = r#"report 95740 "LT WXP Std - Order Conf."
{
    dataset
    {
        dataitem("Sales Hdr. - No/1"; "LT WXP Row")
        {
            column(Hdr_No; "Code") { }
            dataitem("Line-Item.2"; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(Line_Qty; Qty) { } }
        }
    }
}"#;
    assert_eq!(
        part(objects, 95740, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_Std_Order_Conf/95740/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Sales_Hdr_No_1>",
            "    <Hdr_No>Hdr_No</Hdr_No>",
            "    <Line_Item_2>",
            "      <Line_Qty>Line_Qty</Line_Qty>",
            "    </Line_Item_2>",
            "  </Sales_Hdr_No_1>",
            "</NavWordReportXmlPart>",
        ])
    );
}

// WordXmlPartNames.al: WordXmlPart_SpacesDashDot_Extended. The ElementId is the FNV-1a hash of the AL name, as a positive number.
#[test]
fn the_extended_form_names_element_types_ids_and_data_types() {
    let objects = r#"report 95740 "LT WXP Std - Order Conf."
{
    dataset
    {
        dataitem("Sales Hdr. - No/1"; "LT WXP Row")
        {
            column(Hdr_No; "Code") { }
            dataitem("Line-Item.2"; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(Line_Qty; Qty) { } }
        }
    }
}"#;
    assert_eq!(
        part(objects, 95740, true),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-businesscentral/internal/extended/reports/LT_WXP_Std_Order_Conf/95740/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Tooltips />",
            r#"  <Sales_Hdr_No_1 ElementType="DataItem" ElementId="2125348297">"#,
            r#"    <Hdr_No ElementType="Column" ElementId="331048043" DataType="Code">Hdr_No</Hdr_No>"#,
            r#"    <Line_Item_2 ElementType="DataItem" ElementId="666712983">"#,
            r#"      <Line_Qty ElementType="Column" ElementId="83512728" DataType="Decimal">Line_Qty</Line_Qty>"#,
            "    </Line_Item_2>",
            "  </Sales_Hdr_No_1>",
            "</NavWordReportXmlPart>",
        ])
    );
}

// WordXmlPartNames.al: WordXmlPart_DigitUnderscore_Plain, WordXmlPart_Punctuation_Plain, WordXmlPart_NonAscii_Plain.
#[test]
fn a_leading_digit_and_punctuation_are_encoded_and_percent_is_spelled_out() {
    let objects = r#"report 95741 "1LT WXP Digit_Under__x"
{
    dataset { dataitem("2nd_Item"; "LT WXP Row") { column(Lead__2; "Code") { } column(Trail_; Name) { } } }
}
report 95743 "LT WXP A/B & C (x) #1,+:;"
{
    dataset { dataitem("It's % 50 *"; "LT WXP Row") { column(No; "Code") { } } }
}
report 95742 "LT WXP Ümlaut Bericht é ß"
{
    dataset { dataitem("Kopf Ä"; "LT WXP Row") { column(Größe; Qty) { } column(Café; Name) { } } }
}"#;
    assert_eq!(
        part(objects, 95741, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/_x0031_LT_WXP_Digit_Under__x/95741/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <_x0032_nd_Item>",
            "    <Lead__2>Lead__2</Lead__2>",
            "    <Trail_>Trail_</Trail_>",
            "  </_x0032_nd_Item>",
            "</NavWordReportXmlPart>",
        ])
    );
    assert_eq!(
        part(objects, 95743, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_A_B__x0026__C_x__x0023_1_x002C__x002B___x003B_/95743/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <It_x0027_s_Percent_50__x002A_>",
            "    <No>No</No>",
            "  </It_x0027_s_Percent_50__x002A_>",
            "</NavWordReportXmlPart>",
        ])
    );
    assert_eq!(
        part(objects, 95742, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_Ümlaut_Bericht_é_ß/95742/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Kopf_Ä>",
            "    <Café>Café</Café>",
            "    <Größe>Größe</Größe>",
            "  </Kopf_Ä>",
            "</NavWordReportXmlPart>",
        ])
    );
}

// WordXmlPartNames.al: WordXmlPart_TypesA_Extended, WordXmlPart_TypesB_Extended. Columns are sorted by name; a Blob column is an
// empty element.
#[test]
fn columns_are_sorted_and_carry_the_data_type_of_their_source() {
    let objects = r#"report 95745 "LT WXP Types A"
{
    dataset
    {
        dataitem(Rows; "LT WXP Row")
        {
            column(ZCode; "Code") { } column(YName; Name) { } column(XAmount; Amount) { } column(WQty; Qty) { }
            column(VFlag; Flag) { } column(UDay; Day) { } column(TAt; At) { } column(SStamp; Stamp) { }
            column(RPrice; "Unit Price (LCY)") { }
        }
    }
}
report 95746 "LT WXP Types B"
{
    dataset
    {
        dataitem(Rows; "LT WXP Row")
        {
            column(Kind; Kind) { } column(Status; Status) { } column(Id; Id) { } column(Big; Big) { } column(Dur; Dur) { }
            column(Formula; Formula) { } column(Pic; Pic) { } column(Photo; Photo) { } column(Total; Total) { }
            column(Lit; 'abc') { }
        }
    }
    var Total: Decimal;
}"#;
    assert_eq!(
        part(objects, 95745, true),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-businesscentral/internal/extended/reports/LT_WXP_Types_A/95745/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Tooltips />",
            r#"  <Rows ElementType="DataItem" ElementId="578941028">"#,
            r#"    <RPrice ElementType="Column" ElementId="366123210" DataType="Decimal">RPrice</RPrice>"#,
            r#"    <SStamp ElementType="Column" ElementId="381244893" DataType="DateTime">SStamp</SStamp>"#,
            r#"    <TAt ElementType="Column" ElementId="642154132" DataType="Time">TAt</TAt>"#,
            r#"    <UDay ElementType="Column" ElementId="399714692" DataType="Date">UDay</UDay>"#,
            r#"    <VFlag ElementType="Column" ElementId="1459118273" DataType="Boolean">VFlag</VFlag>"#,
            r#"    <WQty ElementType="Column" ElementId="1254702462" DataType="Integer">WQty</WQty>"#,
            r#"    <XAmount ElementType="Column" ElementId="767587047" DataType="Decimal">XAmount</XAmount>"#,
            r#"    <YName ElementType="Column" ElementId="2109778619" DataType="Text">YName</YName>"#,
            r#"    <ZCode ElementType="Column" ElementId="336516762" DataType="Code">ZCode</ZCode>"#,
            "  </Rows>",
            "</NavWordReportXmlPart>",
        ])
    );
    assert_eq!(
        part(objects, 95746, true),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-businesscentral/internal/extended/reports/LT_WXP_Types_B/95746/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Tooltips />",
            r#"  <Rows ElementType="DataItem" ElementId="578941028">"#,
            r#"    <Big ElementType="Column" ElementId="1915598679" DataType="BigInteger">Big</Big>"#,
            r#"    <Dur ElementType="Column" ElementId="104994878" DataType="Duration">Dur</Dur>"#,
            r#"    <Formula ElementType="Column" ElementId="1157393955" DataType="DateFormula">Formula</Formula>"#,
            r#"    <Id ElementType="Column" ElementId="1731978936" DataType="GUID">Id</Id>"#,
            r#"    <Kind ElementType="Column" ElementId="735108757" DataType="Option">Kind</Kind>"#,
            r#"    <Lit ElementType="Column" ElementId="182398796" DataType="String">Lit</Lit>"#,
            r#"    <Photo ElementType="Column" ElementId="775198353" DataType="Media">Photo</Photo>"#,
            r#"    <Pic ElementType="Column" ElementId="559402575" DataType="BLOB" />"#,
            r#"    <Status ElementType="Column" ElementId="1634144551" DataType="Option">Status</Status>"#,
            r#"    <Total ElementType="Column" ElementId="1902056945" DataType="Decimal">Total</Total>"#,
            "  </Rows>",
            "</NavWordReportXmlPart>",
        ])
    );
    assert!(part(objects, 95746, false).contains("\r\n    <Pic />\r\n"));
}

const LABELS: &str = r#"report 95747 "LT WXP Labels"
{
    dataset
    {
        dataitem(First; "LT WXP Row")
        {
            column(FName; Name) { IncludeCaption = true; }
            column(FCode; "Code") { }
            dataitem(Nested; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(NQty; Qty) { IncludeCaption = true; } }
        }
        dataitem(Second; "LT WXP Line") { column(SLine; LineNo) { } }
    }
    labels { ZLabel = 'Last label'; ALabel = 'First label', Comment = 'a comment'; }
}"#;

// WordXmlPartNames.al: WordXmlPart_Labels_Plain, WordXmlPart_Labels_Extended. The labels and a <Column>Caption for every
// IncludeCaption column, sorted; a caption has the ElementId of its column.
#[test]
fn labels_and_captions_come_first_sorted() {
    assert_eq!(
        part(LABELS, 95747, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_Labels/95747/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Labels>",
            "    <ALabel>ALabel</ALabel>",
            "    <FNameCaption>FNameCaption</FNameCaption>",
            "    <NQtyCaption>NQtyCaption</NQtyCaption>",
            "    <ZLabel>ZLabel</ZLabel>",
            "  </Labels>",
            "  <First>",
            "    <FCode>FCode</FCode>",
            "    <FName>FName</FName>",
            "    <Nested>",
            "      <NQty>NQty</NQty>",
            "    </Nested>",
            "  </First>",
            "  <Second>",
            "    <SLine>SLine</SLine>",
            "  </Second>",
            "</NavWordReportXmlPart>",
        ])
    );
    assert_eq!(
        part(LABELS, 95747, true),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-businesscentral/internal/extended/reports/LT_WXP_Labels/95747/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Labels>",
            r#"    <ALabel ElementType="Parameter" ElementId="1949206914" DataType="String">ALabel</ALabel>"#,
            r#"    <FNameCaption ElementType="Parameter" ElementId="1486599940" DataType="String">FNameCaption</FNameCaption>"#,
            r#"    <NQtyCaption ElementType="Parameter" ElementId="1857757097" DataType="String">NQtyCaption</NQtyCaption>"#,
            r#"    <ZLabel ElementType="Parameter" ElementId="1660052375" DataType="String">ZLabel</ZLabel>"#,
            "  </Labels>",
            "  <Tooltips />",
            r#"  <First ElementType="DataItem" ElementId="2008383545">"#,
            r#"    <FCode ElementType="Column" ElementId="1300742378" DataType="Code">FCode</FCode>"#,
            r#"    <FName ElementType="Column" ElementId="1486599940" DataType="Text">FName</FName>"#,
            r#"    <Nested ElementType="DataItem" ElementId="1510830432">"#,
            r#"      <NQty ElementType="Column" ElementId="1857757097" DataType="Decimal">NQty</NQty>"#,
            "    </Nested>",
            "  </First>",
            r#"  <Second ElementType="DataItem" ElementId="1425916523">"#,
            r#"    <SLine ElementType="Column" ElementId="312178264" DataType="Integer">SLine</SLine>"#,
            "  </Second>",
            "</NavWordReportXmlPart>",
        ])
    );
}

const TWO_EXTENSIONS: &str = r#"report 95749 "LT WXP Two Ext Base"
{
    dataset
    {
        dataitem(Head; "LT WXP Row") { column(HCode; "Code") { } column(HQty; Qty) { } }
        dataitem(Tail; "LT WXP Line") { column(TLine; LineNo) { } }
    }
}
reportextension 95751 "LT WXP Two Ext A" extends "LT WXP Two Ext Base"
{
    dataset
    {
        add(Head) { column(AAmount; Amount) { } }
        addfirst(Head) { dataitem(ALines; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(ALineQty; Qty) { } } }
    }
    labels { ExtLabel = 'Extension label'; }
}
reportextension 95752 "LT WXP Two Ext B" extends "LT WXP Two Ext Base"
{
    dataset
    {
        add(Head) { column(BFlag; Flag) { IncludeCaption = true; } }
        add(Tail) { column(BQty; Qty) { } }
        addlast(Tail) { dataitem(BNested; "LT WXP Row") { column(BMidCode; "Code") { } } }
    }
}"#;

// WordXmlPartNames.al: WordXmlPart_TwoExtensions_Plain and _Extended (al-runner issue 6994). The columns, data items and labels
// of every extension join the report's.
#[test]
fn report_extensions_add_their_columns_data_items_and_labels() {
    assert_eq!(
        part(TWO_EXTENSIONS, 95749, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_Two_Ext_Base/95749/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Labels>",
            "    <BFlagCaption>BFlagCaption</BFlagCaption>",
            "    <ExtLabel>ExtLabel</ExtLabel>",
            "  </Labels>",
            "  <Head>",
            "    <AAmount>AAmount</AAmount>",
            "    <BFlag>BFlag</BFlag>",
            "    <HCode>HCode</HCode>",
            "    <HQty>HQty</HQty>",
            "    <ALines>",
            "      <ALineQty>ALineQty</ALineQty>",
            "    </ALines>",
            "  </Head>",
            "  <Tail>",
            "    <BQty>BQty</BQty>",
            "    <TLine>TLine</TLine>",
            "    <BNested>",
            "      <BMidCode>BMidCode</BMidCode>",
            "    </BNested>",
            "  </Tail>",
            "</NavWordReportXmlPart>",
        ])
    );
    let extended = part(TWO_EXTENSIONS, 95749, true);
    assert!(extended.contains(
        "\r\n  <Labels>\r\n    <BFlagCaption ElementType=\"Parameter\" ElementId=\"912455157\" DataType=\"String\">BFlagCaption</BFlagCaption>\r\n    <ExtLabel ElementType=\"Parameter\" ElementId=\"813550006\" DataType=\"String\">ExtLabel</ExtLabel>\r\n  </Labels>\r\n  <Tooltips />\r\n"
    ));
    assert!(extended.contains(
        "\r\n    <BNested ElementType=\"DataItem\" ElementId=\"1904151690\">\r\n      <BMidCode ElementType=\"Column\" ElementId=\"1503903634\" DataType=\"Code\">BMidCode</BMidCode>\r\n    </BNested>\r\n"
    ));
}

// WordXmlPartNames.al: WordXmlPart_Order_Plain (BC 27.5, 28.5, 29.0). Sibling data items keep the report's order; columns and
// labels sort case-insensitively, an underscore before a letter.
#[test]
fn sibling_items_keep_their_order_and_names_sort_case_insensitively() {
    let objects = r#"report 95753 "LT WXP Order"
{
    dataset
    {
        dataitem(Zulu; "LT WXP Row")
        {
            column(beta; Name) { } column(Alpha; "Code") { } column(A_B; Qty) { } column(AB; Amount) { }
            column(zed2; Flag) { IncludeCaption = true; } column(Zed; Day) { }
            dataitem(Yank; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(YQty; Qty) { } }
            dataitem(Bravo; "LT WXP Line") { DataItemLink = RowCode = field("Code"); column(BQty; Qty) { } }
        }
        dataitem(Alpha2; "LT WXP Line") { column(A2Line; LineNo) { } }
    }
    labels { bLow = 'b'; Bz = 'B'; A_Label = 'A'; }
}"#;
    assert_eq!(
        part(objects, 95753, false),
        lines(&[
            HEAD_PLAIN,
            r#"<NavWordReportXmlPart xmlns="urn:microsoft-dynamics-nav/reports/LT_WXP_Order/95753/">"#,
            "  <BCReportInformation>x</BCReportInformation>",
            "  <Labels>",
            "    <A_Label>A_Label</A_Label>",
            "    <bLow>bLow</bLow>",
            "    <Bz>Bz</Bz>",
            "    <zed2Caption>zed2Caption</zed2Caption>",
            "  </Labels>",
            "  <Zulu>",
            "    <A_B>A_B</A_B>",
            "    <AB>AB</AB>",
            "    <Alpha>Alpha</Alpha>",
            "    <beta>beta</beta>",
            "    <Zed>Zed</Zed>",
            "    <zed2>zed2</zed2>",
            "    <Yank>",
            "      <YQty>YQty</YQty>",
            "    </Yank>",
            "    <Bravo>",
            "      <BQty>BQty</BQty>",
            "    </Bravo>",
            "  </Zulu>",
            "  <Alpha2>",
            "    <A2Line>A2Line</A2Line>",
            "  </Alpha2>",
            "</NavWordReportXmlPart>",
        ])
    );
}

// WordXmlPartNames.al: WordXmlPart_TypesC_Extended (BC 27.5, 28.5, 29.0). The DataType of expression columns.
#[test]
fn expression_columns_carry_the_data_type_of_their_expression() {
    let objects = r#"table 95743 "LT WXP Ref" { fields { field(1; "Code"; Code[20]) { } field(17; Ref; RecordId) { } } keys { key(PK; "Code") { } } }
report 95754 "LT WXP Types C"
{
    dataset
    {
        dataitem(Rows; "LT WXP Row")
        {
            column(TextVar; TextVar) { } column(CodeVar; CodeVar) { } column(IntVar; IntVar) { } column(BoolVar; BoolVar) { }
            column(DateVar; DateVar) { } column(FmtCol; Format(Amount)) { } column(LblCol; LblVar) { }
            column(ProcCol; TextProc()) { } column(IntLit; 7) { } column(QualCol; Rows.Amount) { }
            dataitem(Refs; "LT WXP Ref") { column(RefCol; Ref) { } }
        }
    }
    var TextVar: Text; CodeVar: Code[10]; IntVar: Integer; BoolVar: Boolean; DateVar: Date; LblVar: Label 'A label';
    local procedure TextProc(): Text begin exit('p'); end;
}"#;
    let got = part(objects, 95754, true);
    for line in [
        r#"    <BoolVar ElementType="Column" ElementId="2070022482" DataType="Boolean">BoolVar</BoolVar>"#,
        r#"    <CodeVar ElementType="Column" ElementId="1141779971" DataType="Code">CodeVar</CodeVar>"#,
        r#"    <DateVar ElementType="Column" ElementId="1460740492" DataType="Date">DateVar</DateVar>"#,
        r#"    <FmtCol ElementType="Column" ElementId="381582870" DataType="String">FmtCol</FmtCol>"#,
        r#"    <IntLit ElementType="Column" ElementId="801097753" DataType="Integer">IntLit</IntLit>"#,
        r#"    <IntVar ElementType="Column" ElementId="1327515661" DataType="Integer">IntVar</IntVar>"#,
        r#"    <LblCol ElementType="Column" ElementId="247840991" DataType="TextConst">LblCol</LblCol>"#,
        r#"    <ProcCol ElementType="Column" ElementId="1140424675" DataType="Text">ProcCol</ProcCol>"#,
        r#"    <QualCol ElementType="Column" ElementId="1134337924" DataType="Decimal">QualCol</QualCol>"#,
        r#"      <RefCol ElementType="Column" ElementId="1898675948" DataType="RecordID">RefCol</RefCol>"#,
        r#"    <TextVar ElementType="Column" ElementId="315596261" DataType="Text">TextVar</TextVar>"#,
    ] {
        assert!(got.contains(&format!("\r\n{line}\r\n")), "{line}\nin\n{got}");
    }
}

// An underscore sorts before a digit (corpus PR 1336, reports/layouts-saveas/WordXmlPartEmptyItemsOrderArrays.al
// WordXmlPart_Shape_NameOrder_Plain: `A_ A1 AA aB`; al-runner issue 7367). This test refused the pair before that probe.
#[test]
fn an_underscore_sorts_before_a_digit() {
    let src = format!(
        "{TABLES}\nreport 95755 R {{ dataset {{ dataitem(Rows; \"LT WXP Row\") {{ column(A1; Qty) {{ }} column(A_x; Name) {{ }} }} }} }}\ncodeunit 95759 T {{ procedure F(): Text begin exit(Report.WordXmlPart(95755)); end; }}"
    );
    let got = match run(&src, "F") {
        Ok(al_builtins::value::Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    };
    assert!(
        got.contains("<Rows>\r\n    <A_x>A_x</A_x>\r\n    <A1>A1</A1>\r\n  </Rows>"),
        "{got}"
    );
}

// WordXmlPartNames.al: WordXmlPart_ReportInformation_Plain, _ReportInformationTail_Plain, _ReportInformationTail2_Plain (BC 29.0,
// observed with < written as [). Each element's text is its display name.
#[test]
fn the_report_information_lists_display_names() {
    let src = format!(
        "{TABLES}\nreport 95748 \"LT WXP One Ext Base\" {{ dataset {{ dataitem(Base; \"LT WXP Row\") {{ column(BCode; \"Code\") {{ }} }} }} }}\ncodeunit 95759 T {{ procedure F(): Text begin exit(Report.WordXmlPart(95748)); end; }}"
    );
    let got = text(run(&src, "F"));
    let start = got.find("  <BCReportInformation>").expect("the block");
    let end = got.find("  <Base>").expect("the data item");
    let mut want = vec![
        "  <BCReportInformation>",
        "    <ReportMetadata>",
        "      <ExtensionId>Extension ID</ExtensionId>",
        "      <ExtensionName>Extension Name</ExtensionName>",
        "      <ExtensionPublisher>Extension Publisher</ExtensionPublisher>",
        "      <ExtensionVersion>Extension Version</ExtensionVersion>",
        "      <ReportId>Report ID</ReportId>",
        "      <ReportName>Report Name</ReportName>",
        "      <ReportCaption>Report Caption</ReportCaption>",
        "      <AboutThisReportTitle>About This Report Title</AboutThisReportTitle>",
        "      <AboutThisReportText>About This Report Text</AboutThisReportText>",
        "      <ReportHelpLink>Report Help Link</ReportHelpLink>",
        "    </ReportMetadata>",
        "    <ReportRequest>",
        "      <TenantEntraId>Tenant Entra Id</TenantEntraId>",
        "      <EnvironmentName>Environment Name</EnvironmentName>",
        "      <EnvironmentType>Environment Type</EnvironmentType>",
        "      <CompanyName>Company Name</CompanyName>",
        "      <CompanyDisplayName>Company Display Name</CompanyDisplayName>",
        "      <CompanyId>Company ID</CompanyId>",
        "      <UserName>User Name</UserName>",
        "      <DateAndTime>Date and Time</DateAndTime>",
        "      <Language>Language</Language>",
        "      <FormatRegion>Format Region</FormatRegion>",
        "      <DateTimeValues>",
        "        <Year>Year</Year>",
        "        <MonthNumber>Month Number</MonthNumber>",
        "        <DayNumber>Day Number</DayNumber>",
        "        <Hour>Hour</Hour>",
        "        <Minute>Minute</Minute>",
        "      </DateTimeValues>",
        "    </ReportRequest>",
        "    <CompanyMetadata>",
        "      <CompanyName>Company Name</CompanyName>",
        "      <CompanyDisplayName>Company Display Name</CompanyDisplayName>",
        "      <CompanyAddressLines>",
        "        <CompanyAddressLine1>Company Address Line 1</CompanyAddressLine1>",
        "        <CompanyAddressLine2>Company Address Line 2</CompanyAddressLine2>",
        "        <CompanyAddressLine3>Company Address Line 3</CompanyAddressLine3>",
        "        <CompanyAddressLine4>Company Address Line 4</CompanyAddressLine4>",
        "        <CompanyAddressLine5>Company Address Line 5</CompanyAddressLine5>",
        "        <CompanyAddressLine6>Company Address Line 6</CompanyAddressLine6>",
        "        <CompanyAddressLine7>Company Address Line 7</CompanyAddressLine7>",
        "        <CompanyAddressLine8>Company Address Line 8</CompanyAddressLine8>",
        "      </CompanyAddressLines>",
        "      <CompanyPhone>Company Phone</CompanyPhone>",
        "      <CompanyPhoneCaption>Company Phone Caption</CompanyPhoneCaption>",
        "      <CompanyFaxNo>Company Fax No.</CompanyFaxNo>",
        "      <CompanyFaxNoCaption>Company Fax No. Caption</CompanyFaxNoCaption>",
        "      <CompanyEmail>Company Email</CompanyEmail>",
        "      <CompanyEmailCaption>Company Email Caption</CompanyEmailCaption>",
        "      <CompanyHomePage>Company Home Page</CompanyHomePage>",
        "      <CompanyHomePageCaption>Company Home Page Caption</CompanyHomePageCaption>",
        "      <CompanyLogo>Company Logo</CompanyLogo>",
        "      <CompanyVATRegistrationNo>Company VAT Registration No.</CompanyVATRegistrationNo>",
        "      <CompanyVATRegistrationNoCaption>Company VAT Registration No. Caption</CompanyVATRegistrationNoCaption>",
        "      <CompanyRegistrationNo>Company Registration No.</CompanyRegistrationNo>",
        "      <CompanyRegistrationNoCaption>Company Registration No. Caption</CompanyRegistrationNoCaption>",
        "      <CompanyBankName>Company Bank Name</CompanyBankName>",
        "      <CompanyBankNameCaption>Company Bank Name Caption</CompanyBankNameCaption>",
        "      <CompanyBankAccountNo>Company Bank Account No.</CompanyBankAccountNo>",
        "      <CompanyBankAccountNoCaption>Company Bank Account No. Caption</CompanyBankAccountNoCaption>",
        "      <CompanyBankBranchNo>Company Bank Branch No.</CompanyBankBranchNo>",
        "      <CompanyBankBranchNoCaption>Company Bank Branch No. Caption</CompanyBankBranchNoCaption>",
        "      <CompanyIBAN>Company IBAN</CompanyIBAN>",
        "      <CompanyIBANCaption>Company IBAN Caption</CompanyIBANCaption>",
        "      <CompanyBankSWIFT>Company Bank SWIFT</CompanyBankSWIFT>",
        "      <CompanyBankSWIFTCaption>Company Bank SWIFT Caption</CompanyBankSWIFTCaption>",
        "      <CompanyGiroNo>Company Giro No.</CompanyGiroNo>",
        "      <CompanyGiroNoCaption>Company Giro No. Caption</CompanyGiroNoCaption>",
        "    </CompanyMetadata>",
        "  </BCReportInformation>",
    ];
    want.push("");
    assert_eq!(&got[start..end], want.join("\r\n"));
}
