//! `CurrFieldNo := FieldNo(X)` in table code (Base Application's Sales Header `SelltoContactLookup` and Purchase
//! Header `BuyfromContactLookup` do it before they validate a field). The corpus pins the *reading* of CurrFieldNo
//! (testpage/TestPageCurrFieldNo_Tests.al); that an assignment is kept for the rest of the code on that record is not
//! pinned (docs/probes-needed.md).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!(
        "table 60990 \"CFN\" {{ fields {{ field(1; K; Code[10]) {{ }} field(2; V; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }}
           procedure Before(): Integer begin exit(CurrFieldNo); end;
           procedure Set(): Integer begin CurrFieldNo := FieldNo(V); exit(CurrFieldNo); end; }}
         codeunit 50100 C {{ procedure F(): Integer var R: Record CFN; begin {body} end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn an_assignment_to_curr_field_no_is_read_back_on_the_same_record_only() {
    // Set() assigns FieldNo(V) = 2 and reads it back; another record variable still reads 0.
    let v = run("R.K := 'A'; exit(R.Set() * 10 + R.Before());");
    assert!(matches!(v, Ok(Value::Integer(22))), "{v:?}");
    let v = run("R.K := 'A'; exit(R.Before());");
    assert!(matches!(v, Ok(Value::Integer(0))), "{v:?}");
}

/// The assignment belongs to the record variable, not to the memory address it happened to have: a later record
/// variable that gets the address of a dropped one reads 0 (corpus table-objects/P69CurrFieldNo.al,
/// CurrFieldNo_InValidateAndModify failed in a full run once the allocator reused an address).
#[test]
fn a_new_record_does_not_inherit_the_assignment_of_a_dropped_one() {
    let src =
        "table 60990 \"CFN\" { fields { field(1; K; Code[10]) { } field(2; V; Integer) { } } keys { key(PK; K) { } }
           procedure Before(): Integer begin exit(CurrFieldNo); end;
           procedure Set(): Integer begin CurrFieldNo := FieldNo(V); exit(CurrFieldNo); end; }
         codeunit 50100 C {
           procedure Assign() var R: Record CFN; begin R.K := 'A'; R.Set(); end;
           procedure Read(): Integer var R: Record CFN; begin R.K := 'B'; exit(R.Before()); end;
           procedure F(): Integer var I: Integer; begin
             for I := 1 to 200 do begin Assign(); if Read() <> 0 then exit(I); end;
             exit(0); end; }";
    let v = al_testkit::run_in(src, "C", "F");
    assert!(matches!(v, Ok(Value::Integer(0))), "{v:?}");
}
