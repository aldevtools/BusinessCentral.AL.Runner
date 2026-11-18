//! Record copies handed to other code (al-runner issues 8189 and 8185): the xRec a table procedure sees when it is called
//! on a record passed by `var` to a codeunit from a field's OnValidate, and the filters of a record passed by value. The
//! first test of each issue is the reporter's repro, unchanged.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `codeunit` in `src`, each rolled back on its own: its outcome (debug form) and whether it ran through
/// an assumption.
fn run(src: &str, codeunit: &str) -> Vec<(String, String, bool)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("test codeunit");
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Function,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    o.into_iter()
        .map(|(name, outcome)| {
            let flagged = hits
                .iter()
                .any(|(n, h): &(String, Vec<Assumption>)| *n == name && !h.is_empty());
            (name, format!("{outcome:?}"), flagged)
        })
        .collect()
}

/// al-runner issue 8189, the reporter's app (six files) as written.
const ISSUE_8189: &str = r##"
table 50100 "XRP Item"
{
    // The Value field is validated with a record that held 10 before the change. Its OnValidate reaches table
    // code in five ways and every way stores the Value that xRec holds at that point in its own field.
    // On BC all five fields are expected to be 10, the before-image of the trigger.

    Caption = 'XRP Item';
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20])
        {
            Caption = 'No.';
        }
        field(2; Value; Integer)
        {
            Caption = 'Value';

            trigger OnValidate()
            begin
                // Way 1: the trigger itself
                Rec."Seen In Trigger" := xRec.Value;

                // Way 2: a table procedure that the trigger calls as Rec.Proc() (AL Runner issue #7785)
                Rec.RecordSeenViaRecProc();

                // Way 3: Rec as a var parameter to a plain codeunit that calls a table procedure on it
                CallPlainCodeunit();

                // Way 4: Rec and xRec to a codeunit behind an interface; the codeunit calls a table procedure and ignores the xRec parameter
                CallInterfaceCodeunit();

                // Way 5: Rec and xRec to a codeunit that uses the xRec parameter
                CallCodeunitWithXRecParameter();
            end;
        }
        field(3; "Seen In Trigger"; Integer)
        {
            Caption = 'Seen In Trigger';
        }
        field(4; "Seen Via Rec Proc"; Integer)
        {
            Caption = 'Seen Via Rec Proc';
        }
        field(5; "Seen Via Codeunit"; Integer)
        {
            Caption = 'Seen Via Codeunit';
        }
        field(6; "Seen Via Interface"; Integer)
        {
            Caption = 'Seen Via Interface';
        }
        field(7; "Seen Via Codeunit Param"; Integer)
        {
            Caption = 'Seen Via Codeunit Param';
        }
    }

    keys
    {
        key(PK; "No.")
        {
            Clustered = true;
        }
    }

    internal procedure RecordSeenViaRecProc()
    begin
        Rec."Seen Via Rec Proc" := xRec.Value;
    end;

    internal procedure RecordSeenViaCodeunit()
    begin
        Rec."Seen Via Codeunit" := xRec.Value;
    end;

    internal procedure RecordSeenViaInterface()
    begin
        Rec."Seen Via Interface" := xRec.Value;
    end;

    local procedure CallPlainCodeunit()
    var
        PlainMover: Codeunit "XRP Plain Mover";
    begin
        PlainMover.HandleRecord(Rec);
    end;

    local procedure CallInterfaceCodeunit()
    var
        Mover: Interface "XRP IMover";
        InterfaceMover: Codeunit "XRP Interface Mover";
    begin
        Mover := InterfaceMover;
        Mover.Handle(Rec, xRec);
    end;

    local procedure CallCodeunitWithXRecParameter()
    var
        PlainMover: Codeunit "XRP Plain Mover";
    begin
        PlainMover.HandleRecordWithXRec(Rec, xRec);
    end;
}


interface "XRP IMover"
{
    procedure Handle(var Item: Record "XRP Item"; xItem: Record "XRP Item");
}


codeunit 50101 "XRP Interface Mover" implements "XRP IMover"
{
    procedure Handle(var Item: Record "XRP Item"; xItem: Record "XRP Item")
    begin
        // The xItem parameter is ignored on purpose: the table procedure reads xRec of the record that was passed as var
        Item.RecordSeenViaInterface();
    end;
}


codeunit 50100 "XRP Plain Mover"
{
    procedure HandleRecord(var Item: Record "XRP Item")
    begin
        Item.RecordSeenViaCodeunit();
    end;

    procedure HandleRecordWithXRec(var Item: Record "XRP Item"; xItem: Record "XRP Item")
    begin
        Item."Seen Via Codeunit Param" := xItem.Value;
    end;
}


