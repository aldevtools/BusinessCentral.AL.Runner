// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own table 97030, log codeunit 97031, pages 97032, 97033, 97036 to 97039, opener codeunit 97034,
//   test codeunit 97035
// Probe (al-runner issue 8186): a List page stands on its new line (New()). SetValue on Name runs the table field's OnValidate, which inserts
//   Rec itself (Insert(true), with an AutoIncrement key field), and then the control's OnValidate, which calls CurrPage.Update() or
//   CurrPage.Update(false). Readings: the page, part and table triggers each step runs, the row the page shows after the refresh, the rows
//   the table holds, where a later SetValue lands, and Previous()/Next() from there. Cells: a page with OnAfterGetRecord and
//   OnAfterGetCurrRecord, with and without AutoSplitKey, opened with OpenEdit and with Page.RunModal, on an empty view and on a view with
//   one row before, Update() and Update(false); a page with OnAfterGetRecord only and two CardPart FactBoxes linked to the line's key,
//   opened with OpenEdit and with Page.RunModal on a record filtered to one group (a row of another group outside the view); a page
//   without page triggers.
//   Measured on a local BC 29.1 OnPrem w1 container; BC 27.5 and 28.5 are left to CI.
// BC versions: 27, 28, 29

table 97030 "LT NI Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            var
                Log: Codeunit "LT NI Log";
            begin
                Log.Add('T.V(' + Name + ')');
                if "Entry No." = 0 then
                    Insert(true);
            end;
        }
        field(4; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT NI Log";
    begin
        Log.Add('T.I(' + Name + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT NI Log";
    begin
        Log.Add('T.M(' + Name + '/' + Value + ')');
    end;
}

codeunit 97031 "LT NI Log"
{
    SingleInstance = true;

    var
        Buffer: Text;
        UpdateFalse: Boolean;

    procedure Reset(NewUpdateFalse: Boolean)
    begin
        Buffer := '';
        UpdateFalse := NewUpdateFalse;
    end;

    procedure Add(Entry: Text)
    begin
        Buffer += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buffer;
        Buffer := '';
        exit(Result);
    end;

    procedure IsUpdateFalse(): Boolean
    begin
        exit(UpdateFalse);
    end;

    procedure Bool(Value: Boolean): Text
    begin
        if Value then
            exit('T');
        exit('F');
    end;
}

page 97032 "LT NI Lines"
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

page 97033 "LT NI Lines No Split"
{
    PageType = List;
    SourceTable = "LT NI Line";
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

codeunit 97034 "LT NI Opener"
{
    procedure Open()
    var
        Line: Record "LT NI Line";
    begin
        Page.RunModal(Page::"LT NI Lines", Line);
    end;

    procedure OpenWithParts(GroupCode: Code[10])
    var
        Line: Record "LT NI Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NI Lines Parts", Line);
    end;
}

// The host's shape in al-runner issue 8186: only OnAfterGetRecord, and CardPart FactBoxes linked to the line's key.
page 97036 "LT NI Lines Parts"
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
                        CurrPage.Update();
                        Log.Add('P.V.end');
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "LT NI Peek")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(PeekPlain; "LT NI Peek Plain")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NI Log";
}

