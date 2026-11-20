// al-runner issue 8186: shapes next to the corpus probe (Issue8186Probe.al, whose objects these tests use) that the probe does not
// measure. The new path must not take them: each keeps the runner's behavior from before the change, or is refused.

table 97050 "LT NI Plain Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; Name; Text[30]) { }
        field(4; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

// Page 97032 with an error after CurrPage.Update in the control's OnValidate.
page 97051 "LT NI Lines Error"
{
    PageType = List;
    SourceTable = "LT NI Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('P.V(' + Log.Bool(Rec."Entry No." <> 0) + ')');
                        if Log.IsUpdateFalse() then
                            CurrPage.Update(false)
                        else
                            CurrPage.Update();
                        Log.Add('P.V.end');
                        Error('Boom after the update');
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('P.NEW');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('P.INS(' + Rec.Name + ')');
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');
        exit(true);
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NI Log";
}

// Page 97032 over a table whose field does not insert: the control's OnValidate inserts Rec itself.
page 97052 "LT NI Lines Control Insert"
{
    PageType = List;
    SourceTable = "LT NI Plain Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        if Rec."Entry No." = 0 then
                            Rec.Insert(true);
                        CurrPage.Update();
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('P.NEW');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('P.INS(' + Rec.Name + ')');
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');
        exit(true);
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NI Log";
}

codeunit 97053 "LT NI Neighbor Opener"
{
    procedure Run()
    var
        Line: Record "LT NI Line";
    begin
        Page.Run(Page::"LT NI Lines", Line);
    end;

    procedure RunModalFilteredOnName()
    var
        Line: Record "LT NI Line";
    begin
        Line.SetFilter(Name, 'A');
        Page.RunModal(Page::"LT NI Lines", Line);
    end;
}

codeunit 97054 "LT NI Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT NI Log";

    local procedure Initialize(Rows: Text; UpdateFalse: Boolean)
    var
        Line: Record "LT NI Line";
        Row: Text;
        Parts: List of [Text];
    begin
        Line.DeleteAll();
        if Rows <> '' then
            foreach Row in Rows.Split(',') do begin
                Parts := Row.Split(':');
                Line.Init();
                Line."Group Code" := CopyStr(Parts.Get(1), 1, 10);
                Line."Entry No." := 0;
                Line.Name := CopyStr(Parts.Get(2), 1, 30);
                Line.Insert();
            end;
        Commit();
        Log.Reset(UpdateFalse);
    end;

    local procedure Rows(): Text
    var
        Line: Record "LT NI Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        exit(Result);
    end;

    local procedure ErrorAfterUpdate(UpdateFalse: Boolean)
    var
        Lines: TestPage "LT NI Lines Error";
    begin
        Initialize('', UpdateFalse);
        Lines.OpenEdit();
        Lines.New();
        asserterror Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':X/V,', Rows(), 'the close stores the line');
    end;

    [Test]
    procedure ErrorAfterUpdate_Update()
    begin
        ErrorAfterUpdate(false);
    end;

    [Test]
    procedure ErrorAfterUpdate_UpdateFalse()
    begin
        ErrorAfterUpdate(true);
    end;

    [Test]
    procedure FilterExcludesTheLine()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(':A', false);
        Lines.OpenEdit();
        Lines.Filter.SetFilter(Name, 'A');
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':A/,:X/V,', Rows(), 'the line holds Value');
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure RunModalFilterExcludesTheLine()
    var
        Opener: Codeunit "LT NI Neighbor Opener";
    begin
        Initialize(':A', false);
        Opener.RunModalFilteredOnName();
        Assert.AreEqual(':A/,:X/V,', Rows(), 'the line holds Value');
    end;

    [Test]
    procedure TwoRowsBefore()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(':A,:B', false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':A/,:B/,:X/V,', Rows(), 'the line holds Value');
    end;

    [Test]
    procedure RowAfter()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(':A,B:Z', false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':A/,:X/V,B:Z/,', Rows(), 'the line holds Value');
    end;

    [Test]
    procedure ControlValidateInserts()
    var
        Plain: Record "LT NI Plain Line";
        Lines: TestPage "LT NI Lines Control Insert";
    begin
        Plain.DeleteAll();
        Commit();
        Log.Reset(false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Plain.FindFirst();
        Assert.AreEqual('X/V', Plain.Name + '/' + Plain.Value, 'the line holds Value');
    end;

    [Test]
    procedure TrapAndPageRun()
    var
        Lines: TestPage "LT NI Lines";
        Opener: Codeunit "LT NI Neighbor Opener";
    begin
        Initialize('', false);
        Lines.Trap();
        Opener.Run();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':X/V,', Rows(), 'the line holds Value');
    end;


    [Test]
    procedure ReadBeforeTheSetValue()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize('', false);
        Lines.OpenEdit();
        Lines.New();
        Assert.AreEqual('', Lines.Name.Value(), 'the new line');
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(':X/V,', Rows(), 'the line holds Value');
    end;

    // The second open of the same TestPage variable, on the row the first open's new line became: Previous() on the only row of
    // the view goes the way of a plain list (it saves the Value typed).
    [Test]
    procedure ReopenAfterTheLanding_PreviousSaves()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize('', false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Close();
        Lines.OpenEdit();
        Assert.AreEqual('X', Lines.Name.Value(), 'the page opens on the row');
        Lines.Value.SetValue('W');
        Lines.Previous();
        Assert.AreEqual(':X/W,', Rows(), 'Previous saves the Value typed');
        Lines.Close();
    end;

    [ModalPageHandler]
    procedure LinesModalHandler(var Lines: TestPage "LT NI Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;
}

// Update(false) on page 97033, in a codeunit of its own: a test refused earlier in the codeunit above would keep it from running.
codeunit 97055 "LT NI Neighbor Tests 2"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT NI Log";

    [Test]
    procedure NoAutoSplitKey_UpdateFalse()
    var
        Line: Record "LT NI Line";
        Lines: TestPage "LT NI Lines No Split";
    begin
        Line.DeleteAll();
        Commit();
        Log.Reset(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Line.FindFirst();
        Assert.AreEqual('X/V', Line.Name + '/' + Line.Value, 'the line holds Value');
    end;
}