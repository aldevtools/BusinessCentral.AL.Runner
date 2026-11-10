// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

/// A test codeunit without handlers and without `TestPermissions = Disabled`: its one test passes when the call stack
/// names Microsoft's runner and fails with 'no runner frames' when it does not.
const PLAIN: &str = "
codeunit 60400 \"RF Plain\"
{
    Subtype = Test;

    [Test]
    procedure HasRunnerFrames()
    begin
        if StrPos(SessionInformation.Callstack, 'Test Runner by Microsoft') = 0 then
            Error('no runner frames');
    end;
}
";

/// The outcome of the test of [`PLAIN`] on BC `bc`, run on the TestRunnerHub leg (`hub_leg`) or under runner 130450.
fn outcome(bc: &str, hub_leg: bool) -> Outcome {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), PLAIN.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let engine = Engine::with_target(l.program.clone(), Vec::new().into_iter().collect(), Some(target));
    let object = l.program.objects.iter().position(|o| o.id == Some(60400)).unwrap();
    let mut scratch = Scratch {
        hub_leg,
        ..Default::default()
    };
    let mut out = run::run_codeunit_core(
        &engine,
        &l,
        object,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    );
    assert_eq!(out.len(), 1);
    out.remove(0).1
}

// Runner 130450 runs every codeunit that is not on the hub leg (codeunits/run-onrun/RqIsolationModes.al:9-12), and under it
// the six frames show on BC 28 and 29 too (session-system/callstack/CallstackRunnerFrames.al, the `Handler` cases).
#[test]
fn a_codeunit_under_runner_130450_shows_the_runner_frames_on_bc_28() {
    assert!(matches!(outcome("28.5", false), Outcome::Pass), "BC 28.5");
    assert!(matches!(outcome("29.0", false), Outcome::Pass), "BC 29.0");
}

// The hub adds no AL frame on BC 28 and later (CallstackRunnerFrames.al:17-20, the `Plain` cases: no handlers and
// TestPermissions = Disabled).
#[test]
fn a_codeunit_on_the_hub_leg_shows_no_runner_frames_from_bc_28() {
    assert!(matches!(outcome("28.5", true), Outcome::Fail(_)), "BC 28.5");
}

// BC 27 runs every codeunit under runner 130450 (RqIsolationModes.al:11).
#[test]
fn bc_27_shows_the_runner_frames() {
    assert!(matches!(outcome("27.5", false), Outcome::Pass));
}
