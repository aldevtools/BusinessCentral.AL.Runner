//! The `decimalformatter` of a report column whose source field has an AutoFormatType the runtime does not model (al-runner issue
//! 8011, review of PR 8015): a column that states no type of its own takes its field's, and such a type is refused. A field of a
//! modelled type keeps the earlier rule (the field's DecimalPlaces).

const SRC: &str = r#"
table 69470 "AfC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Custom; Decimal) { AutoFormatType = 50733; }
        field(3; T1; Decimal) { AutoFormatType = 1; }
        field(4; B; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
report 69471 "AfC Custom"
{
    UsageCategory = None;
    dataset { dataitem(R; "AfC Row") { column(ColCustomField; Custom) { } } }
}
report 69472 "AfC T1"
{
    UsageCategory = None;
    dataset { dataitem(R; "AfC Row") { column(ColT1Field; T1) { } } }
}
report 69473 "AfC Own"
{
    UsageCategory = None;
    dataset { dataitem(R; "AfC Row") { column(ColOwn; Custom) { AutoFormatType = 1; } } }
}
codeunit 69474 C
{
    local procedure Save(Id: Integer): Text
    var
        Row: Record "AfC Row";
        Tmp: Record "AfC Row" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Cols: Text;
    begin
        Row.Id := 1;
        Row.Custom := 3456.67843;
        Row.T1 := 3456.67843;
        Row.Insert();
        Tmp.B.CreateOutStream(OutS);
        Report.SaveAs(Id, '', ReportFormat::Xml, OutS);
        Tmp.B.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('<Column ') then
                Cols += Line.Trim();
        end;
        exit(Cols);
    end;

    procedure Custom(): Text
    begin
        exit(Save(Report::"AfC Custom"));
    end;

    procedure T1(): Text
    begin
        exit(Save(Report::"AfC T1"));
    end;

    procedure Own(): Text
    begin
        exit(Save(Report::"AfC Own"));
    end;
}
"#;

#[test]
fn a_column_over_a_field_of_an_unmodelled_type_is_refused() {
    let e = format!("{:?}", al_testkit::run_in(SRC, "C", "Custom"));
    assert!(e.contains("Unsupported") && e.contains("AutoFormatType 50733"), "{e}");
}

/// A field of a modelled type, and a column whose own type is modelled over a field of an unmodelled one, keep the earlier rule.
#[test]
fn a_modelled_type_of_the_column_or_its_field_keeps_the_earlier_formatter() {
    let t1 = al_testkit::text(al_testkit::run_in(SRC, "C", "T1"));
    assert!(t1.contains("decimalformatter=\"#,##0.00\""), "{t1}");
    let own = al_testkit::text(al_testkit::run_in(SRC, "C", "Own"));
    assert!(own.contains("decimalformatter=\"#,##0.00\""), "{own}");
}
