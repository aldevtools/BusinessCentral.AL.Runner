//! ADR-066 amendment 3, R1 and R1b: which first access of a SingleInstance global counts as a read of the state an
//! earlier test codeunit left, and which as a write that replaces it.
//!
//! A write in place (`:=` into a record, `Clear`, `Get`, a `Find`, `List.Add`, a `var` argument) replaces the value, so
//! the codeunit does not depend on what it held before. Only a read of the value (a field, `Count`, a comparison) does.
//! A read that is directly followed by a write of the same global (a guard, then the refill) is noted as guarded, with
//! the fingerprint the refill left.

use al_testkit::Built;
use al_vm::single_track::{GlobalKey, SingleOut};

const SRC: &str = r#"
table 50460 "Si Track Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50461 "Si Track State"
{
    SingleInstance = true;

    procedure AssignRec(R: Record "Si Track Row") begin GlobalRec := R; end;
    procedure ClearRec() begin Clear(GlobalRec); end;
    procedure GetRec() begin if GlobalRec.Get(1) then; end;
    procedure FindRec() begin if GlobalRec.FindFirst() then; end;
    procedure AddToList() begin GlobalList.Add(1); end;
    procedure SetInDict() begin GlobalDict.Set('1', 'one'); end;
    procedure FillByVar() var H: Codeunit "Si Track Helper"; begin H.Fill(GlobalRec); end;
    procedure ReadField(): Integer begin exit(GlobalRec."No."); end;
    procedure ReadCount(): Integer begin exit(GlobalList.Count()); end;
    procedure GuardThenRefill(N: Integer)
    begin
        if GlobalRec."No." <> N then
            if GlobalRec.Get(N) then;
    end;
    procedure Stamp() begin if When = 0DT then When := CurrentDateTime(); end;
    procedure AreasFor(K: Text): Text
    begin
        if not GlobalDict.ContainsKey(K) then
            Populate(K);
        exit(GlobalDict.Get(K));
    end;
    local procedure Populate(K: Text)
    var R: Record "Si Track Row";
    begin
        if R.Get(1) then
            GlobalDict.Set(K, R.Name)
        else
            GlobalDict.Set(K, '-');
    end;
    procedure CaptionFor(No: Integer): Text
    begin
        if (GlobalRec."No." <> No) or
           (GlobalRec.Name = 'x')
        then
            if not GlobalRec.Get(No) then
                Clear(GlobalRec);
        exit(GlobalRec.Name);
    end;
    procedure Append() begin Log += 'x;'; end;
    procedure Toggle() begin Loaded := not Loaded; end;
    procedure GuardThenOtherThenRefill()
    begin
        if not Loaded then begin
            Value := 5;
            Loaded := true;
        end;
    end;

    var
        GlobalRec: Record "Si Track Row";
        GlobalList: List of [Integer];
        GlobalDict: Dictionary of [Text, Text];
        Loaded: Boolean;
        Value: Integer;
        Log: Text;
        When: DateTime;
}
codeunit 50462 "Si Track Helper"
{
    procedure Fill(var R: Record "Si Track Row") begin if R.Get(2) then; end;
}
codeunit 50463 "Si Track Driver"
{
    procedure Seed()
    var R: Record "Si Track Row";
    begin
        R."No." := 1; R.Name := 'one'; R.Insert();
        R."No." := 2; R.Name := 'two'; R.Insert();
    end;
    procedure Assign() var S: Codeunit "Si Track State"; R: Record "Si Track Row"; begin R."No." := 7; S.AssignRec(R); end;
    procedure ClearIt() var S: Codeunit "Si Track State"; begin S.ClearRec(); end;
    procedure GetIt() var S: Codeunit "Si Track State"; begin S.GetRec(); end;
    procedure FindIt() var S: Codeunit "Si Track State"; begin S.FindRec(); end;
    procedure AddIt() var S: Codeunit "Si Track State"; begin S.AddToList(); end;
    procedure SetIt() var S: Codeunit "Si Track State"; begin S.SetInDict(); end;
    procedure ByVar() var S: Codeunit "Si Track State"; begin S.FillByVar(); end;
    procedure ReadIt() var S: Codeunit "Si Track State"; begin S.ReadField(); end;
    procedure CountIt() var S: Codeunit "Si Track State"; begin S.ReadCount(); end;
    procedure Guard() var S: Codeunit "Si Track State"; begin S.GuardThenRefill(1); end;
    procedure AreasIt() var S: Codeunit "Si Track State"; begin S.AreasFor('User:me'); end;
    procedure CaptionIt() var S: Codeunit "Si Track State"; begin S.CaptionFor(2); end;
    procedure StampIt() var S: Codeunit "Si Track State"; begin S.Stamp(); end;
    procedure AppendIt() var S: Codeunit "Si Track State"; begin S.Append(); end;
    procedure ToggleIt() var S: Codeunit "Si Track State"; begin S.Toggle(); end;
    procedure GuardOther() var S: Codeunit "Si Track State"; begin S.GuardThenOtherThenRefill(); end;
}
"#;

/// Runs `routine` of the driver on a tracking session (after seeding two rows) and returns the notes.
fn notes(routine: &str) -> (SingleOut, Built) {
    let mut b = Built::from_source(SRC);
    b.run_in("Si Track Driver", "Seed").expect("seeded");
    b.session.track_single_instances();
    b.run_in("Si Track Driver", routine).expect("ran");
    (b.session.single_out().expect("tracked"), b)
}

fn key(b: &Built, global: &str) -> GlobalKey {
    let oi = b
        .lowered
        .program
        .objects
        .iter()
        .position(|o| o.name == "Si Track State")
        .expect("the state codeunit");
    let g = b.lowered.program.objects[oi]
        .globals
        .iter()
        .position(|s| s.name == global)
        .expect("the global");
    (oi, g)
}

