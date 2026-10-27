//! Writes into a FlowField: Validate, TransferFields and a temporary record
//! (record/lt-flowfield-write/DiFwWriteTests.al).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50300 P
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Child Count"; Integer) { FieldClass = FlowField; CalcFormula = count(C where("Parent No." = field("No."))); }
        field(4; "Child Amount"; Decimal) { FieldClass = FlowField; CalcFormula = sum(C.Amount where("Parent No." = field("No."), "Posting Date" = field("Date Filter"))); }
        field(5; "Has Children"; Boolean) { FieldClass = FlowField; CalcFormula = exist(C where("Parent No." = field("No."))); }
        field(6; "First Code"; Text[30]) { FieldClass = FlowField; CalcFormula = lookup(C.Code where("Parent No." = field("No."))); }
        field(7; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(8; "Max Amount"; Decimal) { FieldClass = FlowField; CalcFormula = max(C.Amount where("Parent No." = field("No."))); }
    }
    keys { key(PK; "No.") { } }
}

table 50301 C
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Parent No."; Integer) { }
        field(3; Code; Text[30]) { }
        field(4; Amount; Decimal) { }
        field(5; "Posting Date"; Date) { }
    }
    keys { key(PK; "Entry No.") { } }
}

codeunit 50302 T
{
    local procedure Seed()
    var
        P: Record P;
        C: Record C;
    begin
        P."No." := 1; P.Insert();
        P."No." := 2; P.Insert();
        P."No." := 3; P.Insert();
        AddChild(1, 1, 'A', 10, 20260110D);
        AddChild(2, 1, 'B', 20, 20260210D);
        AddChild(3, 3, 'D', 5, 20260115D);
    end;

    local procedure AddChild(EntryNo: Integer; ParentNo: Integer; NewCode: Text[30]; NewAmount: Decimal; PostingDate: Date)
    var
        C: Record C;
    begin
        C."Entry No." := EntryNo;
        C."Parent No." := ParentNo;
        C.Code := NewCode;
        C.Amount := NewAmount;
        C."Posting Date" := PostingDate;
        C.Insert();
    end;

    local procedure Src(): Text
    var
        C: Record C;
        R: Text;
    begin
        if C.FindSet() then
            repeat
                R += Format(C."Entry No.") + '|' + Format(C."Parent No.") + '|' + C.Code + '|' + Format(C.Amount, 0, 9) + '|' + Format(C."Posting Date", 0, 9) + ';';
            until C.Next() = 0;
        exit(R);
    end;

    procedure ValidateCount(): Text
    var
        P: Record P;
    begin
        Seed();
        P.Get(1);
        asserterror P.Validate("Child Count", 7);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ValidateExistThroughFieldRef(): Text
    var
        P: Record P;
        R: RecordRef;
        F: FieldRef;
    begin
        Seed();
        P.Get(1);
        R.GetTable(P);
        F := R.Field(5);
        asserterror F.Validate(true);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ValidateMax()
    var
        P: Record P;
    begin
        Seed();
        P.Get(1);
        P.Validate("Max Amount", 7);
    end;

    procedure ValidateSum(): Text
    var
        P: Record P;
        R: Text;
    begin
        Seed();
        P.Get(2);
        P.SetRange("Date Filter", 20260401D);
        P.Validate("Child Amount", 7);
        R := Format(P."Child Amount") + '#' + Src() + '#';
        P.Reset();
        P.Get(1);
        P."Child Amount" := 1000;
        P.Validate("Child Amount", 70);
        R += Src() + '#';
        P.Get(1);
        P.Validate("Child Amount", 70);
        R += Src() + '#';
        P.Get(3);
        P.SetRange("Date Filter", 20260101D, 20260131D);
        P.Validate("Child Amount", 2);
        exit(R + Src());
    end;

    procedure ValidateSumOneArgument(): Text
    var
        P: Record P;
        R: Text;
    begin
        Seed();
        P.Get(1);
        P.Validate("Child Amount");
        R := Format(P."Child Amount") + '#' + Src() + '#';
        P.Get(1);
        P.CalcFields("Child Amount");
        P.Validate("Child Amount");
        R += Src() + '#';
        P.Get(1);
        P."Child Amount" := 100;
        P.Validate("Child Amount");
        exit(R + Format(P."Child Amount") + '#' + Src());
    end;

    procedure TransferSameTable(): Text
    var
        S: Record P;
        D: Record P;
    begin
        Seed();
        S.Get(1);
        S."Child Count" := 7;
        D.Get(2);
        D.TransferFields(S, false);
        exit(Format(D."No.") + '/' + Format(D."Child Count"));
    end;

    procedure TemporaryRow(): Text
    var
        T: Record P temporary;
        R: Text;
    begin
        Seed();
        T."No." := 1;
        T."Child Count" := 5;
        T.Insert();
        T.Get(1);
        R := Format(T."Child Count");
        T."Child Count" := 7;
        T.Modify();
        Clear(T."Child Count");
        T.Get(1);
        R += '/' + Format(T."Child Count");
        T.CalcFields("Child Count");
        R += '/' + Format(T."Child Count");
        T.Get(1);
        exit(R + '/' + Format(T."Child Count"));
    end;
}
"#;

