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
    }
    keys { key(PK; "No.") { Clustered = true; } }
    var
        Mover: Codeunit "XN Mover";

    trigger OnModify()
    begin
        if Mode = 5 then
            Mover.ValidateSelf(Rec);
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
    procedure OnModify_ValidateInProc_Refused()
    var
        Item: Record "XN Item";
    begin
        Stored(Item, 5);
        Item.Value := 15;
        Item.Modify(true);
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
        ("DirectFromTrigger_ReadsXRec_Refused", "NotRun", false),
        ("DirectFromTrigger_NoXRecRead_Runs", "Pass", false),
        ("OnModify_ValidateInProc_Refused", "NotRun", false),
        ("DirectFromTrigger_CopyOfXRec_Refused", "NotRun", false),
        ("MeasuredChain_FreshRecord_Refused", "NotRun", false),
        ("MeasuredChain_FieldRefValidate_Refused", "NotRun", false),
        // The measured chain hands the trigger's xRec on whether the procedure reads it or not: flagged.
        ("MeasuredChain_NoXRecRead_Runs", "Pass", true),
        ("NoRunningCall_PlainCall_BlankXRec", "Pass", false),
    ];
    for (name, outcome, flagged) in want {
        let got = o
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("no test {name}"));
        assert!(got.1.starts_with(outcome), "{name}: {}", got.1);
        assert_eq!(got.2, flagged, "{name}");
    }
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
    }
    keys { key(PK; "Code") { Clustered = true; } key(ByName; Name) { } }
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

    [Test]
    procedure NoFilter_KeyOnName_PrimaryKey()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.SetCurrentKey(Name);
        if Key(Line) <> 'Code' then
            Error('key %1', Key(Line));
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
    procedure FilterGroupTwo_GetFilters_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(2);
        Line.SetRange(Status, Line.Status::Posted);
        Line.FilterGroup(0);
        Error('%1', Filters(Line));
    end;

    [Test]
    procedure FilterGroupTwo_FieldRead_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(2);
        Line.SetRange(Status, Line.Status::Open);
        Line.FilterGroup(0);
        if FieldOnly(Line) <> 'B' then
            Error('code %1', FieldOnly(Line));
    end;

    [Test]
    procedure FilterGroupTwo_ResetThenCount_Runs()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.FilterGroup(2);
        Line.SetRange(Code, 'A');
        Line.FilterGroup(0);
        if ResetThenCount(Line) <> 2 then
            Error('count');
    end;

    [Test]
    procedure Descending_GetFilters_Refused()
    var
        Line: Record "BN Row";
    begin
        Seed(Line);
        Line.Ascending(false);
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
    procedure Temporary_KeepsMainFilters()
    var
        Line: Record "BN Row" temporary;
    begin
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Code, 'A');
        if TempFilters(Line) <> 'Code: A' then
            Error('%1', TempFilters(Line));
    end;
}
"##;

#[test]
fn issue_8185_neighbors_refuse_a_read_of_the_view_or_keep_main() {
    let o = run(BY_VALUE_NEIGHBORS, "BN Test");
    let want = [
        ("NoFilter_KeyOnName_PrimaryKey", "Pass", false),
        // Only a read of the view marks the assumption, also on a copy handed on.
        ("GroupZero_FieldRead_NotFlagged", "Pass", false),
        ("GroupZero_PassedOn_FindFirst_Flagged", "Pass", true),
        ("FilterGroupTwo_GetFilters_Refused", "NotRun", false),
        ("FilterGroupTwo_FieldRead_Runs", "Pass", false),
        ("FilterGroupTwo_ResetThenCount_Runs", "Pass", false),
        ("Descending_GetFilters_Refused", "NotRun", false),
        ("ByVar_KeepsFilters", "Pass", false),
        ("Temporary_KeepsMainFilters", "Pass", false),
    ];
    for (name, outcome, flagged) in want {
        let got = o
            .iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("no test {name}"));
        assert!(got.1.starts_with(outcome), "{name}: {}", got.1);
        assert_eq!(got.2, flagged, "{name}");
    }
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
