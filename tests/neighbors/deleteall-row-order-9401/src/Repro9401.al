// The repro of al-runner issue 9401, with the MEASURED errors turned into asserts on the values the reporter measured on BC 28.4.
table 91640 "PZE Row"
{
    fields
    {
        field(1; "No."; Integer) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }

    trigger OnDelete()
    var
        Log: Codeunit "PZE Log";
    begin
        Log.Add(Format("No."));
    end;
}

codeunit 91641 "PZE Log"
{
    SingleInstance = true;
    var
        Seen: Text;

    procedure Add(T: Text)
    begin
        if Seen <> '' then
            Seen += ',';
        Seen += T;
    end;

    procedure Take(): Text
    var
        T: Text;
    begin
        T := Seen;
        Seen := '';
        exit(T);
    end;
}

codeunit 91642 "PZE Helper"
{
    procedure DeleteAllIn(Descending: Boolean): Text
    var
        R: Record "PZE Row";
        Log: Codeunit "PZE Log";
        i: Integer;
    begin
        Log.Take();
        for i := 1 to 3 do begin
            R.Init();
            R."No." := i;
            R.Insert();
        end;
        R.Reset();
        R.Ascending(not Descending);
        R.DeleteAll(true);
        exit(StrSubstNo('order=[%1] left=%2', Log.Take(), R.Count()));
    end;
}

codeunit 91643 "PZE Descending Test"
{
    Subtype = Test;
    [Test]
    procedure DeleteAllDescending()
    var
        H: Codeunit "PZE Helper";
        Assert: Codeunit "DRN Assert";
    begin
        Assert.AreEqual('order=[3,2,1] left=0', H.DeleteAllIn(true), 'descending'); // BC 28.4
    end;
}

codeunit 91644 "PZE Ascending Test"
{
    Subtype = Test;
    [Test]
    procedure DeleteAllAscending()
    var
        H: Codeunit "PZE Helper";
        Assert: Codeunit "DRN Assert";
    begin
        Assert.AreEqual('order=[1,2,3] left=0', H.DeleteAllIn(false), 'ascending'); // BC 28.4
    end;
}