// DiFwWriteTests.al:284, :439, :693-694: Validate on a Count, Exist or Lookup FlowField raises FlowFieldValidation.
#[test]
fn validate_on_a_count_or_exist_flowfield_raises() {
    assert_eq!(
        text(run_in(SRC, "T", "ValidateCount")),
        "The type of the Child Count FlowField in the P table must be Decimal.|FlowFieldValidation"
    );
    assert_eq!(
        text(run_in(SRC, "T", "ValidateExistThroughFieldRef")),
        "The type of the Has Children FlowField in the P table must be Decimal.|FlowFieldValidation"
    );
}

// Validate on a Max FlowField is not pinned.
#[test]
fn validate_on_a_max_flowfield_is_refused() {
    assert!(matches!(
        run_in(SRC, "T", "ValidateMax"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}

// DiFwWriteTests.al:812-819: Validate on a Sum FlowField inserts the difference into the source table.
#[test]
fn validate_on_a_sum_flowfield_writes_the_difference_into_the_source_table() {
    let base = "1|1|A|10|2026-01-10;2|1|B|20|2026-02-10;3|3|D|5|2026-01-15;";
    let p2 = format!("{base}4|2||7|2026-04-01;");
    let p1 = format!("{p2}5|1||40|;");
    assert_eq!(
        text(run_in(SRC, "T", "ValidateSum")),
        format!("7#{p2}#{p1}#{p1}#{p1}6|3||-3|2026-01-01;")
    );
}

// DiFwWriteTests.al:860-926 (QB_ValidateOneArgFlowFieldWithOnValidate): the one-argument Validate on a Sum FlowField takes the value
// the variable holds (0 without CalcFields, the sum after it, an assigned value) and inserts the difference into the source table.
#[test]
fn validate_with_one_argument_on_a_sum_flowfield_uses_the_value_it_holds() {
    let base = "1|1|A|10|2026-01-10;2|1|B|20|2026-02-10;3|3|D|5|2026-01-15;";
    let neg = format!("{base}4|1||-30|;");
    let back = format!("{neg}5|1||100|;");
    assert_eq!(
        text(run_in(SRC, "T", "ValidateSumOneArgument")),
        format!("0#{neg}#{neg}#100#{back}")
    );
}

// DiFwWriteTests.al:504-506: TransferFields within one table copies an assigned FlowField value.
#[test]
fn transferfields_within_a_table_copies_a_flowfield() {
    assert_eq!(text(run_in(SRC, "T", "TransferSameTable")), "2/7");
}

// DiFwWriteTests.al:625-630: a temporary row keeps the FlowField value a Modify wrote, not the one of an Insert.
#[test]
fn a_temporary_row_keeps_the_flowfield_value_of_a_modify() {
    assert_eq!(text(run_in(SRC, "T", "TemporaryRow")), "0/7/2/7");
}

const OR: &str = r#"
table 50340 O
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Total; Integer) { FieldClass = FlowField; CalcFormula = count(O where(Id = field(Id))); }
    }
    keys { key(PK; Id) { } }
}

