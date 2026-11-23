// Neighbors of al-runner issue 8839: a caller sets SetLoadFields, reads a row and hands the record by value. The new rule reads
// the callee (a codeunit's procedure), the caller (a codeunit), the view (every filter in group 0, no ad hoc key, ascending,
// no marks) and the record's state (no SetAutoCalcFields or ReadIsolation). Each shape is a test codeunit of its own, so a
// refusal does not reach the next one. The shapes the rule runs come first; the ones it leaves refused come last.
table 50000 "NB LR Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(3; Status; Option) { OptionMembers = Open,Posted; }
        field(4; Amount; Decimal) { }
        field(5; Doc; Code[10]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByStatusDoc; Status, Doc) { }
    }

    procedure DescribeOwn(R: Record "NB LR Row"): Text
    var
        Callee: Codeunit "NB LR Callee";
    begin
        exit(Callee.DescribeCopy(R));
    end;

    // Table code that hands its own record on: the caller is not a codeunit.
    procedure HandFromTable(): Text
    var
        Callee: Codeunit "NB LR Callee";
    begin
        Rec.SetRange(Status, Rec.Status::Posted);
        Rec.SetLoadFields(Name);
        Rec.FindFirst();
        exit(Callee.Describe(Rec));
    end;
}

codeunit 50001 "NB LR Callee"
{
    procedure Describe(R: Record "NB LR Row"): Text
    begin
        exit(DescribeCopy(R));
    end;

    // Which field the copy has loaded as handed, its view, and whether its own FindSet loads every field.
    procedure DescribeCopy(var R: Record "NB LR Row"): Text
    var
        Result: Text;
    begin
        Result := 'loaded=' + Format(R.AreFieldsLoaded(R.Amount)) + ' f={' + R.GetFilters() + '} key=' + R.CurrentKey();
        R.SetRange(Status, R.Status::Posted);
        R.FindSet();
        exit(Result + ' after=' + Format(R.AreFieldsLoaded(R.Amount)) + ' amount=' + Format(R.Amount));
    end;
}

// The codeunit 5895 "Inventory Adjustment" shape: a codeunit that is not a test codeunit reads rows and hands each one to its
// own local procedure.
codeunit 50002 "NB LR Loop"
{
    procedure Collect(): Text
    var
        Row: Record "NB LR Row";
        Result: Text;
    begin
        Row.SetCurrentKey(Status, Doc);
        Row.SetRange(Status, Row.Status::Posted);
        LoadFields(Row);
        Row.FindSet();
        repeat
            Row.SetRange(Doc, Row.Doc);
            Result += DescribeLocal(Row) + ';';
            Row.SetRange(Doc);
        until Row.Next() = 0;
        exit(Result);
    end;

    local procedure DescribeLocal(R: Record "NB LR Row"): Text
    var
        Callee: Codeunit "NB LR Callee";
    begin
        exit(Callee.DescribeCopy(R));
    end;

    local procedure LoadFields(var Row: Record "NB LR Row")
    begin
        Row.SetLoadFields(Id, Status, Doc);
    end;
}

interface "NB LR IDescribe"
{
    procedure Describe(R: Record "NB LR Row"): Text;
}

codeunit 50003 "NB LR Impl" implements "NB LR IDescribe"
{
    procedure Describe(R: Record "NB LR Row"): Text
    var
        Callee: Codeunit "NB LR Callee";
    begin
        exit(Callee.DescribeCopy(R));
    end;
}

report 50004 "NB LR Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB LR Row")
        {
        }
    }

    procedure Describe(R: Record "NB LR Row"): Text
    var
        Callee: Codeunit "NB LR Callee";
    begin
        exit(Callee.DescribeCopy(R));
    end;
}