codeunit 50102 "XRP XRec Test"
{
    // Measures xRec in table code that is reached from the OnValidate of "XRP Item".Value in five ways, once for a record that
    // was just inserted and once for a record that was read with Get. The record held 10 before Value is validated to 15.
    // Expected on BC: every way sees 10. Compare with AL Runner.

    Subtype = Test;

    var
        OldValue: Integer;
        NewValue: Integer;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure Trigger_AfterInsert_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterInsert(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Trigger itself', Item."Seen In Trigger");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure RecProc_AfterInsert_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterInsert(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Rec.Proc() from the trigger', Item."Seen Via Rec Proc");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure PlainCodeunit_AfterInsert_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterInsert(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Var parameter to a plain codeunit', Item."Seen Via Codeunit");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure Interface_AfterInsert_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterInsert(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Via an interface codeunit', Item."Seen Via Interface");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure XRecParameter_AfterInsert_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterInsert(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('xRec as a parameter', Item."Seen Via Codeunit Param");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure Trigger_AfterGet_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterGet(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Trigger itself', Item."Seen In Trigger");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure RecProc_AfterGet_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterGet(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Rec.Proc() from the trigger', Item."Seen Via Rec Proc");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure PlainCodeunit_AfterGet_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterGet(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Var parameter to a plain codeunit', Item."Seen Via Codeunit");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure Interface_AfterGet_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterGet(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('Via an interface codeunit', Item."Seen Via Interface");
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure XRecParameter_AfterGet_SeesBeforeImage()
    var
        Item: Record "XRP Item";
    begin
        PrepareAfterGet(Item);
        Item.Validate(Value, NewValue);
        CheckSeen('xRec as a parameter', Item."Seen Via Codeunit Param");
    end;

    local procedure PrepareAfterInsert(var Item: Record "XRP Item")
    begin
        OldValue := 10;
        NewValue := 15;

        Item.Init();
        Item."No." := 'T1';
        Item.Value := OldValue;
        Item.Insert(false);
    end;

    local procedure PrepareAfterGet(var Item: Record "XRP Item")
    begin
        PrepareAfterInsert(Item);

        Clear(Item);
        Item.Get('T1');
    end;

    local procedure CheckSeen(Way: Text; SeenValue: Integer)
    var
        WrongValueErr: Label '%1: xRec.Value was %2, expected %3.', Comment = '%1 = way, %2 = value seen, %3 = expected value';
    begin
        if SeenValue <> OldValue then
            Error(WrongValueErr, Way, SeenValue, OldValue);
    end;
}

"##;

#[test]
fn issue_8189_reporter_repro_sees_the_trigger_xrec_in_every_way() {
    let o = run(ISSUE_8189, "XRP XRec Test");
    assert_eq!(o.len(), 10);
    for (name, outcome, flagged) in &o {
        assert_eq!(outcome, "Pass", "{name}");
        // The OnValidate runs all five ways in every test; ways 3 and 4 run through the assumption.
        assert!(*flagged, "{name}");
    }
}

/// al-runner issue 8185, the reporter's app as written: the test ends with Error on purpose to print the values; on BC 28.5
/// its text is the one asserted here.
const ISSUE_8185: &str = r##"
table 50610 "Repro Filter Line"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Status; Option) { OptionMembers = Open,Posted; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 50611 "Repro ByValue Filters"
{
    Subtype = Test;

    [Test]
    procedure FiltersOfARecordPassedByValue()
    var
        Line: Record "Repro Filter Line";
    begin
        Line.DeleteAll();
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Status, Line.Status::Posted);
        Error('caller: {%1} | by value: {%2} | by var: {%3} | FindFirst by value: %4',
            Line.GetFilters(), FiltersByValue(Line), FiltersByVar(Line), FindFirstByValue(Line));
    end;

    local procedure FiltersByValue(Line: Record "Repro Filter Line"): Text
    begin
        exit(Line.GetFilters());
    end;

    local procedure FiltersByVar(var Line: Record "Repro Filter Line"): Text
    begin
        exit(Line.GetFilters());
    end;

    local procedure FindFirstByValue(Line: Record "Repro Filter Line"): Boolean
    begin
        exit(Line.FindFirst());
    end;
}
"##;

#[test]
fn issue_8185_reporter_repro_hands_a_by_value_record_no_filters() {
    let o = run(ISSUE_8185, "Repro ByValue Filters");
    assert_eq!(o.len(), 1);
    let (name, outcome, flagged) = &o[0];
    assert!(
        outcome
            .contains("caller: {Status: Posted} | by value: {} | by var: {Status: Posted} | FindFirst by value: Yes"),
        "{name}: {outcome}"
    );
    assert!(*flagged, "{name}");
}

/// Neighbors of the shape of issue 8189, one fact flipped each: `(test, outcome prefix, flagged)`.
const XREC_NEIGHBORS: &str = r##"
table 50120 "XN Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Value; Integer)
        {
            trigger OnValidate()
            begin
                case Mode of
                    1: Mover.ReadXRec(Rec);
                    2: Mover.WriteOnly(Rec);
                    3: CallMover();
                    4: CallMoverWriteOnly();
                    6: Mover.CopyXRec(Rec);
                end;
            end;
        }
        field(3; Seen; Integer) { }
        field(4; Mode; Integer) { }
        field(5; Other; Integer)
        {
            trigger OnValidate()
            begin
                Rec.Seen := xRec.Value;
            end;
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
    var
        Mover: Codeunit "XN Mover";

    trigger OnModify()
    begin
        case Mode of
            5: Mover.ValidateSelf(Rec);
            7: Mover.ValidateOther(Rec);
        end;
    end;

    procedure ReadXRec()
    begin
        Rec.Seen := xRec.Value;
    end;

    procedure WriteOnly()
    begin
        Rec.Seen := 99;
    end;

    procedure ValidateSelf()
    begin
        Rec.Validate(Seen, 1);
    end;

    procedure ValidateOther()
    begin
        Rec.Validate(Other, 1);
    end;

    procedure CopyXRec()
    var
        Other: Record "XN Item";
    begin
        Other.Copy(xRec);
    end;

    local procedure CallMover()
    begin
        Mover.ReadXRec(Rec);
    end;

    local procedure CallMoverWriteOnly()
    begin
        Mover.WriteOnly(Rec);
    end;
}

codeunit 50121 "XN Mover"
{
    procedure ReadXRec(var Item: Record "XN Item")
    begin
        Item.ReadXRec();
    end;

    procedure WriteOnly(var Item: Record "XN Item")
    begin
        Item.WriteOnly();
    end;

    procedure ValidateSelf(var Item: Record "XN Item")
    begin
        Item.ValidateSelf();
    end;

    procedure ValidateOther(var Item: Record "XN Item")
    begin
        Item.ValidateOther();
    end;

    procedure CopyXRec(var Item: Record "XN Item")
    begin
        Item.CopyXRec();
    end;
}

codeunit 50122 "XN Test"
{
    Subtype = Test;

    local procedure Stored(var Item: Record "XN Item"; Mode: Integer)
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Mode := Mode;
        Item.Insert(false);
    end;

    [Test]
    procedure DirectFromTrigger_ReadsXRec_Refused()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 1);
        Item.Validate(Value, 15);
    end;

    [Test]
    procedure DirectFromTrigger_NoXRecRead_Runs()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 2);
        Item.Validate(Value, 15);
        if Item.Seen <> 99 then
            Error('Seen %1', Item.Seen);
    end;

    [Test]
    procedure OnModify_ValidateInProc_Runs()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 5);
        Item.Value := 15;
        Item.Modify(true);
        if Item.Seen <> 1 then
            Error('Seen %1', Item.Seen);
    end;

    [Test]
    procedure OnModify_ValidateInProc_TriggerReadsXRec_Refused()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 7);
        Item.Value := 15;
        Item.Modify(true);
    end;

    [Test]
    procedure MeasuredChain_ReadByFindFirst_Refused()
    var
        Item: Record "XN Item";
        Found: Record "XN Item";
    begin
        Stored(Item, 3);
        Found.FindFirst();
        Found.Validate(Value, 15);
    end;

    [Test]
    procedure MeasuredChain_Temporary_Refused()
    var
        Item: Record "XN Item" temporary;
    begin
        Stored(Item, 3);
        Item.Validate(Value, 15);
    end;

    [Test]
    procedure DirectFromTrigger_CopyOfXRec_Refused()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 6);
        Item.Validate(Value, 15);
    end;

    [Test]
    procedure MeasuredChain_FreshRecord_Refused()
    var
        Item: Record "XN Item";
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Mode := 3;
        Item.Validate(Value, 15);
    end;

    [Test]
    procedure MeasuredChain_FieldRefValidate_Refused()
    var
        Item: Record "XN Item";
        RecRef: RecordRef;
    begin
        Stored(Item, 3);
        RecRef.GetTable(Item);
        RecRef.Field(2).Validate(15);
    end;

    [Test]
    procedure MeasuredChain_NoXRecRead_Runs()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 4);
        Item.Validate(Value, 15);
        if Item.Seen <> 99 then
            Error('Seen %1', Item.Seen);
    end;

    [Test]
    procedure NoRunningCall_PlainCall_BlankXRec()
    var
        Item: Record "XN Item";
        Mover: Codeunit "XN Mover";
    begin
        Stored(Item, 0);
        Mover.ReadXRec(Item);
        if Item.Seen <> 0 then
            Error('Seen %1', Item.Seen);
    end;
}
"##;

