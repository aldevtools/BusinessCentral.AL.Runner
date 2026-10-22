//! The CompanyAddressLine1..8 elements of the SaveAs Xml dataset header on BC 29 (report/lt-ah-dataset-header/
//! AhDatasetHeader.al:98-151, 260-460): Name, Name 2, Address, Address 2, the city line, County and the country name, empty lines
//! left out, the city line per address format.

use al_builtins::native::Signal;
use al_testkit::text;

const FIXTURE: &str = r#"
table 79 "Company Information"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; Name; Text[100]) { }
        field(3; "Name 2"; Text[50]) { }
        field(4; Address; Text[100]) { }
        field(5; "Address 2"; Text[50]) { }
        field(6; City; Text[30]) { }
        field(30; "Post Code"; Code[20]) { }
        field(31; County; Text[30]) { }
        field(36; "Country/Region Code"; Code[10]) { }
        field(51; "Contact Person"; Text[50]) { }
    }
    keys { key(PK; "Primary Key") { } }
}

table 725 "Custom Address Format"
{
    fields
    {
        field(1; "Country/Region Code"; Code[10]) { }
        field(2; "Line Position"; Option) { OptionMembers = "1","2","3","4","5","6","7","8"; }
        field(4; "Line No."; Integer) { }
        field(5; "Field ID"; Integer) { }
    }
    keys { key(PK; "Country/Region Code", "Line No.") { } }
}

table 726 "Custom Address Format Line"
{
    fields
    {
        field(1; "Country/Region Code"; Code[10]) { }
        field(3; "Field Position"; Option) { OptionMembers = "1","2","3"; }
        field(4; Separator; Text[10]) { }
        field(5; "Field ID"; Integer) { }
        field(7; "Line No."; Integer) { }
    }
    keys { key(PK; "Country/Region Code", "Line No.", "Field ID") { } }
}

enum 50980 "Country/Region Address Format"
{
    value(0; "Post Code+City") { }
    value(1; "City+Post Code") { }
    value(2; "City+County+Post Code") { }
    value(3; "Blank Line+Post Code+City") { }
    value(13; "Custom") { }
}

table 9 "Country/Region"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Name; Text[50]) { }
        field(8; "Address Format"; Enum "Country/Region Address Format") { InitValue = "City+Post Code"; }
    }
    keys { key(PK; Code) { } }
}

table 98 "General Ledger Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(57; "Local Address Format"; Option) { OptionMembers = "Post Code+City","City+Post Code","City+County+Post Code","Blank Line+Post Code+City"; }
    }
    keys { key(PK; "Primary Key") { } }
}

table 50981 "AH Blob"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

report 50982 "AH Header"
{
    dataset { dataitem(T; "AH Blob") { column(TId; Id) { } } }
}

