//! A procedure named `Do` (al-runner issue 7957). alc 18.0 compiles the declaration `local procedure Do()` and the
//! quoted call `"Do"();`, and rejects the unquoted call `Do();` with a syntax error (AL0104). The runner must give a
//! diagnostic for the unquoted call and never panic.
use super::*;

// The reporter's repro, unchanged.
const UNQUOTED: &str = "codeunit 50100 \"Do Panic Test\"
{
    Subtype = Test;

    [Test]
    procedure CallsDo()
    begin
        Do();
    end;

    local procedure Do()
    begin
    end;
}";
const QUOTED: &str =
    "codeunit 50100 \"Do Panic Test\" { procedure CallsDo() begin \"Do\"(); end; local procedure Do() begin end; }";

#[test]
fn the_unquoted_call_is_a_syntax_error_not_a_panic() {
    let ws = Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), UNQUOTED.into())],
        }],
        &Default::default(),
    );
    assert!(!ws.diagnostics.is_empty(), "the parser reports the unquoted call");
    // Lowering the broken body gives a refusal or a diagnostic; it does not panic.
    // A panic in lowering is caught and becomes an "internal error" refusal.
    let (_, l) = lower_pos(UNQUOTED);
    let reasons: Vec<&Refusal> = l.unsupported.iter().flatten().flatten().collect();
    assert!(!reasons.is_empty(), "the broken body is refused");
    assert!(!reasons.iter().any(|r| r.contains("internal error")), "{reasons:?}");
}

#[test]
fn the_declaration_and_the_quoted_call_compile() {
    let (ws, l) = lower_src(QUOTED);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    assert!(
        l.al_diagnostics.is_empty(),
        "{:?}",
        l.al_diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );
    assert!(format!("{:?}", l.unsupported) == "[[[], []]]", "{:?}", l.unsupported);
}