#[test]
fn issue_8189_neighbors_refuse_a_read_of_xrec_or_keep_main() {
    let o = run(XREC_NEIGHBORS, "XN Test");
    let want = [
        ("DirectFromTrigger_ReadsXRec_Refused", XREC_REFUSED, false),
        ("DirectFromTrigger_NoXRecRead_Runs", PASS, false),
        // The Validate runs; only a read of the xRec it hands on refuses.
        ("OnModify_ValidateInProc_Runs", PASS, false),
        ("OnModify_ValidateInProc_TriggerReadsXRec_Refused", XREC_REFUSED, false),
        ("DirectFromTrigger_CopyOfXRec_Refused", XREC_REFUSED, false),
        ("MeasuredChain_FreshRecord_Refused", XREC_REFUSED, false),
        ("MeasuredChain_FieldRefValidate_Refused", XREC_REFUSED, false),
        // The reporter measured a record read by Get or inserted by itself, both stored.
        ("MeasuredChain_ReadByFindFirst_Refused", XREC_REFUSED, false),
        ("MeasuredChain_Temporary_Refused", XREC_REFUSED, false),
        // The measured chain hands the trigger's xRec on whether the procedure reads it or not: flagged.
        ("MeasuredChain_NoXRecRead_Runs", PASS, true),
        ("NoRunningCall_PlainCall_BlankXRec", PASS, false),
    ];
    check(&o, &want);
}

/// Neighbors of the shape of issue 8189 in the buffer and in the object whose call runs: a Get of a row the open transaction
/// modified (after its Insert, or after a Commit), a running trigger of a table extension, and a table extension's code
/// calling another table extension's procedure.
const XREC_GET_AND_EXTENSIONS: &str = r##"
table 50140 "XG Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Value; Integer)
        {
            trigger OnValidate()
            begin
                case Mode of
                    1: CallMover();
                    2: Mover.CallExtProc(Rec);
                    3: Mover.CallExtBProc(Rec);
                end;
            end;
        }
        field(3; Mode; Integer) { }
        field(4; Seen; Integer) { }
        field(5; Calls; Integer) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
    var
        Mover: Codeunit "XG Mover";

    procedure BaseSeen()
    begin
        Rec.Seen := xRec.Value;
    end;

    local procedure CallMover()
    begin
        Mover.CallBaseProc(Rec);
    end;
}

tableextension 50141 "XG Item Ext" extends "XG Item"
{
    fields
    {
        field(50140; ExtTrig; Integer)
        {
            trigger OnValidate()
            var
                M: Codeunit "XG Mover";
            begin
                M.CallBaseProc(Rec);
            end;
        }
    }

    procedure ExtSeen()
    begin
        Rec.Seen := xRec.Value;
    end;
}

tableextension 50142 "XG Item Ext B" extends "XG Item"
{
    procedure ExtBCallsExtSeen()
    begin
        Rec.ExtSeen();
    end;
}

codeunit 50143 "XG Mover"
{
    procedure CallBaseProc(var Item: Record "XG Item")
    begin
        Item.BaseSeen();
    end;

    procedure CallExtProc(var Item: Record "XG Item")
    begin
        Item.ExtSeen();
    end;

    procedure CallExtBProc(var Item: Record "XG Item")
    begin
        Item.ExtBCallsExtSeen();
    end;
}

codeunit 50144 "XG Test"
{
    Subtype = Test;

    local procedure Stored(var Item: Record "XG Item"; Mode: Integer)
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Mode := Mode;
        Item.Insert(false);
    end;

    [Test]
    procedure MeasuredChain_GetOfRowInsertedInOpenTx_Flagged()
    var
        Item: Record "XG Item";
        Read: Record "XG Item";
    begin
        Stored(Item, 1);
        Read.Get('A');
        Read.Validate(Value, 15);
        if Read.Seen <> 10 then
            Error('Seen %1', Read.Seen);
    end;

    [Test]
    procedure MeasuredChain_GetOfRowModifiedInOpenTx_Refused()
    var
        Item: Record "XG Item";
        Other: Record "XG Item";
        Read: Record "XG Item";
    begin
        Stored(Item, 1);
        Commit();
        Other.Get('A');
        Other.Calls := 1;
        Other.Modify();
        Read.Get('A');
        Read.Validate(Value, 15);
    end;

    [Test]
    procedure MeasuredChain_GetOfRowInsertedThenModified_Refused()
    var
        Item: Record "XG Item";
        Other: Record "XG Item";
        Read: Record "XG Item";
    begin
        Stored(Item, 1);
        Other.Get('A');
        Other.Calls := 1;
        Other.Modify();
        Read.Get('A');
        Read.Validate(Value, 15);
    end;

    [Test]
    procedure ExtTrigger_Codeunit_BaseProc_ReadsXRec_Refused()
    var
        Item: Record "XG Item";
    begin
        Stored(Item, 0);
        Item.Validate(ExtTrig, 1);
    end;

    [Test]
    procedure BaseTrigger_Codeunit_ExtProc_ReadsXRec_Refused()
    var
        Item: Record "XG Item";
    begin
        Stored(Item, 2);
        Item.Validate(Value, 15);
    end;

    [Test]
    procedure ExtCode_CallsOtherExtProc_PlainCall()
    var
        Item: Record "XG Item";
    begin
        Stored(Item, 3);
        Item.Validate(Value, 15);
        if Item.Seen <> 0 then
            Error('Seen %1', Item.Seen);
    end;
}
"##;

