//! The CompanyAddressLine1..8 elements of the SaveAs Xml dataset header on BC 29 (report/lt-ah-dataset-header/
//! AhDatasetHeader.al:98-151): Name, Address, Address 2, the city line, County and the country name, empty lines left out.

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
    }
    keys { key(PK; "Primary Key") { } }
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

    procedure AllFilledNoCounty(): Text begin Setup(0); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', '', '')); end;
    procedure AllFilledCounty(): Text begin Setup(0); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', 'Countyshire', '')); end;
    procedure CountryCodeOnly(): Text begin InitCountry(); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', '', 'AHX')); end;
    procedure AllFiveFilled(): Text begin InitCountry(); exit(Lines('', 'Street 1', 'Floor 2', 'Town', 'ZIP 12', 'Countyshire', 'AHX')); end;
    procedure NoAddress2(): Text begin InitCountry(); exit(Lines('', 'Street 1', '', 'Town', 'ZIP 12', 'Countyshire', 'AHX')); end;
    procedure OnlyCity(): Text begin Setup(0); exit(Lines('', '', '', 'Town', '', '', '')); end;
    procedure OnlyPostCode(): Text begin Setup(0); exit(Lines('', '', '', '', 'ZIP 12', '', '')); end;
    procedure OnlyCounty(): Text begin Setup(0); exit(Lines('', '', '', '', '', 'Countyshire', '')); end;
    procedure OnlyCountry(): Text begin InitCountry(); exit(Lines('', '', '', '', '', '', 'AHX')); end;
    procedure AllEmpty(): Text begin exit(Lines('', '', '', '', '', '', '')); end;

    // Not pinned: each is refused.
    procedure WithName2(): Text begin Setup(0); exit(Lines('Second', 'Street 1', '', '', '', '', '')); end;
    procedure CountryOnlyCity(): Text begin InitCountry(); exit(Lines('', '', '', 'Town', '', '', 'AHX')); end;
    procedure CountryOnlyPostCode(): Text begin InitCountry(); exit(Lines('', '', '', '', 'ZIP 12', '', 'AHX')); end;
    procedure CountryOtherFormat(): Text begin Country(Enum::"Country/Region Address Format"::"Post Code+City"); exit(Lines('', '', '', 'Town', 'ZIP 12', '', 'AHX')); end;
    procedure CountryMissing(): Text begin exit(Lines('', '', '', 'Town', 'ZIP 12', '', 'AHX')); end;
    procedure LocalOtherFormat(): Text begin Setup(1); exit(Lines('', '', '', 'Town', 'ZIP 12', '', '')); end;
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

/// What AhDatasetHeader.al does not pin is refused: Name 2, a country with only one of City and Post Code, another address
/// format, a country without a row, and a local format other than the one of the corpus database.
#[test]
fn unpinned_address_shapes_are_refused() {
    assert_refused("WithName2", "Name 2");
    assert_refused("CountryOnlyCity", "City");
    assert_refused("CountryOnlyPostCode", "City");
    assert_refused("CountryOtherFormat", "Address Format");
    assert_refused("CountryMissing", "AHX");
    assert_refused("LocalOtherFormat", "Local Address Format");
    assert_refused("LocalNoSetup", "General Ledger Setup");
}