codeunit 50341 T
{
    procedure F(): Text
    var
        O: Record O;
    begin
        O.Id := 1; O.Name := 'one'; O.Insert();
        O.Id := 2; O.Name := ''; O.Insert();
        O.FilterGroup(-1);
        O.SetRange(Name, '');
        O.SetRange(Total, 0);
        O.SetFilter(Total, '>6');
        exit(O.GetFilters() + '|' + Format(O.Count()));
    end;
}
"#;

// record/PWFilterGroupOr.al:OrGroup_FlowFieldFilter_BQ: a FlowField filter in the OR group is dropped.
#[test]
fn a_flowfield_filter_in_the_or_group_is_dropped() {
    assert_eq!(text(run_in(OR, "T", "F")), "Name: ''|1");
}

const BULK: &str = r#"
table 50350 M
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } }
    keys { key(PK; Id) { } }
}

codeunit 50351 T
{
    procedure F(): Text
    var
        M: Record M;
        Before: DateTime;
        R: Text;
    begin
        M.Id := 1; M.Amount := 10; M.Insert();
        M.Get(1);
        Before := M.SystemModifiedAt;
        Sleep(50);
        M.ModifyAll(Amount, 10, true);
        M.Get(1);
        R := Format(M.SystemModifiedAt <> Before);
        Sleep(50);
        M.ModifyAll(Amount, 10, false);
        M.Get(1);
        exit(R + '/' + Format(M.SystemModifiedAt <> Before));
    end;
}
"#;

// record/lt-record-write/RcBTriggers.al:ModifyAll_Equal_Value_Rows_BQ: ModifyAll(.., false) stamps a row that keeps its value.
#[test]
fn modifyall_without_triggers_stamps_a_row_that_keeps_its_value() {
    assert_eq!(text(run_in(BULK, "T", "F")), "No/Yes");
}

const ORDER: &str = r#"
table 50360 F
{
    fields
    {
        field(1; A; Integer) { }
        field(2; DF; Date) { FieldClass = FlowFilter; }
        field(3; B; Integer) { }
        field(4; N; Integer) { FieldClass = FlowField; CalcFormula = count(F); }
    }
    keys { key(PK; A) { } }
}

codeunit 50361 T
{
    procedure F(): Text
    var
        RR: RecordRef;
        i: Integer;
        R: Text;
    begin
        RR.Open(Database::F);
        for i := 1 to RR.FieldCount() do
            R += Format(RR.FieldIndex(i).Number()) + ';';
        exit(R);
    end;
}
"#;

// metadata/lt-custom-report-layout-columns/CustomReportLayoutColumns.al:PW_FieldIndex_FlowFilterAmongNormalFields.
#[test]
fn fieldindex_lists_normal_fields_then_flowfilters_then_flowfields() {
    assert_eq!(text(run_in(ORDER, "T", "F")), "1;3;2;4;");
}