codeunit 50983 "AH Header Tests"
{
    local procedure Lines(Name2: Text; Address: Text; Address2: Text; City: Text; PostCode: Text; County: Text; CountryCode: Code[10]): Text
    var
        Info: Record "Company Information";
        B: Record "AH Blob";
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Result: Text;
    begin
        Info.Name := 'Co';
        Info."Name 2" := Name2;
        Info.Address := Address;
        Info."Address 2" := Address2;
        Info.City := City;
        Info."Post Code" := PostCode;
        Info.County := County;
        Info."Country/Region Code" := CountryCode;
        Info."Contact Person" := 'Contact';
        Info.Insert();
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS);
        Report.SaveAs(Report::"AH Header", '', ReportFormat::Xml, OutS);
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('CompanyAddressLine') and not Line.Contains('CompanyAddressLines') then
                Result += Line.Trim();
        end;
        exit(Result);
    end;

    local procedure Setup(LocalFormat: Integer)
    var
        GLSetup: Record "General Ledger Setup";
    begin
        GLSetup."Local Address Format" := LocalFormat;
        GLSetup.Insert();
    end;

    local procedure Country(Format: Enum "Country/Region Address Format")
    var
        C: Record "Country/Region";
    begin
        C.Init();
        C.Code := 'AHX';
        C.Name := 'Ahland';
        C."Address Format" := Format;
        C.Insert();
    end;

    local procedure InitCountry()
    var
        C: Record "Country/Region";
    begin
        C.Init();
        C.Code := 'AHX';
        C.Name := 'Ahland';
        C.Insert();
    end;

    // The rows Validate("Address Format", Custom) writes for a country of the City+Post Code format (AhDatasetHeader.al:369-380).
    local procedure CustomRows(Separator: Text)
    var
        F: Record "Custom Address Format";
        L: Record "Custom Address Format Line";
        Ids: List of [Integer];
        Pos: Integer;
    begin
        Country(Enum::"Country/Region Address Format"::Custom);
        Ids.AddRange(2, 3, 51, 4, 5, 0, 36);
        for Pos := 1 to 7 do begin
            F."Country/Region Code" := 'AHX';
            F."Line No." := Pos;
            F."Line Position" := Pos - 1;
            F."Field ID" := Ids.Get(Pos);
            F.Insert();
        end;
        L."Country/Region Code" := 'AHX';
        L."Line No." := 6;
        L."Field Position" := 0;
        L."Field ID" := 6;
        L.Separator := Separator;
        L.Insert();
        L."Field Position" := 1;
        L."Field ID" := 30;
        L.Separator := '';
        L.Insert();
    end;

    procedure AllFilledNoCounty(): Text begin Setup(0); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', '', '')); end;
    procedure AllFilledCounty(): Text begin Setup(0); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', 'Countyshire', '')); end;
    procedure CountryCodeOnly(): Text begin InitCountry(); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', '', 'AHX')); end;
    procedure AllFiveFilled(): Text begin InitCountry(); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', 'Countyshire', 'AHX')); end;
    procedure NoAddress2(): Text begin InitCountry(); exit(Lines('', 'Street 1', '', 'Town', 'ZIP 12', 'Countyshire', 'AHX')); end;
    procedure OnlyCity(): Text begin Setup(0); exit(Lines('', '', '', 'Town', '', '', '')); end;
    procedure OnlyPostCode(): Text begin Setup(0); exit(Lines('', '', '', '', 'ZIP 12', '', '')); end;
    procedure OnlyCounty(): Text begin Setup(0); exit(Lines('', '', '', '', '', 'Countyshire', '')); end;
    procedure OnlyCountry(): Text begin InitCountry(); exit(Lines('', '', '', '', '', '', 'AHX')); end;
    procedure AllEmpty(): Text begin Setup(0); exit(Lines('', '', '', '', '', '', '')); end;

    // Writer RB, AhDatasetHeader.al:260-460: Name 2, every address format, a code without a row, every local format.
    procedure WithName2(): Text begin Setup(0); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', 'Shire', '')); end;
    procedure PostCodeCity(County: Text): Text begin Country(Enum::"Country/Region Address Format"::"Post Code+City"); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, 'AHX')); end;
    procedure CityCountyPostCode(County: Text): Text begin Country(Enum::"Country/Region Address Format"::"City+County+Post Code"); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, 'AHX')); end;
    procedure BlankLine(County: Text): Text begin Country(Enum::"Country/Region Address Format"::"Blank Line+Post Code+City"); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, 'AHX')); end;
    procedure Custom(County: Text): Text begin CustomRows(''); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, 'AHX')); end;
    procedure CountryMissing(County: Text): Text begin exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, 'AHX')); end;
    procedure LocalFormat(Format: Integer; County: Text): Text begin Setup(Format); exit(Lines('Second', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', County, '')); end;

    procedure GermanSession(): Text
    var
        R: Text;
    begin
        Setup(0);
        GlobalLanguage(1031);
        R := Lines('', 'Street 1', '', 'Town', 'ZIP 12', '', '');
        GlobalLanguage(1033);
        exit(R);
    end;

    // Not pinned: each is refused.
    procedure CountryOnlyCity(): Text begin InitCountry(); exit(Lines('', '', '', 'Town', '', '', 'AHX')); end;
    procedure CountryOnlyPostCode(): Text begin InitCountry(); exit(Lines('', '', '', '', 'ZIP 12', '', 'AHX')); end;
    procedure CountryOtherFormatOnlyCity(): Text begin Country(Enum::"Country/Region Address Format"::"Post Code+City"); exit(Lines('', '', '', 'Town', '', '', 'AHX')); end;
    procedure CountyFormatNoPostCode(): Text begin Country(Enum::"Country/Region Address Format"::"City+County+Post Code"); exit(Lines('', '', '', 'Town', '', 'Shire', 'AHX')); end;
    procedure CountyFormatNoCity(): Text begin Country(Enum::"Country/Region Address Format"::"City+County+Post Code"); exit(Lines('', '', '', '', 'ZIP 12', 'Shire', 'AHX')); end;
    procedure LocalOtherFormatOnlyPostCode(): Text begin Setup(3); exit(Lines('', '', '', '', 'ZIP 12', '', '')); end;
    procedure CustomOtherSeparator(): Text begin CustomRows(','); exit(Lines('', '', '', 'Town', 'ZIP 12', '', 'AHX')); end;
    procedure CustomOnlyCity(): Text begin CustomRows(''); exit(Lines('', '', '', 'Town', '', '', 'AHX')); end;
    procedure LocalNoSetup(): Text begin exit(Lines('', '', '', 'Town', 'ZIP 12', '', '')); end;
}
"#;

fn lines(routine: &str) -> String {
    text(al_testkit::run_in(FIXTURE, "AH Header Tests", routine))
}

/// The eight elements as the corpus helper reads them (AhDatasetHeader.al:44-66): `n=[text] `.
fn expect(lines: &[&str]) -> String {
    (1..=8)
        .map(|n| {
            let v = lines.get(n - 1).copied().unwrap_or("");
            format!("<CompanyAddressLine{n}>{v}</CompanyAddressLine{n}>")
        })
        .collect()
}

fn assert_refused(routine: &str, needle: &str) {
    match al_testkit::run_in(FIXTURE, "AH Header Tests", routine) {
        Err(Signal::Unsupported(m)) if m.contains(needle) => {}
        other => panic!("{routine}: expected a refusal containing {needle:?}, got {other:?}"),
    }
}

/// AhDatasetHeader.al:125-126: without a country, the city line is the post code, a space and the city.
#[test]
fn without_a_country_the_city_line_is_post_code_then_city() {
    assert_eq!(
        lines("AllFilledNoCounty"),
        expect(&["Co", "Street 1", "Floor 2", "ZIP 12 Town"])
    );
    assert_eq!(
        lines("AllFilledCounty"),
        expect(&["Co", "Street 1", "Floor 2", "ZIP 12 Town", "Countyshire"])
    );
}

/// AhDatasetHeader.al:127-129: with a country of the City+Post Code format (the InitValue), the city line is the
/// city, a comma and the post code; County, then the country's name follow.
#[test]
fn with_a_country_the_city_line_is_city_then_post_code() {
    assert_eq!(
        lines("CountryCodeOnly"),
        expect(&["Co", "Street 1", "Floor 2", "Town, ZIP 12", "Ahland"])
    );
    assert_eq!(
        lines("AllFiveFilled"),
        expect(&["Co", "Street 1", "Floor 2", "Town, ZIP 12", "Countyshire", "Ahland"])
    );
    assert_eq!(
        lines("NoAddress2"),
        expect(&["Co", "Street 1", "Town, ZIP 12", "Countyshire", "Ahland"])
    );
}

/// AhDatasetHeader.al:152-156: empty lines are left out; a post code alone keeps its trailing space.
#[test]
fn partly_filled_addresses_leave_out_empty_lines() {
    assert_eq!(lines("OnlyCity"), expect(&["Co", "Town"]));
    assert_eq!(lines("OnlyPostCode"), expect(&["Co", "ZIP 12 "]));
    assert_eq!(lines("OnlyCounty"), expect(&["Co", "Countyshire"]));
    assert_eq!(lines("OnlyCountry"), expect(&["Co", "Ahland"]));
    assert_eq!(lines("AllEmpty"), expect(&["Co"]));
}

fn lines_with(routine: &str, args: Vec<al_builtins::value::Value>) -> String {
    text(al_testkit::run_with(FIXTURE, "AH Header Tests", routine, args).map(|(v, _)| v))
}

fn county(routine: &str, county: &str) -> String {
    lines_with(
        routine,
        vec![al_builtins::value::Value::Text(county.to_string().into())],
    )
}

const FULL: [&str; 4] = ["Co", "Second", "Street 1", "Floor 2"];

fn full(rest: &[&str]) -> String {
    expect(&FULL.iter().chain(rest).copied().collect::<Vec<_>>())
}

/// AhDatasetHeader.al:309-361 (writer RB): Name 2 is line 2; Post Code+City and Blank Line+Post Code+City write the post code,
/// a space and the city (no blank line), County on its own line; City+County+Post Code writes the city, a comma, the county
/// and the post code (the city, a comma and the post code without a county); the Contact Person is never written.
#[test]
fn each_country_address_format_writes_its_city_line() {
    assert_eq!(lines("WithName2"), full(&["ZIP 12 Town", "Shire"]));
    for f in ["PostCodeCity", "BlankLine"] {
        assert_eq!(county(f, "Shire"), full(&["ZIP 12 Town", "Shire", "Ahland"]), "{f}");
        assert_eq!(county(f, ""), full(&["ZIP 12 Town", "Ahland"]), "{f}");
    }
    assert_eq!(
        county("CityCountyPostCode", "Shire"),
        full(&["Town, Shire ZIP 12", "Ahland"])
    );
    assert_eq!(county("CityCountyPostCode", ""), full(&["Town, ZIP 12", "Ahland"]));
}

/// AhDatasetHeader.al:366-388: the Custom rows Validate writes from City+Post Code give the city, a space and the post code,
/// and no County; a code without a Country/Region row writes as City+Post Code with the code as the country line.
#[test]
fn custom_rows_and_a_code_without_a_row() {
    assert_eq!(county("Custom", "Shire"), full(&["Town ZIP 12", "Ahland"]));
    assert_eq!(county("Custom", ""), full(&["Town ZIP 12", "Ahland"]));
    assert_eq!(
        county("CountryMissing", "Shire"),
        full(&["Town, ZIP 12", "Shire", "AHX"])
    );
    assert_eq!(county("CountryMissing", ""), full(&["Town, ZIP 12", "AHX"]));
}

/// AhDatasetHeader.al:390-430: without a country, the General Ledger Setup's Local Address Format 0..3 gives the city line of the
/// matching country format, and no country line.
#[test]
fn every_local_address_format() {
    use al_builtins::value::Value;
    let local = |f: i32, c: &str| {
        lines_with(
            "LocalFormat",
            vec![Value::Integer(f), Value::Text(c.to_string().into())],
        )
    };
    for f in [0, 3] {
        assert_eq!(local(f, "Shire"), full(&["ZIP 12 Town", "Shire"]), "{f}");
        assert_eq!(local(f, ""), full(&["ZIP 12 Town"]), "{f}");
    }
    assert_eq!(local(1, "Shire"), full(&["Town, ZIP 12", "Shire"]));
    assert_eq!(local(1, ""), full(&["Town, ZIP 12"]));
    assert_eq!(local(2, "Shire"), full(&["Town, Shire ZIP 12"]));
    assert_eq!(local(2, ""), full(&["Town, ZIP 12"]));
}

/// What AhDatasetHeader.al does not pin is refused: a country with only one of City and Post Code, City+County+Post Code
/// without a post code or a city, a local format other than Post Code+City with only one of them, Custom rows of another shape
/// or with only one of City and Post Code, and a local format without a General Ledger Setup row.
#[test]
fn unpinned_address_shapes_are_refused() {
    assert_refused("CountryOnlyCity", "City");
    assert_refused("CountryOnlyPostCode", "City");
    assert_refused("CountryOtherFormatOnlyCity", "City");
    assert_refused("CountyFormatNoPostCode", "City");
    assert_refused("CountyFormatNoCity", "City");
    assert_refused("LocalOtherFormatOnlyPostCode", "City");
    assert_refused("CustomOtherSeparator", "Custom");
    assert_refused("CustomOnlyCity", "City");
    assert_refused("LocalNoSetup", "General Ledger Setup");
    // The company captions of the header in German.
    assert_refused("GermanSession", "company captions");
}
