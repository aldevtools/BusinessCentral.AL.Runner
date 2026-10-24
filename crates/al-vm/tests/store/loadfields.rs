//! Partial records: `SetLoadFields`, `AddLoadFields`, `AreFieldsLoaded`. The only thing AL can
//! observe is `AreFieldsLoaded` (a read of an omitted field loads it), so these tests assert it.
//! Each test names the corpus test that pins what it asserts.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 50800 "LF Keyed"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; Code; Code[20]) { }
        field(4; Amount; Decimal) { }
        field(5; Note; Text[50]) { }
        field(6; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("LF Keyed"); }
    }
    keys { key(PK; "Entry No.") { } key(Key2; Amount) { } }
}
"#;

fn text(vars: &str, body: &str) -> Result<String, Signal> {
    let src = format!(
        "{TABLES}codeunit 50100 C {{ procedure F(): Text var Rec: Record \"LF Keyed\"; a: Text; b: Boolean; {vars} begin {body} end; }}"
    );
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.routines.iter().any(|r| r.name == "F"))
        .expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![])?.0 {
        Value::Text(t) => Ok(t.to_string()),
        other => panic!("{other:?}"),
    }
}

/// Seeds row 1 (Name alpha, Code A1, Amount 75, Note n1) and row 2 (beta, B2, 5, n2).
const SEED: &str = "Rec.Init(); Rec.\"Entry No.\" := 1; Rec.Name := 'alpha'; Rec.Code := 'A1'; Rec.Amount := 75; Rec.Note := 'n1'; Rec.Insert();
    Rec.\"Entry No.\" := 2; Rec.Name := 'beta'; Rec.Code := 'B2'; Rec.Amount := 5; Rec.Note := 'n2'; Rec.Insert();
    Clear(Rec);";

fn yn(b: &str) -> String {
    format!("a += Format({b});")
}

#[test]
fn set_load_fields_without_arguments_resets_to_a_full_load() {
    // record/TestRecordPartialLoad.al:61-99 (PartialLoad_SetLoadFieldsNoArgs_ResetsToFullLoad).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount); Rec.Get(1); {} Rec.SetLoadFields(); Rec.Get(1); {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Note)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)"),
        yn("Rec.Note = 'n1'")
    );
    assert_eq!(text("", &body).unwrap(), "NoYesYes");
}

#[test]
fn are_fields_loaded_is_false_before_any_fetch() {
    // record/TestRecordPartialLoad.al:102-141 (PartialLoad_AreFieldsLoaded_BeforeFetch_ReturnsFalse).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount); {} {} Rec.Get(2); {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.\"Entry No.\")"),
        yn("Rec.AreFieldsLoaded(Rec.Amount)")
    );
    assert_eq!(text("", &body).unwrap(), "NoNoYes");
}

#[test]
fn reading_an_omitted_field_loads_its_real_value() {
    // record/TestRecordPartialLoad.al:144-190 (PartialLoad_ReadOmittedField_JitLoadsRealValue); the read loads every field
    // (record/lt-loadfields-pagesource-validate/P81LoadFields.al:1047-1120 RN_JitRead_UnloadedField_LoadsWhich).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount); Rec.Get(1); {} a += Rec.Name; {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "NoalphaYesYes");
}

#[test]
fn the_fields_of_the_current_key_are_loaded_too() {
    // record/TestRecordPartialLoad.al:193-232 (PartialLoad_CurrentKeyFields_AreForceLoaded).
    let body = format!(
        "{SEED} Rec.SetCurrentKey(\"Entry No.\"); Rec.SetLoadFields(Rec.Name); Rec.FindFirst(); {}
         Clear(Rec); Rec.SetCurrentKey(Amount); Rec.SetLoadFields(Rec.Name); Rec.FindFirst(); {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.Amount")
    );
    assert_eq!(text("", &body).unwrap(), "NoYes5");
}

#[test]
fn the_fields_of_an_active_filter_are_loaded_too() {
    // record/TestRecordPartialLoad.al:235-285 (PartialLoad_FilteredFields_AreForceLoaded).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Name); Rec.SetRange(\"Entry No.\", 2); Rec.FindFirst(); {}
         Clear(Rec); Rec.SetLoadFields(Rec.Name); Rec.SetRange(Code, 'B2'); Rec.FindFirst(); {} a += Rec.Code; exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Code)"),
        yn("Rec.AreFieldsLoaded(Rec.Code)")
    );
    assert_eq!(text("", &body).unwrap(), "NoYesB2");
}

