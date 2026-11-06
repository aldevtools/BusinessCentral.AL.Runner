//! Probe P7 of ADR-066 (al-runner issue 6924): within one test run, a SingleInstance codeunit's globals that one test
//! codeunit set (in a test, and in its OnRun) are still there in the next test codeunit, in run order; rows are rolled
//! back per test codeunit (corpus al-language/codeunits/TestCuP7ASetter.al:24-58, TestCuP7BReader.al:24-37). Test
//! codeunits run in parallel; the outcome must be the serial one.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The state: a SingleInstance codeunit (in a dependency app, as Microsoft's test libraries are) and a table.
const BASE: &str = r#"
table 50250 "Carry Row" { fields { field(1; "Key"; Code[20]) { } } keys { key(PK; "Key") { } } }
codeunit 50251 "Carry State"
{
    SingleInstance = true;
    procedure NoteRan(What: Text) begin if Ran <> '' then Ran += ';'; Ran += What; end;
    procedure GetRan(): Text begin exit(Ran); end;
    procedure Bump() begin Count += 1; end;
    procedure GetCount(): Integer begin exit(Count); end;
    procedure Keep(Value: Text) begin Held.Add(Value); end;
    procedure HeldCount(): Integer begin exit(Held.Count()); end;
    var
        Ran: Text;
        Count: Integer;
        Held: List of [Text];
}
"#;

/// Test codeunits 60250.., in run order. The first sets the state in OnRun and in a test and inserts a committed row;
/// each later one checks what all before it left and adds to it.
const IDS: [u32; 8] = [60250, 60251, 60252, 60253, 60254, 60255, 60256, 60257];

fn source() -> String {
    let mut out = String::new();
    for (i, id) in IDS.iter().enumerate() {
        let expected_ran: Vec<String> = (0..=i).map(|k| format!("{}", IDS[k])).collect();
        let expected_ran = expected_ran.join(";");
        out.push_str(&format!(
            r#"
codeunit {id} "Carry {id}"
{{
    Subtype = Test;
    trigger OnRun()
    var State: Codeunit "Carry State";
    begin
        State.NoteRan('{id}');
    end;

    [Test] procedure Sees{id}()
    var State: Codeunit "Carry State"; Row: Record "Carry Row"; I: Integer; W: BigInteger;
    begin
        // Some work, so that the codeunits overlap on the test threads.
        for I := 1 to 100000 do W += I;
        if State.GetRan() <> '{expected_ran}' then Error('codeunit {id}: ran %1', State.GetRan());
        if State.GetCount() <> {i} then Error('codeunit {id}: count %1', State.GetCount());
        if Row.Get('ROW') then Error('codeunit {id}: the row of an earlier codeunit is still there');
        State.Bump();
        Row."Key" := 'ROW';
        Row.Insert();
        Commit();
    end;
}}
"#
        ));
    }
    out
}

/// A codeunit that leaves a List in the state, and one after it that reads it.
const HANDLE: &str = r#"
codeunit 60260 "Carry List Writer"
{
    Subtype = Test;
    [Test] procedure KeepsAValue()
    var State: Codeunit "Carry State";
    begin
        State.Keep('x');
    end;
}
codeunit 60261 "Carry List Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheList()
    var State: Codeunit "Carry State";
    begin
        if State.HeldCount() <> 1 then Error('count %1', State.HeldCount());
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

fn run(src: &str, opts: &RunOptions) -> Vec<(String, Outcome)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts");
    rt.results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone()))
        .collect()
}

#[test]
fn single_instance_globals_reach_the_next_test_codeunit_and_rows_do_not() {
    let results = run(&source(), &RunOptions::default());
    assert_eq!(results.len(), IDS.len());
    for (test, outcome) in &results {
        assert_eq!(*outcome, Outcome::Pass, "{test}");
    }
}

#[test]
fn a_selected_codeunit_starts_from_what_the_codeunits_that_ran_before_it_left() {
    // Only the fourth codeunit runs: no earlier codeunit ran in this run, so it sees the state fresh (as on BC).
    let opts = RunOptions {
        only: Some(vec![(0, IDS[3])]),
        ..Default::default()
    };
    let results = run(&source(), &opts);
    assert_eq!(results.len(), 1);
    match &results[0].1 {
        Outcome::Fail(m) => assert!(m.contains(&format!("codeunit {}: ran {}", IDS[3], IDS[3])), "{m}"),
        o => panic!("expected the fresh state's failure, got {o:?}"),
    }
}

#[test]
fn a_handle_left_in_a_single_instance_is_refused_where_a_later_codeunit_reads_it() {
    let results = run(HANDLE, &RunOptions::default());
    let reader = results.iter().find(|(t, _)| t == "ReadsTheList").expect("reader ran");
    match &reader.1 {
        Outcome::NotRun(why) => assert!(why.contains("carries only simple values"), "{why}"),
        o => panic!("expected a refusal, got {o:?}"),
    }
    let writer = results.iter().find(|(t, _)| t == "KeepsAValue").expect("writer ran");
    assert_eq!(writer.1, Outcome::Pass);
}
