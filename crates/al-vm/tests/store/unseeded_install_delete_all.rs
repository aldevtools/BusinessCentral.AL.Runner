//! A DeleteAll that install code runs on a platform table a fresh tenant fills with rows nothing pins
//! (`al_builtins::platform::UNSEEDED_ROWS`). Only the test session's own DeleteAll is pinned to empty it
//! (al-language-onprem/metadata/PtOnPremTables.al:324, Y17_PowerBIBlob_BlobRoundTrip): whether the demo rows exist yet
//! when an install runs is not. So an install DeleteAll leaves the table unseeded, and a read in the tests is refused in
//! every copy of the baseline: the captured one, the one read back from its bytes and the thawed shared one.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 2000000144 "Power BI Blob"
{
    DataPerCompany = false;
    fields { field(1; Id; Guid) { } field(2; Name; Text[100]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50990 "UE Code"
{
    procedure Fill()
    var
        B: Record "Power BI Blob";
    begin
        B.Id := CreateGuid(); B.Insert();
        B.DeleteAll();
        B.Id := CreateGuid(); B.Insert();
    end;

    procedure CountRows(): Text
    var
        B: Record "Power BI Blob";
    begin
        exit(Format(B.Count()));
    end;
}
"#;

#[test]
fn an_install_delete_all_leaves_an_unseeded_platform_table_unseeded_in_every_baseline_copy() {
    let mut b = al_testkit::Built::from_source(SRC);
    b.session.set_install_scope(true);
    b.run_in("UE Code", "Fill").unwrap();
    b.session.set_install_scope(false);
    let captured = b.session.capture_baseline();
    let read_back = al_vm::Baseline::from_bytes(&captured.to_bytes().expect("plain rows")).expect("the same layout");
    let thawed = captured.freeze().expect("shareable").thaw(None);
    let mut answered = Vec::new();
    for (copy, baseline) in [
        ("captured", &captured),
        ("read back from its bytes", &read_back),
        ("thawed", &thawed),
    ] {
        let mut fresh = al_testkit::Built::from_source(SRC);
        fresh.session.restore_baseline(baseline);
        let count = fresh.run_in("UE Code", "CountRows");
        if !matches!(count, Err(Signal::Unsupported(_))) {
            answered.push(format!("{copy}: {count:?}"));
        }
        // The pinned shape still answers: the session's own DeleteAll, then an Insert (PtOnPremTables.al:324-346).
        fresh.session.restore_baseline(baseline);
        fresh.run_in("UE Code", "Fill").unwrap();
        assert_eq!(al_testkit::text(fresh.run_in("UE Code", "CountRows")), "1", "{copy}");
    }
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}