#[test]
fn issue_8189_get_of_a_written_row_and_running_extension_triggers_refuse() {
    let o = run(XREC_GET_AND_EXTENSIONS, "XG Test");
    let want = [
        // The reporter's second buffer: a Get of the row the open transaction inserted, nothing written to it since.
        ("MeasuredChain_GetOfRowInsertedInOpenTx_Flagged", PASS, true),
        // A Get of a row the open transaction modified, after a Commit or after its Insert: not measured.
        ("MeasuredChain_GetOfRowModifiedInOpenTx_Refused", XREC_REFUSED, false),
        (
            "MeasuredChain_GetOfRowInsertedThenModified_Refused",
            XREC_REFUSED,
            false,
        ),
        // The running call is a trigger of a table extension, or the called procedure is one of a table extension.
        ("ExtTrigger_Codeunit_BaseProc_ReadsXRec_Refused", XREC_REFUSED, false),
        ("BaseTrigger_Codeunit_ExtProc_ReadsXRec_Refused", XREC_REFUSED, false),
        // A table extension's own code calling another table extension's procedure: a plain call, as on main.
        ("ExtCode_CallsOtherExtProc_PlainCall", PASS, false),
    ];
    check(&o, &want);
}

/// A test outcome that passes, and the start of each refusal reason the neighbor tests expect.
const PASS: &str = "Pass";
const XREC_REFUSED: &str = "xRec in a table procedure called through other code";
const VIEW_REFUSED: &str = "a record handed by value whose view the probe did not measure";

/// Asserts each `(name, outcome, flagged)` of `want` on the outcomes `o` of `run`: [`PASS`] is a pass, a refusal reason is
/// `NotRun` with a text that names it. Lists every test that differs.
fn check(o: &[(String, String, bool)], want: &[(&str, &str, bool)]) {
    let mut wrong = Vec::new();
    for &(name, outcome, flagged) in want {
        let got = o
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("no test {name}"));
        let ok = if outcome == PASS {
            got.1.starts_with(PASS)
        } else {
            got.1.starts_with("NotRun") && got.1.contains(outcome)
        };
        if !ok || got.2 != flagged {
            wrong.push(format!("{name}: {} (flagged {})", got.1, got.2));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The table's own OnInsert, OnModify and OnDelete call `Rec.Proc()` (Base Application's Sales Header OnInsert calls
/// Rec.SetSellToCustomerFromFilter, which validates a field): a plain call, whose xRec is blank, as on main.
const XREC_OWN_TRIGGERS: &str = r##"
table 50140 "XT Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Value; Integer) { }
        field(3; Seen; Integer) { }
        field(4; Mode; Integer) { }
        field(5; Other; Integer)
        {
            trigger OnValidate()
            begin
                Rec.Seen := xRec.Value + 100;
            end;
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }

    trigger OnInsert()
    begin
        Dispatch();
    end;

    trigger OnModify()
    begin
        Dispatch();
    end;

    trigger OnDelete()
    begin
        Dispatch();
        Log.Note(Rec.Seen);
    end;

    var
        Log: Codeunit "XT Log";

    local procedure Dispatch()
    begin
        case Mode of
            1: Rec.ReadXRec();
            2: Rec.ValidateOther();
        end;
    end;

    procedure ReadXRec()
    begin
        Rec.Seen := xRec.Value + 100;
    end;

    procedure ValidateOther()
    begin
        Rec.Validate(Other, 5);
    end;
}

codeunit 50141 "XT Log"
{
    SingleInstance = true;
    var
        Last: Integer;

    procedure Note(Seen: Integer)
    begin
        Last := Seen;
    end;

    procedure Seen(): Integer
    begin
        exit(Last);
    end;
}

codeunit 50142 "XT Test"
{
    Subtype = Test;

    local procedure Stored(var Item: Record "XT Item"; Mode: Integer)
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Insert(false);
        Item.Mode := Mode;
        Item.Value := 15;
    end;

    local procedure Expect(Seen: Integer)
    begin
        if Seen <> 100 then
            Error('Seen %1', Seen);
    end;

    [Test]
    procedure OnInsert_RecProc_ReadsXRec()
    var
        Item: Record "XT Item";
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Mode := 1;
        Item.Insert(true);
        Expect(Item.Seen);
    end;

    [Test]
    procedure OnInsert_RecProc_Validate()
    var
        Item: Record "XT Item";
    begin
        Item."No." := 'A';
        Item.Value := 10;
        Item.Mode := 2;
        Item.Insert(true);
        Expect(Item.Seen);
    end;

    [Test]
    procedure OnModify_RecProc_ReadsXRec()
    var
        Item: Record "XT Item";
    begin
        Stored(Item, 1);
        Item.Modify(true);
        Expect(Item.Seen);
    end;

    [Test]
    procedure OnModify_RecProc_Validate()
    var
        Item: Record "XT Item";
    begin
        Stored(Item, 2);
        Item.Modify(true);
        Expect(Item.Seen);
    end;

    [Test]
    procedure OnDelete_RecProc_ReadsXRec()
    var
        Item: Record "XT Item";
        Log: Codeunit "XT Log";
    begin
        Stored(Item, 1);
        Item.Delete(true);
        Expect(Log.Seen());
    end;

    [Test]
    procedure OnDelete_RecProc_Validate()
    var
        Item: Record "XT Item";
        Log: Codeunit "XT Log";
    begin
        Stored(Item, 2);
        Item.Delete(true);
        Expect(Log.Seen());
    end;
}
"##;

#[test]
fn a_tables_own_triggers_calling_rec_proc_keep_the_plain_xrec() {
    let o = run(XREC_OWN_TRIGGERS, "XT Test");
    let want = [
        ("OnInsert_RecProc_ReadsXRec", PASS, false),
        ("OnInsert_RecProc_Validate", PASS, false),
        ("OnModify_RecProc_ReadsXRec", PASS, false),
        ("OnModify_RecProc_Validate", PASS, false),
        ("OnDelete_RecProc_ReadsXRec", PASS, false),
        ("OnDelete_RecProc_Validate", PASS, false),
    ];
    check(&o, &want);
}

/// Neighbors of the shape of issue 8185, one fact flipped each.
const BY_VALUE_NEIGHBORS: &str = r##"
table 50130 "BN Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Status; Option) { OptionMembers = Open,Posted; }
        field(3; Name; Text[30]) { }
        field(4; NameFilter; Text[30]) { FieldClass = FlowFilter; }
        field(5; Named; Integer) { FieldClass = FlowField; CalcFormula = count("BN Row" where(Name = field(NameFilter))); }
        field(6; SameName; Integer) { FieldClass = FlowField; CalcFormula = count("BN Row" where(Name = field(Name))); }
    }
    keys { key(PK; "Code") { Clustered = true; } key(ByName; Name) { } }
}