#[test]
fn narrowing_after_a_full_fetch_keeps_the_buffer_in_hand() {
    // record/TestRecordPartialLoadNarrowAfterFetch.al:56-85 (..._AfterFetch_NoRefetch_ReportsBufferInHand).
    let body = format!(
        "{SEED} Rec.Get(1); {} Rec.SetLoadFields(Rec.Amount); {} a += Rec.Note; exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Note)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "YesYesn1");
}

#[test]
fn widening_without_a_fetch_loads_nothing() {
    // record/TestRecordPartialLoadNarrowAfterFetch.al:88-118 (..._AfterNarrowFetch_WidenNoRefetch_StaysUnloaded).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount); Rec.Get(1); {} Rec.SetLoadFields(); {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Note)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "NoNo");
}

#[test]
fn a_refetch_after_narrowing_keeps_what_a_full_fetch_loaded() {
    // record/TestRecordPartialLoadNarrowAfterFetch.al:121-185 (..._AfterFetch_WithRefetch_KeepsTheFieldLoaded),
    // including the negative half on a row nobody fetched wide.
    let body = format!(
        "{SEED} Rec.Get(1); Rec.SetLoadFields(Rec.Amount); Rec.Get(1); {} a += Rec.Note;
         Clear(Rec); Rec.SetLoadFields(Rec.Amount); Rec.Get(2); {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Note)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "Yesn1No");
}

#[test]
fn are_fields_loaded_takes_several_fields_and_needs_all_of_them() {
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount, Rec.Name); Rec.Get(1); {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Amount, Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Amount, Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "YesNo");
}

#[test]
fn add_load_fields_extends_a_partial_load_for_the_next_fetch() {
    // record/TestRecordExtended.al:44-65 only call it; the documented effect is that the next
    // fetch also loads the added fields (probes-needed: the return value).
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Amount); Rec.AddLoadFields(Rec.Name); Rec.Get(1); {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "YesNo");
}

#[test]
fn add_load_fields_on_a_full_load_does_nothing_and_answers_false() {
    // record/TestRecordExtended.al:44-65 (callable; the result is not asserted).
    let body = format!(
        "{SEED} b := Rec.AddLoadFields(Rec.Name); a := Format(b); Rec.Get(1); {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Note)")
    );
    assert_eq!(text("", &body).unwrap(), "NoYes");
}