const VARIANT: &str = r#"
table 50370 VR { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 2000000006 Company
{
    DataPerCompany = false;
    fields { field(1; Name; Text[30]) { } field(8000; Id; Guid) { } }
    keys { key(PK; Name) { } }
}

codeunit 50371 T
{
    procedure F(): Text
    var
        C: Record Company;
        R: Record VR;
        RecRef: RecordRef;
        V: Variant;
        Out: Text;
    begin
        C.Name := 'OTHER';
        C.Insert();
        R.ChangeCompany('OTHER');
        R.SetFilter(Id, '>=2');
        RecRef.GetTable(R);
        Out := Format(RecRef.CurrentCompany() = 'OTHER');
        V := R;
        RecRef.GetTable(V);
        exit(Out + '|' + Format(RecRef.CurrentCompany() = CompanyName()) + '|' + RecRef.GetFilters());
    end;
}
"#;

// types/lt-cw-variant-leads/CwVariantChangeCompany.al:CW_Variant_KeepsOrDropsChangeCompany: through a Variant, GetTable drops the company.
#[test]
fn gettable_through_a_variant_drops_the_company_and_keeps_the_filters() {
    assert_eq!(text(run_in(VARIANT, "T", "F")), "Yes|Yes|Id: >=2");
}

const TEMP_PARAM: &str = r#"
table 50380 TP { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }

codeunit 50381 T
{
    local procedure Look(R: Record TP temporary): Text
    begin
        exit(Format(R.IsTemporary()) + ' ' + Format(R.Count()) + ' ' + Format(R.Id) + ' [' + R.GetFilters() + ']');
    end;

    procedure F(): Text
    var
        R: Record TP;
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
        R.Id := 3; R.Insert();
        R.SetRange(Id, 2, 3);
        R.FindFirst();
        exit(Look(R) + '|' + R.GetFilters());
    end;
}
"#;

// record/lt-qd-temp-param/QdStoredToTempParam.al:90-119 StoredRecord_ToTemporaryParameter: a stored record handed by value
// to a `temporary` parameter of a plain procedure gives the callee a temporary record with the record's values over an empty
// table, without the caller's filters (`callee.filters` empty); the caller keeps its own (`caller.filters = Id: 2..3`).
#[test]
fn a_stored_record_handed_to_a_temporary_parameter_of_a_procedure_drops_the_filters() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(text(run_in(TEMP_PARAM, "T", "F")), "Yes 0 2 []|Id: 2..3");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

const TEMP_SUBSCRIBER: &str = r#"
table 50382 TQ { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50383 TL { SingleInstance = true; var T: Text; procedure Add(S: Text) begin T += S; end; procedure Get(): Text begin exit(T); end; }
codeunit 50384 TS
{
    [EventSubscriber(ObjectType::Table, Database::TQ, 'OnAfterInsertEvent', '', false, false)]
    local procedure Ins(Rec: Record TQ temporary)
    var L: Codeunit TL;
    begin
        L.Add(Format(Rec.IsTemporary()) + ' ' + Format(Rec.Count()) + ' ' + Format(Rec.Id));
    end;
}
codeunit 50385 T
{
    procedure F(): Text
    var
        R: Record TQ;
        L: Codeunit TL;
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert(true);
        exit(L.Get());
    end;
}
"#;

// events/lt-cl-event-leads/ClEventLeads.al:70 (`ins.temporary temp=Yes count=0 id=103`): an OnAfterInsertEvent subscriber whose Rec
// is declared temporary gets a temporary copy of the stored record over an empty table.
#[test]
fn a_stored_record_handed_to_a_temporary_subscriber_parameter_is_an_empty_temporary_copy() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(text(run_in(TEMP_SUBSCRIBER, "T", "F")), "Yes 0 1Yes 0 2");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

const SUM_TRIGGERS: &str = r#"
codeunit 50392 L { SingleInstance = true; var T: Text; procedure Add(S: Text) begin T += S + ','; end; procedure Get(): Text begin exit(T); end; }
table 50390 SR
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Amount; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum(SE.Amount where(Row = field(Id)));
            trigger OnValidate() var L: Codeunit L; begin L.Add('OnValidate:' + Format(Amount) + '/' + Format(xRec.Amount)); end;
        }
    }
    keys { key(PK; Id) { } }
}
table 50391 SE
{
    fields { field(1; No; Integer) { } field(2; Row; Code[10]) { } field(3; Amount; Decimal) { } }
    keys { key(PK; No) { } }
    trigger OnInsert() var L: Codeunit L; begin L.Add('OnInsert:' + Format(No) + '/' + Format(Amount)); end;
}
codeunit 50393 T
{
    procedure F(): Text
    var
        R: Record SR;
        E: Record SE;
        L: Codeunit L;
    begin
        R.Id := 'R1'; R.Insert();
        E.No := 1; E.Row := 'R1'; E.Amount := 30; E.Insert();
        R.Get('R1');
        R.CalcFields(Amount);
        R.Validate(Amount, 50);
        exit(L.Get());
    end;
}
"#;

