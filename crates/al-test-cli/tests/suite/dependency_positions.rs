//! ADR-041: the routines of a dependency app carry source positions, so `SessionInformation.Callstack` names the line of
//! each of their frames (Microsoft's Error Message Management reads the call stack and every frame needs its line;
//! 197 Tests-Misc and 17 Tests-SCM-Service tests were refused with "a frame whose format is not pinned"). The text of a
//! frame is pinned by session/TestSessionInformationCallstackContracts.al:130-143 and
//! session/TestSessionCallstackSourcesContracts.al:Callstack_FromTablePageReportAndExtensionCode.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const DEP: &str = r#"
codeunit 50111 "Dep Cu"
{
    procedure Stack(): Text
    begin
        exit(SessionInformation.Callstack());
    end;
}
"#;

const MAIN: &str = r#"
codeunit 50100 "T"
{
    Subtype = Test;
    [Test]
    procedure DependencyFrameHasItsLine()
    var D: Codeunit "Dep Cu"; S: Text;
    begin
        S := D.Stack();
        if not S.StartsWith('"Dep Cu"(CodeUnit 50111).Stack line 2\') then Error('stack <%1>', S);
    end;
}
"#;

#[test]
fn a_frame_of_a_dependency_routine_names_its_line() {
    let app = |id: &str, deps: &[&str], file: &str, src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(file.into(), src.into())],
    };
    let (_, l) = al_sema::lower::compile_apps_with(
        vec![app("main", &["dep"], "main.al", MAIN), app("dep", &[], "dep.al", DEP)],
        vec![AppKind::Workspace, AppKind::Dependency],
        &["BC27PLUS".to_string()].into(),
        None,
        al_sema::check::Roots::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(std::sync::Arc::new((*l.program).clone()), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    let out = run_codeunit(&engine, &l, oi);
    assert!(out.iter().all(|(_, o)| *o == Outcome::Pass), "{out:?}");
}
