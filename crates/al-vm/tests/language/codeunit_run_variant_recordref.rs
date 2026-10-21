//! `Codeunit.Run(Id, V)` of a codeunit with TableNo, where the Variant V holds a RecordRef. It compiles (a RecordRef
//! itself does not: must-not-compile 1786/1787), and Microsoft's Batch Processing Mgt. (codeunit 1380, InvokeProcessing)
//! relies on it. What Rec holds then, and whether a change OnRun makes reaches the RecordRef, is not pinned, so the run is
//! refused (docs/probes-needed.md). Before, the runtime ran OnRun on a blank Rec (Tests-ERM 139444 "Unhandled UI: Page 700").

use al_builtins::native::Signal;
use al_testkit::run;

const SRC: &str = r#"
table 50100 "Rep T"
{
    fields { field(1; K; Integer) { } field(2; D; Text[30]) { } }
    keys { key(PK; K) { } }
}
codeunit 50101 "Rep Run"
{
    TableNo = "Rep T";
    trigger OnRun()
    begin
        Rec.D := 'changed';
        Rec.Modify();
    end;
}
codeunit 50102 "Rep Caller"
{
    procedure RunWithVariantOfRecRef(): Text
    var
        T: Record "Rep T";
        RecRef: RecordRef;
        V: Variant;
    begin
        T.K := 2;
        T.D := 'two';
        T.Insert();
        RecRef.Open(Database::"Rep T");
        RecRef.FindLast();
        V := RecRef;
        Codeunit.Run(Codeunit::"Rep Run", V);
        T.Get(2);
        exit(T.D);
    end;
}
"#;

#[test]
fn codeunit_run_with_a_variant_holding_a_recordref_is_refused() {
    let r = run(SRC, "RunWithVariantOfRecRef");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref why)) if why.contains("RecordRef")),
        "{r:?}"
    );
}
