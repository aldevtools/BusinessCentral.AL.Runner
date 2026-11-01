// Tables and pages of the corpus probe pages/lt-y45-save-before-action (Y45Tables.al, Y45Pages.al), for the unit tests in dirty_save.rs.
table 92880 "LT Y45 Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20])
        {
            trigger OnValidate()
            begin
                Log.Add('vCode ' + Rec."Code");
            end;
        }
        field(2; Name; Text[50])
        {
            trigger OnValidate()
            begin
                Log.Add('vName ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
            end;
        }
        field(3; Qty; Integer)
        {
            trigger OnValidate()
            begin
                Log.Add('vQty ' + Format(Rec.Qty));
                if Rec.Qty < 0 then
                    Error('Qty must not be negative.');
            end;
        }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        Log.Add('tI ' + Rec."Code" + ' ' + Rec.Name + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    trigger OnModify()
    begin
        Log.Add('tM ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
        if Rec.Name = 'boom' then
            Error('Modify refused for boom.');
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

table 92881 "LT Y45 Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Doc; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Desc; Text[50])
        {
            trigger OnValidate()
            begin
                Log.Add('vDesc ' + Rec.Desc + '/' + xRec.Desc);
            end;
        }
    }

    keys
    {
        key(PK; Doc, "Line No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        Log.Add('lI ' + Rec.Doc + ' ' + Format(Rec."Line No.") + ' ' + Rec.Desc + ' head=' + Log.HeadDb(Rec.Doc));
    end;

    trigger OnModify()
    begin
        Log.Add('lM ' + Format(Rec."Line No.") + ' ' + Rec.Desc + '/' + xRec.Desc + ' db=' + Log.LineDb(Rec.Doc, Rec."Line No."));
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

codeunit 92880 "LT Y45 Log"
{
    SingleInstance = true;

    var
        Buf: Text;

    procedure Add(Entry: Text)
    begin
        if Buf <> '' then
            Buf += ';';
        Buf += Entry;
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buf;
        Buf := '';
        exit(Result);
    end;

    procedure HeadDb(HeadCode: Code[20]): Text
    var
        Head: Record "LT Y45 Head";
    begin
        if Head.Get(HeadCode) then
            exit(Head.Name + '/' + Format(Head.Qty));
        exit('none');
    end;

    procedure LineDb(DocCode: Code[20]; LineNo: Integer): Text
    var
        Line: Record "LT Y45 Line";
    begin
        if Line.Get(DocCode, LineNo) then
            exit(Line.Desc);
        exit('none');
    end;

    procedure Act(Tag: Text; Head: Record "LT Y45 Head")
    begin
        Add(Tag + ' ' + Head."Code" + ' rec=' + Head.Name + '/' + Format(Head.Qty) + ' db=' + HeadDb(Head."Code") + ' l1=' + LineDb(Head."Code", 10000));
    end;

    procedure LineAct(Tag: Text; Line: Record "LT Y45 Line")
    begin
        Add(Tag + ' ' + Format(Line."Line No.") + ' rec=' + Line.Desc + ' db=' + LineDb(Line.Doc, Line."Line No.") + ' head=' + HeadDb(Line.Doc));
    end;
}

codeunit 92881 "LT Y45 Subscribers"
{
    [EventSubscriber(ObjectType::Table, Database::"LT Y45 Head", 'OnBeforeModifyEvent', '', false, false)]
    local procedure HeadBeforeModify(var Rec: Record "LT Y45 Head"; var xRec: Record "LT Y45 Head"; RunTrigger: Boolean)
    begin
        Log.Add('eBM ' + Rec.Name + ' t=' + Format(RunTrigger) + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    [EventSubscriber(ObjectType::Table, Database::"LT Y45 Head", 'OnAfterModifyEvent', '', false, false)]
    local procedure HeadAfterModify(var Rec: Record "LT Y45 Head"; var xRec: Record "LT Y45 Head"; RunTrigger: Boolean)
    begin
        Log.Add('eAM db=' + Log.HeadDb(Rec."Code"));
    end;

    [EventSubscriber(ObjectType::Table, Database::"LT Y45 Head", 'OnBeforeInsertEvent', '', false, false)]
    local procedure HeadBeforeInsert(var Rec: Record "LT Y45 Head"; RunTrigger: Boolean)
    begin
        Log.Add('eBI ' + Rec."Code" + ' t=' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"LT Y45 Line", 'OnBeforeModifyEvent', '', false, false)]
    local procedure LineBeforeModify(var Rec: Record "LT Y45 Line"; var xRec: Record "LT Y45 Line"; RunTrigger: Boolean)
    begin
        Log.Add('eBML ' + Rec.Desc);
    end;

    [EventSubscriber(ObjectType::Table, Database::"LT Y45 Line", 'OnBeforeInsertEvent', '', false, false)]
    local procedure LineBeforeInsert(var Rec: Record "LT Y45 Line"; RunTrigger: Boolean)
    begin
        Log.Add('eBIL ' + Rec.Desc);
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

codeunit 92882 "LT Y45 RunObj"
{
    TableNo = "LT Y45 Head";

    trigger OnRun()
    var
        Log: Codeunit "LT Y45 Log";
    begin
        Log.Act('cu', Rec);
    end;
}

page 92880 "LT Y45 Head Card"
{
    PageType = Card;
    SourceTable = "LT Y45 Head";
    ApplicationArea = All;
    UsageCategory = None;
    Caption = 'LT Y45 Head Card';

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Qty; Rec.Qty) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadAct)
            {
                ApplicationArea = All;
                Caption = 'Read';

                trigger OnAction()
                begin
                    Log.Act('act', Rec);
                end;
            }
            action(RunCu)
            {
                ApplicationArea = All;
                Caption = 'Run codeunit';
                RunObject = codeunit "LT Y45 RunObj";
            }
            action(RunPg)
            {
                ApplicationArea = All;
                Caption = 'Run page';
                RunObject = page "LT Y45 Peek";
                RunPageLink = "Code" = field("Code");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('pN');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('pI ' + Rec."Code" + ' ' + Rec.Name);
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('pM ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('pQC ' + Format(CloseAction) + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    trigger OnClosePage()
    begin
        Log.Add('pClose');
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

page 92881 "LT Y45 Head List"
{
    PageType = List;
    SourceTable = "LT Y45 Head";
    ApplicationArea = All;
    UsageCategory = None;
    Caption = 'LT Y45 Head List';
    Editable = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Qty; Rec.Qty) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadAct)
            {
                ApplicationArea = All;
                Caption = 'Read';

                trigger OnAction()
                begin
                    Log.Act('act', Rec);
                end;
            }
            action(RunCu)
            {
                ApplicationArea = All;
                Caption = 'Run codeunit';
                RunObject = codeunit "LT Y45 RunObj";
            }
            action(RunPg)
            {
                ApplicationArea = All;
                Caption = 'Run page';
                RunObject = page "LT Y45 Peek";
                RunPageLink = "Code" = field("Code");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('pN');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('pI ' + Rec."Code" + ' ' + Rec.Name);
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('pM ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('pQC ' + Format(CloseAction) + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    trigger OnClosePage()
    begin
        Log.Add('pClose');
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

page 92883 "LT Y45 Doc"
{
    PageType = Document;
    SourceTable = "LT Y45 Head";
    ApplicationArea = All;
    UsageCategory = None;
    Caption = 'LT Y45 Doc';

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Qty; Rec.Qty) { ApplicationArea = All; }
            }
            part(Lines; "LT Y45 Lines")
            {
                ApplicationArea = All;
                SubPageLink = Doc = field("Code");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadAct)
            {
                ApplicationArea = All;
                Caption = 'Read';

                trigger OnAction()
                begin
                    Log.Act('act', Rec);
                end;
            }
            action(RunCu)
            {
                ApplicationArea = All;
                Caption = 'Run codeunit';
                RunObject = codeunit "LT Y45 RunObj";
            }
            action(RunPg)
            {
                ApplicationArea = All;
                Caption = 'Run page';
                RunObject = page "LT Y45 Peek";
                RunPageLink = "Code" = field("Code");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('pN');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('pI ' + Rec."Code" + ' ' + Rec.Name);
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('pM ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('pQC ' + Format(CloseAction) + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    trigger OnClosePage()
    begin
        Log.Add('pClose');
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

page 92884 "LT Y45 Card Part"
{
    PageType = Card;
    SourceTable = "LT Y45 Head";
    ApplicationArea = All;
    UsageCategory = None;
    Caption = 'LT Y45 Card Part';

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Qty; Rec.Qty) { ApplicationArea = All; }
            }
            part(Lines; "LT Y45 Lines")
            {
                ApplicationArea = All;
                SubPageLink = Doc = field("Code");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadAct)
            {
                ApplicationArea = All;
                Caption = 'Read';

                trigger OnAction()
                begin
                    Log.Act('act', Rec);
                end;
            }
            action(RunCu)
            {
                ApplicationArea = All;
                Caption = 'Run codeunit';
                RunObject = codeunit "LT Y45 RunObj";
            }
            action(RunPg)
            {
                ApplicationArea = All;
                Caption = 'Run page';
                RunObject = page "LT Y45 Peek";
                RunPageLink = "Code" = field("Code");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('pN');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('pI ' + Rec."Code" + ' ' + Rec.Name);
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('pM ' + Rec.Name + '/' + xRec.Name + ' db=' + Log.HeadDb(Rec."Code"));
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('pQC ' + Format(CloseAction) + ' db=' + Log.HeadDb(Rec."Code"));
    end;

    trigger OnClosePage()
    begin
        Log.Add('pClose');
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

page 92882 "LT Y45 Lines"
{
    PageType = ListPart;
    SourceTable = "LT Y45 Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    Caption = 'LT Y45 Lines';

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Desc; Rec.Desc) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(LineAct)
            {
                ApplicationArea = All;
                Caption = 'Read line';

                trigger OnAction()
                begin
                    Log.LineAct('lact', Rec);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('lC ' + Format(Rec."Line No."));
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('lpN');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('lpI ' + Format(Rec."Line No.") + ' ' + Rec.Desc);
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('lpM ' + Format(Rec."Line No.") + ' ' + Rec.Desc + '/' + xRec.Desc + ' db=' + Log.LineDb(Rec.Doc, Rec."Line No."));
        exit(true);
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

page 92885 "LT Y45 Peek"
{
    PageType = Card;
    SourceTable = "LT Y45 Head";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;
    Caption = 'LT Y45 Peek';

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        Log.Add('peek open f=' + Rec.GetFilter("Code") + ' db=' + Log.HeadDb(CopyStr(Rec.GetFilter("Code"), 1, 20)));
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('peek C ' + Rec.Name);
    end;

    var
        Log: Codeunit "LT Y45 Log";
}

codeunit 92899 C
{
    var
        Log: Codeunit "LT Y45 Log";

    local procedure Init()
    var
        Head: Record "LT Y45 Head";
        Line: Record "LT Y45 Line";
    begin
        Head.DeleteAll();
        Line.DeleteAll();
        Head."Code" := 'H1';
        Head.Name := 'old';
        Head.Qty := 1;
        Head.Insert();
        Head."Code" := 'H2';
        Head.Name := 'two';
        Head.Qty := 2;
        Head.Insert();
        AddLine('H1', 10000, 'a');
        AddLine('H1', 20000, 'b');
        AddLine('H2', 10000, 'c');
        Log.Take();
    end;

    local procedure AddLine(DocCode: Code[20]; LineNo: Integer; Desc: Text[50])
    var
        Line: Record "LT Y45 Line";
    begin
        Line.Doc := DocCode;
        Line."Line No." := LineNo;
        Line.Desc := Desc;
        Line.Insert();
    end;

    procedure CardRunCu(): Text
    var
        TP: TestPage "LT Y45 Head Card";
    begin
        Init();
        TP.OpenEdit();
        TP.Name.SetValue('new');
        Log.Take();
        TP.RunCu.Invoke();
        exit(Log.Take());
    end;

    procedure ListRunCu(): Text
    var
        TP: TestPage "LT Y45 Head List";
    begin
        Init();
        TP.OpenEdit();
        TP.Name.SetValue('new');
        Log.Take();
        TP.RunCu.Invoke();
        exit(Log.Take());
    end;

    procedure CardRunPg(): Text
    var
        TP: TestPage "LT Y45 Head Card";
        Peek: TestPage "LT Y45 Peek";
        Seen: Text;
    begin
        Init();
        TP.OpenEdit();
        TP.Name.SetValue('new');
        Log.Take();
        Peek.Trap();
        TP.RunPg.Invoke();
        Seen := Peek.Name.Value;
        exit(Seen + '|' + Log.Take());
    end;

    procedure CardSameName(): Text
    var
        TP: TestPage "LT Y45 Head Card";
    begin
        Init();
        TP.OpenEdit();
        TP.Name.SetValue('old');
        Log.Take();
        TP.ReadAct.Invoke();
        exit(Log.Take());
    end;

    procedure DocSetNameLineAction(): Text
    var
        TP: TestPage "LT Y45 Doc";
    begin
        Init();
        TP.OpenEdit();
        TP.Name.SetValue('new');
        Log.Take();
        TP.Lines.LineAct.Invoke();
        exit(Log.Take());
    end;

    procedure DocSetLineAction(): Text
    var
        TP: TestPage "LT Y45 Doc";
    begin
        Init();
        TP.OpenEdit();
        TP.Lines.Desc.SetValue('new');
        Log.Take();
        TP.ReadAct.Invoke();
        exit(Log.Take());
    end;

    procedure DocSetLineLineAction(): Text
    var
        TP: TestPage "LT Y45 Doc";
    begin
        Init();
        TP.OpenEdit();
        TP.Lines.Desc.SetValue('new');
        Log.Take();
        TP.Lines.LineAct.Invoke();
        exit(Log.Take());
    end;

    procedure DocNewLineAction(): Text
    var
        TP: TestPage "LT Y45 Doc";
    begin
        Init();
        TP.OpenEdit();
        TP.Lines.Last();
        TP.Lines.New();
        TP.Lines.Desc.SetValue('nl');
        Log.Take();
        TP.ReadAct.Invoke();
        exit(Log.Take());
    end;
}
