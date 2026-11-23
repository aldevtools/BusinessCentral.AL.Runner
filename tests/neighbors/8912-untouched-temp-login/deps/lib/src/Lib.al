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
        LabelOwner: Codeunit "UT Label Store Owner";
        ExtraOwner: Codeunit "UT Extra Store Owner";
        SwappedOwner: Codeunit "UT Swapped Store Owner";
        ArrOwner: Codeunit "UT Arr Store Owner";
        WrapOwner: Codeunit "UT Wrapped Store Owner";
        RowOwner: Codeunit "UT Store Row Owner";
    begin
        LabelOwner.Use();
        ExtraOwner.Use();
        SwappedOwner.Use();
        ArrOwner.Use();
        WrapOwner.Use();
        RowOwner.Use();
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

// Neighbors of the store shape of corpus PR 1561 (a temporary record, an array of RecordRef of two dimensions, an array of
// Integer): the store with a Label besides (Backup Storage has one), with one more global, with the globals in another
// order, with an array of temporary records, held one level deeper, and written by the login.
codeunit 61864 "UT Label Store"
{
    var
        Rows: Record "UT Row" temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        NotFoundErr: Label 'Could not find %1.';

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count() + Indices[1] + Backups[1, 1].Number);
    end;
}

codeunit 61865 "UT Extra Store"
{
    var
        Rows: Record "UT Row" temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        Extra: Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1] + Extra);
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count() + Indices[1] + Backups[1, 1].Number);
    end;
}

codeunit 61866 "UT Swapped Store"
{
    var
        Backups: array[2, 3] of RecordRef;
        Rows: Record "UT Row" temporary;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count() + Indices[1] + Backups[1, 1].Number);
    end;
}

codeunit 61867 "UT Arr Store"
{
    var
        Rows: array[2] of Record "UT Row" temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;

    procedure Count(): Integer
    begin
        exit(Rows[1].Count() + Indices[1] + Backups[1, 1].Number);
    end;
}

codeunit 61868 "UT Store Wrap"
{
    var
        Inner: Codeunit "UT Store";

    procedure Get(): Integer
    begin
        exit(Inner.Get());
    end;

    procedure Count(): Integer
    begin
        exit(Inner.Count());
    end;
}

codeunit 61869 "UT Label Store Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Label Store";

    procedure Use()
    begin
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

codeunit 61870 "UT Extra Store Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Extra Store";

    procedure Use()
    begin
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

codeunit 61871 "UT Swapped Store Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Swapped Store";

    procedure Use()
    begin
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

codeunit 61872 "UT Arr Store Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Arr Store";

    procedure Use()
    begin
        Store.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

codeunit 61873 "UT Wrapped Store Owner"
{
    SingleInstance = true;

    var
        Wrap: Codeunit "UT Store Wrap";

    procedure Use()
    begin
        Wrap.Get();
    end;

    procedure Count(): Integer
    begin
        exit(Wrap.Count());
    end;
}

codeunit 61874 "UT Row Store"
{
    var
        Rows: Record "UT Row" temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Insert()
    begin
        Rows.Id := 1;
        Rows.Insert();
    end;

    procedure Count(): Integer
    begin
        exit(Rows.Count() + Indices[1] + Backups[1, 1].Number);
    end;
}

codeunit 61875 "UT Store Row Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "UT Row Store";

    procedure Use()
    begin
        Store.Insert();
    end;

    procedure Count(): Integer
    begin
        exit(Store.Count());
    end;
}

