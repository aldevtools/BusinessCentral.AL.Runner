// Review round 1 neighbors of the kinds store (assumption clear-all-store-global-kinds): a store a call made but that
// wrote nothing, for each handle kind whose untouched value has no comparable form; a Label with a property; an Interface
// global that holds an instance of a codeunit outside the measured kinds. Each refuses as on main.
codeunit 86306 "NKR JsonObject Store"
{
    var
        N: Integer;
        X: JsonObject;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86307 "NKR JsonArray Store"
{
    var
        N: Integer;
        X: JsonArray;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86308 "NKR TextBuilder Store"
{
    var
        N: Integer;
        X: TextBuilder;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86309 "NKR RecordRef Store"
{
    var
        N: Integer;
        X: RecordRef;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86310 "NKR FieldRef Store"
{
    var
        N: Integer;
        X: FieldRef;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86311 "NKR KeyRef Store"
{
    var
        N: Integer;
        X: KeyRef;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86312 "NKR Label Store"
{
    var
        N: Integer;
        L: Label 'Shown', Comment = 'A label with a property';

    procedure Mark()
    begin
        N := 4;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;

    procedure Caption(): Text
    begin
        exit(L);
    end;
}

codeunit 86313 "NKR Byte Leaf" implements "CKK Face"
{
    var
        V: Integer;
        By: Byte;

    procedure Set(NewV: Integer)
    begin
        V := NewV;
        By := 1;
    end;

    procedure Get(): Integer
    begin
        exit(V);
    end;
}

codeunit 86314 "NKR Face Store"
{
    var
        N: Integer;
        F: Interface "CKK Face";

    procedure Mark()
    var
        Leaf: Codeunit "NKR Byte Leaf";
    begin
        Leaf.Set(2);
        F := Leaf;
        N := 1;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 86315 "NKR Owner"
{
    var
        JsonObjectStore: Codeunit "NKR JsonObject Store";
        JsonArrayStore: Codeunit "NKR JsonArray Store";
        TextBuilderStore: Codeunit "NKR TextBuilder Store";
        RecordRefStore: Codeunit "NKR RecordRef Store";
        FieldRefStore: Codeunit "NKR FieldRef Store";
        KeyRefStore: Codeunit "NKR KeyRef Store";
        LabelStore: Codeunit "NKR Label Store";
        FaceStore: Codeunit "NKR Face Store";

    procedure SetJsonObject(var S: Codeunit "NKR JsonObject Store")
    begin
        JsonObjectStore := S;
    end;

    procedure SetJsonArray(var S: Codeunit "NKR JsonArray Store")
    begin
        JsonArrayStore := S;
    end;

    procedure SetTextBuilder(var S: Codeunit "NKR TextBuilder Store")
    begin
        TextBuilderStore := S;
    end;

    procedure SetRecordRef(var S: Codeunit "NKR RecordRef Store")
    begin
        RecordRefStore := S;
    end;

    procedure SetFieldRef(var S: Codeunit "NKR FieldRef Store")
    begin
        FieldRefStore := S;
    end;

    procedure SetKeyRef(var S: Codeunit "NKR KeyRef Store")
    begin
        KeyRefStore := S;
    end;

    procedure SetLabel(var S: Codeunit "NKR Label Store")
    begin
        LabelStore := S;
    end;

    procedure SetFace(var S: Codeunit "NKR Face Store")
    begin
        FaceStore := S;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// The TableNo owner (the Sales-Post shape): ClearAll in a local procedure that OnRun calls.
codeunit 86316 "NKR TNo Owner"
{
    TableNo = "CKK Row";

    var
        Store: Codeunit "NKR JsonObject Store";

    trigger OnRun()
    begin
        ClearAllVariables();
    end;

    local procedure ClearAllVariables()
    begin
        ClearAll();
    end;

    procedure SetStore(var S: Codeunit "NKR JsonObject Store")
    begin
        Store := S;
    end;
}

codeunit 86317 "NKR Review Tests"
{
    Subtype = Test;

    // A store with an untouched JsonObject global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_JsonObject()
    var
        S: Codeunit "NKR JsonObject Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetJsonObject(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A store with an untouched JsonArray global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_JsonArray()
    var
        S: Codeunit "NKR JsonArray Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetJsonArray(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A store with an untouched TextBuilder global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_TextBuilder()
    var
        S: Codeunit "NKR TextBuilder Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetTextBuilder(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A store with an untouched RecordRef global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_RecordRef()
    var
        S: Codeunit "NKR RecordRef Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetRecordRef(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A store with an untouched FieldRef global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_FieldRef()
    var
        S: Codeunit "NKR FieldRef Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetFieldRef(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A store with an untouched KeyRef global that a call made but that wrote nothing, plain owner: refused as on main.
    [Test]
    procedure KindsStore_PlainOwner_CalledUnwritten_KeyRef()
    var
        S: Codeunit "NKR KeyRef Store";
        O: Codeunit "NKR Owner";
    begin
        S.Get();
        O.SetKeyRef(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // The same store with an untouched JsonObject, TableNo owner run by Codeunit.Run: refused as on main.
    [Test]
    procedure KindsStore_TableNoOwner_CalledUnwritten_JsonObject()
    var
        S: Codeunit "NKR JsonObject Store";
        O: Codeunit "NKR TNo Owner";
        Row: Record "CKK Row";
    begin
        S.Get();
        O.SetStore(S);
        O.Run(Row);
        if S.Get() <> 0 then
            Error('not reset');
    end;

    // A Label with a Comment, which the probe does not measure: refused as on main.
    [Test]
    procedure KindsStore_LabelWithComment()
    var
        S: Codeunit "NKR Label Store";
        O: Codeunit "NKR Owner";
    begin
        S.Mark();
        O.SetLabel(S);
        O.ClearAllGlobals();
        if (S.Get() <> 0) or (S.Caption() <> 'Shown') then
            Error('not reset');
    end;

    // An Interface global that holds an instance of a codeunit with a Byte global: refused as on main.
    [Test]
    procedure KindsStore_InterfaceHoldsUnmeasured()
    var
        S: Codeunit "NKR Face Store";
        O: Codeunit "NKR Owner";
    begin
        S.Mark();
        O.SetFace(S);
        O.ClearAllGlobals();
        if S.Get() <> 0 then
            Error('not reset');
    end;
}
