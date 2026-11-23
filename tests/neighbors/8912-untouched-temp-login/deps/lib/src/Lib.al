// A stand-in test library (publisher Microsoft, invented code) whose SingleInstance codeunits hold temporary records, as
// Tests-TestLibraries' "Backup Management" does. Its "UT Ping" subscriber runs inside the partner's login and leaves each
// codeunit in one shape.
table 61840 "UT Row"
{
    TableType = Temporary;

    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// A temporary record and a codeunit global: the login makes the instance and uses nothing.
codeunit 61841 "UT Untouched"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;
        Store: Codeunit "UT Store";

    procedure Touch()
    begin
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count() + Store.Get());
    end;
}

// The login reads the temporary table (IsEmpty) and writes nothing.
codeunit 61842 "UT Read"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;

    procedure Read()
    begin
        if not Rows.IsEmpty() then
            Error('rows');
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count());
    end;
}

// The login makes the codeunit global's instance by a first call; its globals keep their declared values.
codeunit 61843 "UT Used Store"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Store";

    procedure Use()
    begin
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

// The login inserts a row and deletes it again: the table is empty but was written.
codeunit 61844 "UT Written"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;

    procedure Write()
    begin
        Rows.Id := 1;
        Rows.Insert();
        Rows.Delete();
        Rows.Id := 0;
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count());
    end;
}

// The login shares the table with a second global (Copy(Rows, true)).
codeunit 61845 "UT Shared"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;
        Other: Record "UT Row" temporary;

    procedure Share()
    begin
        Other.Copy(Rows, true);
    end;

    procedure Count(): Integer
    begin
        Other.Id := 5;
        Other.Insert();
        exit(Rows.Count());
    end;
}

// The login sets a filter.
codeunit 61846 "UT Filtered"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;

    procedure Filter()
    begin
        Rows.SetRange(Id, 1);
    end;

    procedure Count(): Integer
    begin
        Rows.Id := 2;
        Rows.Insert();
        exit(Rows.Count());
    end;
}

// A plain codeunit: a temporary record, an array of RecordRef and Integers, as Backup Storage holds.
codeunit 61847 "UT Store"
{
    var
        Rows: Record "UT Row" temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;

    procedure OpenRef()
    begin
        Backups[1, 1].Open(Database::"UT Row", true);
    end;

    procedure Count(): Integer
    var
        Open: Integer;
    begin
        if Backups[1, 1].Number <> 0 then
            Open := 100;
        exit(Rows.Count() + Indices[1] + Open);
    end;
}

// The login opens a RecordRef of the codeunit global's instance.
codeunit 61848 "UT Opened Ref"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Store";

    procedure Open()
    begin
        Store.OpenRef();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

// An array of temporary records: its elements share one table (record/temporary/RecordTempArrayElements.al:67-93); the login
// makes the instance and uses nothing.
codeunit 61860 "UT Temp Array"
{
    SingleInstance = true;

    var
        Arr: array[2] of Record "UT Row" temporary;

    procedure Touch()
    begin
    end;

    procedure Count(): Integer
    begin
        exit(Arr[1].Count());
    end;
}

// The login makes the codeunit global's instance by a first call and hands it to a second global.
codeunit 61861 "UT Aliased Store"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Plain Store";
        Alias: Codeunit "UT Plain Store";

    procedure Share()
    begin
        Alias := Store;
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        Alias.Bump();
        exit(Store.Get());
    end;
}

// The login keeps a RecordRef on the temporary record (RecordRef.GetTable).
codeunit 61862 "UT Ref Kept"
{
    SingleInstance = true;

    var
        Rows: Record "UT Row" temporary;
        Ref: RecordRef;

    procedure Keep()
    begin
        Ref.GetTable(Rows);
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count());
    end;
}

codeunit 61849 "UT Lib Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UT Ping", 'OnPing', '', false, false)]
    local procedure OnPing()
    var
        Untouched: Codeunit "UT Untouched";
        Read: Codeunit "UT Read";
        UsedStore: Codeunit "UT Used Store";
        Written: Codeunit "UT Written";
        Shared: Codeunit "UT Shared";
        Filtered: Codeunit "UT Filtered";
        OpenedRef: Codeunit "UT Opened Ref";
        TempArray: Codeunit "UT Temp Array";
        Aliased: Codeunit "UT Aliased Store";
        RefKept: Codeunit "UT Ref Kept";
    begin
        TempArray.Touch();
        Aliased.Share();
        RefKept.Keep();
        Untouched.Touch();
        Read.Read();
        UsedStore.Use();
        Written.Write();
        Shared.Share();
        Filtered.Filter();
        OpenedRef.Open();
    end;
}

// A plain codeunit with one Integer, the probe's store shape.
codeunit 61863 "UT Plain Store"
{
    var
        N: Integer;

    procedure Get(): Integer
    begin
        exit(N);
    end;

    procedure Bump()
    begin
        N += 1;
    end;
}
