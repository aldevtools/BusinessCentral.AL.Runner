//! Report properties that change nothing a run of the data items can observe. Pageworks declares `Extensible` on its
//! reports and Microsoft's "Standard Sales - Invoice" declares `EnableHyperlinks`; each refused the whole run with
//! "report property ... is not supported yet" (200 of Pageworks' 1013 tests were not run, 14 of them for these two).
//!
//! `Extensible` says whether a report extension may extend the report: a compile-time fact. It is not a column of the
//! Report Metadata table (the column list in recordref-fieldref/PnC21Metadata.al:ReportMetadata_Defaults). No
//! corpus test runs a report that declares it, so this is a documentation-only assumption (docs/probes-needed.md).
//! `EnableHyperlinks` is a layout property (the hyperlinks of an RDLC layout): like the other layout properties a run
//! that renders a layout is refused, a dataset run is not. It is a Report Metadata column, whose value stays refused
//! as an unpinned column.

use al_testkit::text;

const SRC: &str = r#"
table 69500 "RP Tab" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
report 69501 "RP Report"
{
    Extensible = true;
    EnableHyperlinks = true;
    UsageCategory = None;
    dataset { dataitem(T; "RP Tab") { column(Id; Id) { } } }
}
codeunit 69502 C
{
    procedure Xml(): Text var R: Record "RP Tab"; Out: OutStream; Inp: InStream; Line: Text; All: Text; begin
        R.Id := 7; R.Insert();
        R.Data.CreateOutStream(Out);
        if not Report.SaveAs(Report::"RP Report", '', ReportFormat::Xml, Out) then exit('failed: ' + GetLastErrorText());
        R.CalcFields(Data);
        R.Data.CreateInStream(Inp);
        while not Inp.EOS do begin Inp.ReadText(Line); All += Line; end;
        exit(All);
    end;
    procedure Pdf(): Boolean var R: Record "RP Tab"; Out: OutStream; begin
        R.Data.CreateOutStream(Out);
        exit(Report.SaveAs(Report::"RP Report", '', ReportFormat::Pdf, Out));
    end;
}
"#;

// A dataset run is not a rendered run: it writes the same dataset as a report without the two properties.
#[test]
fn a_report_declaring_extensible_and_enablehyperlinks_writes_its_dataset() {
    let xml = text(al_testkit::run_in(SRC, "C", "Xml"));
    assert!(xml.contains("<Column name=\"Id\">7</Column>"), "{xml}");
}

// A rendered format stays refused, and the reason is the format, not the property.
#[test]
fn a_rendered_format_is_still_refused() {
    match al_testkit::run_in(SRC, "C", "Pdf") {
        Err(al_builtins::native::Signal::Unsupported(r)) => assert!(r.contains("Pdf"), "{r}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