codeunit 50132 "BN Holder"
{
    SingleInstance = true;
    var
        Kept: Record "BN Row";

    procedure Keep(Line: Record "BN Row")
    begin
        Kept.Copy(Line);
    end;

    procedure KeptFilters(): Text
    begin
        exit(Kept.GetFilters());
    end;
}

page 50133 "BN Rows"
{
    PageType = List;
    SourceTable = "BN Row";
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
}

codeunit 50131 "BN Test"
{
    Subtype = Test;

    local procedure Seed(var Line: Record "BN Row")
    begin
        Line.DeleteAll();
        Line.Code := 'A';
        Line.Name := 'z';
        Line.Insert();
        Line.Code := 'B';
        Line.Name := 'y';
        Line.Insert();
    end;

    local procedure Filters(Line: Record "BN Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    local procedure Key(Line: Record "BN Row"): Text
    begin
        exit(Line.CurrentKey());
    end;

    local procedure KeyAndFirst(Line: Record "BN Row"): Text
    begin
        Line.FindFirst();
        exit(Line.CurrentKey() + ' ' + Line.Code);
    end;

    local procedure FieldOnly(Line: Record "BN Row"): Text
    begin
        exit(Line.Code);
    end;

    local procedure ResetThenCount(Line: Record "BN Row"): Integer
    begin
        Line.Reset();
        exit(Line.Count());
    end;

    local procedure FiltersByVar(var Line: Record "BN Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    local procedure FindFirstPassedOn(Line: Record "BN Row"): Boolean
    begin
        exit(FindFirstHere(Line));
    end;

    local procedure FindFirstHere(Line: Record "BN Row"): Boolean
    begin
        exit(Line.FindFirst());
    end;

    local procedure TempFilters(Line: Record "BN Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    local procedure NoViewMethods(Line: Record "BN Row"): Text
    var
        Id: RecordId;
    begin
        Id := Line.RecordId();
        Line.SetLoadFields(Name);
        Line.Get('A');
        Line.Rename('C');
        exit(Format(Id) + Line.FieldCaption(Name) + Line.Code);
    end;

    local procedure NamedCount(Line: Record "BN Row"): Integer
    begin
        Line.CalcFields(Named);
        exit(Line.Named);
    end;

    local procedure CopyFrom(Line: Record "BN Row"): Integer
    var
        Other: Record "BN Row";
    begin
        Other.Copy(Line);
        exit(Other.Count());
    end;

    local procedure CopyThenModify(Line: Record "BN Row"): Boolean
    var
        Other: Record "BN Row";
    begin
        Other.Copy(Line);
        Other.Name := 'q';
        exit(Other.Modify());
    end;

    local procedure CopyFiltersFrom(Line: Record "BN Row"): Text
    var
        Other: Record "BN Row";
    begin
        Other.CopyFilters(Line);
        exit(Other.GetFilters());
    end;

    local procedure TransferFrom(Line: Record "BN Row"): Text
    var
        Other: Record "BN Row";
    begin
        Other.TransferFields(Line);
        exit(Other.Code);
    end;

    local procedure GetTableFilters(Line: Record "BN Row"): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(Line);
        exit(RecRef.GetFilters());
    end;

    local procedure RunPage(Line: Record "BN Row"): Boolean
    var
        Rows: TestPage "BN Rows";
    begin
        Rows.Trap();
        Page.Run(Page::"BN Rows", Line);
        exit(Rows.First());
    end;

    [Test]
    procedure NoFilter_AdHocKey_PrimaryKey()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(Status);
        if Key(Line) <> 'Code' then
            Error('key %1', Key(Line));
    end;

    [Test]
    procedure NoFilter_RowDependentFlowFieldKey_PrimaryKey()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(SameName);
        if KeyAndFirst(Line) <> 'Code A' then
            Error('%1', KeyAndFirst(Line));
    end;

    [Test]
    procedure NoFilter_DeclaredKey_PrimaryKey()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(Name);
        if Key(Line) <> 'Code' then
            Error('key %1', Key(Line));
    end;

    [Test]
    procedure FilterGroupThree_MethodsThatReadNoView_Run()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Status, Line.Status::Open);
        Line.FilterGroup(0);
        Line.Get('B');
        if NoViewMethods(Line) <> 'BN Row: BNameC' then
            Error('%1', NoViewMethods(Line));
    end;

    [Test]
    procedure FilterGroupThree_CalcFields_NoFlowFilter_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Status, Line.Status::Open);
        Line.FilterGroup(0);
        if NamedCount(Line) <> 2 then
            Error('%1', NamedCount(Line));
    end;

    [Test]
    procedure FilterGroupThree_CalcFields_FlowFilter_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetFilter(NameFilter, 'z');
        Line.FilterGroup(3);
        Line.SetRange(Status, Line.Status::Open);
        Line.FilterGroup(0);
        Error('%1', NamedCount(Line));
    end;

    [Test]
    procedure FilterGroupThree_CopyFrom_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        Error('%1', CopyFrom(Line));
    end;

    [Test]
    procedure FilterGroupThree_CopyThenModify_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        if not CopyThenModify(Line) then
            Error('not modified');
    end;

    [Test]
    procedure FilterGroupThree_CopyFiltersFrom_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        Error('%1', CopyFiltersFrom(Line));
    end;

    [Test]
    procedure FilterGroupThree_TransferFieldsFrom_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        if TransferFrom(Line) <> 'B' then
            Error('%1', TransferFrom(Line));
    end;

    [Test]
    procedure FilterGroupThree_GetTable_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        Error('%1', GetTableFilters(Line));
    end;

    [Test]
    procedure FilterGroupThree_PageRun_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        RunPage(Line);
    end;

    [Test]
    procedure GroupZero_PageRun_Flagged()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'Z');
        if not RunPage(Line) then
            Error('no row');
    end;

    [Test]
    procedure GroupZero_CopyKept_Flagged()
    var
        Line: Record "BN Row";
        Holder: Codeunit "BN Holder";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Holder.Keep(Line);
    end;

    [Test]
    procedure GroupZero_CopyKept_LaterRead_Flagged()
    var
        Holder: Codeunit "BN Holder";
    begin
        if Holder.KeptFilters() <> '' then
            Error('%1', Holder.KeptFilters());
    end;

    [Test]
    procedure GroupZero_FieldRead_NotFlagged()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        if FieldOnly(Line) <> 'B' then
            Error('code %1', FieldOnly(Line));
    end;

    [Test]
    procedure GroupZero_PassedOn_FindFirst_Flagged()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'Z');
        if not FindFirstPassedOn(Line) then
            Error('not found');
    end;

    [Test]
    procedure FilterGroupThree_GetFilters_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Status, Line.Status::Posted);
        Line.FilterGroup(0);
        Error('%1', Filters(Line));
    end;

    [Test]
    procedure FilterGroupThree_FieldRead_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Status, Line.Status::Open);
        Line.FilterGroup(0);
        if FieldOnly(Line) <> 'B' then
            Error('code %1', FieldOnly(Line));
    end;

    [Test]
    procedure FilterGroupThree_ResetThenCount_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(3);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        if ResetThenCount(Line) <> 2 then
            Error('count');
    end;

    [Test]
    procedure Descending_GetFilters_Flagged()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.Ascending(false);
        if Filters(Line) <> '' then
            Error('%1', Filters(Line));
    end;

    [Test]
    procedure ByVar_KeepsFilters()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        if FiltersByVar(Line) <> 'Code: A' then
            Error('%1', FiltersByVar(Line));
    end;

    [Test]
    procedure Temporary_NoFilters_Flagged()
    var
        Line: Record "BN Row" temporary;
    begin
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Code, 'A');
        if TempFilters(Line) <> '' then
            Error('%1', TempFilters(Line));
    end;
}
"##;

