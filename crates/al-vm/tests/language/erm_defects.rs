//! Runtime defects found in Tests-ERM (issue 9). Each test says where the construct comes from.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 50900 "ALT T"
{
    fields { field(1; Id; Integer) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; Id) { } }
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{SRC}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect::<Vec<Vec<Option<String>>>>();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        return Err(Signal::unsupported(r.clone()));
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

/// Tests-ERM "Discount Setup Notifications" declares `GeneralPostingSetup: array[2] of Record "General
/// Posting Setup"` and calls `GeneralPostingSetup[1].FieldNo(..)` and `.Init()`. Every element is a
/// record variable of its own, like any record variable (collections/TestArray.al:19-29 pins arrays of
/// simple values).
#[test]
fn the_elements_of_an_array_of_records_are_record_variables() {
    let r = run(
        "procedure F(): Text var A: array[2] of Record \"ALT T\"; begin
            A[1].Init(); A[1].Id := 5; A[1].Descr := 'one'; A[1].Insert();
            A[2].Id := 6; A[2].Descr := 'two'; A[2].Insert();
            exit(Format(A[1].FieldNo(Descr)) + '|' + A[2].Descr + '|' + Format(A[1].Count()) + '|' + Format(A[1].Id)); end;",
    );
    assert_eq!(text(r), "2|two|2|5");
}

#[test]
fn two_elements_of_an_array_of_records_do_not_share_their_buffer() {
    let r = run(
        "procedure F(): Text var A: array[2] of Record \"ALT T\"; begin
            A[1].Id := 1; A[2].Id := 2; A[1].SetRange(Id, 1);
            exit(Format(A[1].Id) + '|' + Format(A[2].Id) + '|' + Format(A[1].HasFilter()) + '|' + Format(A[2].HasFilter())); end;",
    );
    assert_eq!(text(r), "1|2|Yes|No");
}

/// Base Application's `Company Information.IsDigit` does `Letter: Code[1]; Letter[1] := LetterChar`. A Code
/// upper-cases the character (types/TestCharIndexAssignContracts.al:CharacterAssignedIntoAPosition).
#[test]
fn assigning_a_character_into_a_code_upper_cases_it() {
    let r = run("procedure F(): Boolean var L: Code[1]; begin L[1] := 'a'; exit(L = 'A'); end;");
    assert!(matches!(&r, Ok(Value::Boolean(true))), "{r:?}");
}

/// Tests-ERM "ERM Copy Name To Ledg. Entries".AddBlankEntry opens a table that has no row in the demo
/// data, so `FindLast` is false and the FieldRef is never assigned, then calls `FieldRef.Value(..)`.
/// types/PSUnassignedFieldRef.al pins the read members as errors; assigning Value on one is not pinned, so it is
/// refused and says why.
#[test]
fn assigning_value_on_a_field_ref_that_was_never_assigned_is_refused_as_not_pinned() {
    let r = run("procedure F() var F: FieldRef; begin F.Value(1); end;");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("FieldRef.Value") && m.contains("never assigned") && m.contains("not pinned")),
        "{r:?}"
    );
}

/// types/PSUnassignedFieldRef.al UnassignedFieldRef_Members: reading Name on a never-assigned FieldRef raises.
#[test]
fn name_of_a_field_ref_that_was_never_assigned_raises_not_initialized() {
    let r = run("procedure F() var F: FieldRef; begin F.Name(); end;");
    assert!(
        matches!(&r, Err(Signal::Error(e)) if e.code == "NotInitialized" && e.message == "Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized."),
        "{r:?}"
    );
}

/// Base Application's `Error Message Handler.ShowNotification` does `Notification.SetData('RegisterID', RegisterID)`
/// with a Guid (27 Tests-ERM tests). The Guid is its text form (record/Y70Probes.al:264-343).
#[test]
fn notification_set_data_takes_a_guid_as_its_text_form() {
    let r = run(
        "procedure F(): Boolean var N: Notification; G: Guid; begin G := CreateGuid(); N.SetData('k', G); exit(N.GetData('k') = Format(G)); end;",
    );
    assert!(matches!(&r, Ok(Value::Boolean(true))), "{r:?}");
}

/// Evaluate into a Text takes the source as it is, an empty one too; into a Code it trims and upper-cases (corpus
/// recordref-fieldref/P128EvalBasic.al:P128_Evaluate_IntoText, P128_Evaluate_IntoCode). Purchase Prices.GetRecFilters
/// does `Evaluate(StartingDateFilter, Rec.GetFilter("Starting Date"))` into a Text (52 Tests-ERM tests).
#[test]
fn evaluate_into_text_and_code() {
    let r = run(
        "procedure F(): Text var T: Text; C: Code[5]; Ok: Boolean; begin T := 'KEEP'; Ok := Evaluate(T, ''); C := 'KEEP'; Ok := Ok and Evaluate(C, ' ab '); exit(Format(Ok) + '|[' + T + ']|[' + C + ']'); end;",
    );
    assert_eq!(text(r), "Yes|[]|[AB]");
}

/// recordref-fieldref/P135RecordIdTestField.al: TestField on a RecordId field fails while nothing was assigned
/// (Tests-ERM, 41 tests behind `Job Queue Entry.TestField("Record ID to Process")`-like checks) and passes for an assigned id,
/// the id of an initialized record included.
#[test]
fn test_field_of_a_record_id_field() {
    let src = r#"
table 50960 "TRI Row" { fields { field(1; Id; Integer) { } field(2; Ref; RecordId) { } } keys { key(PK; Id) { } } }
codeunit 50961 C
{
    procedure F(): Text
    var R: Record "TRI Row"; T: Text;
    begin
        R.Id := 1;
        asserterror R.TestField(Ref);
        T := GetLastErrorText();
        R.Ref := R.RecordId;
        R.TestField(Ref);
        exit(T);
    end;
}
"#;
    assert_eq!(
        text(al_testkit::run_in(src, "C", "F")),
        "Ref must have a value in TRI Row: Id=1. It cannot be zero or empty."
    );
}

/// recordref-fieldref/PnE08RecordMisc.al:TestField_Blob: a Blob with no content fails TestField with the plain text, with or
/// without CalcFields (noDataNoCalc, noDataCalc); one written through a stream and not yet saved passes (dataNoCalcPassed).
/// A Blob that holds content the database gave is not pinned: it passes only as a flagged assumption (Tests-ERM, 25 tests).
#[test]
fn test_field_of_a_blob_field() {
    let src = r#"
table 50970 "TBF Row" { fields { field(1; Id; Integer) { } field(2; Pic; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50971 C
{
    procedure F(): Text
    var R: Record "TBF Row"; T: Text; Out: OutStream;
    begin
        R.Id := 1;
        R.Insert();
        asserterror R.TestField(Pic);
        T := GetLastErrorText();
        R.CalcFields(Pic);
        asserterror R.TestField(Pic);
        T := T + '|' + GetLastErrorText();
        R.Pic.CreateOutStream(Out);
        Out.WriteText('x');
        R.TestField(Pic);
        exit(T + '|passed');
    end;
}
"#;
    let e = "Pic must have a value in TBF Row: Id=1. It cannot be zero or empty.";
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), format!("{e}|{e}|passed"));
}