fn written_first(routine: &str, global: &str) {
    let (out, b) = notes(routine);
    let k = key(&b, global);
    assert!(
        !out.observed.contains_key(&k),
        "{routine}: a write in place is no read of the earlier state"
    );
    assert!(out.wrote.contains(&k), "{routine}: the codeunit left its own value");
}

#[test]
fn an_assignment_into_a_global_record_is_a_write() {
    written_first("Assign", "GlobalRec");
}

#[test]
fn clear_of_a_global_record_is_a_write() {
    written_first("ClearIt", "GlobalRec");
}

#[test]
fn get_into_a_global_record_is_a_write() {
    written_first("GetIt", "GlobalRec");
}

#[test]
fn find_into_a_global_record_is_a_write() {
    written_first("FindIt", "GlobalRec");
}

#[test]
fn list_add_on_a_global_list_is_a_write() {
    written_first("AddIt", "GlobalList");
}

#[test]
fn dictionary_set_on_a_global_dictionary_is_a_write() {
    written_first("SetIt", "GlobalDict");
}

#[test]
fn a_global_passed_by_var_as_its_first_use_is_a_write() {
    written_first("ByVar", "GlobalRec");
}

#[test]
fn a_field_read_and_a_count_stay_reads() {
    let (out, b) = notes("ReadIt");
    assert!(out.observed.contains_key(&key(&b, "GlobalRec")));
    let (out, b) = notes("CountIt");
    assert!(out.observed.contains_key(&key(&b, "GlobalList")));
}

#[test]
fn a_guard_directly_followed_by_the_refill_is_guarded_with_the_refill_fingerprint() {
    let (out, b) = notes("Guard");
    let k = key(&b, "GlobalRec");
    assert!(out.observed.contains_key(&k), "the guard read it");
    assert!(out.guarded.contains(&k), "{:?}", out.guarded);
    let refill = out.refill.get(&k).expect("the refill's fingerprint");
    assert_ne!(
        Some(refill),
        out.observed.get(&k),
        "the refill left another value than the guard saw"
    );
    assert_eq!(Some(refill), out.ends.get(&k), "nothing changed it after the refill");
}

#[test]
fn a_guard_followed_by_a_write_of_another_global_is_not_guarded() {
    let (out, b) = notes("GuardOther");
    let k = key(&b, "Loaded");
    assert!(out.observed.contains_key(&k));
    assert!(
        !out.guarded.contains(&k),
        "another global was written between the guard and the refill"
    );
}

#[test]
fn an_append_or_a_toggle_uses_what_it_read_and_is_not_guarded() {
    // `Log += x` and `Loaded := not Loaded` write a value made from the one they read: no refill.
    let (out, b) = notes("AppendIt");
    let k = key(&b, "Log");
    assert!(
        out.observed.contains_key(&k) && !out.guarded.contains(&k),
        "{:?}",
        out.guarded
    );
    let (out, b) = notes("ToggleIt");
    let k = key(&b, "Loaded");
    assert!(
        out.observed.contains_key(&k) && !out.guarded.contains(&k),
        "{:?}",
        out.guarded
    );
}

#[test]
fn an_assignment_of_a_value_made_without_the_global_after_a_guard_is_the_refill() {
    // Get Shortcut Dimension Values.GetGLSetup (Base Application, BC 29): `if WhenGotGLSetup = 0DT then
    // WhenGotGLSetup := CurrentDateTime`.
    let (out, b) = notes("StampIt");
    let k = key(&b, "When");
    assert!(
        out.observed.contains_key(&k) && out.guarded.contains(&k),
        "{:?}",
        out.guarded
    );
}

#[test]
fn a_dictionary_checked_then_filled_is_not_guarded() {
    // `Set` keeps the other entries, which an earlier test codeunit may have left and this one may read (corpus
    // al-language-xhandler-lib/XhLog.al, read by BiTestHandlerCrossApp.al:87-103): no refill.
    let (out, b) = notes("AreasIt");
    let k = key(&b, "GlobalDict");
    assert!(
        out.observed.contains_key(&k) && !out.guarded.contains(&k),
        "{:?}",
        out.guarded
    );
}

#[test]
fn a_two_field_guard_then_get_or_clear_is_guarded() {
    // Purch. Line CaptionClass Mgmt.GetPurchaseLineCaptionClass (Base Application, BC 29).
    let (out, b) = notes("CaptionIt");
    let k = key(&b, "GlobalRec");
    assert!(
        out.observed.contains_key(&k) && out.guarded.contains(&k),
        "{:?}",
        out.guarded
    );
}

/// ADR-066 amendment 7, option 1: a first read notes the test it came in (0: before the first test, in OnRun), so the
/// lane can tell whether a failed test, or one before it, read a global an earlier codeunit left otherwise.
#[test]
fn a_first_read_notes_the_test_it_came_in() {
    let mut b = Built::from_source(SRC);
    b.run_in("Si Track Driver", "Seed").expect("seeded");
    b.session.track_single_instances();
    b.session.single_test_begins();
    b.run_in("Si Track Driver", "ReadIt").expect("ran");
    b.session.single_test_begins();
    b.run_in("Si Track Driver", "CountIt").expect("ran");
    // A second read of a global already read counts for the first test only.
    b.run_in("Si Track Driver", "ReadIt").expect("ran");
    let out = b.session.single_out().expect("tracked");
    let (rec, list) = (key(&b, "GlobalRec"), key(&b, "GlobalList"));
    assert_eq!(out.read_in.get(&rec), Some(&1), "{:?}", out.read_in);
    assert_eq!(out.read_in.get(&list), Some(&2), "{:?}", out.read_in);
}
