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
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
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

/// A codeunit with one test that sets the SingleInstance flag and then is refused while it runs (`Sleep(-1)`).
const REFUSING: &str = r#"
codeunit 60652 "Seq Refusing"
{
    Subtype = Test;
    [Test] procedure PassesFirst() var S: Codeunit "Seq State"; begin S.SetFlag('x'); end;
    [Test] procedure IsRefusedWhileItRuns() var S: Codeunit "Seq State"; begin S.SetFlag('y'); Sleep(-1); end;
}
"#;

fn lenient(id: u32) -> String {
    format!(
        r#"
codeunit {id} "Seq Lenient {id}"
{{
    Subtype = Test;
    [Test] procedure Lenient{id}() begin end;
}}
"#
    )
}

/// A codeunit whose test reads the flag and bumps the count of the SingleInstance codeunit.
fn flag_reader(id: u32) -> String {
    format!(
        r#"
codeunit {id} "Seq Flag Reader {id}"
{{
    Subtype = Test;
    [Test] procedure Reads{id}() var S: Codeunit "Seq State"; N: Integer; begin if S.GetFlag() = 'never' then Error('no'); N := S.Bump(); end;
}}
"#
    )
}

fn flagged(rt: &CorpusRuntime, t: &str) -> bool {
    result(rt, t).assumptions.contains(&Assumption::StrictAfterRefused)
}

/// A test refused while it ran may have left state BC would not have: a later codeunit of its session whose reach can read
/// what the refused test could have changed is flagged. One that cannot, earlier ones and other sessions are not.
#[test]
fn only_codeunits_that_can_read_what_the_refused_test_changed_are_flagged_in_that_session() {
    let src = [
        lenient(60651),
        REFUSING.to_string(),
        flag_reader(60653),
        flag_reader(60654),
        lenient(60655),
    ]
    .concat();
    let rt = run(&src, &suites("A: 60651 60652 60653 60655\nB: 60654\n"));
    assert!(
        matches!(result(&rt, "IsRefusedWhileItRuns").outcome, Outcome::NotRun(_)),
        "{:?}",
        rt.results()
    );
    assert!(!flagged(&rt, "Lenient60651"), "before the refused test");
    assert!(!flagged(&rt, "PassesFirst"), "the codeunit of the refused test itself");
    assert!(flagged(&rt, "Reads60653"), "it reads the flag the refused test sets");
    assert_eq!(result(&rt, "Reads60653").outcome, Outcome::Pass);
    assert!(
        !flagged(&rt, "Lenient60655"),
        "it reads nothing the refused test could change"
    );
    assert!(!flagged(&rt, "Reads60654"), "another session");
}

/// What a flagged codeunit writes may be wrong too, so a later codeunit that reads it is flagged: the flag reader bumps the
/// count, and a codeunit that only bumps the count is flagged after it, though the refused test never touched the count.
#[test]
fn a_flagged_codeunit_taints_what_it_writes() {
    let bumper = r#"
codeunit 60662 "Seq Bumper"
{
    Subtype = Test;
    [Test] procedure OnlyBumps() var S: Codeunit "Seq State"; N: Integer; begin N := S.Bump(); end;
}
"#;
    let src = [
        REFUSING.to_string(),
        flag_reader(60653),
        bumper.to_string(),
        lenient(60663),
    ]
    .concat();
    let rt = run(&src, &sequential());
    assert!(flagged(&rt, "Reads60653"));
    assert!(
        flagged(&rt, "OnlyBumps"),
        "it reads the count the flagged codeunit wrote"
    );
    assert!(!flagged(&rt, "Lenient60663"));
}

