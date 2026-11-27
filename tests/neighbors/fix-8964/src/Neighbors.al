// Neighbors of the kinds store (assumption clear-all-store-global-kinds): each shape a fact the new cells do not read.
codeunit 86302 "NKK Byte Store"
{
    var
        I: Integer;
        By: Byte;

    procedure Mark()
    begin
        I := 5;
        By := 1;
    end;

    procedure Get(): Integer
    begin
        exit(I);
    end;
}

codeunit 86303 "NKK Owner"
{
    var
        Store: Codeunit "CKK Mid";
        ByteStore: Codeunit "NKK Byte Store";

    procedure SetStore(var S: Codeunit "CKK Mid")
    begin
        Store := S;
    end;

    procedure SetByteStore(var S: Codeunit "NKK Byte Store")
    begin
        ByteStore := S;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 86304 "NKK SI Owner"
{
    SingleInstance = true;

    var
        Store: Codeunit "CKK Mid";

    procedure SetStore(var S: Codeunit "CKK Mid")
    begin
        Store := S;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 86305 "NKK Neighbor Tests"
{
    Subtype = Test;

    // A global kind the probe does not measure (Byte): refused as on main.
    [Test]
    procedure KindsStore_UnmeasuredByteGlobal()
    var
        S: Codeunit "NKK Byte Store";
        O: Codeunit "NKK Owner";
    begin
        S.Mark();
        O.SetByteStore(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A kinds store a call made but that wrote nothing, plain owner: not measured, refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten()
    var
        S: Codeunit "CKK Mid";
        O: Codeunit "NKK Owner";
    begin
        S.Set(0);
        O.SetStore(S);
        O.ClearAllGlobals();
    end;

    // A SingleInstance owner of a kinds store: not measured, refused as on main.
    [Test]
    procedure KindsStore_SingleInstanceOwner()
    var
        S: Codeunit "CKK Mid";
        O: Codeunit "NKK SI Owner";
    begin
        S.Set(3);
        O.SetStore(S);
        O.ClearAllGlobals();
    end;
}
