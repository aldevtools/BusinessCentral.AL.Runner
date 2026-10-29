// The reporter's repro of al-runner issue 6117 (green on real BC), app and test files in one source.
table 50100 "Block Edge Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Code[20]) { }
        field(3; Blocked; Boolean) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50101 "Block Edge Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}

page 50102 "Block Edge Logged List"
{
    PageType = List;
    SourceTable = "Block Edge Row";
    Editable = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { }
                field(Name; Rec.Name) { }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R' + Format(Rec.Id));
    end;

    var
        Log: Codeunit "Block Edge Log";
}

page 50103 "Block Edge Plain List"
{
    PageType = List;
    SourceTable = "Block Edge Row";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { }
                field(Name; Rec.Name) { }
                field(Blocked; Rec.Blocked) { }
            }
        }
    }
}

codeunit 50104 "Block Edge Picker"
{
    procedure PickUnblocked(): Integer
    var
        Row: Record "Block Edge Row";
    begin
        Row.SetRange(Blocked, false);
        if Page.RunModal(Page::"Block Edge Plain List", Row) = Action::LookupOK then
            exit(Row.Id);
        exit(-1);
    end;
}

codeunit 50121 "Block Edge Controls"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        Log: Codeunit "Block Edge Log";
        Target: Integer;
        SeenInLookup: Text;

    [Test]
    procedure GoToKeyInsideBlockControl()
    var
        P: TestPage "Block Edge Logged List";
        Observed: Text;
    begin
        SeedRows(45);
        P.OpenView();
        Log.Take();
        P.GoToKey(30);
        Observed := 'gotokey=' + P.Id.Value() + '/' + P.Name.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('gotokey=30/N30:R19;R20;R21;R22;R23;R24;R25;R26;R27;R28;R29;R30;R31;R32;R33;R34;R35;R36;R37;R38;R39;R40; next=31:', Observed, 'GoToKey onto row 30 of 45.');
    end;

    [Test]
    [HandlerFunctions('PickHandler')]
    procedure LookupGoToRecordInsideBlockControl()
    var
        Picker: Codeunit "Block Edge Picker";
        Picked: Integer;
    begin
        SeedRows(45);
        Target := 30;
        Picked := Picker.PickUnblocked();
        Assert.AreEqual('30 30/N30', Format(Picked) + ' ' + SeenInLookup, 'Lookup onto row 30 of 45.');
    end;

    [ModalPageHandler]
    procedure PickHandler(var Lookup: TestPage "Block Edge Plain List")
    var
        Row: Record "Block Edge Row";
    begin
        Row.Get(Target);
        Lookup.GoToRecord(Row);
        SeenInLookup := Lookup.Id.Value() + '/' + Lookup.Name.Value();
        Lookup.OK().Invoke();
    end;

    local procedure SeedRows(Count: Integer)
    var
        Row: Record "Block Edge Row";
        i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to Count do begin
            Row.Init();
            Row.Id := i;
            Row.Name := 'N' + Format(i);
            Row.Insert();
        end;
        Log.Take();
        SeenInLookup := '';
    end;
}

codeunit 50120 "Block Edge Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        Log: Codeunit "Block Edge Log";
        Target: Integer;
        SeenInLookup: Text;

    [Test]
    procedure GoToKeyLastRowOfFullBlockMoreRowsAfter()
    var
        P: TestPage "Block Edge Logged List";
        Observed: Text;
    begin
        SeedRows(45);
        P.OpenView();
        Log.Take();
        P.GoToKey(40);
        Observed := 'gotokey=' + P.Id.Value() + '/' + P.Name.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('gotokey=40/N40:R19;R20;R21;R22;R23;R24;R25;R26;R27;R28;R29;R30;R31;R32;R33;R34;R35;R36;R37;R38;R39;R40;R39;R40;R41;R42;R43;R44;R45; next=41:', Observed, 'GoToKey onto row 40 of 45.');
    end;

    [Test]
    procedure GoToKeyLastRowOfFullBlockFullBlockAfter()
    var
        P: TestPage "Block Edge Logged List";
        Observed: Text;
    begin
        SeedRows(80);
        P.OpenView();
        Log.Take();
        P.GoToKey(40);
        Observed := 'gotokey=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.GoToKey(62);
        Observed += ' gotokey62=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('gotokey=40:R19;R20;R21;R22;R23;R24;R25;R26;R27;R28;R29;R30;R31;R32;R33;R34;R35;R36;R37;R38;R39;R40;R39;R40;R41;R42;R43;R44;R45;R46;R47;R48;R49;R50;R51;R52;R53;R54;R55;R56;R57;R58;R59;R60; next=41: gotokey62=62:R59;R60;R61;R62;R63;R64;R65;R66;R67;R68;R69;R70;R71;R72;R73;R74;R75;R76;R77;R78;R79;R80;', Observed, 'GoToKey onto row 40 of 80.');
    end;

    [Test]
    [HandlerFunctions('PickHandler')]
    procedure LookupGoToRecordLastRowOfFullBlockThenOK()
    var
        Picker: Codeunit "Block Edge Picker";
        Picked: Integer;
    begin
        SeedRows(45);
        Target := 40;
        Picked := Picker.PickUnblocked();
        Assert.AreEqual('40 40/N40', Format(Picked) + ' ' + SeenInLookup, 'Lookup onto row 40 of 45.');
    end;

    [ModalPageHandler]
    procedure PickHandler(var Lookup: TestPage "Block Edge Plain List")
    var
        Row: Record "Block Edge Row";
    begin
        Row.Get(Target);
        Lookup.GoToRecord(Row);
        SeenInLookup := Lookup.Id.Value() + '/' + Lookup.Name.Value();
        Lookup.OK().Invoke();
    end;

    local procedure SeedRows(Count: Integer)
    var
        Row: Record "Block Edge Row";
        i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to Count do begin
            Row.Init();
            Row.Id := i;
            Row.Name := 'N' + Format(i);
            Row.Insert();
        end;
        Log.Take();
        SeenInLookup := '';
    end;
}

