//! A `Format = Xml` port whose schema root is a table element (the DemoTool's xmlport 101899 "Demo Data Configuration"),
//! pinned by platform/lt-ag-contoso/AgXmlPortImport.al (xmlports 87562 and 87563 at :784-833): QX_RootTableElement_Import
//! (:514), QX_RootTableElement_Export (:557) and QX_RootTableElement_Export_TwoRows (:584).

use al_testkit::{error_message, text};

const SRC: &str = r#"
table 69970 "RT Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69971 "RT Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Starting Year"; Integer) { trigger OnValidate() begin "Working Date" := DMY2Date(1, 1, "Starting Year"); end; }
        field(3; "Working Date"; Date) { }
        field(4; "Company Type"; Option) { OptionMembers = VAT,"Sales Tax"; }
        field(5; "Advanced Setup"; Boolean) { }
        field(6; "Currency Code"; Code[10]) { }
        field(7; "Additional Currency"; Code[10]) { }
        field(8; "Data Type"; Option) { OptionMembers = Standard,Evaluation,Extended; }
    }
    keys { key(PK; "Primary Key") { } }
}
xmlport 69972 "RT Port"
{
    Encoding = UTF8;
    Format = Xml;
    schema
    {
        tableelement(Setup; "RT Setup")
        {
            XmlName = 'QxSetup';
            fieldelement(StartingYear; Setup."Starting Year") { }
            fieldelement(CompanyType; Setup."Company Type") { }
            fieldelement(AdvancedSetup; Setup."Advanced Setup") { }
            fieldelement(CurrencyCode; Setup."Currency Code") { }
            fieldelement(AdditionalCurrency; Setup."Additional Currency") { }
            fieldelement(DataType; Setup."Data Type") { }
        }
    }
    requestpage { layout { } actions { } }
}
xmlport 69973 "RT Plain"
{
    Format = Xml;
    UseRequestPage = false;
    schema
    {
        tableelement(Setup; "RT Setup")
        {
            XmlName = 'QxSetup';
            fieldelement(StartingYear; Setup."Starting Year") { }
            fieldelement(CompanyType; Setup."Company Type") { }
            fieldelement(AdvancedSetup; Setup."Advanced Setup") { }
            fieldelement(CurrencyCode; Setup."Currency Code") { }
            fieldelement(AdditionalCurrency; Setup."Additional Currency") { }
            fieldelement(DataType; Setup."Data Type") { }
        }
    }
}
codeunit 69974 C
{
    procedure Xml(RootName: Text; Year: Text): Text
    begin
        exit('<' + RootName + '><StartingYear>' + Year + '</StartingYear><CompanyType>VAT</CompanyType><AdvancedSetup>FALSE</AdvancedSetup>' +
            '<CurrencyCode>GBP</CurrencyCode><AdditionalCurrency/><DataType>Extended</DataType></' + RootName + '>');
    end;

    procedure DoImport(Content: Text)
    var B: Record "RT Blob" temporary; P: XmlPort "RT Port"; OutS: OutStream; InS: InStream;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Content);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        P.SetSource(InS); P.Import();
    end;

    procedure Rows(): Text
    var S: Record "RT Setup"; R: Text;
    begin
        if S.FindSet() then
            repeat
                R += '[' + S."Primary Key" + '|' + Format(S."Starting Year") + '|' + Format(S."Working Date", 0, 9) + '|' + Format(S."Company Type") + '|' +
                    Format(S."Advanced Setup") + '|' + S."Currency Code" + '|' + S."Additional Currency" + '|' + Format(S."Data Type") + ']';
            until S.Next() = 0;
        exit(R);
    end;

    procedure ImportOne(): Text
    begin
        DoImport('<?xml version="1.0" encoding="UTF-8" standalone="no"?>' + Xml('QxSetup', '2008'));
        exit(Rows());
    end;

    procedure ImportTwice(): Text
    begin
        DoImport(Xml('QxSetup', '2008'));
        DoImport(Xml('QxSetup', '2009'));
        exit(Rows());
    end;

    procedure OtherRoot() begin DoImport(Xml('Other', '2011')); end;
    procedure TwoRoots() begin DoImport(Xml('QxSetup', '2012') + Xml('QxSetup', '2013')); end;
    procedure EmptyRoot() begin DoImport('<QxSetup/>'); end;

    procedure Export(Plain: Boolean): Text
    var B: Record "RT Blob" temporary; P: XmlPort "RT Port"; Q: XmlPort "RT Plain"; OutS: OutStream; InS: InStream; Line: Text; R: Text;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        if Plain then begin Q.SetDestination(OutS); Q.Export(); end else begin P.SetDestination(OutS); P.Export(); end;
        if Plain then B.Data.CreateInStream(InS, TextEncoding::UTF16) else B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS do begin InS.ReadText(Line); R += Line + '|'; end;
        exit(R);
    end;

    procedure Seed(Two: Boolean)
    var S: Record "RT Setup";
    begin
        S."Starting Year" := 2008; S."Working Date" := DMY2Date(1, 1, 2008); S."Currency Code" := 'GBP'; S."Data Type" := S."Data Type"::Extended;
        S.Insert();
        if Two then begin S."Primary Key" := 'B'; S.Insert(); end;
    end;

    procedure ExportNone(): Text begin exit(Export(false)); end;
    procedure ExportOne(): Text begin Seed(false); exit(Export(false) + '#' + Export(true)); end;
    procedure ExportTwo(): Text begin Seed(true); exit(Export(false) + '#' + Export(true)); end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