#[test]
fn issue_8185_neighbors_refuse_a_read_of_the_view_or_keep_main() {
    let o = run(BY_VALUE_NEIGHBORS, "BN Test");
    let want = [
        ("NoFilter_AdHocKey_PrimaryKey", PASS, false),
        // The copy does not take the FlowField key's mark (record/keys-sorting/PwFlowFieldKeyRule.al:249).
        ("NoFilter_RowDependentFlowFieldKey_PrimaryKey", PASS, false),
        // A declared secondary key: the primary key (corpus PR 1430, ByValueRecordView.al ByValue_SecondaryKey_NoFilter).
        ("NoFilter_DeclaredKey_PrimaryKey", PASS, true),
        // Only a read of the view marks the assumption, also on a copy handed on.
        ("GroupZero_FieldRead_NotFlagged", PASS, false),
        ("GroupZero_PassedOn_FindFirst_Flagged", PASS, true),
        ("FilterGroupThree_GetFilters_Refused", VIEW_REFUSED, false),
        ("FilterGroupThree_FieldRead_Runs", PASS, false),
        ("FilterGroupThree_ResetThenCount_Runs", PASS, false),
        ("FilterGroupThree_MethodsThatReadNoView_Run", PASS, false),
        ("FilterGroupThree_CalcFields_NoFlowFilter_Runs", PASS, false),
        ("FilterGroupThree_CalcFields_FlowFilter_Refused", VIEW_REFUSED, false),
        // A hand-off of the copy to code that reads its view refuses; TransferFields reads its fields only.
        // Copy hands the copy's mark on: a read of the target's view refuses, its Modify runs (pages/link-filter/
        // LinkCopyRecordMethods.al:105-107 copies a by-value Rec and modifies the copy).
        ("FilterGroupThree_CopyFrom_Refused", VIEW_REFUSED, false),
        ("FilterGroupThree_CopyThenModify_Runs", PASS, false),
        ("FilterGroupThree_CopyFiltersFrom_Refused", VIEW_REFUSED, false),
        ("FilterGroupThree_TransferFieldsFrom_Runs", PASS, false),
        ("FilterGroupThree_GetTable_Refused", VIEW_REFUSED, false),
        ("FilterGroupThree_PageRun_Refused", VIEW_REFUSED, false),
        ("GroupZero_PageRun_Flagged", PASS, true),
        // Copy hands the assumed view on: a later test that reads the kept copy is flagged too.
        ("GroupZero_CopyKept_Flagged", PASS, true),
        ("GroupZero_CopyKept_LaterRead_Flagged", PASS, true),
        // Descending and a temporary record into a stored parameter: no filters (ByValueRecordView.al
        // ByValue_Descending_NoFilter, ByValue_TemporaryToPlainParameter).
        ("Descending_GetFilters_Flagged", PASS, true),
        ("ByVar_KeepsFilters", PASS, false),
        ("Temporary_NoFilters_Flagged", PASS, true),
    ];
    check(&o, &want);
}

/// al-runner issue 7935, the reporter's repro as written: on BC row 1 keeps 'alpha'.
const ISSUE_7935: &str = r##"
table 50430 "RM Row"
{
    TableType = Temporary;
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { Clustered = true; } }
}

codeunit 50435 "RM Hold"
{
    var
        HB: Record "RM Row" temporary;

    procedure Fill(Txt: Text)
    var
        OS: OutStream;
    begin
        HB.Data.CreateOutStream(OS);
        OS.WriteText(Txt);
    end;

    procedure ToRec(var RR: RecordRef; FNo: Integer)
    var
        FR: FieldRef;
    begin
        FR := RR.Field(FNo);
        FR.Value := HB.Data;
    end;
}

codeunit 50436 "RM Lst"
{
    var
        Rec: Record "RM Row" temporary;

    procedure Add(H: Codeunit "RM Hold")
    var
        RR: RecordRef;
    begin
        Rec.Key1 := Rec.Count() + 1;
        Clear(Rec.Data);                 // remove this line and the test passes
        RR.GetTable(Rec);
        H.ToRec(RR, Rec.FieldNo(Data));
        RR.SetTable(Rec);
        Rec.Insert();
    end;

    procedure Text(K: Integer) Txt: Text
    var
        IS: InStream;
    begin
        Rec.Get(K);
        Rec.CalcFields(Data);
        Rec.Data.CreateInStream(IS);
        IS.ReadText(Txt);
    end;
}

codeunit 50437 "RM Tests"
{
    Subtype = Test;

    [Test]
    procedure SameHolderTwoAdds()
    var
        L: Codeunit "RM Lst";
        H: Codeunit "RM Hold";
    begin
        H.Fill('alpha');
        L.Add(H);
        H.Fill('beta');
        L.Add(H);
        if L.Text(1) <> 'alpha' then
            Error('row 1 read back <%1>, expected <alpha>', L.Text(1));
    end;
}
"##;

#[test]
fn issue_7935_reporter_repro_keeps_the_blob_of_the_first_row() {
    let o = run(ISSUE_7935, "RM Tests");
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, "Pass", "{}", o[0].0);
}

/// al-runner issue 8006, the reporter's repro as written: on BC the Modify of 17530101D fails.
const ISSUE_8006: &str = r##"
table 50170 "Visit Plan"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; "Planned On"; Date) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 50171 "Modify 1753 Test"
{
    Subtype = Test;
    [Test]
    procedure ModifyWith17530101()
    var
        Plan: Record "Visit Plan";
    begin
        Plan."Code" := 'A';
        Plan.Insert();
        Plan."Planned On" := 17530101D;
        Plan.Modify();
    end;
}