#[test]
fn unpinned_cases_are_refused() {
    // A primary key field under a secondary current key, and SetBaseLoadFields are not pinned.
    let r = text(
        "",
        &format!(
            "{SEED} Rec.SetCurrentKey(Amount); Rec.SetLoadFields(Rec.Name); Rec.FindFirst(); exit(Format(Rec.AreFieldsLoaded(Rec.\"Entry No.\")));"
        ),
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
    // A record filled by assignment and never fetched reports its fields loaded
    // (record/lt-are-fields-loaded-unfetched/AreFieldsLoadedUnfetched.al:42-68).
    assert_eq!(
        text(
            "",
            "Rec.\"Entry No.\" := 9; exit(Format(Rec.AreFieldsLoaded(Rec.\"Entry No.\")));"
        )
        .unwrap(),
        "Yes"
    );
    let r = text(
        "",
        &format!("{SEED} Rec.SetLoadFields(Rec.Name); exit(Format(Rec.SetBaseLoadFields()));"),
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn set_base_load_fields_on_a_full_load_is_callable_and_answers_false() {
    // record/TestRecordExtended.al:71-80 (Record_SetBaseLoadFields_IsCallable); the result is
    // the test's comment, not an assertion (probes-needed).
    assert_eq!(text("", "exit(Format(Rec.SetBaseLoadFields()));").unwrap(), "No");
}

// ---------------------------------------------------------------- RecordRef

#[test]
fn a_record_ref_takes_field_numbers_and_its_reads_still_see_stored_values() {
    // record/TestRecordPartialLoad.al: a read of an omitted field returns its stored value, so a
    // RecordRef with a load set reads like one without (RecordRef has no AreFieldsLoaded here).
    let r = text(
        "RR: RecordRef;",
        &format!(
            "{SEED} RR.Open(50800); RR.SetLoadFields(2, 3); b := RR.AddLoadFields(4);
             RR.FindFirst(); exit(Format(RR.Field(1).Value) + '/' + Format(RR.Field(5).Value) + '/' + Format(b));"
        ),
    );
    // Without a request AddLoadFields adds nothing and answers false on a Record
    // (add_load_fields_on_a_full_load_does_nothing_and_answers_false); here a request exists.
    assert_eq!(r.unwrap(), "1/n1/Yes");
}

#[test]
fn a_record_ref_load_set_with_no_numbers_goes_back_to_a_full_load() {
    // record/TestRecordPartialLoad.al:61-99 for a Record; the RecordRef form is the same call.
    let r = text(
        "RR: RecordRef;",
        &format!(
            "{SEED} RR.Open(50800); RR.SetLoadFields(2); RR.SetLoadFields(); RR.FindFirst(); exit(Format(RR.Field(5).Value));"
        ),
    );
    assert_eq!(r.unwrap(), "n1");
}

#[test]
fn a_field_number_the_table_lacks_is_an_error_with_no_pinned_text() {
    // record/TestRecordPartialLoad.al:PartialLoad_SetLoadFields_UnknownFieldNo_Throws pins that
    // the call fails and that the text names the number; the text is not pinned.
    let r = text(
        "RR: RecordRef;",
        "RR.Open(50800); RR.SetLoadFields(9999); exit('no error');",
    );
    match r {
        Err(Signal::Error(e)) => assert_eq!(e.message, al_builtins::native::UNPINNED_ERROR),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_member_of_the_option_a_call_answers_names_its_value() {
    // Base Application "Find Record Management": `RecRef.SecurityFiltering(RecRef.SecurityFiltering::Ignored)`
    // (reached by the report 91 tests, handlers/TestPrecompiledOptionCaption.al).
    let r = text(
        "RR: RecordRef; X: SecurityFilter;",
        "RR.Open(50800); X := RR.SecurityFiltering; RR.SecurityFiltering(RR.SecurityFiltering::Ignored);
         exit(Format(RR.SecurityFiltering = RR.SecurityFiltering::Ignored));",
    );
    assert_eq!(r.unwrap(), "Yes");
}

// ---------------------------------------------------------------- Record.LoadFields

#[test]
fn load_fields_on_a_record_never_fetched_does_not_read_the_row_and_answers_false() {
    // types/PYLoadFindThis.al (BC 27.5 and 29.0): the call does not read the row, even when the blank key has one;
    // the function form answers false. Base Application's "Service Document Archive Mgmt." calls it as a statement
    // before Get().
    let r = text(
        "",
        "Rec.Init(); Rec.\"Entry No.\" := 0; Rec.Name := 'zero'; Rec.Note := 'n0'; Rec.Insert(); Clear(Rec);
         Rec.LoadFields(Rec.Name); a := Rec.Name + '/' + Rec.Note + '/'; Clear(Rec); b := Rec.LoadFields(Rec.Name);
         exit(a + Format(b) + '/' + Format(Rec.AreFieldsLoaded(Rec.Name)));",
    );
    assert_eq!(r.unwrap(), "//No/No");
}

#[test]
fn load_fields_as_a_statement_without_a_row_raises_nothing() {
    // Base Application "Service Document Archive Mgmt.".GetNextOccurrenceNo calls
    // `ServiceHeaderArchive.LoadFields(...)` as a statement on a fresh record before SetRange and
    // FindLast, on every service document insert, where no archive row has the blank key: BC raises
    // nothing there. The record is left as it was (nothing fetched).
    let r = text(
        "",
        &format!(
            "{SEED} Rec.LoadFields(Rec.Name); a := Format(Rec.AreFieldsLoaded(Rec.Name)); Rec.FindLast(); exit(a + '/' + Rec.Name);"
        ),
    );
    assert_eq!(r.unwrap(), "No/beta");
}

#[test]
fn load_fields_with_its_result_used_on_a_fetched_record_is_refused() {
    // Pinned for a record never fetched (false); what BC answers for a fetched one is not
    // (docs/probes-needed.md). The statement form on a fetched record changes nothing.
    let missing = text("", "b := Rec.LoadFields(Rec.Name); exit(Format(b));");
    assert_eq!(missing.unwrap(), "No");
    let fetched = text(
        "",
        &format!("{SEED} Rec.Get(1); b := Rec.LoadFields(Rec.Name); exit('ran');"),
    );
    assert!(matches!(fetched, Err(Signal::Unsupported(_))), "{fetched:?}");
    let statement = text(
        "",
        &format!("{SEED} Rec.Get(1); Rec.LoadFields(Rec.Name); exit(Rec.Name);"),
    );
    assert_eq!(statement.unwrap(), "alpha");
}

#[test]
fn set_load_fields_with_a_flow_field_loads_it_and_the_named_fields() {
    // record/lt-priority-assumptions/PriorityAssumptions.al:282-312 (SetLoadFields_FlowFieldAndNormal, real BC 29): the
    // FlowField reads as loaded, the key too, and the other stored fields are not.
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Cnt); Rec.FindFirst(); {} {} {} {} Clear(Rec); Rec.SetLoadFields(Rec.Name, Rec.Cnt); Rec.FindFirst(); {} {} {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.Cnt)"),
        yn("Rec.AreFieldsLoaded(Rec.\"Entry No.\")"),
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.Cnt)"),
        yn("Rec.AreFieldsLoaded(Rec.Name, Rec.Cnt)")
    );
    assert_eq!(text("", &body).unwrap(), "NoNoYesYesYesNoYesYes");
}

#[test]
fn add_load_fields_with_a_flow_field_is_an_error() {
    // record/lt-priority-assumptions/PriorityAssumptions.al:314-344 (AddLoadFields_FlowFieldAndNormal, real BC 29): the
    // text and the code `DB:Ok`, after a SetLoadFields and on a record without a request.
    let msg = "The field 'Cnt' in the 'LF Keyed' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields.";
    for body in [
        "Rec.SetLoadFields(Rec.Name); Rec.AddLoadFields(Rec.Cnt); exit('no error');",
        "Rec.AddLoadFields(Rec.Cnt); exit('no error');",
    ] {
        match text("", body) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.message, msg);
                assert_eq!(e.code, "DB:Ok");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn add_load_fields_of_a_stored_field_after_set_load_fields_loads_a_flow_field_too() {
    // PriorityAssumptions.al:314-344: Name and Qty requested, the FlowField reads as loaded all the same.
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Name); Rec.AddLoadFields(Rec.Amount); Rec.FindFirst(); {} {} {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Name)"),
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.Cnt)")
    );
    assert_eq!(text("", &body).unwrap(), "YesYesYes");
}

