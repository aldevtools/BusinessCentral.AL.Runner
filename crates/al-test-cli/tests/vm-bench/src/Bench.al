// The benchmark app of the VM speed budget (ADR-031, crates/al-test-cli/tests/perf_vm.rs). Our own code, no Microsoft
// source. One test codeunit per benchmark, because a test codeunit is the unit that runs on a session of its own.
// The iteration counts below are the ones perf_vm.rs divides by; change both together.

table 70000 "Bench Row"
{
    TableType = Temporary;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Amount; Decimal) { }
        field(3; Description; Text[100]) { }
        field(4; Qty; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 70009 "Bench Blob Row"
{
    TableType = Temporary;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Data; Blob) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// What a run costs besides its loop: the session and the engine. perf_vm.rs subtracts it.
codeunit 70001 "Bench Empty"
{
    Subtype = Test;

    [Test]
    procedure Empty()
    begin
    end;
}

// 300000 iterations
codeunit 70002 "Bench Int Loop"
{
    Subtype = Test;

    [Test]
    procedure IntLoop()
    var
        i: Integer;
        s: Integer;
    begin
        for i := 1 to 300000 do
            s := (s + i) mod 1000003;
    end;
}

// 100000 iterations
codeunit 70003 "Bench Decimal Loop"
{
    Subtype = Test;

    [Test]
    procedure DecimalLoop()
    var
        i: Integer;
        d: Decimal;
    begin
        for i := 1 to 100000 do
            d := d + i * 1.5 - 0.25;
    end;
}

// 100000 iterations
codeunit 70004 "Bench Call Loop"
{
    Subtype = Test;

    [Test]
    procedure CallLoop()
    var
        i: Integer;
        s: Integer;
    begin
        for i := 1 to 100000 do
            s := Add(s, i);
    end;

    local procedure Add(a: Integer; b: Integer): Integer
    begin
        exit((a + b) mod 1000003);
    end;
}

// 30000 iterations
codeunit 70005 "Bench Text Loop"
{
    Subtype = Test;

    [Test]
    procedure TextLoop()
    var
        i: Integer;
        t: Text;
        n: Integer;
    begin
        for i := 1 to 30000 do begin
            t := 'Item ' + Format(i mod 97);
            n += StrLen(CopyStr(t, 2, 3));
            if StrPos(t, '5') > 0 then
                n += 1;
        end;
    end;
}

// 50000 iterations
codeunit 70006 "Bench Builtin Loop"
{
    Subtype = Test;

    [Test]
    procedure BuiltinLoop()
    var
        i: Integer;
        d: Decimal;
    begin
        for i := 1 to 50000 do
            d += Round(Abs(i / 3), 0.01) + Power(2, 3);
    end;
}

// 5000 rows: 5000 Init/Insert, one SetRange + FindSet/Next pass, 5000 Get/Modify (15000 record operations)
codeunit 70007 "Bench Record Loop"
{
    Subtype = Test;

    [Test]
    procedure RecordLoop()
    var
        R: Record "Bench Row";
        i: Integer;
        sum: Integer;
    begin
        for i := 1 to 5000 do begin
            R.Init();
            R."No." := i;
            R.Amount := i;
            R.Description := 'row';
            R.Qty := i mod 5;
            R.Insert();
        end;
        R.SetRange(Qty, 3);
        if R.FindSet() then
            repeat
                sum += R.Qty;
            until R.Next() = 0;
        R.Reset();
        for i := 1 to 5000 do begin
            R.Get(i);
            R.Amount := R.Amount + 1;
            R.Modify();
        end;
    end;
}

// A table of 24 fields, 8 of them FlowFields placed between the others (al-runner issue 5911: FieldIndex cost grew with the
// number of FlowFields). Never inserted into: the FlowFields are never calculated.
table 70001 "Bench Flow Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; "Code 2"; Code[20]) { }
        field(3; "Code 3"; Code[20]) { }
        field(4; "Flow 4"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 3" = field("Code 3"))); Editable = false; }
        field(5; "Code 5"; Code[20]) { }
        field(6; "Flow 6"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 5" = field("Code 5"))); Editable = false; }
        field(7; "Code 7"; Code[20]) { }
        field(8; "Code 8"; Code[20]) { }
        field(9; "Flow 9"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 8" = field("Code 8"))); Editable = false; }
        field(10; "Code 10"; Code[20]) { }
        field(11; "Code 11"; Code[20]) { }
        field(12; "Flow 12"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 11" = field("Code 11"))); Editable = false; }
        field(13; "Code 13"; Code[20]) { }
        field(14; "Code 14"; Code[20]) { }
        field(15; "Flow 15"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 14" = field("Code 14"))); Editable = false; }
        field(16; "Code 16"; Code[20]) { }
        field(17; "Code 17"; Code[20]) { }
        field(18; "Flow 18"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 17" = field("Code 17"))); Editable = false; }
        field(19; "Code 19"; Code[20]) { }
        field(20; "Code 20"; Code[20]) { }
        field(21; "Flow 21"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 20" = field("Code 20"))); Editable = false; }
        field(22; "Code 22"; Code[20]) { }
        field(23; "Flow 23"; Integer) { FieldClass = FlowField; CalcFormula = count("Bench Flow Row" where("Code 22" = field("Code 22"))); Editable = false; }
        field(24; "Code 24"; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// 24000 iterations: 1000 passes over the 24 fields of a table with FlowFields, one RecordRef.FieldIndex each
codeunit 70008 "Bench FieldIndex Loop"
{
    Subtype = Test;

    [Test]
    procedure FieldIndexLoop()
    var
        Rr: RecordRef;
        Fr: FieldRef;
        i: Integer;
        f: Integer;
        n: Integer;
    begin
        Rr.Open(Database::"Bench Flow Row");
        for i := 1 to 1000 do
            for f := 1 to 24 do begin
                Fr := Rr.FieldIndex(f);
                n += Fr.Number();
            end;
    end;
}

// 100000 iterations: each is one OutStream.Write and one InStream.Read of a Byte on a temporary Blob (the loop shape of the
// GZip trailer codeunit 75290, which the 2 s wall budget of perf_budget.rs only catches when it gets several times slower)
codeunit 70009 "Bench Stream Byte Loop"
{
    Subtype = Test;

    [Test]
    procedure StreamByteLoop()
    var
        Row: Record "Bench Blob Row" temporary;
        OutS: OutStream;
        InS: InStream;
        B: Byte;
        S: Byte;
        i: Integer;
    begin
        Row.Data.CreateOutStream(OutS);
        for i := 1 to 100000 do begin
            B := i mod 256;
            OutS.Write(B);
        end;
        Row.Insert();
        Row.CalcFields(Data);
        Row.Data.CreateInStream(InS);
        for i := 1 to 100000 do begin
            InS.Read(S);
            B := S;
        end;
    end;
}
