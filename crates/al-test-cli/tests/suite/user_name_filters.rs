//! Filters on the user-name system FlowFields (SystemCreatedByUserName 2000000005 to SystemModifiedByFullName 2000000008, BC 29)
//! filter on the value the field calculates for each row: the user name or full name of the user in SystemCreatedBy or
//! SystemModifiedBy (system-fields/lt-y64-username-filters/Y64UserNameFilters.al; al-runner issue 6645). With the corpus symbol
//! BC29PLUS the schema of every table holds the four fields.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn run(objects: &str) -> Vec<(String, Outcome)> {
    let sources = vec![AppSource {
        id: "app0".into(),
        name: "app0".into(),
        dependencies: vec![],
        files: vec![("app0.al".into(), objects.to_string())],
    }];
    let (ws, l) = al_sema::lower::compile_apps(sources, vec![AppKind::Workspace], &al_version::corpus_symbols(29));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    run_codeunit(&Engine::new(l.program.clone(), blocked), &l, oi)
}

const OBJECTS: &str = "
table 69680 \"UN F Row\" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69681 T {
    Subtype = Test;

    local procedure Seed()
    var
        Row: Record \"UN F Row\";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Name := 'alpha';
        Row.Insert();
        Row.Id := 2;
        Row.Name := 'bravo';
        Row.Insert();
    end;

    [Test]
    procedure RecordSetRangeCountsTheRowsOfTheUser()
    var
        Row: Record \"UN F Row\";
    begin
        Seed();
        Row.SetRange(SystemCreatedByUserName, UserId());
        if Row.Count() <> 2 then
            Error('own rows %1', Row.Count());
        Row.SetRange(SystemCreatedByUserName, 'nobody-here');
        if Row.Count() <> 0 then
            Error('other rows %1', Row.Count());
    end;

    [Test]
    procedure ViewWhereAndFieldRefFilter()
    var
        RecRef: RecordRef;
    begin
        Seed();
        RecRef.Open(Database::\"UN F Row\");
        RecRef.SetView('WHERE(SystemModifiedByUserName=FILTER(<>''''),Name=FILTER(b*))');
        if RecRef.Count() <> 1 then
            Error('view count %1', RecRef.Count());
        if RecRef.Field(2000000007).GetFilter() <> '<>''''' then
            Error('getfilter %1', RecRef.Field(2000000007).GetFilter());
        RecRef.Field(2000000007).SetRange('nobody-here');
        if RecRef.Count() <> 0 then
            Error('after setrange %1', RecRef.Count());
    end;

    [Test]
    procedure ValueBeforeAndAfterCalcFields()
    var
        Row: Record \"UN F Row\";
    begin
        Seed();
        Row.FindFirst();
        if Row.SystemCreatedByUserName <> '' then
            Error('before %1', Row.SystemCreatedByUserName);
        Row.CalcFields(SystemCreatedByUserName);
        if Row.SystemCreatedByUserName <> UserId() then
            Error('after %1', Row.SystemCreatedByUserName);
    end;
}";

#[test]
fn filters_on_the_user_name_flow_fields_filter_the_rows() {
    for (name, outcome) in run(OBJECTS) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

/// `Sleep := 1` is the call `Sleep(1)` (record/ProbePropertyAssignmentForms.al:564 Y29_Collections_Text_Sleep_AssignForms).
#[test]
fn sleep_assignment_form_is_the_call() {
    let o = run("
codeunit 69682 T {
    Subtype = Test;

    [Test]
    procedure SleepAssign()
    begin
        Sleep := 1;
    end;
}");
    assert_eq!(o[0], ("SleepAssign".to_string(), Outcome::Pass), "{o:?}");
}
