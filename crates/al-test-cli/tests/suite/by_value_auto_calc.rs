//! A record handed by value whose caller set SetAutoCalcFields (al-runner issue 8538): the copy keeps the caller's field
//! values but not its SetAutoCalcFields, flagged by `by-value-record-no-auto-calc` (corpus PR 1473,
//! record/filters/ByValueAutoCalcFields.al). The shapes the probe did not measure keep the refusal or main's answer.

use crate::record_copy_reports::{PASS, VIEW_REFUSED, check, run};

const AUTO_CALC: &str = r##"
table 50180 "BA Head"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Code[10]) { }
        field(3; "Line Total"; Decimal) { FieldClass = FlowField; CalcFormula = sum("BA Detail".Amount where("Head No." = field("No."))); }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByKind; Kind) { }
    }

    procedure DescribeOwn(Copy: Record "BA Head"): Text
    var
        Callee: Codeunit "BA Callee";
    begin
        exit(Callee.DescribeVar(Copy));
    end;
}

table 50181 "BA Detail"
{
    fields { field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Amount; Decimal) { } }
    keys { key(PK; "Head No.", "Line No.") { Clustered = true; } }
}

table 50182 "BA Buffer"
{
    TableType = Temporary;
    fields { field(1; "Line No."; Integer) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }

    procedure DescribeHead(H: Record "BA Head"): Text
    var
        Callee: Codeunit "BA Callee";
    begin
        exit(Callee.DescribeVar(H));
    end;
}

page 50183 "BA Heads"
{
    PageType = List;
    SourceTable = "BA Head";

    procedure DescribeHead(H: Record "BA Head"): Text
    var
        Callee: Codeunit "BA Callee";
    begin
        exit(Callee.DescribeVar(H));
    end;
}

codeunit 50184 "BA Callee"
{
    procedure Describe(H: Record "BA Head"): Text
    begin
        exit(DescribeVar(H));
    end;

    procedure DescribeVar(var H: Record "BA Head"): Text
    var
        Result: Text;
    begin
        Result := 'no=' + H."No." + ' total=' + Format(H."Line Total", 0, 9) + ' f={' + H.GetFilters() + '} n=' + Format(H.Count());
        H.FindLast();
        exit(Result + ' last=' + H."No." + ' lastTotal=' + Format(H."Line Total", 0, 9));
    end;

    procedure DescribeVariant(Value: Variant): Text
    var
        RecRef: RecordRef;
        Rows: Integer;
    begin
        RecRef.GetTable(Value);
        Rows := RecRef.Count();
        RecRef.FindLast();
        exit(StrSubstNo('no=%1 f={%2} n=%3 lastTotal=%4', RecRef.Field(1).Value, RecRef.GetFilters(), Rows, Format(RecRef.Field(3).Value, 0, 9)));
    end;

    procedure GetOfCopy(H: Record "BA Head"): Text
    begin
        H.Get('H3');
        exit('total=' + Format(H."Line Total", 0, 9));
    end;

    procedure CopyOfCopy(H: Record "BA Head"): Text
    var
        Other: Record "BA Head";
    begin
        Other.Copy(H);
        Other.FindLast();
        exit('total=' + Format(Other."Line Total", 0, 9));
    end;

    procedure ClearedOnCopy(H: Record "BA Head"): Text
    begin
        H.SetAutoCalcFields();
        H.FindLast();
        exit('total=' + Format(H."Line Total", 0, 9));
    end;
}

table 50186 "BA Holder"
{
    fields { field(1; "Line No."; Integer) { } field(2; Info; Text[250]) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }

    procedure FromTableCode(WithFilter: Boolean): Text
    var
        Head: Record "BA Head";
        Callee: Codeunit "BA Callee";
    begin
        if WithFilter then
            Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        exit(Callee.Describe(Head));
    end;
}