// pages/lt-cf-page-leads/CfPageLeads.al:EditableFlowField_SetValue: OnValidate first, then the source row with its OnInsert.
#[test]
fn validate_on_a_sum_flowfield_runs_onvalidate_then_inserts_with_the_trigger() {
    assert_eq!(text(run_in(SUM_TRIGGERS, "T", "F")), "OnValidate:50/30,OnInsert:2/20,");
}

const TEMP_PARAM_KEYS: &str = r#"
table 50386 TK { fields { field(1; Id; Integer) { } field(2; Name; Text[10]) { } } keys { key(PK; Id) { } key(ByName; Name) { } } }
codeunit 50387 Callee
{
    procedure Keys(R: Record TK temporary): Text
    var S: Text;
    begin
        S := R.CurrentKey() + ' ' + Format(R.Ascending()) + ' ' + R.GetView(false) + ' ' + Format(R.Id) + '=' + R.Name + ' ';
        Fill(R);
        exit(S + Order(R));
    end;
    procedure Marks(R: Record TK temporary): Text
    var S: Text;
    begin
        S := Format(R.MarkedOnly()) + ' ' + Format(R.Mark()) + ' ' + Format(R.Id) + ' ' + Format(R.Count()) + ' ';
        Fill(R);
        S += Format(R.Count()) + ' ' + Order(R);
        R.MarkedOnly(false);
        if R.FindSet() then
            repeat
                S += Format(R.Mark());
            until R.Next() = 0;
        exit(S);
    end;
    local procedure Fill(var R: Record TK temporary)
    begin
        R.Id := 1; R.Name := 'c'; R.Insert();
        R.Id := 2; R.Name := 'a'; R.Insert();
        R.Id := 3; R.Name := 'b'; R.Insert();
    end;
    local procedure Order(var R: Record TK temporary): Text
    var Ids: Text;
    begin
        if R.FindSet() then
            repeat
                Ids += Format(R.Id) + ',';
            until R.Next() = 0;
        exit(Ids);
    end;
}
codeunit 50388 T
{
    local procedure Seed()
    var R: Record TK;
    begin
        R.Id := 1; R.Name := 'c'; R.Insert();
        R.Id := 2; R.Name := 'a'; R.Insert();
        R.Id := 3; R.Name := 'b'; R.Insert();
    end;
    procedure KeyAndOrder(): Text
    var R: Record TK; C: Codeunit Callee; S: Text;
    begin
        Seed();
        R.SetCurrentKey(Name);
        R.Ascending(false);
        R.FindFirst();
        S := C.Keys(R);
        exit(S + '|' + R.CurrentKey() + ' ' + Format(R.Ascending()));
    end;
    procedure Marks(): Text
    var R: Record TK; C: Codeunit Callee; S: Text;
    begin
        Seed();
        R.Get(1); R.Mark(true);
        R.Get(3); R.Mark(true);
        R.MarkedOnly(true);
        S := C.Marks(R);
        exit(S + '|' + Format(R.MarkedOnly()) + ' ' + Format(R.Count()) + ' ' + Format(R.Mark()));
    end;
}
"#;

