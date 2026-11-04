//! A Guid compared with a Text reads the text as a Guid; a text that is no Guid raises InvalidGuidFormat
//! (types/CxScalars.al:203-267, Guid_ComparedWithText and Guid_ComparedWithLiterals).

use al_builtins::native::Signal;

fn code(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

const PROCS: &str = "[TryFunction] local procedure TryEq(G: Guid; T: Text; var R: Boolean) begin R := G = T; end; \
    [TryFunction] local procedure TryRev(G: Guid; T: Text; var R: Boolean) begin R := T <> G; end; \
    local procedure Cmp(G: Guid; T: Text): Text var A: Boolean; B: Boolean; begin \
    if not TryEq(G, T, A) then exit(GetLastErrorCode()); if not TryRev(G, T, B) then exit('rev ' + GetLastErrorCode()); \
    exit(Format(A) + Format(B)); end;";

#[test]
fn a_guid_compared_with_a_text_reads_the_text_as_a_guid() {
    let src = code(&format!(
        "{PROCS} procedure F(): Text var G: Guid; N: Guid; begin Evaluate(G, '{{0A1B2C3D-4E5F-6A7B-8C9D-0E1F2A3B4C5D}}'); \
         exit(Cmp(G, '{{0A1B2C3D-4E5F-6A7B-8C9D-0E1F2A3B4C5D}}') + '|' + Cmp(G, '0a1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d') + '|' + \
         Cmp(G, '{{00000000-0000-0000-0000-000000000001}}') + '|' + Cmp(N, '') + '|' + Cmp(G, 'garbage') + '|' + Cmp(G, 'abc')); end;"
    ));
    assert_eq!(
        al_testkit::text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v)),
        "YesNo|YesNo|NoYes|InvalidGuidFormat|InvalidGuidFormat|InvalidGuidFormat"
    );
}

#[test]
fn a_guid_compared_with_a_text_of_an_unpinned_form_is_refused() {
    // The 32 digit form and parentheses are Guids for Evaluate, but no corpus test compares them with a Guid.
    let src = code(&format!(
        "{PROCS} procedure F(): Text var G: Guid; begin exit(Cmp(G, '00000000000000000000000000000000')); end;"
    ));
    match al_testkit::run_with(&src, "C", "F", vec![]) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Guid"), "{why}"),
        other => panic!("{other:?}"),
    }
}
