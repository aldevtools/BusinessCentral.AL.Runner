// The corpus probe ListNewLineExtraCodeFactbox.al of corpus pull request 1532 (al-runner issue 8734), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: a List page with page triggers and a CardPart FactBox whose table field's OnValidate inserts the page's new line,
//   and the code around CurrPage.SaveRecord() and CurrPage.Update in the control's OnValidate, are what is under test
// Fixtures used: own table 97691, pages 97693, 97694, 97697 and 97698, codeunits 97692, 97695, 97696, 97699 and test codeunit
//   97700
// Probe (al-runner issue 8734): a List page with AutoSplitKey, OnAfterGetRecord and OnAfterGetCurrRecord, and a CardPart
//   FactBox linked to the line's key whose OnAfterGetRecord sets a variable it shows, stands on its new line; the SetValue on
//   Name runs the table field's OnValidate, which inserts the row (Insert(true), an AutoIncrement key field), then the
//   control's OnValidate. In the repro of 8734 it sets another field of Rec, saves, modifies the row through another record
//   and calls CurrPage.Update(false). The second page logs its triggers and its part's, and varies the code one step at a
//   time (only the save, the modify through another record after the save, the field set before the save; each also with one
//   row before the new line). The page is opened with Page.RunModal on a record filtered to one group.
//   Readings: what the page and its part show after the Name and the Value SetValue, the rows after OK, and the triggers'
//   log with a mark at each step of the test.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 97691 "LT NK Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            begin
                if "Entry No." = 0 then
                    Insert(true);
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 97692 "LT NK Log"
{
    SingleInstance = true;

    var
        Entries: Text;
        Mode: Integer;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;

    procedure Reset(NewMode: Integer)
    begin
        Entries := '';
        Mode := NewMode;
    end;

    procedure GetMode(): Integer
    begin
        exit(Mode);
    end;
}

page 97693 "LT NK Info"
{
    PageType = CardPart;
    SourceTable = "LT NK Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Mark; Rec.Mark) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

// The repro of 8734: page triggers, a CardPart FactBox, and a field set, the save, a modify through another record and
// Update(false) in the Name control's OnValidate.
page 97694 "LT NK Lines"
{
    PageType = List;
    SourceTable = "LT NK Line";
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
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NK Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrentName := Rec.Name;
    end;

    local procedure StampMark(Line: Record "LT NK Line")
    var
        Other: Record "LT NK Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Reads: Integer;
        CurrentName: Text;
}

codeunit 97695 "LT NK Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NK Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NK Lines", Line);
    end;
}

codeunit 97696 "LT NK Stamp"
{
    procedure StampMark(Line: Record "LT NK Line")
    var
        Other: Record "LT NK Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;
}

page 97697 "LT NK Var Info"
{
    PageType = CardPart;
    SourceTable = "LT NK Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Mark; Rec.Mark) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "LT NK Log";
    begin
        Info := Rec.Name;
        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + ')');
    end;

    var
        Info: Text;
}

// The logging page: its triggers and its part's log what they see; the mode of the log decides the control's code.
page 97698 "LT NK Var Lines"
{
    PageType = List;
    SourceTable = "LT NK Line";
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
                    var
                        Log: Codeunit "LT NK Log";
                        Stamp: Codeunit "LT NK Stamp";
                    begin
                        case Log.GetMode() of
                            1:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.SaveRecord();
                                    Stamp.StampMark(Rec);
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            3:
                                begin
                                    CurrPage.SaveRecord();
                                    Stamp.StampMark(Rec);
                                    CurrPage.Update(false);
                                end;
                            4:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                        end;
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NK Var Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "LT NK Log";
    begin
        Log.Add('AGR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + ')');
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "LT NK Log";
    begin
        Log.Add('AGCR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + ')');
    end;
}

codeunit 97699 "LT NK Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NK Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NK Var Lines", Line);
    end;
}

