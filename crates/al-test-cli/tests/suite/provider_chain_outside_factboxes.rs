//! A Provider chain whose Provider is declared in the FactBoxes area of the host page itself (crates/al-vm/src/pages/
//! provider_hidden.rs `chain_outside_pinned_areas`, al-runner issue 7748). On a stored host whose Provider shows a row it runs;
//! a new host and a Provider that shows no row stay refused (corpus pull request 1324 measured them; the runner does not model
//! them).

use al_testrun::Outcome;

const SRC: &str = r#"
table 69970 "PC Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 69971 "PC Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; "Doc No.", "Line No.") { } }
}
table 69972 "PC Detail"
{
    fields { field(1; "Line Ref"; Integer) { } field(2; Note; Text[30]) { } }
    keys { key(PK; "Line Ref") { } }
}
page 69973 "PC Lines"
{
    PageType = ListPart; SourceTable = "PC Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field(Descr; Rec.Descr) { } } } }
}
page 69974 "PC Detail Part"
{
    PageType = CardPart; SourceTable = "PC Detail"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Note; Rec.Note) { } } }
}
page 69975 "PC Host"
{
    PageType = Card; SourceTable = "PC Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes)
        {
            part(Lines; "PC Lines") { SubPageLink = "Doc No." = field("No."); }
            part(Detail; "PC Detail Part") { Provider = Lines; SubPageLink = "Line Ref" = field("Line No."); }
        }
    }
}
codeunit 69976 C
{
    Subtype = Test;

    local procedure Seed()
    var Head: Record "PC Head"; Line: Record "PC Line"; Detail: Record "PC Detail";
    begin
        Detail.DeleteAll(); Line.DeleteAll(); Head.DeleteAll();
        Head."No." := 'H1'; Head.Insert(); Head."No." := 'H2'; Head.Insert();
        Line."Doc No." := 'H1'; Line."Line No." := 10000; Line.Descr := 'l1'; Line.Insert();
        Detail."Line Ref" := 10000; Detail.Note := 'n1'; Detail.Insert();
        Detail."Line Ref" := 0; Detail.Note := 'blank'; Detail.Insert();
    end;

    // A stored host whose Provider shows a row: runs.
    [Test]
    procedure StoredHost_ProviderRow()
    var P: TestPage "PC Host";
    begin
        Seed(); P.OpenView(); P.GoToKey('H1');
        if P.Detail.Note.Value <> 'n1' then Error('detail=%1', P.Detail.Note.Value);
        P.Close();
    end;

    // A stored host whose Provider shows no row, while a row holds the blank link value: refused.
    [Test]
    procedure StoredHost_ProviderNoRow()
    var P: TestPage "PC Host";
    begin
        Seed(); P.OpenView(); P.GoToKey('H2');
        Error('detail=%1', P.Detail.Note.Value);
    end;
}

codeunit 69977 D
{
    Subtype = Test;

    // A new host: refused as the page opens.
    [Test]
    procedure NewHost()
    var P: TestPage "PC Host";
    begin
        P.OpenNew();
        Error('opened');
    end;
}
"#;

fn outcome(codeunit: &str, name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, codeunit)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

#[test]
fn a_stored_host_whose_provider_shows_a_row_runs() {
    assert_eq!(outcome("C", "StoredHost_ProviderRow"), Outcome::Pass);
}

#[test]
fn a_provider_that_shows_no_row_is_refused() {
    match outcome("C", "StoredHost_ProviderNoRow") {
        Outcome::NotRun(why) => assert!(why.contains("Provider part that shows no row"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_new_host_with_such_a_chain_is_refused() {
    match outcome("D", "NewHost") {
        Outcome::NotRun(why) => assert!(why.contains("a new host with a Provider-chain part"), "{why}"),
        other => panic!("{other:?}"),
    }
}