// ---- al-runner issue 5764 (record/lt-loadfields-pagesource-validate/P81LoadFields.al:1047-1243, writer RN) ----

const RN_SRC: &str = r#"
table 50810 "LF Row"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Integer) { }
        field(4; Notes; Blob) { }
        field(7; KindFilter; Code[10]) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Code) { } }
}
codeunit 50100 C
{
    local procedure Seed()
    var R: Record "LF Row";
    begin
        R.DeleteAll();
        R.Code := 'A'; R.Name := 'alpha'; R.Qty := 7; R.Insert();
        R.Code := 'B'; R.Name := 'beta'; R.Qty := 8; R.Insert();
    end;
    local procedure Stored(): Text
    var R: Record "LF Row";
    begin
        R.Get('A'); exit(R.Name + '|' + Format(R.Qty));
    end;
    local procedure L(var R: Record "LF Row"): Text
    begin
        exit(Format(R.AreFieldsLoaded(R.Name)) + Format(R.AreFieldsLoaded(R.Qty)) + Format(R.AreFieldsLoaded(R.Notes)) +
            Format(R.AreFieldsLoaded(R.KindFilter)) + Format(R.AreFieldsLoaded(R.SystemId)));
    end;
    procedure Jit(): Text
    var R: Record "LF Row"; K: Record "LF Row"; S: Text;
    begin
        Seed();
        R.SetLoadFields(Name); R.Get('A');
        S := L(R) + '|' + Format(R.Qty) + '|' + L(R);
        K.SetLoadFields(Code); K.FindFirst();
        S += '#' + L(K) + '|' + K.Name + '|' + L(K);
        K.Next();
        exit(S + '|' + L(K));
    end;
    procedure UnloadedChanged(): Text
    var P: Record "LF Row"; O: Record "LF Row"; S: Text;
    begin
        Seed();
        P.SetLoadFields(Name); P.Get('A');
        O.Get('A'); O.Qty := 99; O.Modify();
        P.Name := 'partial';
        S := Format(P.Modify()) + '|' + Stored() + '|' + Format(P.AreFieldsLoaded(P.Qty));
        exit(S + '|' + Format(P.Qty));
    end;
    procedure JitAfterChange(): Text
    var P: Record "LF Row"; O: Record "LF Row";
    begin
        Seed();
        P.SetLoadFields(Name); P.Get('A');
        O.Get('A'); O.Qty := 99; O.Modify();
        exit(Format(P.Qty));
    end;
    procedure AssignedUnloaded(): Text
    var P: Record "LF Row"; O: Record "LF Row";
    begin
        Seed();
        P.SetLoadFields(Name); P.Get('A');
        O.Get('A'); O.Name := 'other'; O.Modify();
        P.Qty := 5; P.Modify();
        exit(Stored() + '|' + P.Name);
    end;
    procedure SetResults(): Text
    var R: Record "LF Row"; S: Text;
    begin
        S := Format(R.SetLoadFields(KindFilter)) + Format(R.SetLoadFields(Name));
        exit(S + Format(R.AddLoadFields(Name)) + Format(R.AddLoadFields(KindFilter)) + Format(R.AddLoadFields(Qty)));
    end;
}
"#;