codeunit 50005 "NB LR Seed"
{
    procedure Seed()
    var
        Row: Record "NB LR Row";
    begin
        Row.DeleteAll();
        Insert(1, 'A', 0, 10, 'D1');
        Insert(2, 'B', 1, 20, 'D2');
        Insert(3, 'C', 1, 30, 'D3');
    end;

    local procedure Insert(Id: Integer; Name: Text[20]; Status: Option; Amount: Decimal; Doc: Code[10])
    var
        Row: Record "NB LR Row";
    begin
        Row.Id := Id;
        Row.Name := Name;
        Row.Status := Status;
        Row.Amount := Amount;
        Row.Doc := Doc;
        Row.Insert();
    end;

    procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;
}

codeunit 50010 "NB LR Filter FindSet"
{
    Subtype = Test;

    [Test]
    procedure LoadFieldsThenFindSet_Flagged()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindSet();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50011 "NB LR Read Then Load"
{
    Subtype = Test;

    [Test]
    procedure FindFirstThenLoadFields_Flagged()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.FindFirst();
        Row.SetLoadFields(Name);
        Seed.Expect(Callee.Describe(Row), 'loaded=Yes f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50012 "NB LR No Filter"
{
    Subtype = Test;

    // Main ran this shape and kept the caller's SetLoadFields on the copy, so the copy's FindSet left Amount unloaded.
    [Test]
    procedure NoFilterThenFindFirst_Flagged()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50013 "NB LR Codeunit Loop"
{
    Subtype = Test;

    [Test]
    procedure CodeunitLoopToLocalProcedure_Flagged()
    var
        Seed: Codeunit "NB LR Seed";
        Loop: Codeunit "NB LR Loop";
    begin
        Seed.Seed();
        Seed.Expect(Loop.Collect(), 'loaded=No f={} key=Id after=Yes amount=20;loaded=No f={} key=Id after=Yes amount=20;');
    end;
}

codeunit 50014 "NB LR Get"
{
    Subtype = Test;

    [Test]
    procedure LoadFieldsThenGet_Flagged()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.Get(2);
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50020 "NB LR Group 2"
{
    Subtype = Test;

    [Test]
    procedure FilterInGroup2_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.FilterGroup(2);
        Row.SetRange(Status, Row.Status::Posted);
        Row.FilterGroup(0);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50021 "NB LR Ad Hoc Key"
{
    Subtype = Test;

    [Test]
    procedure AdHocKey_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetCurrentKey(Amount);
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50022 "NB LR Descending"
{
    Subtype = Test;

    [Test]
    procedure Descending_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.Ascending(false);
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50023 "NB LR Marks"
{
    Subtype = Test;

    [Test]
    procedure Marks_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Row.Mark(true);
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50024 "NB LR Table Procedure"
{
    Subtype = Test;

    [Test]
    procedure ToTableProcedure_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Row.DescribeOwn(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50025 "NB LR Report Procedure"
{
    Subtype = Test;

    [Test]
    procedure ToReportProcedure_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Rpt: Report "NB LR Report";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Rpt.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50026 "NB LR Interface"
{
    Subtype = Test;

    [Test]
    procedure ThroughInterface_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Impl: Codeunit "NB LR Impl";
        Describer: Interface "NB LR IDescribe";
    begin
        Seed.Seed();
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Describer := Impl;
        Seed.Expect(Describer.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50027 "NB LR From Table"
{
    Subtype = Test;

    [Test]
    procedure FromTableCode_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
    begin
        Seed.Seed();
        Seed.Expect(Row.HandFromTable(), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}

codeunit 50028 "NB LR Read Isolation"
{
    Subtype = Test;

    [Test]
    procedure ReadIsolation_Refused()
    var
        Row: Record "NB LR Row";
        Seed: Codeunit "NB LR Seed";
        Callee: Codeunit "NB LR Callee";
    begin
        Seed.Seed();
        Row.ReadIsolation := IsolationLevel::ReadCommitted;
        Row.SetRange(Status, Row.Status::Posted);
        Row.SetLoadFields(Name);
        Row.FindFirst();
        Seed.Expect(Callee.Describe(Row), 'loaded=No f={} key=Id after=Yes amount=20');
    end;
}
