//! al-runner issue 6924: chain codeunits whose tests take numbers from a sequence of the install code hand those numbers
//! on, so each one starts where the one before it stopped. Helper threads run them ahead from a predicted start: the
//! chain's start moved on by the numbers the codeunits before it are predicted to take (`chain_walk.rs`). The walk keeps
//! such a run only when the actual start is that predicted start, so every outcome is the serial one. Here each
//! codeunit checks the exact numbers it gets, and one takes more numbers when its first number is odd, so a prediction
//! made from the chain's start is wrong for it.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SerialWhy, SuiteRun};

const BASE: &str = r#"
codeunit 50241 SpecInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
"#;

/// The codeunit ids, in chain order.
const IDS: [u32; 10] = [60240, 60241, 60242, 60243, 60244, 60245, 60246, 60247, 60248, 60249];
/// The codeunit (index into `IDS`) whose count of numbers depends on the number it gets first.
const ODD: usize = 4;

/// The test codeunits, each checking the numbers the serial run gives it.
fn source() -> String {
    source_with(false)
}

/// [`source`]; with `counter`, each codeunit also checks and moves on a SingleInstance counter: codeunit `i` sees `i`
/// (ADR-066 amendment 1, P7: al-language/codeunits/TestCuP7BReader.al:24-37).
fn source_with(counter: bool) -> String {
    let mut next: i64 = 0;
    let mut out = String::new();
    if counter {
        out.push_str(
            "codeunit 60250 \"Spec Counter\" { SingleInstance = true; var N: Integer; \
             procedure Get(): Integer begin exit(N); end; procedure Bump() begin N += 1; end; }\n",
        );
    }
    for (i, id) in IDS.iter().enumerate() {
        let mut body = String::new();
        if counter {
            body.push_str(&format!(
                "if C.Get() <> {i} then Error('codeunit {id}: counter %1, expected {i}', C.Get()); C.Bump();\n"
            ));
        }
        if i == ODD {
            let first = next;
            body.push_str(&format!(
                "N := NumberSequence.Next('SHARED'); if N <> {first} then Error('codeunit {id}: expected {first}, got %1', N);\n\
                 if N mod 2 = 1 then begin N := NumberSequence.Next('SHARED'); N := NumberSequence.Next('SHARED'); end;\n"
            ));
            next += if first % 2 == 1 { 3 } else { 1 };
        } else {
            for _ in 0..(i % 3 + 1) {
                body.push_str(&format!(
                    "N := NumberSequence.Next('SHARED'); if N <> {next} then Error('codeunit {id}: expected {next}, got %1', N);\n"
                ));
                next += 1;
            }
        }
        out.push_str(&format!(
            r#"
codeunit {id} "Spec {id}"
{{
    Subtype = Test;
    [Test] procedure Takes{id}()
    var I: Integer; N: BigInteger; W: BigInteger;{counter_var}
    begin
        // Some work, so that the helper threads have time to run ahead of the walk.
        for I := 1 to 200000 do W += I;
        {body}
    end;
}}
"#,
            counter_var = if counter { " C: Codeunit \"Spec Counter\";" } else { "" }
        ));
    }
    out
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

/// The test threads the run uses (`AL_TEST_THREADS`, else one per core).
fn workers() -> usize {
    std::env::var("AL_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
}

#[test]
fn chain_codeunits_that_hand_on_numbers_run_ahead_from_a_predicted_start_with_the_serial_outcome() {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &source()), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    assert_eq!(rt.results().len(), IDS.len());
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), IDS.len(), "every codeunit takes numbers");
    assert!(chain.members.iter().all(|m| m.left), "every codeunit hands on numbers");
    if workers() > 2 {
        assert!(
            chain.members.iter().any(|m| m.ahead),
            "no chain codeunit ran ahead: {:?}",
            chain.members.iter().map(|m| (m.id, m.ahead)).collect::<Vec<_>>()
        );
    }
}

/// `--timings` says why each chain codeunit that ran on the walk's thread did not run ahead: one cause per serial
/// codeunit, the first from the chain's start (which the helpers do not run) among them.
#[test]
fn each_chain_codeunit_that_ran_serially_has_a_cause() {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &source()), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    let chain = &rt.phases.chain;
    let serial: Vec<u32> = chain.members.iter().filter(|m| !m.ahead).map(|m| m.id).collect();
    let caused: Vec<u32> = chain.not_ahead.iter().map(|m| m.id).collect();
    assert_eq!(caused, serial, "{:?}", chain.not_ahead);
    if workers() > 2 {
        assert_eq!(
            chain.not_ahead.first().map(|m| (m.id, m.why)),
            Some((chain.members[0].id, SerialWhy::Closed)),
            "{:?}",
            chain.not_ahead
        );
    }
    let lines = chain.lines();
    assert!(
        lines.iter().any(|l| l.starts_with("serial carry chain by cause: ")),
        "{lines:?}"
    );
}