fn rn(routine: &str) -> String {
    al_testkit::text(al_testkit::run(RN_SRC, routine))
}

// RN_JitRead_UnloadedField_LoadsWhich: a read of one unloaded field loads every field; the field read joins the load set for
// the next row. A Blob, a FlowFilter and SystemId read as loaded after a partial fetch (:172-304).
#[test]
fn a_read_of_an_unloaded_field_loads_every_field_and_joins_the_load_set() {
    assert_eq!(
        rn("Jit"),
        "YesNoYesYesYes|7|YesYesYesYesYes#NoNoYesYesYes|alpha|YesYesYesYesYes|YesNoYesYesYes"
    );
}

// RN_StaleModify_UnloadedFieldChangedElsewhere, _JitReadAfterOtherChange, _UnloadedFieldSetHere_LoadedChangedElsewhere.
#[test]
fn modify_of_a_partial_record_keeps_a_field_never_loaded_or_assigned() {
    assert_eq!(rn("UnloadedChanged"), "Yes|partial|99|Yes|99");
    assert_eq!(rn("JitAfterChange"), "99");
    assert_eq!(rn("AssignedUnloaded"), "alpha|5|alpha");
}

// BR_SetLoadFields_ReturnValue_FlowFieldAndFlowFilter (:991-1028): SetLoadFields answers Yes; AddLoadFields answers No for a
// FlowFilter and for a field the request holds already; Yes for a new field (record/TestRecordExtended.al:58-65).
#[test]
fn set_and_add_load_fields_results() {
    assert_eq!(rn("SetResults"), "YesYesNoNoYes");
}

// misc/lt-probes-needed-2/PnBErrorsCollections.al:640-674 SetLoadFields_ThenRead_AndClear: the field read joins the load set
// for Next, and after Clear no field of the partial buffer is loaded.
#[test]
fn after_clear_a_partial_buffer_holds_no_field() {
    let body = format!(
        "{SEED} Rec.SetLoadFields(Rec.Name); Rec.FindFirst(); Rec.Amount := Rec.Amount; Rec.Next(); {} Clear(Rec); {} exit(a);",
        yn("Rec.AreFieldsLoaded(Rec.Amount)"),
        yn("Rec.AreFieldsLoaded(Rec.Name)")
    );
    assert_eq!(text("", &body).unwrap(), "YesNo");
}
