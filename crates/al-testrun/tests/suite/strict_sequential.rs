//! ADR-070, migration step 2: the plain sequential strict run (`RunOptions::strict_sequential` with `strict_order`).
//! Every test codeunit of a BC session runs one after the other in BC's order on one thread; the single instances, rows,
//! number sequence values and storage move along (P7: codeunits/TestCuP7BReader.al:24-37 against TestCuP7ASetter.al:24-58).
//! BC sessions are the suite and hub leg, or each line of `--test-suites`. A later codeunit of a session that ran after a
//! test refused while it ran is flagged (`Assumption::StrictAfterRefused`).

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
codeunit 50601 SeqInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SEQ', 1, 1); end;
}
codeunit 50602 "Seq State"
{
    SingleInstance = true;
    procedure Bump(): Integer begin Count += 1; exit(Count); end;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Count: Integer; Flag: Text;
}
table 50603 "Seq Row"
{
    fields { field(1; "No."; Integer) { } }
    keys { key(PK; "No.") { } }
}
"#;

/// A test codeunit whose only test passes when it is the `rank`th that bumped the shared count.
fn ranked(id: u32, rank: u32) -> String {
    format!(
        r#"
codeunit {id} "Seq Ranked {id}"
{{
    Subtype = Test;
    [Test] procedure Rank{id}()
    var S: Codeunit "Seq State"; N: Integer;
    begin
        N := S.Bump();
        if N <> {rank} then Error('codeunit {id} is number %1 in its session, not {rank}', N);
    end;
}}
"#
    )
}

fn run(src: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

fn sequential() -> RunOptions {
    RunOptions {
        strict_order: true,
        strict_sequential: true,
        ..Default::default()
    }
}

fn suites(text: &str) -> RunOptions {
    RunOptions {
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse(text).expect("a valid suites file"),
        )),
        ..sequential()
    }
}

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

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a al_testrun::runtime::TestResult {
    rt.results().iter().find(|r| r.test == test).unwrap_or_else(|| {
        panic!(
            "{test} ran: {:?}",
            rt.results().iter().map(|r| &r.test).collect::<Vec<_>>()
        )
    })
}

fn assert_all_pass_clean(rt: &CorpusRuntime) {
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
        assert!(r.assumptions.is_empty(), "{} is flagged: {:?}", r.test, r.assumptions);
    }
}

/// Every codeunit is a member of the walk, in id order (the source lists them out of order), and the single instances
/// move from one to the next.
#[test]
fn every_codeunit_runs_in_id_order_and_the_single_instances_move_on() {
    let src = [ranked(60603, 3), ranked(60601, 1), ranked(60604, 4), ranked(60602, 2)].concat();
    let rt = run(&src, &sequential());
    assert_all_pass_clean(&rt);
    let chain = &rt.phases.chain;
    let ids: Vec<u32> = chain.members.iter().map(|m| m.id).collect();
    assert_eq!(ids, vec![60601, 60602, 60603, 60604], "{chain:?}");
    assert_eq!(chain.singles_kept, 4, "{chain:?}");
    assert_eq!(chain.single_lane.rerun, 0, "{:?}", chain.single_lane);
}

/// Gap 5 of the review: a member whose OnRun writes a table is in today's chain, and a plain codeunit before it is not,
/// so it does not see what that one set. Here every codeunit is in the walk: the member sees the flag, clean.
#[test]
fn a_member_sees_the_single_instance_state_of_a_plain_codeunit_before_it() {
    let src = r#"
codeunit 60611 "Seq Plain Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Seq State"; begin S.SetFlag('left'); end;
}
codeunit 60612 "Seq Writing Reader"
{
    Subtype = Test;
    trigger OnRun() var R: Record "Seq Row"; begin R."No." := 1; R.Insert(); end;
    [Test] procedure ReadsTheFlagAfterAPlainCodeunit()
    var S: Codeunit "Seq State";
    begin
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;
    let rt = run(src, &sequential());
    assert_all_pass_clean(&rt);
}