/// Chain codeunits that also hand on SingleInstance state run ahead from a predicted SingleInstance state too, and the
/// walk runs the others from its own: none of them runs again after the parallel part for SingleInstance state.
#[test]
fn chain_codeunits_that_hand_on_single_instance_state_do_not_run_again_after_the_walk() {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &source_with(true)), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    assert_eq!(rt.results().len(), IDS.len());
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), IDS.len(), "every codeunit takes numbers");
    let again: Vec<u32> = chain.reruns.iter().map(|r| r.id).collect();
    assert!(again.is_empty(), "ran again after the walk: {again:?}");
}

/// A SingleInstance value al-runner does not carry (a temporary table with an AutoIncrement field that holds rows):
/// the chain codeunits after the one that filled it are refused where they read it, and their runs, given the very
/// same value as the serial state holds, are not run again after the walk.
#[test]
fn chain_codeunits_that_read_a_value_that_is_not_carried_are_refused_and_not_run_again() {
    let mut src = source_with(true);
    src.push_str(
        r#"
table 60251 "Spec Auto" { fields { field(1; "No"; Integer) { AutoIncrement = true; } } keys { key(PK; "No") { } } }
codeunit 60252 "Spec Holder"
{
    SingleInstance = true;
    var T: Record "Spec Auto" temporary;
    procedure Fill() begin T.Init(); T.Insert(); end;
    procedure Count(): Integer begin exit(T.Count()); end;
}
codeunit 60253 "Spec Fill"
{
    Subtype = Test;
    [Test] procedure FillsHolder() var H: Codeunit "Spec Holder"; N: BigInteger; begin N := NumberSequence.Next('SHARED'); H.Fill(); end;
}
"#,
    );
    // The codeunits after 60253 read it.
    for id in [60254u32, 60255, 60256] {
        src.push_str(&format!(
            "codeunit {id} \"Spec Read {id}\" {{ Subtype = Test; [Test] procedure Reads{id}() var H: Codeunit \"Spec Holder\"; \
             N: BigInteger; I: Integer; W: BigInteger; begin for I := 1 to 200000 do W += I; N := NumberSequence.Next('SHARED'); \
             if H.Count() <> 1 then Error('x'); end; }}\n"
        ));
    }
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    for r in rt.results() {
        if r.test.starts_with("Reads") {
            assert!(
                matches!(&r.outcome, Outcome::NotRun(why) if why.contains("AutoIncrement field")),
                "{}: {:?}",
                r.test,
                r.outcome
            );
        } else {
            assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
        }
    }
    let again: Vec<u32> = rt.phases.chain.reruns.iter().map(|r| r.id).collect();
    assert!(again.is_empty(), "ran again after the walk: {again:?}");
}

/// A record buffer a SingleInstance holds of a row the first chain codeunit wrote (rolled back with that codeunit, so
/// its fields are carried without its row version): the later codeunits, given that very value, read its fields and
/// are not run again after the walk, although their first read encodes the buffer otherwise.
#[test]
fn chain_codeunits_given_the_serial_record_buffer_are_not_run_again() {
    let mut src = source_with(true);
    src.push_str(
        r#"
table 60257 "Spec Row" { fields { field(1; "No"; Code[20]) { } field(2; Qty; Integer) { } } keys { key(PK; "No") { } } }
codeunit 60258 "Spec Row Holder"
{
    SingleInstance = true;
    var R: Record "Spec Row";
    procedure Fill() begin R."No" := 'A'; R.Qty := 7; R.Insert(); end;
    procedure Qty(): Integer begin exit(R.Qty); end;
}
codeunit 60259 "Spec Row Fill"
{
    Subtype = Test;
    [Test] procedure FillsRow() var H: Codeunit "Spec Row Holder"; N: BigInteger; begin N := NumberSequence.Next('SHARED'); H.Fill(); end;
}
"#,
    );
    for id in [60260u32, 60261, 60262] {
        src.push_str(&format!(
            "codeunit {id} \"Spec Row Read {id}\" {{ Subtype = Test; [Test] procedure ReadsRow{id}() var H: Codeunit \"Spec Row Holder\"; \
             N: BigInteger; I: Integer; W: BigInteger; begin for I := 1 to 200000 do W += I; N := NumberSequence.Next('SHARED'); \
             if H.Qty() <> 7 then Error('qty %1', H.Qty()); end; }}\n"
        ));
    }
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    let again: Vec<u32> = rt.phases.chain.reruns.iter().map(|r| r.id).collect();
    assert!(again.is_empty(), "ran again after the walk: {again:?}");
}
