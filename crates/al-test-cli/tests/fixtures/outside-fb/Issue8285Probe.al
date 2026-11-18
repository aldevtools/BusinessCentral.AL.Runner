// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own table 97470, log codeunit 97471, pages 97472 to 97475, opener codeunit 97476, test
//   codeunit 97477
// Probe (al-runner issue 8285, the follow-up of 8186 measured in ListNewLineCodeInsertUpdate.al): a List page stands on its new
//   line (New()). SetValue on Name runs the table field's OnValidate, which inserts Rec itself (Insert(true), with an
//   AutoIncrement key field), and then the control's OnValidate, which calls CurrPage.SaveRecord() and then
//   CurrPage.Update(false), or CurrPage.Update(true). Readings: the page and table triggers each step runs, the row the page
//   shows after the refresh, the rows the table holds, where a later SetValue lands, and Previous()/Next() from there. Cells: a
//   page with the five row triggers and AutoSplitKey, and a page without page triggers, each opened with OpenEdit on an empty
//   view and on a view with one row before, and with Page.RunModal on a record filtered to one group (a row of another group
//   outside the view); a page with OnAfterGetRecord only and one CardPart FactBox without triggers linked to the line's key
//   (SaveRecord then Update(false) only), opened the same three ways.
//   Measured on a local BC 29.1 OnPrem w1 container; BC 27.5 and 28.5 are left to CI.
// BC versions: 27, 28, 29

table 97470 "LT NS Line"
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
                Log: Codeunit "LT NS Log";
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
        Log: Codeunit "LT NS Log";
    begin
        Log.Add('T.I(' + Name + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT NS Log";
    begin
        Log.Add('T.M(' + Name + '/' + Value + ')');
    end;
}

codeunit 97471 "LT NS Log"
{
    SingleInstance = true;

    var
        Buffer: Text;
        SaveThenUpdateFalse: Boolean;

    procedure Reset(NewSaveThenUpdateFalse: Boolean)
    begin
        Buffer := '';
        SaveThenUpdateFalse := NewSaveThenUpdateFalse;
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

    procedure IsSaveThenUpdateFalse(): Boolean
    begin
        exit(SaveThenUpdateFalse);
    end;

    procedure Bool(Value: Boolean): Text
    begin
        if Value then
            exit('T');
        exit('F');
    end;
}

// The five row triggers and AutoSplitKey, as page 97032 of ListNewLineCodeInsertUpdate.al.
page 97472 "LT NS Lines"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            Log.Add('P.SR');
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
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
        Log: Codeunit "LT NS Log";
}

// No page trigger, as the repro of al-runner issue 8285 and page 97039 of ListNewLineCodeInsertUpdate.al.
page 97473 "LT NS Lines Quiet"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "LT NS Log";
}

// The host's shape in al-runner issue 8285: only OnAfterGetRecord, and one CardPart FactBox without triggers linked to the
// line's key.
page 97474 "LT NS Lines Part"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "LT NS Peek")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