/// Gap 1 of the review: a codeunit in the chain only for the numbers its tests take runs with fresh single instances in
/// today's strict path, and passes flagged after a lane re-run. In the sequential run it keeps them and passes clean.
#[test]
fn a_codeunit_that_takes_numbers_keeps_the_single_instances_and_passes_clean() {
    let src = r#"
codeunit 60621 "Seq Numbers Setter"
{
    Subtype = Test;
    [Test] procedure TakesANumberAndSetsTheFlag()
    var N: BigInteger; S: Codeunit "Seq State";
    begin
        N := NumberSequence.Next('SEQ');
        S.SetFlag('left');
    end;
}
codeunit 60622 "Seq Numbers Reader"
{
    Subtype = Test;
    [Test] procedure TakesTheNextNumberAndReadsTheFlag()
    var N: BigInteger; S: Codeunit "Seq State";
    begin
        N := NumberSequence.Next('SEQ');
        if N <> 2 then Error('number %1', N);
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;
    let rt = run(src, &sequential());
    assert_all_pass_clean(&rt);
    assert_eq!(
        rt.phases.chain.single_lane.rerun, 0,
        "{:?}",
        rt.phases.chain.single_lane
    );
}

/// Rows an OnRun trigger wrote reach the codeunits after it, and the ones before it do not see them.
#[test]
fn rows_an_onrun_trigger_wrote_reach_the_later_codeunits_only() {
    let reader = |id: u32, want: bool| {
        format!(
            r#"
codeunit {id} "Seq Row Reader {id}"
{{
    Subtype = Test;
    [Test] procedure Reads{id}()
    var R: Record "Seq Row";
    begin
        if R.Get(7) <> {want} then Error('row 7 here: %1', R.Get(7));
    end;
}}
"#
        )
    };
    let writer = r#"
codeunit 60632 "Seq Row Writer"
{
    Subtype = Test;
    trigger OnRun() var R: Record "Seq Row"; begin R."No." := 7; R.Insert(); end;
    [Test] procedure Writes() begin end;
}
"#;
    let src = [reader(60631, false), writer.to_string(), reader(60633, true)].concat();
    let rt = run(&src, &sequential());
    assert_all_pass_clean(&rt);
}

/// A `--test-suites` file makes each line a BC session: the file's order, fresh state at the start of each, a codeunit
/// in no suite a session of its own.
#[test]
fn each_test_suites_line_is_a_session_in_the_files_order() {
    let src = [
        ranked(60641, 2),
        ranked(60642, 1),
        ranked(60643, 1),
        ranked(60644, 2),
        ranked(60645, 3),
        ranked(60646, 1),
    ]
    .concat();
    // Suite A runs 60642 before 60641; B starts fresh; 60645 is in both and is third in each; 60646 is in none.
    let rt = run(&src, &suites("A: 60642 60641 60645\nB: 60643 60644 60645\n"));
    assert_all_pass_clean(&rt);
    assert_eq!(rt.results().len(), 6, "each codeunit is reported once");
}

/// A test refused while it ran leaves state BC would not have: the later codeunits of its session are flagged, the
/// earlier ones and other sessions are not.
#[test]
fn codeunits_after_a_test_refused_while_it_ran_are_flagged_in_that_session_only() {
    let refusing = r#"
codeunit 60652 "Seq Refusing"
{
    Subtype = Test;
    [Test] procedure PassesFirst() var S: Codeunit "Seq State"; begin S.SetFlag('x'); end;
    [Test] procedure IsRefusedWhileItRuns() begin Sleep(-1); end;
}
"#;
    let lenient = |id: u32| {
        format!(
            r#"
codeunit {id} "Seq Lenient {id}"
{{
    Subtype = Test;
    [Test] procedure Lenient{id}() begin end;
}}
"#
        )
    };
    let src = [lenient(60651), refusing.to_string(), lenient(60653), lenient(60654)].concat();
    let rt = run(&src, &suites("A: 60651 60652 60653\nB: 60654\n"));
    assert!(
        matches!(result(&rt, "IsRefusedWhileItRuns").outcome, Outcome::NotRun(_)),
        "{:?}",
        rt.results()
    );
    let flagged = |t: &str| result(&rt, t).assumptions.contains(&Assumption::StrictAfterRefused);
    assert!(!flagged("Lenient60651"), "before the refused test");
    assert!(!flagged("PassesFirst"), "the codeunit of the refused test itself");
    assert!(flagged("Lenient60653"), "after it, same session");
    assert_eq!(result(&rt, "Lenient60653").outcome, Outcome::Pass);
    assert!(!flagged("Lenient60654"), "another session");
}

/// The default of `strict_order` is unchanged: without the internal option the chain holds only what it did before.
#[test]
fn without_the_internal_option_plain_codeunits_stay_out_of_the_chain() {
    let src = [ranked(60661, 1), ranked(60662, 2)].concat();
    let rt = run(
        &src,
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    assert!(rt.phases.chain.members.is_empty(), "{:?}", rt.phases.chain.members);
}

/// A test stopped by the time limit hands on partial state: the later codeunits of its session are flagged too.
#[test]
fn codeunits_after_a_test_stopped_by_the_time_limit_are_flagged() {
    let src = r#"
codeunit 60671 "Seq Slow"
{
    Subtype = Test;
    [Test] procedure RunsOutOfTime() begin Sleep(5000); end;
}
codeunit 60672 "Seq After Slow"
{
    Subtype = Test;
    [Test] procedure RunsAfterTheTimeLimitStop() begin end;
}
"#;
    let rt = run(
        src,
        &RunOptions {
            test_time_limit: Some(std::time::Duration::from_secs(1)),
            ..sequential()
        },
    );
    assert!(
        matches!(&result(&rt, "RunsOutOfTime").outcome, Outcome::NotRun(m) if m.contains("time limit")),
        "{:?}",
        result(&rt, "RunsOutOfTime").outcome
    );
    let after = result(&rt, "RunsAfterTheTimeLimitStop");
    assert_eq!(after.outcome, Outcome::Pass);
    assert!(after.assumptions.contains(&Assumption::StrictAfterRefused));
}