codeunit 50172 "Insert 1753 Test"
{
    Subtype = Test;
    [Test]
    procedure InsertWith17530101()
    var
        Plan: Record "Visit Plan";
    begin
        Plan."Code" := 'B';
        Plan."Planned On" := 17530101D;
        Plan.Insert();
    end;
}

codeunit 50173 "Modify 1753-01-02 Test"
{
    Subtype = Test;
    [Test]
    procedure ModifyWith17530102()
    var
        Plan: Record "Visit Plan";
    begin
        Plan."Code" := 'C';
        Plan.Insert();
        Plan."Planned On" := 17530102D;
        Plan.Modify();
    end;
}
"##;

#[test]
fn issue_8006_reporter_repro_fails_the_modify_of_17530101d() {
    let o = run(ISSUE_8006, "Modify 1753 Test");
    assert_eq!(o.len(), 1);
    assert!(
        o[0].1
            .starts_with("Fail(\"Cannot write the value 01/01/1753 to the field Planned On"),
        "{}",
        o[0].1
    );
}

/// al-runner issue 8248, the reporter's app as written. The first three tests end with Error on purpose to print a count; on BC
/// 29.0 each prints `counts 2 of 2 rows`, and UniquenessCheckSeesTheLinkedRow passes.
const ISSUE_8248: &str = r##"
table 51260 "Repro Value"
{
    fields
    {
        field(1; ID; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; "Store Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(4; "Link Id"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = lookup("Repro Value Link"."Remote Id" where("Value ID" = field(ID), Store = field("Store Filter")));
        }
    }
    keys { key(PK; ID) { Clustered = true; } }

    procedure CountSameName(Copy: Record "Repro Value"): Integer
    begin
        // the way the base app checks uniqueness: a by-value copy, new filters only
        Copy.SetFilter(ID, '<>%1', Copy.ID);
        Copy.SetRange(Name, Copy.Name);
        exit(Copy.Count());
    end;
}

table 51261 "Repro Value Link"
{
    fields
    {
        field(1; "Value ID"; Integer) { }
        field(2; Store; Code[10]) { }
        field(3; "Remote Id"; Integer) { }
    }
    keys { key(PK; "Value ID", Store) { Clustered = true; } }
}

codeunit 51262 "Repro By-Value Filter Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Setup(var Rec: Record "Repro Value")
    var
        Link: Record "Repro Value Link";
    begin
        Rec.DeleteAll();
        Link.DeleteAll();
        Rec.ID := 1; Rec.Name := 'Aquagrey'; Rec.Insert();
        Link."Value ID" := 1; Link.Store := 'MAIN'; Link."Remote Id" := 77; Link.Insert();
        Rec.ID := 2; Rec.Name := 'Aquagrey'; Rec.Insert();
    end;

    local procedure CountAll(Copy: Record "Repro Value"): Integer
    begin
        exit(Copy.Count());
    end;

    [Test]
    procedure PlainFilterInByValueCopy()
    var
        Rec: Record "Repro Value";
    begin
        Setup(Rec);
        Rec.SetRange(ID, 2);
        Error('by-value copy counts %1 of 2 rows (plain filter ID=2 set in the caller)', CountAll(Rec));
    end;

    [Test]
    procedure FlowFieldFilterInByValueCopy()
    var
        Rec: Record "Repro Value";
    begin
        Setup(Rec);
        Rec.SetRange("Store Filter", 'MAIN');
        Rec.SetRange("Link Id", 0);
        Error('by-value copy counts %1 of 2 rows (FlowField filter Link Id=0 set in the caller)', CountAll(Rec));
    end;

    [Test]
    procedure FilterGroupMinusOneInByValueCopy()
    var
        Rec: Record "Repro Value";
    begin
        Setup(Rec);
        Rec.FilterGroup(-1);
        Rec.SetFilter(ID, '2');
        Rec.SetFilter(Name, 'Nothing');
        Rec.FilterGroup(0);
        Error('by-value copy counts %1 of 2 rows (filter group -1 set in the caller)', CountAll(Rec));
    end;

    [Test]
    procedure UniquenessCheckSeesTheLinkedRow()
    var
        Rec: Record "Repro Value";
    begin
        Setup(Rec);
        // the caller searched for an unlinked row with this name, then works on row 2
        Rec.SetRange("Store Filter", 'MAIN');
        Rec.SetRange("Link Id", 0);
        Rec.Get(2);
        if Rec.CountSameName(Rec) <> 1 then
            Error('The uniqueness check found %1 rows named %2; the linked row 1 has the same name.', Rec.CountSameName(Rec), Rec.Name);
    end;
}
"##;

#[test]
fn issue_8248_reporter_repro_gives_the_by_value_copy_no_filters() {
    let o = run(ISSUE_8248, "Repro By-Value Filter Test");
    let want = [
        ("PlainFilterInByValueCopy", "by-value copy counts 2 of 2 rows"),
        ("FlowFieldFilterInByValueCopy", "by-value copy counts 2 of 2 rows"),
        ("FilterGroupMinusOneInByValueCopy", "by-value copy counts 2 of 2 rows"),
        ("UniquenessCheckSeesTheLinkedRow", PASS),
    ];
    let mut wrong = Vec::new();
    for (name, text) in want {
        let (_, outcome, flagged) = o
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("no test {name}"));
        let ok = if text == PASS {
            outcome.starts_with(PASS)
        } else {
            outcome.contains(text)
        };
        if !ok || !flagged {
            wrong.push(format!("{name}: {outcome} (flagged {flagged})"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The shapes of corpus PR 1430 (record/filters/ByValueRecordView.al) beyond issue 8185, and their neighbors, one fact flipped
/// each.
const BY_VALUE_VIEWS: &str = r##"
table 50150 "BW Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Status; Option) { OptionMembers = Open,Posted; }
        field(3; Name; Text[30]) { }
        field(4; Amount; Decimal) { }
    }
    keys { key(PK; "Code") { Clustered = true; } key(ByName; Name) { } }

    procedure FiltersOfOther(Other: Record "BW Other"): Text
    begin
        exit(Other.GetFilters());
    end;
}

table 50151 "BW Other"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Status; Option) { OptionMembers = Open,Posted; } }
    keys { key(PK; "Code") { Clustered = true; } }
}

tableextension 50152 "BW Row Ext" extends "BW Row"
{
    procedure ExtFilters(Line: Record "BW Row"): Text
    begin
        exit(Line.GetFilters());
    end;
}