page 97475 "LT NS Peek"
{
    PageType = CardPart;
    SourceTable = "LT NS Line";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { ApplicationArea = All; }
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

codeunit 97476 "LT NS Opener"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        Line: Record "LT NS Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}

codeunit 97477 "LT NS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT NS Log";
        Expected: List of [Text];
        StepNo: Integer;
        CloseAfterValue: Boolean;

    // Rows of group G1 hold the line under test; with OtherGroupRow a row of group G0 lies outside a view filtered to G1.
    local procedure Initialize(OtherGroupRow: Boolean; RowBefore: Boolean; GroupCode: Code[10]; SaveThenUpdateFalse: Boolean)
    var
        Line: Record "LT NS Line";
    begin
        Line.DeleteAll();
        if OtherGroupRow then begin
            Line.Init();
            Line."Group Code" := 'G0';
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
        Log.Reset(SaveThenUpdateFalse);
        Clear(Expected);
        StepNo := 0;
        CloseAfterValue := false;
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

    local procedure Finish()
    begin
        Assert.AreEqual(Expected.Count(), StepNo, 'every expected step was read');
    end;

    local procedure GroupRows(): Text
    var
        Line: Record "LT NS Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        exit(Result);
    end;

    local procedure DriveLines(var Lines: TestPage "LT NS Lines")
    begin
        Check('open', Log.Take());
        Lines.New();
        Check('new', Log.Take());
        Lines.Name.SetValue('X');
        Check('setname', Log.Take());
        Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows1', GroupRows());
        Lines.Value.SetValue('V');
        Check('setvalue', Log.Take());
        Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows2', GroupRows());
        if CloseAfterValue then
            exit;
        Check('prev', Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value());
        Check('prevlog', Log.Take());
        Check('next', Log.Bool(Lines.Next()) + '/' + Lines.Name.Value());
        Check('nextlog', Log.Take());
    end;

    local procedure DriveQuiet(var Lines: TestPage "LT NS Lines Quiet")
    begin
        Check('open', Log.Take());
        Lines.New();
        Check('new', Log.Take());
        Lines.Name.SetValue('X');
        Check('setname', Log.Take());
        Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows1', GroupRows());
        Lines.Value.SetValue('V');
        Check('setvalue', Log.Take());
        Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('rows2', GroupRows());
        if CloseAfterValue then
            exit;
        Check('prev', Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value());
        Check('prevlog', Log.Take());
        Check('next', Log.Bool(Lines.Next()) + '/' + Lines.Name.Value());
        Check('nextlog', Log.Take());
    end;

    // The part is read only after the SetValue on Name, so the calls before it are New() and that SetValue alone.
    local procedure DrivePart(var Lines: TestPage "LT NS Lines Part")
    begin
        Check('open', Log.Take());
        Lines.New();
        Check('new', Log.Take());
        Lines.Name.SetValue('X');
        Check('setname', Log.Take());
        Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('part', Lines.Peek.Name.Value() + '/' + Lines.Peek.Value.Value());
        Check('rows1', GroupRows());
        Lines.Value.SetValue('V');
        Check('setvalue', Log.Take());
        Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Check('part2', Lines.Peek.Name.Value() + '/' + Lines.Peek.Value.Value());
        Check('rows2', GroupRows());
        if CloseAfterValue then
            exit;
        Check('prev', Log.Bool(Lines.Previous()) + '/' + Lines.Name.Value());
        Check('prevlog', Log.Take());
        Check('prevpart', Lines.Peek.Name.Value() + '/' + Lines.Peek.Value.Value());
        Check('next', Log.Bool(Lines.Next()) + '/' + Lines.Name.Value());
        Check('nextlog', Log.Take());
        Check('nextpart', Lines.Peek.Name.Value() + '/' + Lines.Peek.Value.Value());
    end;

    local procedure Closed()
    begin
        Check('close', Log.Take());
        Check('rows3', GroupRows());
        Finish();
    end;

    // Page 97472: the five row triggers.

    [Test]
    procedure ListNewLine_SaveRecordUpdateFalse_EmptyView_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines";
    begin
        Initialize(false, false, '', true);
        Expect('open=P.NEW;P.AGCR();P.NEW;P.AGCR(); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.SR;P.V.end;P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=:X/, | setvalue= | shown2=X/V | rows2=:X/, | prev=F/X | prevlog= | next=T/' +
            ' | nextlog=P.MOD(X/V);T.M(X/V);P.NEW;P.AGCR(); | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveLines(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    procedure ListNewLine_SaveRecordUpdateFalse_RowBefore_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines";
    begin
        Initialize(false, true, '', true);
        Expect('open=P.AGR(A);P.AGR(A);P.AGCR(A); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.SR;P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=:A/,:X/, | setvalue= | shown2=X/V | rows2=:A/,:X/, | prev=T/A' +
            ' | prevlog=P.MOD(X/V);T.M(X/V);P.AGR(A);P.AGR(A);P.AGCR(A); | next=T/X' +
            ' | nextlog=P.AGR(X);P.AGR(X);P.AGCR(X); | close= | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveLines(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModalFiltered_SaveRecordUpdateFalse_RowBefore_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', true);
        Expect('open=P.AGR(A);P.AGR(A);P.AGCR(A); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.SR;P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue= | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/,' +
            ' | prev=T/A | prevlog=P.MOD(X/V);T.M(X/V);P.AGR(A);P.AGR(A);P.AGCR(A); | next=T/X' +
            ' | nextlog=P.AGR(X);P.AGR(X);P.AGCR(X); | close= | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModalFiltered_SaveRecordUpdateFalse_EmptyView_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', true);
        Expect('open=P.NEW;P.AGCR();P.NEW;P.AGCR(); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.SR;P.V.end;P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=G0:O/,G1:X/, | setvalue= | shown2=X/V | rows2=G0:O/,G1:X/, | prev=F/X' +
            ' | prevlog= | next=T/ | nextlog=P.MOD(X/V);T.M(X/V);P.NEW;P.AGCR(); | close=' +
            ' | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines", 'G1');
        Closed();
    end;

    [Test]
    procedure ListNewLine_UpdateTrue_EmptyView_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines";
    begin
        Initialize(false, false, '', false);
        Expect('open=P.NEW;P.AGCR();P.NEW;P.AGCR(); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.V.end;P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=:X/, | setvalue= | shown2=X/V | rows2=:X/, | prev=F/X | prevlog= | next=T/' +
            ' | nextlog=P.MOD(X/V);T.M(X/V);P.NEW;P.AGCR(); | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveLines(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    procedure ListNewLine_UpdateTrue_RowBefore_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines";
    begin
        Initialize(false, true, '', false);
        Expect('open=P.AGR(A);P.AGR(A);P.AGCR(A); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=:A/,:X/, | setvalue= | shown2=X/V | rows2=:A/,:X/, | prev=T/A' +
            ' | prevlog=P.MOD(X/V);T.M(X/V);P.AGR(A);P.AGR(A);P.AGCR(A); | next=T/X' +
            ' | nextlog=P.AGR(X);P.AGR(X);P.AGCR(X); | close= | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveLines(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModalFiltered_UpdateTrue_RowBefore_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', false);
        Expect('open=P.AGR(A);P.AGR(A);P.AGCR(A); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue= | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/,' +
            ' | prev=T/A | prevlog=P.MOD(X/V);T.M(X/V);P.AGR(A);P.AGR(A);P.AGCR(A); | next=T/X' +
            ' | nextlog=P.AGR(X);P.AGR(X);P.AGCR(X); | close= | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('LinesModalHandler')]
    procedure ListNewLine_RunModalFiltered_UpdateTrue_EmptyView_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', false);
        Expect('open=P.NEW;P.AGCR();P.NEW;P.AGCR(); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.V.end;P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=G0:O/,G1:X/, | setvalue= | shown2=X/V | rows2=G0:O/,G1:X/, | prev=F/X' +
            ' | prevlog= | next=T/ | nextlog=P.MOD(X/V);T.M(X/V);P.NEW;P.AGCR(); | close=' +
            ' | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines", 'G1');
        Closed();
    end;

    // Page 97473: no page trigger.

    [Test]
    procedure ListNewLine_Quiet_SaveRecordUpdateFalse_EmptyView_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines Quiet";
    begin
        Initialize(false, false, '', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:X/, | setvalue= | shown2=X/V' +
            ' | rows2=:X/, | prev=F/X | prevlog= | next=T/ | nextlog=T.M(X/V); | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    procedure ListNewLine_Quiet_SaveRecordUpdateFalse_RowBefore_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines Quiet";
    begin
        Initialize(false, true, '', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:A/,:X/, | setvalue=' +
            ' | shown2=X/V | rows2=:A/,:X/, | prev=T/A | prevlog=T.M(X/V); | next=T/X | nextlog= | close=' +
            ' | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_SaveRecordUpdateFalse_RowBefore_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/, | prev=T/A | prevlog=T.M(X/V); | next=T/X | nextlog=' +
            ' | close= | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_SaveRecordUpdateFalse_EmptyView_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:X/, | prev=F/X | prevlog= | next=T/ | nextlog=T.M(X/V); | close=' +
            ' | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    [Test]
    procedure ListNewLine_Quiet_UpdateTrue_EmptyView_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines Quiet";
    begin
        Initialize(false, false, '', false);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:X/, | setvalue= | shown2=X/V' +
            ' | rows2=:X/, | prev=F/X | prevlog= | next=T/ | nextlog=T.M(X/V); | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    procedure ListNewLine_Quiet_UpdateTrue_RowBefore_ShowsTheLine()
    var
        Lines: TestPage "LT NS Lines Quiet";
    begin
        Initialize(false, true, '', false);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=:A/,:X/, | setvalue=' +
            ' | shown2=X/V | rows2=:A/,:X/, | prev=T/A | prevlog=T.M(X/V); | next=T/X | nextlog= | close=' +
            ' | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveQuiet(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_UpdateTrue_RowBefore_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', false);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/, | prev=T/A | prevlog=T.M(X/V); | next=T/X | nextlog=' +
            ' | close= | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_UpdateTrue_EmptyView_ShowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', false);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:X/, | prev=F/X | prevlog= | next=T/ | nextlog=T.M(X/V); | close=' +
            ' | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    // Page 97474: OnAfterGetRecord only, one CardPart FactBox without triggers.

    [Test]
    procedure ListNewLine_PartHost_SaveRecordUpdateFalse_EmptyView_PartFollowsTheLine()
    var
        Lines: TestPage "LT NS Lines Part";
    begin
        Initialize(false, false, '', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/); | shown=X/ | part=X/ | rows1=:X/,' +
            ' | setvalue= | shown2=X/V | part2=X/ | rows2=:X/, | prev=F/X | prevlog= | prevpart=X/ | next=T/' +
            ' | nextlog=T.M(X/V); | nextpart=/ | close= | rows3=:X/V,');
        Lines.OpenEdit();
        DrivePart(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    procedure ListNewLine_PartHost_SaveRecordUpdateFalse_RowBefore_PartFollowsTheLine()
    var
        Lines: TestPage "LT NS Lines Part";
    begin
        Initialize(false, true, '', true);
        Expect('open=P.AGR(A/); | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/);P.AGR(A/); | shown=X/ | part=X/' +
            ' | rows1=:A/,:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=:A/,:X/, | prev=T/A' +
            ' | prevlog=P.AGR(X/);T.M(X/V); | prevpart=A/ | next=T/X | nextlog= | nextpart=X/V | close=' +
            ' | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DrivePart(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('PartModalHandler')]
    procedure ListNewLine_PartHost_RunModalFiltered_SaveRecordUpdateFalse_RowBefore_PartFollowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', true);
        Expect('open=P.AGR(A/); | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/);P.AGR(A/); | shown=X/ | part=X/' +
            ' | rows1=G0:O/,G1:A/,G1:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=G0:O/,G1:A/,G1:X/,' +
            ' | prev=T/A | prevlog=P.AGR(X/);T.M(X/V); | prevpart=A/ | next=T/X | nextlog= | nextpart=X/V' +
            ' | close= | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Part", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('PartModalHandler')]
    procedure ListNewLine_PartHost_RunModalFiltered_SaveRecordUpdateFalse_EmptyView_PartFollowsTheLine()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', true);
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/); | shown=X/ | part=X/' +
            ' | rows1=G0:O/,G1:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=G0:O/,G1:X/, | prev=F/X' +
            ' | prevlog= | prevpart=X/ | next=T/ | nextlog=T.M(X/V); | nextpart=/ | close=' +
            ' | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Part", 'G1');
        Closed();
    end;


    [Test]
    procedure ListNewLine_SaveRecordUpdateFalse_RowBefore_CloseSavesTheValue()
    var
        Lines: TestPage "LT NS Lines";
    begin
        Initialize(false, true, '', true);
        CloseAfterValue := true;
        Expect('open=P.AGR(A);P.AGR(A);P.AGCR(A); | new=P.NEW;P.AGCR();' +
            ' | setname=T.V(X);T.I(X);P.V(T);P.MOD(X/);T.M(X/);P.SR;P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);' +
            ' | shown=X/ | rows1=:A/,:X/, | setvalue= | shown2=X/V | rows2=:A/,:X/,' +
            ' | close=P.MOD(X/V);T.M(X/V); | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DriveLines(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_SaveRecordUpdateFalse_RowBefore_OkSavesTheValue()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', true);
        CloseAfterValue := true;
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/, | close=T.M(X/V); | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('QuietModalHandler')]
    procedure ListNewLine_Quiet_RunModalFiltered_UpdateTrue_RowBefore_OkSavesTheValue()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', false);
        CloseAfterValue := true;
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/); | shown=X/ | rows1=G0:O/,G1:A/,G1:X/, | setvalue=' +
            ' | shown2=X/V | rows2=G0:O/,G1:A/,G1:X/, | close=T.M(X/V); | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Quiet", 'G1');
        Closed();
    end;

    [Test]
    procedure ListNewLine_PartHost_SaveRecordUpdateFalse_RowBefore_CloseSavesTheValue()
    var
        Lines: TestPage "LT NS Lines Part";
    begin
        Initialize(false, true, '', true);
        CloseAfterValue := true;
        Expect('open=P.AGR(A/); | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/);P.AGR(A/); | shown=X/ | part=X/' +
            ' | rows1=:A/,:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=:A/,:X/,' +
            ' | close=P.AGR(X/);T.M(X/V); | rows3=:A/,:X/V,');
        Lines.OpenEdit();
        DrivePart(Lines);
        Lines.Close();
        Closed();
    end;

    [Test]
    [HandlerFunctions('PartModalHandler')]
    procedure ListNewLine_PartHost_RunModalFiltered_SaveRecordUpdateFalse_RowBefore_OkSavesTheValue()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, true, 'G1', true);
        CloseAfterValue := true;
        Expect('open=P.AGR(A/); | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/);P.AGR(A/); | shown=X/ | part=X/' +
            ' | rows1=G0:O/,G1:A/,G1:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=G0:O/,G1:A/,G1:X/,' +
            ' | close=P.AGR(X/);T.M(X/V); | rows3=G0:O/,G1:A/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Part", 'G1');
        Closed();
    end;

    [Test]
    [HandlerFunctions('PartModalHandler')]
    procedure ListNewLine_PartHost_RunModalFiltered_SaveRecordUpdateFalse_EmptyView_OkSavesTheValue()
    var
        Opener: Codeunit "LT NS Opener";
    begin
        Initialize(true, false, 'G1', true);
        CloseAfterValue := true;
        Expect('open= | new= | setname=T.V(X);T.I(X);T.M(X/);P.AGR(X/); | shown=X/ | part=X/' +
            ' | rows1=G0:O/,G1:X/, | setvalue= | shown2=X/V | part2=X/ | rows2=G0:O/,G1:X/,' +
            ' | close=T.M(X/V); | rows3=G0:O/,G1:X/V,');
        Opener.Open(Page::"LT NS Lines Part", 'G1');
        Closed();
    end;

    [ModalPageHandler]
    procedure LinesModalHandler(var Lines: TestPage "LT NS Lines")
    begin
        DriveLines(Lines);
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure QuietModalHandler(var Lines: TestPage "LT NS Lines Quiet")
    begin
        DriveQuiet(Lines);
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure PartModalHandler(var Lines: TestPage "LT NS Lines Part")
    begin
        DrivePart(Lines);
        Lines.OK().Invoke();
    end;
}