page 97037 "LT NI Peek"
{
    PageType = CardPart;
    SourceTable = "LT NI Line";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('F.AGR(' + Rec.Name + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('F.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NI Log";
}

page 97038 "LT NI Peek Plain"
{
    PageType = CardPart;
    SourceTable = "LT NI Line";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

// The reporter's repro page: no page trigger.
page 97039 "LT NI Lines Quiet"
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
                        CurrPage.Update();
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 97035 "LT NI Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT NI Log";
        ModalRowBefore: Boolean;
        Expected: List of [Text];
        StepNo: Integer;

    local procedure Initialize(RowBefore: Boolean; UpdateFalse: Boolean)
    var
        Line: Record "LT NI Line";
    begin
        Line.DeleteAll();
        if RowBefore then begin
            Line.Init();
            Line.Name := 'A';
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
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        exit(Result);
    end;

    local procedure ExpectedOpen(RowBefore: Boolean): Text
    begin
        if RowBefore then
            exit('P.AGR(A);P.AGR(A);P.AGCR(A);');
        exit('P.NEW;P.AGCR();P.NEW;P.AGCR();');
    end;

    local procedure ExpectedSetName(RowBefore: Boolean; UpdateFalse: Boolean): Text
    var
        Result: Text;
    begin
        Result := 'T.V(X);T.I(X);P.V(T);';
        if not UpdateFalse then
            Result += 'P.MOD(X/);T.M(X/);';
        Result += 'P.V.end;';
        if RowBefore then
            exit(Result + 'P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);');
        exit(Result + 'P.AGR(X);P.AGR(X);P.AGCR(X);');
    end;

    local procedure ExpectedRowsBefore(RowBefore: Boolean): Text
    begin
        if RowBefore then
            exit('A/,');
        exit('');
    end;

    // The steps after OnNewRecord, the same for both pages: what each step logs, what the page shows and what the table holds.
    local procedure CheckAfterSetName(RowBefore: Boolean; UpdateFalse: Boolean; SetNameLog: Text; Shown: Text)
    begin
        Assert.AreEqual(ExpectedSetName(RowBefore, UpdateFalse), SetNameLog,
            'SetValue on Name: the table field inserts the line, CurrPage.Update modifies it (Update() only), and the refresh reads it');
        Assert.AreEqual('X/', Shown, 'the page shows the line the table field inserted');
        Assert.AreEqual(ExpectedRowsBefore(RowBefore) + 'X/,', Rows(), 'one row inserted, no second insert');
    end;

    local procedure CheckAfterSetValue(RowBefore: Boolean; SetValueLog: Text; Shown: Text)
    begin
        Assert.AreEqual('', SetValueLog, 'SetValue on Value runs no trigger');
        Assert.AreEqual('X/V', Shown, 'Value lands on the inserted line');
        Assert.AreEqual(ExpectedRowsBefore(RowBefore) + 'X/,', Rows(), 'Value is not saved yet');
    end;

    local procedure CheckMoves(RowBefore: Boolean; PrevMove: Text; PrevLog: Text; NextMove: Text; NextLog: Text)
    begin
        if RowBefore then begin
            Assert.AreEqual('T/A', PrevMove, 'Previous moves to the row before');
            Assert.AreEqual('P.MOD(X/V);T.M(X/V);P.AGR(A);P.AGR(A);P.AGCR(A);', PrevLog, 'Previous saves Value, then reads the row before');
            Assert.AreEqual('T/X', NextMove, 'Next moves back to the inserted line');
            Assert.AreEqual('P.AGR(X);P.AGR(X);P.AGCR(X);', NextLog, 'Next reads the inserted line');
        end else begin
            Assert.AreEqual('F/X', PrevMove, 'Previous finds no row before the inserted line');
            Assert.AreEqual('', PrevLog, 'Previous runs no trigger');
            Assert.AreEqual('T/', NextMove, 'Next moves to a new line');
            Assert.AreEqual('P.MOD(X/V);T.M(X/V);P.NEW;P.AGCR();', NextLog, 'Next saves Value, then starts a new line');
        end;
    end;

    local procedure DriveSplit(var Lines: TestPage "LT NI Lines"; RowBefore: Boolean; UpdateFalse: Boolean)
    var
        PrevMove: Text;
        PrevLog: Text;
        NextMove: Text;
    begin
        Assert.AreEqual(ExpectedOpen(RowBefore), Log.Take(), 'the open');
        Lines.New();
        Assert.AreEqual('P.NEW;P.AGCR();', Log.Take(), 'New()');
        Lines.Name.SetValue('X');
        CheckAfterSetName(RowBefore, UpdateFalse, Log.Take(), Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        CheckAfterSetValue(RowBefore, Log.Take(), Lines.Name.Value() + '/' + Lines.Value.Value());
        PrevMove := Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value();
        PrevLog := Log.Take();
        NextMove := Log.Bool(Lines.Next()) + '/' + Lines.Name.Value();
        CheckMoves(RowBefore, PrevMove, PrevLog, NextMove, Log.Take());
    end;

    local procedure DriveNoSplit(var Lines: TestPage "LT NI Lines No Split"; RowBefore: Boolean)
    var
        PrevMove: Text;
        PrevLog: Text;
        NextMove: Text;
    begin
        Assert.AreEqual(ExpectedOpen(RowBefore), Log.Take(), 'the open');
        Lines.New();
        Assert.AreEqual('P.NEW;P.AGCR();', Log.Take(), 'New()');
        Lines.Name.SetValue('X');
        CheckAfterSetName(RowBefore, false, Log.Take(), Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        CheckAfterSetValue(RowBefore, Log.Take(), Lines.Name.Value() + '/' + Lines.Value.Value());
        PrevMove := Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value();
        PrevLog := Log.Take();
        NextMove := Log.Bool(Lines.Next()) + '/' + Lines.Name.Value();
        CheckMoves(RowBefore, PrevMove, PrevLog, NextMove, Log.Take());
    end;

    local procedure CheckClosed(RowBefore: Boolean)
    begin
        Assert.AreEqual('', Log.Take(), 'closing the page saves nothing more');
        Assert.AreEqual(ExpectedRowsBefore(RowBefore) + 'X/V,', Rows(), 'the inserted line holds Value');
    end;

    [Test]
    procedure ListNewLine_FieldValidateInserts_Update_EmptyView_ModifiesAndShowsTheLine()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(false, false);
        Lines.OpenEdit();
        DriveSplit(Lines, false, false);
        Lines.Close();
        CheckClosed(false);
    end;

    [Test]
    procedure ListNewLine_FieldValidateInserts_Update_RowBefore_ModifiesAndShowsTheLine()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(true, false);
        Lines.OpenEdit();
        DriveSplit(Lines, true, false);
        Lines.Close();
        CheckClosed(true);
    end;

    [Test]
    procedure ListNewLine_NoAutoSplitKey_FieldValidateInserts_Update_EmptyView_ModifiesAndShowsTheLine()
    var
        Lines: TestPage "LT NI Lines No Split";
    begin
        Initialize(false, false);
        Lines.OpenEdit();
        DriveNoSplit(Lines, false);
        Lines.Close();
        CheckClosed(false);
    end;

    [Test]
    procedure ListNewLine_NoAutoSplitKey_FieldValidateInserts_Update_RowBefore_ModifiesAndShowsTheLine()
    var
        Lines: TestPage "LT NI Lines No Split";
    begin
        Initialize(true, false);
        Lines.OpenEdit();
        DriveNoSplit(Lines, true);
        Lines.Close();
        CheckClosed(true);
    end;

    [Test]
    procedure ListNewLine_FieldValidateInserts_UpdateFalse_EmptyView_ShowsTheLineWithoutModify()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(false, true);
        Lines.OpenEdit();
        DriveSplit(Lines, false, true);
        Lines.Close();
        CheckClosed(false);
    end;

    [Test]
    procedure ListNewLine_FieldValidateInserts_UpdateFalse_RowBefore_ShowsTheLineWithoutModify()
    var
        Lines: TestPage "LT NI Lines";
    begin
        Initialize(true, true);
        Lines.OpenEdit();
        DriveSplit(Lines, true, true);
        Lines.Close();
        CheckClosed(true);
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModal_FieldValidateInserts_Update_EmptyView_ModifiesAndShowsTheLine()
    var
        Opener: Codeunit "LT NI Opener";
    begin
        Initialize(false, false);
        ModalRowBefore := false;
        Opener.Open();
        CheckClosed(false);
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModal_FieldValidateInserts_Update_RowBefore_ModifiesAndShowsTheLine()
    var
        Opener: Codeunit "LT NI Opener";
    begin
        Initialize(true, false);
        ModalRowBefore := true;
        Opener.Open();
        CheckClosed(true);
    end;

    [ModalPageHandler]
    procedure LinesModalHandler(var Lines: TestPage "LT NI Lines")
    begin
        DriveSplit(Lines, ModalRowBefore, false);
        Lines.OK().Invoke();
    end;

    // The host's shape in al-runner issue 8186 (only OnAfterGetRecord, two CardPart FactBoxes linked to the line's key) and the
    // reporter's page without page triggers. Each step's reading is compared with the next expected `name=value` in order.

    local procedure InitializeGroups(OtherGroupRow: Boolean; RowBefore: Boolean; GroupCode: Code[10])
    var
        Line: Record "LT NI Line";
    begin
        Line.DeleteAll();
        if OtherGroupRow then begin
            Line.Init();
            Line."Group Code" := 'G1';
            Line.Name := 'O';
            Line.Insert();
        end;
        if RowBefore then begin
            Line.Init();
            Line."Group Code" := GroupCode;
            Line.Name := 'A';
            Line.Insert();
        end;
        Commit();
        Log.Reset(false);
        Clear(Expected);
        StepNo := 0;
    end;

    local procedure Expect(Steps: Text)
    var
        Step: Text;
    begin
        foreach Step in Steps.Split(' | ') do
            Expected.Add(Step);
    end;

    local procedure Check(Name: Text; Actual: Text)
    begin
        StepNo += 1;
        Assert.AreEqual(Expected.Get(StepNo), Name + '=' + Actual, 'step ' + Format(StepNo));
    end;

    local procedure GroupRows(): Text
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

    local procedure DriveParts(var Lines: TestPage "LT NI Lines Parts")
    begin
        Check('open', Log.Take());
        Check('openparts', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
        Lines.New();
        Check('new', Log.Take());
        Check('newparts', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
        Lines.Name.SetValue('X');
        Check('setname', Log.Take());
        Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('parts', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
        Check('partslog', Log.Take());
        Check('rows1', GroupRows());
        Lines.Value.SetValue('V');
        Check('setvalue', Log.Take());
        Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('parts2', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
        Check('rows2', GroupRows());
        Check('prev', Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value());
        Check('prevlog', Log.Take());
        Check('prevparts', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
        Check('next', Log.Bool(Lines.Next()) + '/' + Lines.Name.Value());
        Check('nextlog', Log.Take());
        Check('nextparts', Lines.Peek.Name.Value() + '/' + Lines.PeekPlain.Value.Value());
    end;

    local procedure DriveQuiet(var Lines: TestPage "LT NI Lines Quiet")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Check('setname', Log.Take());
        Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows1', GroupRows());
        Lines.Value.SetValue('V');
        Check('setvalue', Log.Take());
        Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows2', GroupRows());
        Check('prev', Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value());
        Check('prevlog', Log.Take());
        Check('next', Log.Bool(Lines.Next()) + '/' + Lines.Name.Value());
        Check('nextlog', Log.Take());
    end;

    [Test]
    procedure ListNewLine_PartsHost_FieldValidateInserts_Update_RowBefore_PartsFollowTheLine()
    var
        Lines: TestPage "LT NI Lines Parts";
    begin
        InitializeGroups(false, true, '');
        Expect('open=P.AGR(A);F.AGR(A);F.AGCR(A); | openparts=A/ | new= | newparts=A/' +
            ' | setname=F.AGCR();T.V(X);T.I(X);P.V(T);T.M(X/);P.V.end;P.AGR(X);P.AGR(A);F.AGR(X);F.AGCR(X); | shown=X/ | parts=X/' +
            ' | partslog= | rows1=:A/,:X/, | setvalue=F.AGR(X);F.AGCR(X); | shown2=X/V | parts2=X/ | rows2=:A/,:X/,' +
            ' | prev=T/A | prevlog=P.AGR(X);T.M(X/V);F.AGR(A);F.AGCR(A); | prevparts=A/' +
            ' | next=T/X | nextlog=F.AGR(X);F.AGCR(X); | nextparts=X/V | close= | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveParts(Lines);
        Lines.Close();
        Check('close', Log.Take());
        Check('rows3', GroupRows());
    end;

    [Test]
    procedure ListNewLine_PartsHost_FieldValidateInserts_Update_EmptyView_PartsFollowTheLine()
    var
        Lines: TestPage "LT NI Lines Parts";
    begin
        InitializeGroups(false, false, '');
        Expect('open=F.AGCR(); | openparts=/ | new= | newparts=/' +
            ' | setname=F.AGCR();T.V(X);T.I(X);P.V(T);T.M(X/);P.V.end;P.AGR(X);F.AGR(X);F.AGCR(X); | shown=X/ | parts=X/' +
            ' | partslog= | rows1=:X/, | setvalue=F.AGR(X);F.AGCR(X); | shown2=X/V | parts2=X/ | rows2=:X/,' +
            ' | prev=F/X | prevlog= | prevparts=X/' +
            ' | next=T/ | nextlog=T.M(X/V);F.AGCR(); | nextparts=/ | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveParts(Lines);
        Lines.Close();
        Check('close', Log.Take());
        Check('rows3', GroupRows());
    end;

    [Test]
    [HandlerFunctions('PartsModalHandler')]
    procedure ListNewLine_PartsHost_RunModalFiltered_FieldValidateInserts_Update_EmptyView_PartsFollowTheLine()
    var
        Opener: Codeunit "LT NI Opener";
    begin
        InitializeGroups(true, false, 'G2');
        Expect('open=F.AGCR(); | openparts=/ | new= | newparts=/' +
            ' | setname=F.AGCR();T.V(X);T.I(X);P.V(T);T.M(X/);P.V.end;P.AGR(X);F.AGR(X);F.AGCR(X); | shown=X/ | parts=X/' +
            ' | partslog= | rows1=G1:O/,G2:X/, | setvalue= | shown2=X/V | parts2=X/ | rows2=G1:O/,G2:X/,' +
            ' | prev=F/X | prevlog= | prevparts=X/' +
            ' | next=T/ | nextlog=T.M(X/V);F.AGCR(); | nextparts=/ | ok= | afterrun= | rows3=G1:O/,G2:X/V,');
        Opener.OpenWithParts('G2');
        Check('afterrun', Log.Take());
        Check('rows3', GroupRows());
    end;

    [Test]
    [HandlerFunctions('PartsModalHandler')]
    procedure ListNewLine_PartsHost_RunModalFiltered_FieldValidateInserts_Update_RowBefore_PartsFollowTheLine()
    var
        Opener: Codeunit "LT NI Opener";
    begin
        InitializeGroups(true, true, 'G2');
        Expect('open=P.AGR(A);F.AGR(A);F.AGCR(A); | openparts=A/ | new= | newparts=A/' +
            ' | setname=F.AGCR();T.V(X);T.I(X);P.V(T);T.M(X/);P.V.end;P.AGR(X);P.AGR(A);F.AGR(X);F.AGCR(X); | shown=X/ | parts=X/' +
            ' | partslog= | rows1=G1:O/,G2:A/,G2:X/, | setvalue= | shown2=X/V | parts2=X/ | rows2=G1:O/,G2:A/,G2:X/,' +
            ' | prev=T/A | prevlog=P.AGR(X);T.M(X/V);F.AGR(A);F.AGCR(A); | prevparts=A/' +
            ' | next=T/X | nextlog=F.AGR(X);F.AGCR(X); | nextparts=X/V | ok= | afterrun= | rows3=G1:O/,G2:A/,G2:X/V,');
        Opener.OpenWithParts('G2');
        Check('afterrun', Log.Take());
        Check('rows3', GroupRows());
    end;

    [Test]
    procedure ListNewLine_NoPageTriggers_FieldValidateInserts_Update_RowBefore_ModifiesTheLine()
    var
        Lines: TestPage "LT NI Lines Quiet";
    begin
        InitializeGroups(false, true, '');
        Expect('setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:A/,:X/, | setvalue= | shown2=X/V | rows2=:A/,:X/,' +
            ' | prev=T/A | prevlog=T.M(X/V); | next=T/X | nextlog= | close= | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Check('close', Log.Take());
        Check('rows3', GroupRows());
    end;

    [Test]
    procedure ListNewLine_NoPageTriggers_FieldValidateInserts_Update_EmptyView_ModifiesTheLine()
    var
        Lines: TestPage "LT NI Lines Quiet";
    begin
        InitializeGroups(false, false, '');
        Expect('setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:X/, | setvalue= | shown2=X/V | rows2=:X/,' +
            ' | prev=F/X | prevlog= | next=T/ | nextlog=T.M(X/V); | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Check('close', Log.Take());
        Check('rows3', GroupRows());
    end;

    [ModalPageHandler]
    procedure PartsModalHandler(var Lines: TestPage "LT NI Lines Parts")
    begin
        DriveParts(Lines);
        Lines.OK().Invoke();
        Check('ok', Log.Take());
    end;
}