codeunit 50185 "BA Test"
{
    Subtype = Test;

    var
        Callee: Codeunit "BA Callee";

    local procedure Seed()
    var
        Head: Record "BA Head";
        Detail: Record "BA Detail";
    begin
        Head.DeleteAll();
        Detail.DeleteAll();
        Add('H1', 'A', 10);
        Add('H2', 'A', 20);
        Add('H3', 'B', 30);
    end;

    local procedure Add(No: Code[20]; Kind: Code[10]; Amount: Decimal)
    var
        Head: Record "BA Head";
        Detail: Record "BA Detail";
    begin
        Head."No." := No;
        Head.Kind := Kind;
        Head.Insert();
        Detail."Head No." := No;
        Detail."Line No." := 1;
        Detail.Amount := Amount;
        Detail.Insert();
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    local procedure ExpectCallerCalculates(var Head: Record "BA Head")
    begin
        Head.FindLast();
        if Head."Line Total" = 0 then
            Error('the caller lost its SetAutoCalcFields');
    end;

    local procedure DescribeCopy(HeadCopy: Record "BA Head"): Text
    begin
        exit(Callee.DescribeVariant(HeadCopy));
    end;

    // The reporter's repro of issue 8538.
    [Test]
    procedure Reporter_LocalProcedureVariantGetTable_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Expect(DescribeCopy(Head), 'no=H3 f={} n=3 lastTotal=0');
        ExpectCallerCalculates(Head);
    end;

    [Test]
    procedure Codeunit_Filter_AfterFind_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Expect(Callee.Describe(Head), 'no=H1 total=10 f={} n=3 last=H3 lastTotal=0');
    end;

    // Main kept SetAutoCalcFields on this copy (no filter, the plain view) and calculated 30.
    [Test]
    procedure Codeunit_NoFilter_NoRead_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.Describe(Head), 'no= total=0 f={} n=3 last=H3 lastTotal=0');
        ExpectCallerCalculates(Head);
    end;

    [Test]
    procedure TableProcOtherTable_SecondaryKey_NoFilter_Flagged()
    var
        Head: Record "BA Head";
        Buffer: Record "BA Buffer";
    begin
        Seed();
        Head.SetCurrentKey(Kind);
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Expect(Buffer.DescribeHead(Head), 'no=H1 total=10 f={} n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure TableProcOwnRec_Filter_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Expect(Head.DescribeOwn(Head), 'no=H1 total=10 f={} n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure AdHocKey_Refused()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetCurrentKey("No.", Kind);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.Describe(Head), '');
    end;

    [Test]
    procedure MarkedOnly_Refused()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.Get('H1');
        Head.Mark(true);
        Head.MarkedOnly(true);
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.Describe(Head), '');
    end;

    [Test]
    procedure Group4Filter_Refused()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.FilterGroup(4);
        Head.SetRange(Kind, 'A');
        Head.FilterGroup(0);
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.Describe(Head), '');
    end;

    // A page procedure is not a callee the by-value view rules measure: the copy keeps main's view, SetAutoCalcFields
    // included.
    [Test]
    procedure PageProcedure_NoFilter_KeepsMain()
    var
        Head: Record "BA Head";
        Heads: Page "BA Heads";
    begin
        Seed();
        Head.SetAutoCalcFields("Line Total");
        Expect(Heads.DescribeHead(Head), 'no= total=0 f={} n=3 last=H3 lastTotal=30');
    end;

    [Test]
    procedure GetOfCopy_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.GetOfCopy(Head), 'total=0');
    end;

    [Test]
    procedure CopyOfCopy_Flagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.CopyOfCopy(Head), 'total=0');
    end;

    // The copy's own SetAutoCalcFields() leaves nothing that rests on its caller's set.
    [Test]
    procedure ClearedOnCopy_NotFlagged()
    var
        Head: Record "BA Head";
    begin
        Seed();
        Head.SetAutoCalcFields("Line Total");
        Expect(Callee.ClearedOnCopy(Head), 'total=0');
    end;

    // A codeunit procedure called from table code with SetAutoCalcFields is not measured: refused with a filter, main's
    // view (SetAutoCalcFields kept) without one.
    [Test]
    procedure CodeunitFromTableCode_Filter_Refused()
    var
        Holder: Record "BA Holder";
    begin
        Seed();
        Expect(Holder.FromTableCode(true), '');
    end;

    [Test]
    procedure CodeunitFromTableCode_NoFilter_KeepsMain()
    var
        Holder: Record "BA Holder";
    begin
        Seed();
        Expect(Holder.FromTableCode(false), 'no= total=0 f={} n=3 last=H3 lastTotal=30');
    end;
}
"##;

#[test]
fn a_by_value_copy_drops_the_callers_set_auto_calc_fields_in_the_measured_shapes() {
    let o = run(AUTO_CALC, "BA Test");
    let want = [
        ("Reporter_LocalProcedureVariantGetTable_Flagged", PASS, true),
        ("Codeunit_Filter_AfterFind_Flagged", PASS, true),
        ("Codeunit_NoFilter_NoRead_Flagged", PASS, true),
        ("TableProcOtherTable_SecondaryKey_NoFilter_Flagged", PASS, true),
        ("TableProcOwnRec_Filter_Flagged", PASS, true),
        ("AdHocKey_Refused", VIEW_REFUSED, false),
        ("MarkedOnly_Refused", VIEW_REFUSED, false),
        ("Group4Filter_Refused", VIEW_REFUSED, false),
        ("PageProcedure_NoFilter_KeepsMain", PASS, false),
        ("GetOfCopy_Flagged", PASS, true),
        ("CopyOfCopy_Flagged", PASS, true),
        ("ClearedOnCopy_NotFlagged", PASS, false),
        ("CodeunitFromTableCode_Filter_Refused", VIEW_REFUSED, false),
        ("CodeunitFromTableCode_NoFilter_KeepsMain", PASS, false),
    ];
    check(&o, &want);
}