page 50153 "BW Rows"
{
    PageType = List;
    SourceTable = "BW Row";
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }

    procedure PageFilters(Line: Record "BW Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    procedure PageKey(Line: Record "BW Row"): Text
    begin
        exit(Line.CurrentKey());
    end;
}

codeunit 50154 "BW Callee"
{
    procedure Describe(Line: Record "BW Row"): Text
    begin
        exit('f={' + Line.GetFilters() + '} n=' + Format(Line.Count()) + ' key=' + Line.CurrentKey() + ' asc='
            + Format(Line.Ascending()) + ' grp=' + Format(Line.FilterGroup()) + ' mo=' + Format(Line.MarkedOnly()));
    end;

    procedure Loaded(Line: Record "BW Row"): Text
    begin
        Line.FindFirst();
        exit(Format(Line.AreFieldsLoaded(Amount)));
    end;

    procedure Temp(Line: Record "BW Row"): Text
    begin
        exit('temp=' + Format(Line.IsTemporary()) + ' f={' + Line.GetFilters() + '}');
    end;
}

codeunit 50155 "BW Test"
{
    Subtype = Test;

    var
        Callee: Codeunit "BW Callee";

    local procedure Seed(var Line: Record "BW Row")
    begin
        Line.DeleteAll();
        Line.Code := 'A';
        Line.Name := 'z';
        Line.Insert();
        Line.Code := 'B';
        Line.Name := 'y';
        Line.Status := Line.Status::Posted;
        Line.Insert();
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    local procedure InGroup(var Line: Record "BW Row"; Group: Integer)
    begin
        Line.FilterGroup(Group);
        Line.SetRange(Status, Line.Status::Posted);
        Line.FilterGroup(0);
    end;

    [Test]
    procedure Group2_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        InGroup(Line, 2);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure Group4_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        InGroup(Line, 4);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure GroupMinus1_TwoFields_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.FilterGroup(-1);
        Line.SetFilter(Code, 'B');
        Line.SetFilter(Name, 'NOTHING');
        Line.FilterGroup(0);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure CallerStandsInGroup2_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetRange(Status, Line.Status::Posted);
        Line.FilterGroup(2);
        Line.SetRange(Status, Line.Status::Posted);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure Descending_SecondaryKey_Marks_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(Name);
        Line.Ascending(false);
        Line.Get('B');
        Line.Mark(true);
        Line.MarkedOnly(true);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure DeclaredKey_NoFilter_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(Name);
        Expect(Callee.Describe(Line), 'f={} n=2 key=Code asc=Yes grp=0 mo=No');
    end;

    [Test]
    procedure SetLoadFields_BeforeAnyRead_Flagged()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetLoadFields(Name);
        Line.SetRange(Status, Line.Status::Posted);
        Expect(Callee.Loaded(Line), 'Yes');
    end;

    [Test]
    procedure TemporaryToPlainParameter_Flagged()
    var
        Line: Record "BW Row" temporary;
    begin
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Code, 'A');
        Expect(Callee.Temp(Line), 'temp=No f={}');
    end;

    [Test]
    procedure TemporaryToPlainParameter_NoFilter_NotFlagged()
    var
        Line: Record "BW Row" temporary;
    begin
        Line.Code := 'A';
        Line.Insert();
        Expect(Callee.Temp(Line), 'temp=No f={}');
    end;

    [Test]
    procedure Group3_Refused()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        InGroup(Line, 3);
        Expect(Callee.Describe(Line), '');
    end;

    [Test]
    procedure CallerStandsInGroup4_Refused()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.FilterGroup(4);
        Expect(Callee.Describe(Line), '');
    end;

    [Test]
    procedure FieldDescending_Refused()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetAscending(Code, false);
        Expect(Callee.Describe(Line), '');
    end;

    [Test]
    procedure PageProcedure_Group0Filter_Refused()
    var
        Line: Record "BW Row";
        Rows: Page "BW Rows";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(Rows.PageFilters(Line), '');
    end;

    [Test]
    procedure PageProcedure_NoFilter_KeepsMain()
    var
        Line: Record "BW Row";
        Rows: Page "BW Rows";
    begin
        Seed(Line);
        Expect(Rows.PageKey(Line), 'Code');
    end;

    [Test]
    procedure TableExtensionProcedure_Group0Filter_Refused()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(Line.ExtFilters(Line), '');
    end;

    [Test]
    procedure TableProcedure_OtherTable_Refused()
    var
        Line: Record "BW Row";
        Other: Record "BW Other";
    begin
        Seed(Line);
        Other.SetRange(Code, 'A');
        Expect(Line.FiltersOfOther(Other), '');
    end;

    [Test]
    procedure ByVar_KeepsFilters()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(ByVar(Line), 'Code: A');
    end;

    local procedure ByVar(var Line: Record "BW Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    // Last: the filter of group 1 it leaves is seen by every record of the table in the session.
    [Test]
    procedure Group1_Refused()
    var
        Line: Record "BW Row";
    begin
        Seed(Line);
        InGroup(Line, 1);
        Expect(Callee.Describe(Line), '');
    end;
}
"##;

#[test]
fn by_value_views_of_the_probe_run_flagged_and_their_neighbors_refuse_or_keep_main() {
    let o = run(BY_VALUE_VIEWS, "BW Test");
    let want = [
        ("Group2_Flagged", PASS, true),
        ("Group4_Flagged", PASS, true),
        ("GroupMinus1_TwoFields_Flagged", PASS, true),
        ("CallerStandsInGroup2_Flagged", PASS, true),
        ("Descending_SecondaryKey_Marks_Flagged", PASS, true),
        ("DeclaredKey_NoFilter_Flagged", PASS, true),
        ("SetLoadFields_BeforeAnyRead_Flagged", PASS, true),
        ("TemporaryToPlainParameter_Flagged", PASS, true),
        // No filter to drop: what main gave (record/temporary/TempRecordByValue.al pins the stored table).
        ("TemporaryToPlainParameter_NoFilter_NotFlagged", PASS, false),
        ("Group1_Refused", VIEW_REFUSED, false),
        ("Group3_Refused", VIEW_REFUSED, false),
        ("CallerStandsInGroup4_Refused", VIEW_REFUSED, false),
        ("FieldDescending_Refused", VIEW_REFUSED, false),
        ("PageProcedure_Group0Filter_Refused", VIEW_REFUSED, false),
        ("PageProcedure_NoFilter_KeepsMain", PASS, false),
        ("TableExtensionProcedure_Group0Filter_Refused", VIEW_REFUSED, false),
        ("TableProcedure_OtherTable_Refused", VIEW_REFUSED, false),
        ("ByVar_KeepsFilters", PASS, false),
    ];
    check(&o, &want);
}