// record/lt-qd-temp-param/QdStoredToTempParam.al:149-180 QF_StoredRecord_KeyAndOrder_ToTemporaryParameter (corpus bump 37): the
// callee's temporary record has the primary key, ascending (`callee.view = VERSION(1) SORTING(Field1)`), the current values, and
// sorts its own rows by the primary key; the caller keeps its key and order.
#[test]
fn a_stored_record_handed_to_a_temporary_parameter_gets_the_primary_key_ascending() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(
        text(run_in(TEMP_PARAM_KEYS, "T", "KeyAndOrder")),
        "Id Yes VERSION(1) SORTING(Field1) 1=c 1,2,3,|Name No"
    );
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

// QdStoredToTempParam.al:182-213 QF_StoredRecord_Marks_ToTemporaryParameter: the callee has no marks and MarkedOnly No; its own
// rows are all unmarked; the caller keeps its marks and MarkedOnly.
#[test]
fn a_stored_record_handed_to_a_temporary_parameter_has_no_marks() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(
        text(run_in(TEMP_PARAM_KEYS, "T", "Marks")),
        "No No 3 0 3 1,2,3,NoNoNo|Yes 2 Yes"
    );
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

// al-runner issue 5857: a temporary row keeps the FlowFilter value its Insert wrote; a Modify does not write a new one
// (record/lt-flowfield-write/DiFwWriteTests.al:937 SF_FlowFilter_TemporaryTableType_InsertThenRead, :975
// SF_FlowFilter_TemporaryVariable_InsertThenRead). A Rename of such a row is not pinned.
const TEMP_FF: &str = r#"
table 50320 TB
{
    TableType = Temporary;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Doc"; Code[20]) { FieldClass = FlowFilter; }
        field(3; Description; Text[30]) { }
    }
}
table 50321 NP
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; "Date Filter"; Date) { FieldClass = FlowFilter; }
    }
}
codeunit 50322 T
{
    procedure TableType(): Text
    var
        B: Record TB;
        R: Text;
    begin
        B.Init(); B."Entry No." := 1; B.Doc := 'DOC-001'; B.Description := 'First'; B.Insert();
        B.Init(); B."Entry No." := 2; B.Doc := 'DOC-002'; B.Description := 'Second'; B.Insert();
        R := B.Doc;
        B.FindFirst();
        R += '|' + B.Description + ':' + B.Doc + ':' + B.GetFilter(Doc);
        B.Next();
        R += '|' + B.Description + ':' + B.Doc;
        B.Get(1);
        B.Doc := 'DOC-009';
        B.Modify();
        B.Get(2);
        B.Get(1);
        R += '|' + B.Doc + '|' + Format(B.Count());
        exit(R);
    end;

    procedure TempVariable(): Text
    var
        P: Record NP temporary;
        R: Text;
    begin
        P.Init(); P."No." := 1; P."Date Filter" := 20260110D; P.Insert();
        P.Init(); P."No." := 2; P."Date Filter" := 20260210D; P.Insert();
        P.FindFirst();
        R := Format(P."No.") + ':' + Format(P."Date Filter", 0, 9);
        P.Get(2);
        R += '|' + Format(P."No.") + ':' + Format(P."Date Filter", 0, 9);
        exit(R);
    end;

    procedure RenameRefused()
    var
        P: Record NP temporary;
    begin
        P."No." := 1; P."Date Filter" := 20260110D; P.Insert();
        P.Rename(5);
    end;

    procedure RenameWithoutValue(): Text
    var
        P: Record NP temporary;
    begin
        P."No." := 1; P.Insert();
        P.Rename(5);
        exit(Format(P.Count()));
    end;
}
"#;

#[test]
fn a_temporary_row_keeps_the_flowfilter_value_of_its_insert() {
    assert_eq!(
        text(run_in(TEMP_FF, "T", "TableType")),
        "DOC-002|First:DOC-001:|Second:DOC-002|DOC-001|2"
    );
    assert_eq!(text(run_in(TEMP_FF, "T", "TempVariable")), "1:2026-01-10|2:2026-02-10");
    assert_eq!(text(run_in(TEMP_FF, "T", "RenameWithoutValue")), "1");
    match run_in(TEMP_FF, "T", "RenameRefused") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("FlowFilter"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
