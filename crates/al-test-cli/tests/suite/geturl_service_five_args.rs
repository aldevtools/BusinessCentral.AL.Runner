//! GetUrl of a service client type (SOAP, OData, ODataV4, Api) with five arguments: an object and a record, no UseFilters
//! (al-runner issue 7586 item 5). The pinned shapes answer the empty text (GetUrlServiceClientRecord.al:166-182); their
//! neighbors refuse. An object a Web Service or Tenant Web Service row names refuses with every argument list: BC gives an
//! address with the service name there (GetUrlServiceClientRecord.al:311-317), which is not built.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 50100 "U Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "U Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 50100 "U List" { PageType = List; SourceTable = "U Row"; layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
codeunit 50000 T { Subtype = Test;
  [Test] procedure Pinned() var Row: Record "U Row"; begin
    if GetUrl(ClientType::ODataV4, CompanyName(), ObjectType::Page, Page::"U List", Row) <> '' then Error('not empty'); end;
  [Test] procedure Temporary() var Row: Record "U Row" temporary; begin
    if GetUrl(ClientType::ODataV4, CompanyName(), ObjectType::Page, Page::"U List", Row) <> '' then Error('not empty'); end;
  [Test] procedure OtherTable() var Row: Record "U Other"; begin
    if GetUrl(ClientType::ODataV4, CompanyName(), ObjectType::Page, Page::"U List", Row) <> '' then Error('not empty'); end;
  [Test] procedure Report() var Row: Record "U Row"; begin
    if GetUrl(ClientType::ODataV4, CompanyName(), ObjectType::Report, 50100, Row) <> '' then Error('not empty'); end;
  [Test] procedure PublishedSeedPage() begin
    if GetUrl(ClientType::ODataV4, CompanyName(), ObjectType::Page, 42) <> '' then Error('not empty'); end;
}
"#;

#[test]
fn five_argument_service_forms() {
    let o = outcomes(SRC);
    let get = |name: &str| {
        o.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
            .1
            .clone()
    };
    assert_eq!(get("Pinned"), Outcome::Pass);
    for name in ["Temporary", "OtherTable", "Report"] {
        match get(name) {
            Outcome::NotRun(why) => assert!(why.contains("this object type or argument list"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
    match get("PublishedSeedPage") {
        Outcome::NotRun(why) => assert!(why.contains("published as a web service"), "{why}"),
        other => panic!("PublishedSeedPage: {other:?}"),
    }
}