codeunit 97700 "LT NK Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithTriggersAndFactBox_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NK Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithTriggersAndFactBox_RowBefore_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NK Opener";
    begin
        Initialize();
        AddRow('B');
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'B///;A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ExtraCodeOnPageWithTriggersAndFactBox_LogsTheTriggers()
    begin
        Initialize();
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('log', 'AGCR(//);AGCR(//);|open;AGCR(//);|new;AGR(A/Resolved/Stamped);AGR(A/Resolved/Stamped);AGCR(A/Resolved/Stamped);P.AGR(A/Resolved/Stamped);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ExtraCodeOnPageWithTriggersAndFactBox_RowBefore_LogsTheTriggers()
    begin
        Initialize();
        AddRow('B');
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('log', 'AGR(B//);AGR(B//);AGCR(B//);P.AGR(B//);|open;AGCR(//);|new;AGR(A/Resolved/Stamped);AGR(B//);AGR(A/Resolved/Stamped);AGR(A/Resolved/Stamped);AGCR(A/Resolved/Stamped);P.AGR(A/Resolved/Stamped);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'B///;A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordOnPageWithTriggersAndFactBox_LogsTheTriggers()
    begin
        Initialize();
        RunVar(2);
        Check('shown', 'name=A/// part=//A value=A///V part=//A ', Shown);
        Check('log', 'AGCR(//);AGCR(//);|open;AGCR(//);|new;AGR(A//);AGR(A//);AGCR(A//);P.AGR(A//);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A///V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecordOnPageWithTriggersAndFactBox_LogsTheTriggers()
    begin
        Initialize();
        RunVar(3);
        Check('shown', 'name=A//Stamped/ part=/Stamped/A value=A//Stamped/V part=/Stamped/A ', Shown);
        Check('log', 'AGCR(//);AGCR(//);|open;AGCR(//);|new;AGR(A//Stamped);AGR(A//Stamped);AGCR(A//Stamped);P.AGR(A//Stamped);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A//Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetBeforeSaveRecordOnPageWithTriggersAndFactBox_LogsTheTriggers()
    begin
        Initialize();
        RunVar(4);
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('log', 'AGCR(//);AGCR(//);|open;AGCR(//);|new;AGR(A/Resolved/);AGR(A/Resolved/);AGCR(A/Resolved/);P.AGR(A/Resolved/);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A/Resolved//V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordOnPageWithTriggersAndFactBox_RowBefore_LogsTheTriggers()
    begin
        Initialize();
        AddRow('B');
        RunVar(2);
        Check('shown', 'name=A/// part=//A value=A///V part=//A ', Shown);
        Check('log', 'AGR(B//);AGR(B//);AGCR(B//);P.AGR(B//);|open;AGCR(//);|new;AGR(A//);AGR(B//);AGR(A//);AGR(A//);AGCR(A//);P.AGR(A//);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'B///;A///V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecordOnPageWithTriggersAndFactBox_RowBefore_LogsTheTriggers()
    begin
        Initialize();
        AddRow('B');
        RunVar(3);
        Check('shown', 'name=A//Stamped/ part=/Stamped/A value=A//Stamped/V part=/Stamped/A ', Shown);
        Check('log', 'AGR(B//);AGR(B//);AGCR(B//);P.AGR(B//);|open;AGCR(//);|new;AGR(A//Stamped);AGR(B//);AGR(A//Stamped);AGR(A//Stamped);AGCR(A//Stamped);P.AGR(A//Stamped);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'B///;A//Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetBeforeSaveRecordOnPageWithTriggersAndFactBox_RowBefore_LogsTheTriggers()
    begin
        Initialize();
        AddRow('B');
        RunVar(4);
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('log', 'AGR(B//);AGR(B//);AGCR(B//);P.AGR(B//);|open;AGCR(//);|new;AGR(A/Resolved/);AGR(B//);AGR(A/Resolved/);AGR(A/Resolved/);AGCR(A/Resolved/);P.AGR(A/Resolved/);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'B///;A/Resolved//V;', Rows());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NK Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NK Var Lines")
    var
        LogCu: Codeunit "LT NK Log";
    begin
        LogCu.Add('|open');
        Lines.New();
        LogCu.Add('|new');
        Lines.Name.SetValue('A');
        LogCu.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        LogCu.Add('|read');
        Lines.Value.SetValue('V');
        LogCu.Add('|value');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        LogCu.Add('|read');
        Lines.OK().Invoke();
        LogCu.Add('|ok');
    end;

    local procedure Initialize()
    var
        Line: Record "LT NK Line";
        LogCu: Codeunit "LT NK Log";
    begin
        Line.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
    end;

    local procedure AddRow(Name: Text[30])
    var
        Line: Record "LT NK Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure RunVar(M: Integer)
    var
        LogCu: Codeunit "LT NK Log";
        Opener: Codeunit "LT NK Var Opener";
    begin
        LogCu.Reset(M);
        Opener.Open('G1');
    end;

    local procedure Log(): Text
    var
        LogCu: Codeunit "LT NK Log";
    begin
        exit(LogCu.Get());
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NK Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
