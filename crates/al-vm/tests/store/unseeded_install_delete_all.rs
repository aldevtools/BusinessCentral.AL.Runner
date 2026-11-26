//! A DeleteAll or Truncate that install code runs on a platform table a fresh tenant fills with rows nothing pins
//! (`al_builtins::platform::UNSEEDED_ROWS`: Power BI Blob, Power BI Default Selection). Only the test session's own
//! DeleteAll is pinned to empty it (al-language-onprem/metadata/PtOnPremTables.al:324 Y17_PowerBIBlob_BlobRoundTrip,
//! :365-385 Y40 for Power BI Default Selection): whether the demo rows exist yet when an install runs is not. So no
//! install shape empties the table, and every read in the tests refuses in every copy of the baseline: the captured one,
//! the one read back from its bytes and the thawed shared one (review of PR 8526).

use al_builtins::native::Signal;

const SRC: &str = r#"
table 2000000144 "Power BI Blob"
{
    DataPerCompany = false;
    fields { field(1; Id; Guid) { } field(2; Name; Text[100]) { } }
    keys { key(PK; Id) { } }
}
table 2000000145 "Power BI Default Selection"
{
    DataPerCompany = false;
    fields { field(1; Id; Guid) { } field(2; Context; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50990 "UE Code"
{
    procedure ClearBlob()
    var
        B: Record "Power BI Blob";
    begin
        B.Id := CreateGuid(); B.Name := 'UE Before'; B.Insert();
        B.DeleteAll();
        B.Id := CreateGuid(); B.Name := 'UE After'; B.Insert();
    end;

    procedure ClearSelection()
    var
        S: Record "Power BI Default Selection";
    begin
        S.Id := CreateGuid(); S.Context := 'UE BEFORE'; S.Insert();
        S.DeleteAll();
        S.Id := CreateGuid(); S.Context := 'UE AFTER'; S.Insert();
    end;

    procedure FilterBlob()
    var
        B: Record "Power BI Blob";
    begin
        B.Id := CreateGuid(); B.Name := 'UE Keep'; B.Insert();
        B.Id := CreateGuid(); B.Name := 'UE Drop'; B.Insert();
        B.SetRange(Name, 'UE Drop');
        B.DeleteAll();
    end;

    procedure TruncateBlob()
    var
        B: Record "Power BI Blob";
    begin
        B.Id := CreateGuid(); B.Name := 'UE Before'; B.Insert();
        B.Truncate();
        B.Id := CreateGuid(); B.Name := 'UE After'; B.Insert();
    end;

    procedure UndoBlob()
    var
        B: Record "Power BI Blob";
    begin
        if Codeunit.Run(Codeunit::"UE Clear And Fail") then
            Error('the clearing codeunit did not fail');
        B.Id := CreateGuid(); B.Name := 'UE After'; B.Insert();
    end;

    procedure BlobCount(): Text
    var
        B: Record "Power BI Blob";
    begin
        exit(Format(B.Count()));
    end;

    procedure BlobFindFirst(): Text
    var
        B: Record "Power BI Blob";
    begin
        B.FindFirst();
        exit(B.Name);
    end;

    procedure BlobIsEmpty(): Text
    var
        B: Record "Power BI Blob";
    begin
        exit(Format(B.IsEmpty()));
    end;

    procedure BlobGetMissing(): Text
    var
        B: Record "Power BI Blob";
    begin
        exit(Format(B.Get(CreateGuid())));
    end;

    procedure SelectionCount(): Text
    var
        S: Record "Power BI Default Selection";
    begin
        exit(Format(S.Count()));
    end;

    procedure SelectionFindFirst(): Text
    var
        S: Record "Power BI Default Selection";
    begin
        S.FindFirst();
        exit(S.Context);
    end;

    procedure SelectionIsEmpty(): Text
    var
        S: Record "Power BI Default Selection";
    begin
        exit(Format(S.IsEmpty()));
    end;

    procedure SelectionGetMissing(): Text
    var
        S: Record "Power BI Default Selection";
    begin
        exit(Format(S.Get(CreateGuid())));
    end;

    procedure SessionBlob(): Text
    var
        B: Record "Power BI Blob";
    begin
        B.DeleteAll();
        B.Id := CreateGuid(); B.Name := 'UE Session'; B.Insert();
        exit(Format(B.Count()));
    end;

    procedure SessionSelection(): Text
    var
        S: Record "Power BI Default Selection";
    begin
        S.DeleteAll();
        S.Id := CreateGuid(); S.Context := 'UE SESSION'; S.Insert();
        exit(Format(S.Count()));
    end;
}
codeunit 50991 "UE Clear And Fail"
{
    trigger OnRun()
    var
        B: Record "Power BI Blob";
    begin
        B.Id := CreateGuid(); B.Name := 'UE Rolled Back'; B.Insert();
        B.DeleteAll();
        Error('UE clearing fails');
    end;
}
"#;

const BLOB_READS: [&str; 4] = ["BlobCount", "BlobFindFirst", "BlobIsEmpty", "BlobGetMissing"];
const SELECTION_READS: [&str; 4] = [
    "SelectionCount",
    "SelectionFindFirst",
    "SelectionIsEmpty",
    "SelectionGetMissing",
];

/// The baseline that install routine `install` of "UE Code" leaves, as captured, read back from its bytes and thawed.
fn baseline_copies(install: &str) -> [(&'static str, al_vm::Baseline); 3] {
    let mut b = al_testkit::Built::from_source(SRC);
    b.session.set_install_scope(true);
    b.run_in("UE Code", install).unwrap();
    b.session.set_install_scope(false);
    let captured = b.session.capture_baseline();
    let read_back = al_vm::Baseline::from_bytes(&captured.to_bytes().expect("plain rows")).expect("the same layout");
    let thawed = captured.freeze().expect("shareable").thaw(None);
    [
        ("captured", captured),
        ("read back from its bytes", read_back),
        ("thawed", thawed),
    ]
}

/// Each read of `reads`, on each copy of the baseline `install` leaves, that answered instead of refusing.
fn answered(install: &str, reads: &[&str]) -> Vec<String> {
    let mut answered = Vec::new();
    for (copy, baseline) in baseline_copies(install) {
        for read in reads {
            let mut fresh = al_testkit::Built::from_source(SRC);
            fresh.session.restore_baseline(&baseline);
            let r = fresh.run_in("UE Code", read);
            if !matches!(r, Err(Signal::Unsupported(_))) {
                answered.push(format!("{copy} {read}: {r:?}"));
            }
        }
    }
    answered
}

#[test]
fn an_install_delete_all_on_power_bi_blob_leaves_every_read_refused_on_every_copy() {
    let answered = answered("ClearBlob", &BLOB_READS);
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}

#[test]
fn an_install_delete_all_on_power_bi_default_selection_leaves_every_read_refused_on_every_copy() {
    let answered = answered("ClearSelection", &SELECTION_READS);
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}

// Truncate deletes through the same store DeleteAll as an unfiltered DeleteAll, so it leaves the same mark.
#[test]
fn an_install_truncate_on_power_bi_blob_leaves_every_read_refused_on_every_copy() {
    let answered = answered("TruncateBlob", &BLOB_READS);
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}

// A DeleteAll with a filter marks nothing (al-data `MemStore::delete_all`).
#[test]
fn a_filtered_install_delete_all_leaves_every_read_refused_on_every_copy() {
    let answered = answered("FilterBlob", &BLOB_READS);
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}

// A failed Codeunit.Run takes back its writes, the DeleteAll's mark with them.
#[test]
fn an_install_delete_all_a_failed_codeunit_run_rolled_back_leaves_every_read_refused_on_every_copy() {
    let answered = answered("UndoBlob", &BLOB_READS);
    assert!(
        answered.is_empty(),
        "a read of the unseeded table must refuse: {answered:?}"
    );
}

// The pinned shape: the test session's own DeleteAll, then an Insert, answers on every copy, whatever the install did
// (PtOnPremTables.al:324-356, :365-385).
#[test]
fn a_session_delete_all_answers_on_every_copy_after_any_install_shape() {
    for install in ["ClearBlob", "ClearSelection", "TruncateBlob", "FilterBlob", "UndoBlob"] {
        for (copy, baseline) in baseline_copies(install) {
            for session in ["SessionBlob", "SessionSelection"] {
                let mut fresh = al_testkit::Built::from_source(SRC);
                fresh.session.restore_baseline(&baseline);
                assert_eq!(
                    al_testkit::text(fresh.run_in("UE Code", session)),
                    "1",
                    "{install}, {copy}, {session}"
                );
            }
        }
    }
}