/// A refused test whose reach is not known (a codeunit run by an id known only at run time) may have changed anything:
/// every later codeunit of its session is flagged.
#[test]
fn a_refused_test_with_an_unknown_reach_flags_every_later_codeunit() {
    let refusing = r#"
codeunit 60672 "Seq Unknown Reach"
{
    Subtype = Test;
    [Test] procedure RunsAnythingThenIsRefused() var Id: Integer; begin Id := 50000; if Id = 1 then Codeunit.Run(Id); Sleep(-1); end;
}
"#;
    let src = [refusing.to_string(), lenient(60673)].concat();
    let rt = run(&src, &sequential());
    assert!(matches!(
        result(&rt, "RunsAnythingThenIsRefused").outcome,
        Outcome::NotRun(_)
    ));
    assert!(flagged(&rt, "Lenient60673"));
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
    [Test] procedure RunsOutOfTime() var S: Codeunit "Seq State"; begin S.SetFlag('slow'); Sleep(5000); end;
}
codeunit 60672 "Seq After Slow"
{
    Subtype = Test;
    [Test] procedure RunsAfterTheTimeLimitStop() var S: Codeunit "Seq State"; begin if S.GetFlag() = 'never' then Error('no'); end;
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

/// A codeunit that is not run as a whole but could leave nothing in the database (its tests roll back, its OnRun cannot
/// write) does not hold back the codeunits after it: they run and are flagged, not refused.
#[test]
fn a_plain_codeunit_that_is_not_run_as_a_whole_flags_the_later_ones_but_does_not_refuse_them() {
    let src = r#"
codeunit 60681 "Seq Not Run"
{
    Subtype = Test;
    [Test] procedure RefusedAsAWhole() var N: BigInteger; begin N := NumberSequence.Next('SEQ'); Sleep(-1); end;
}
codeunit 60682 "Seq Reads Numbers After"
{
    Subtype = Test;
    [Test] procedure TakesANumberAfterTheRefusedCodeunit()
    var N: BigInteger;
    begin
        N := NumberSequence.Next('SEQ');
    end;
}
"#;
    let rt = run(src, &sequential());
    let after = result(&rt, "TakesANumberAfterTheRefusedCodeunit");
    assert_eq!(after.outcome, Outcome::Pass, "{:?}", after.outcome);
    assert!(after.assumptions.contains(&Assumption::StrictAfterRefused));
}

/// Review of the draft (finding 1): what ran before a test was refused ran on BC too, so the single instances move on
/// past the stop. Only the stopped codeunit's reach differs from BC's, and that is what the later codeunits are checked
/// against. The refused codeunit sets the flag only; the codeunit after it counts on the count the one before it bumped:
/// on BC it is the second bumper, and here it passes, unflagged, as it reads nothing the refused codeunit can change.
#[test]
fn the_single_instances_move_on_past_a_test_refused_while_it_ran() {
    let src = [ranked(60640, 1), REFUSING.to_string(), ranked(60695, 2)].concat();
    let rt = run(&src, &sequential());
    assert!(
        matches!(result(&rt, "IsRefusedWhileItRuns").outcome, Outcome::NotRun(_)),
        "{:?}",
        rt.results()
    );
    let after = result(&rt, "Rank60695");
    assert_eq!(after.outcome, Outcome::Pass, "{:?}", rt.results());
    assert!(
        !flagged(&rt, "Rank60695"),
        "it reads only the count, which the refused codeunit does not change"
    );
}

/// Review of the draft (finding 2): a later codeunit whose own reach is not known (a codeunit run by an id known only at
/// run time) may read anything the stopped codeunit changed, so it is flagged.
#[test]
fn a_later_codeunit_with_an_unknown_reach_is_flagged() {
    let reader = r#"
codeunit 60696 "Seq Unknown Reader"
{
    Subtype = Test;
    [Test] procedure ReadsThroughAnUnknownId() var Id: Integer; begin Id := 50000; if Id = 1 then Codeunit.Run(Id); end;
}
"#;
    let src = [REFUSING.to_string(), reader.to_string()].concat();
    let rt = run(&src, &sequential());
    assert_eq!(result(&rt, "ReadsThroughAnUnknownId").outcome, Outcome::Pass);
    assert!(flagged(&rt, "ReadsThroughAnUnknownId"));
}

/// A test codeunit of the hub leg (RequiredTestIsolation = Function) whose only test passes when it is the `rank`th that
/// bumped the shared count.
fn hub_ranked(id: u32, rank: u32) -> String {
    ranked(id, rank).replace(
        "    Subtype = Test;\n",
        "    Subtype = Test;\n    RequiredTestIsolation = Function;\n",
    )
}

/// Second review of the draft (finding A): a codeunit refused as a whole after its session started (here a Disabled
/// codeunit next to a Company Triggers subscriber that writes when the test session opens the company, `run.rs`) drops the
/// single instances it was handed. The codeunit after it starts with fresh ones, though on BC it sees what the codeunit
/// before set, so it must be flagged (or right).
#[test]
fn a_codeunit_refused_as_a_whole_after_its_session_started_flags_the_later_ones() {
    let writer = r#"
codeunit 60697 "Seq Company Open Writer"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure Writes()
    var R: Record "Seq Row"; N: Integer;
    begin
        if R.FindLast() then N := R."No.";
        R."No." := N + 1;
        R.Insert();
    end;
}
codeunit 60698 "Seq Disabled Refused"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure IsRefusedAsAWhole() begin end;
}
"#;
    let src = [hub_ranked(60690, 1), writer.to_string(), hub_ranked(60699, 2)].concat();
    let rt = run(
        &src,
        &RunOptions {
            required_isolation: true,
            ..sequential()
        },
    );
    assert!(
        matches!(&result(&rt, "IsRefusedAsAWhole").outcome, Outcome::NotRun(m) if m.contains("Company Triggers")),
        "{:?}",
        rt.results()
    );
    assert_eq!(result(&rt, "Rank60690").outcome, Outcome::Pass, "{:?}", rt.results());
    let after = result(&rt, "Rank60699");
    assert!(
        flagged(&rt, "Rank60699"),
        "the single instances were dropped, so it must be flagged: {:?}",
        after
    );
}