const ROW: &str = "<QxSetup>|  <StartingYear>2008</StartingYear>|  <CompanyType>VAT</CompanyType>|  <AdvancedSetup>No</AdvancedSetup>|  <CurrencyCode>GBP</CurrencyCode>|  <AdditionalCurrency />|  <DataType>Extended</DataType>|</QxSetup>|";
const UTF8: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|";
const UTF16: &str = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|";

// AgXmlPortImport.al:539-540: one element is one row, its fields validated (Working Date comes from OnValidate).
#[test]
fn an_import_of_one_element_stores_one_row() {
    assert_eq!(text(run("ImportOne")), "[|2008|2008-01-01|VAT|No|GBP||Extended]");
}

// AgXmlPortImport.al:541-544: the same element again inserts the row a second time (the insert error comes at the latest
// with the next read).
#[test]
fn an_import_of_a_row_that_is_there_fails_on_insert() {
    assert_eq!(
        error_message(run("ImportTwice")),
        "The record in table RT Setup already exists. Identification fields and values: Primary Key=''"
    );
}

// AgXmlPortImport.al:547-554: another root name, two root elements, an empty root element.
#[test]
fn an_import_checks_the_root_element() {
    assert_eq!(
        error_message(run("OtherRoot")),
        "Imported XML cannot validate with the schema: The 'Other' element is not declared."
    );
    assert_eq!(
        error_message(run("TwoRoots")),
        "There are multiple root elements. Line 1, position 202."
    );
    assert_eq!(
        error_message(run("EmptyRoot")),
        "Imported XML cannot validate with the schema: The element 'QxSetup' has incomplete content. List of possible elements expected: 'StartingYear'."
    );
}

// AgXmlPortImport.al:575-580, 600-603: one root element per row; with no row, one element of the record as it stands.
#[test]
fn an_export_writes_one_root_element_per_row() {
    assert_eq!(
        text(run("ExportNone")),
        format!(
            "{UTF8}<QxSetup>|  <StartingYear>0</StartingYear>|  <CompanyType>VAT</CompanyType>|  <AdvancedSetup>No</AdvancedSetup>|  <CurrencyCode />|  <AdditionalCurrency />|  <DataType>Standard</DataType>|</QxSetup>|"
        )
    );
    assert_eq!(text(run("ExportOne")), format!("{UTF8}{ROW}#{UTF16}{ROW}"));
    assert_eq!(text(run("ExportTwo")), format!("{UTF8}{ROW}{ROW}#{UTF16}{ROW}{ROW}"));
}
