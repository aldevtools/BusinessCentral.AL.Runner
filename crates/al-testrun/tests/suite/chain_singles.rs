//! ADR-066 amendment 8: the carry chain's walk keeps the SingleInstance state from member to member (serial, ADR-070).
//!
//! ADR-066 amendment 11, option 1: a member that is in the chain only because its tests take numbers
//! (`ChainWhy::TestsTakeNumbers`) runs with fresh single instances in the walk, as codeunits outside the chain do. Its
//! order dependence is flagged and run again in the lane as amendments 4, 5 and 7 say. A member whose OnRun takes the
//! number keeps amendment 8.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{ChainWhy, CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
codeunit 50261 SinglesAheadInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
codeunit 50262 "Singles Ahead Count"
{
    SingleInstance = true;
    procedure Bump() begin Count += 1; end;
    var Count: Integer;
}
codeunit 50263 "Singles Ahead Flag"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
"#;

/// Chain members: each takes a number of the install code's sequence (so it is in the carry chain and hands the number
/// on) and reads and writes the SingleInstance count. Each passes in any order. With `in_onrun` the OnRun trigger takes
/// the number (`ChainWhy::OnRunWritesState`: the walk keeps the single instances);
/// otherwise the test does (`ChainWhy::TestsTakeNumbers`: fresh single instances, amendment 11).
const MEMBERS: u32 = 40;

fn source(in_onrun: bool) -> String {
    (0..MEMBERS)
        .map(|i| {
            let id = 60260 + i;
            let (onrun, take) = if in_onrun {
                (
                    "trigger OnRun() var N: BigInteger; begin N := NumberSequence.Next('SHARED'); end;",
                    "",
                )
            } else {
                ("", "N := NumberSequence.Next('SHARED');")
            };
            format!(
                r#"
codeunit {id} "Singles Ahead {id}"
{{
    Subtype = Test;
    {onrun}
    [Test] procedure Bumps{id}()
    var I: Integer; N: BigInteger; W: BigInteger; S: Codeunit "Singles Ahead Count";
    begin
        {take}
        S.Bump();
    end;
}}
"#
            )
        })
        .collect()
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

#[test]
fn a_chain_whose_members_use_single_instance_state_keeps_it_from_member_to_member() {
    let rt = run(
        &source(true),
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    let chain = &rt.phases.chain;
    assert_eq!(
        chain.members.len(),
        MEMBERS as usize,
        "every codeunit's OnRun takes numbers"
    );
    assert!(
        chain.members.iter().all(|m| m.why == ChainWhy::OnRunWritesState),
        "{:?}",
        chain.members
    );
    assert_eq!(chain.singles_kept, MEMBERS as usize, "{:?}", chain);
}

/// ADR-066 amendment 11, option 1: members that are in the chain only for the numbers their tests take run with fresh
/// single instances although they read and write SingleInstance state. Each first access of the count is `Count += 1`, a write under amendment 3, R1, so no
/// member is order dependent: none is flagged and none runs again (the SingleInstance handling of codeunits outside
/// the chain, not amendment 8's).
#[test]
fn numbers_only_members_run_with_fresh_single_instances_although_they_read_single_instance_state() {
    let rt = run(
        &source(false),
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    let chain = &rt.phases.chain;
    assert_eq!(
        chain.members.len(),
        MEMBERS as usize,
        "every codeunit's tests take numbers"
    );
    assert!(
        chain.members.iter().all(|m| m.why == ChainWhy::TestsTakeNumbers),
        "{:?}",
        chain.members
    );
    assert_eq!(chain.single_lane.rerun, 0, "{:?}", chain.single_lane);
    let flagged = rt
        .results()
        .iter()
        .filter(|r| r.assumptions.contains(&Assumption::SingleInstanceOrder))
        .count();
    assert_eq!(flagged, 0, "`Count += 1` is a write first (amendment 3, R1)");
    assert_eq!(
        chain.singles_kept, 0,
        "no member kept the walk's single instances: {chain:?}"
    );
}

/// ADR-066 amendment 11, option 1, with amendment 4, decision 2: a numbers-only member that fails with fresh
/// SingleInstance state because it needs what an earlier member left (P7: codeunits/TestCuP7BReader.al:24-37 against
/// TestCuP7ASetter.al:24-58) runs again in the lane in BC's order, passes and is flagged.
#[test]
fn a_numbers_only_member_that_needs_an_earlier_members_single_instance_state_runs_again_and_passes_flagged() {
    let src = r#"
codeunit 60300 "Numbers Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag()
    var N: BigInteger; S: Codeunit "Singles Ahead Flag";
    begin
        N := NumberSequence.Next('SHARED');
        S.SetFlag('left');
    end;
}
codeunit 60301 "Numbers Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheFlag()
    var N: BigInteger; S: Codeunit "Singles Ahead Flag";
    begin
        N := NumberSequence.Next('SHARED');
        if N <> 1 then Error('number %1', N);
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;
    let rt = run(
        src,
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    let result = |t: &str| {
        rt.results()
            .iter()
            .find(|r| r.test == t)
            .unwrap_or_else(|| panic!("{t} ran"))
    };
    let chain = &rt.phases.chain;
    assert!(
        chain.members.iter().all(|m| m.why == ChainWhy::TestsTakeNumbers),
        "{:?}",
        chain.members
    );
    assert_eq!(result("ReadsTheFlag").outcome, Outcome::Pass, "{:?}", rt.results());
    assert!(
        result("ReadsTheFlag")
            .assumptions
            .contains(&Assumption::SingleInstanceOrder)
    );
    assert_eq!(chain.single_lane.rerun, 1, "{:?}", chain.single_lane);
    assert_eq!(result("SetsTheFlag").outcome, Outcome::Pass);
    assert!(
        !result("SetsTheFlag")
            .assumptions
            .contains(&Assumption::SingleInstanceOrder)
    );
}
