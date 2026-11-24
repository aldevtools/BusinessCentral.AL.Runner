// singleinstance-shared-on-purpose: the Write/Read test pairs read what the previous test's page close logged, and every other test resets the shared log with Help.Setup first.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-page-triggers
// Scope: in-scope
// Fixtures used: none (this file defines them)
// Area: errors and unhandled UI when a page closes, matrix of WHERE the error is raised (OnQueryClosePage, OnClosePage, the page's
//   OnModifyRecord or OnInsertRecord while the page saves on close, the table's OnModify or OnInsert, a handler, a Confirm nobody handles)
//   crossed with HOW the page closes (TestPage.Close, OK, Cancel, Close() in a handler, a handler that returns, end of a test) and the
//   page shape (card, list, document with parts, lookup list opened from a TableRelation). Each test states its cell in the CLAIM line
//   and asserts: how the call ended (the RunModal result or the error text and code), the triggers that ran in order, the rows afterwards.
//   Conventions of the observation: 'returned' is written only when the call came back; it holds the result, the trigger log so far and
//   the rows (inside the failed statement the rows are the page's writes). 'end|text' is CEM_NO_ERROR when the call came back, else the
//   error. 'log' and 'rows' are read after the statement; an error rolls the rows back to the last Commit().
// local object: the matrix needs its own pages with switchable failing triggers
// BC versions: 27, 28, 29
//
// The fixtures of the close-errors matrix. Codeunit "CEM Cfg" switches ONE failure on for ONE page (Scope) and records every trigger that
// runs; the pages, the tables and the handlers of the test codeunits all ask it what to do. Nothing here fails unless a test switches it on.

table 99500 "CEM Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Note; Text[30]) { }
        field(4; Ref; Code[10]) { TableRelation = "CEM Code"; }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Cfg: Codeunit "CEM Cfg";
    begin
        Cfg.Add('T-ins ' + Rec."Code" + ' ' + Rec.Name);
        if Rec.Name = 'FAILINS' then
            Error('CEM table insert refused');
    end;

    trigger OnModify()
    var
        Cfg: Codeunit "CEM Cfg";
    begin
        Cfg.Add('T-mod ' + Rec."Code" + ' ' + Rec.Name);
        if Rec.Name = 'FAILMOD' then
            Error('CEM table modify refused');
    end;
}

table 99501 "CEM Code"
{
    DataClassification = SystemMetadata;
    LookupPageId = "CEM Codes";
    DrillDownPageId = "CEM Codes";

    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Description; Text[30]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

table 99502 "CEM Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; DocCode; Code[10]) { }
        field(2; LineNo; Integer) { }
        field(3; Qty; Decimal) { }
        field(4; RefCode; Code[10]) { TableRelation = "CEM Code"; }
    }

    keys
    {
        key(PK; DocCode, LineNo) { Clustered = true; }
    }

    trigger OnModify()
    var
        Cfg: Codeunit "CEM Cfg";
    begin
        Cfg.Add('L-mod ' + Rec.DocCode + ' ' + Format(Rec.LineNo) + ' ' + Format(Rec.Qty));
        if Rec.Qty = -1 then
            Error('CEM line modify refused');
    end;
}

codeunit 99503 "CEM Cfg"
{
    SingleInstance = true;

    var
        Source: Text;
        Scope: Text;
        HandlerMode: Integer;
        Lines: Text;

    procedure Set(NewSource: Text; NewScope: Text; NewMode: Integer)
    begin
        Source := NewSource;
        Scope := NewScope;
        HandlerMode := NewMode;
        Lines := '';
    end;

    procedure Quiet()
    begin
        Source := 'none';
    end;

    procedure Mode(): Integer
    begin
        exit(HandlerMode);
    end;

    procedure Add(Line: Text)
    begin
        Lines += Line + '; ';
    end;

    procedure Peek(): Text
    begin
        exit(Lines);
    end;

    procedure Take() Result: Text
    begin
        Result := Lines;
        Lines := '';
    end;

    local procedure Active(Where: Text; What: Text): Boolean
    begin
        exit((Source = What) and (Scope = Where));
    end;

    procedure Query(Where: Text; CloseAction: Action): Boolean
    begin
        Add('Q-' + Where + ' ' + Format(CloseAction));
        if Active(Where, 'query') then
            Error('CEM query close error');
        exit(true);
    end;

    procedure Closed(Where: Text)
    var
        Answer: Boolean;
    begin
        Add('C-' + Where);
        if Active(Where, 'close') then
            Error('CEM close page error');
        if Active(Where, 'confirm') then begin
            Answer := Confirm('CEM close confirm?');
            Add('C-' + Where + ' confirm=' + Format(Answer));
        end;
    end;

    procedure ModRec(Where: Text; NameValue: Text)
    begin
        Add('M-' + Where + ' ' + NameValue);
        if Active(Where, 'modrec') then
            Error('CEM page modify error');
    end;

    procedure InsRec(Where: Text; NameValue: Text)
    begin
        Add('I-' + Where + ' ' + NameValue);
        if Active(Where, 'insrec') then
            Error('CEM page insert error');
    end;
}

page 99504 "CEM Card"
{
    PageType = Card;
    SourceTable = "CEM Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { ApplicationArea = All; }
            field(Name; Rec.Name) { ApplicationArea = All; }
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Ref; Rec.Ref) { ApplicationArea = All; }
        }
    }

    var
        Cfg: Codeunit "CEM Cfg";

    trigger OnModifyRecord(): Boolean
    begin
        Cfg.ModRec('card', Rec.Name);
        exit(true);
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Cfg.InsRec('card', Rec.Name);
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        exit(Cfg.Query('card', CloseAction));
    end;

    trigger OnClosePage()
    begin
        Cfg.Closed('card');
    end;
}

page 99505 "CEM List"
{
    PageType = List;
    SourceTable = "CEM Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Ref; Rec.Ref) { ApplicationArea = All; }
            }
        }
    }

    var
        Cfg: Codeunit "CEM Cfg";

    trigger OnModifyRecord(): Boolean
    begin
        Cfg.ModRec('list', Rec.Name);
        exit(true);
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Cfg.InsRec('list', Rec.Name);
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        exit(Cfg.Query('list', CloseAction));
    end;

    trigger OnClosePage()
    begin
        Cfg.Closed('list');
    end;
}

page 99506 "CEM Codes"
{
    PageType = List;
    SourceTable = "CEM Code";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    var
        Cfg: Codeunit "CEM Cfg";

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        exit(Cfg.Query('codes', CloseAction));
    end;

    trigger OnClosePage()
    begin
        Cfg.Closed('codes');
    end;
}

page 99507 "CEM Lines Part"
{
    PageType = ListPart;
    SourceTable = "CEM Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(DocCode; Rec.DocCode) { ApplicationArea = All; }
                field(LineNo; Rec.LineNo) { ApplicationArea = All; }
                field(Qty; Rec.Qty) { ApplicationArea = All; }
                field(RefCode; Rec.RefCode) { ApplicationArea = All; }
            }
        }
    }
}

page 99508 "CEM Info Part"
{
    PageType = CardPart;
    SourceTable = "CEM Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
        }
    }
}

page 99509 "CEM Doc"
{
    PageType = Document;
    SourceTable = "CEM Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update();
                    end;
                }
                field(Ref; Rec.Ref) { ApplicationArea = All; }
            }
            part(Lines; "CEM Lines Part")
            {
                ApplicationArea = All;
                SubPageLink = DocCode = field("Code");
            }
        }
        area(FactBoxes)
        {
            part(Info; "CEM Info Part")
            {
                ApplicationArea = All;
                SubPageLink = "Code" = field("Code");
            }
            part(Hidden; "CEM Info Part")
            {
                ApplicationArea = All;
                Visible = false;
                SubPageLink = "Code" = field("Code");
            }
            part(Extra; "CEM Info Part")
            {
                ApplicationArea = All;
                Visible = ExtraVisible;
            }
            systempart(RecordLinks; Links)
            {
                ApplicationArea = All;
                Visible = false;
            }
        }
    }

    var
        Cfg: Codeunit "CEM Cfg";
        ExtraVisible: Boolean;

    trigger OnModifyRecord(): Boolean
    begin
        Cfg.ModRec('doc', Rec.Name);
        exit(true);
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        exit(Cfg.Query('doc', CloseAction));
    end;

    trigger OnClosePage()
    begin
        Cfg.Closed('doc');
    end;
}

page 99510 "CEM Wizard"
{
    PageType = NavigatePage;
    SourceTable = "CEM Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ActionNext)
            {
                ApplicationArea = All;
                InFooterBar = true;

                trigger OnAction()
                var
                    Cfg: Codeunit "CEM Cfg";
                begin
                    Cfg.Add('A-next');
                end;
            }
        }
    }

    var
        Cfg: Codeunit "CEM Cfg";

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        exit(Cfg.Query('wizard', CloseAction));
    end;

    trigger OnClosePage()
    begin
        Cfg.Closed('wizard');
    end;
}

codeunit 99511 "CEM Runner"
{
    procedure RunCardStatic(var Row: Record "CEM Row")
    begin
        Page.Run(Page::"CEM Card", Row);
    end;

    procedure RunCardVariable(var Row: Record "CEM Row")
    var
        CardPage: Page "CEM Card";
    begin
        CardPage.SetRecord(Row);
        CardPage.Run();
    end;

    procedure RunWizard(var Row: Record "CEM Row")
    var
        Wizard: Page "CEM Wizard";
    begin
        Wizard.SetRecord(Row);
        Wizard.Run();
    end;

    procedure RunListStatic(var Row: Record "CEM Row")
    begin
        Page.Run(Page::"CEM List", Row);
    end;

    procedure ModalCardVariable(var Row: Record "CEM Row") Result: Action
    var
        CardPage: Page "CEM Card";
    begin
        CardPage.SetRecord(Row);
        Result := CardPage.RunModal();
    end;

    procedure ModalListVariable(var Row: Record "CEM Row") Result: Action
    var
        ListPage: Page "CEM List";
    begin
        ListPage.SetTableView(Row);
        Result := ListPage.RunModal();
    end;
}

codeunit 99512 "CEM Help"
{
    procedure Setup(Source: Text; Scope: Text; Mode: Integer)
    var
        Cfg: Codeunit "CEM Cfg";
        Row: Record "CEM Row";
        CodeRec: Record "CEM Code";
        Line: Record "CEM Line";
    begin
        Row.DeleteAll();
        CodeRec.DeleteAll();
        Line.DeleteAll();
        Cfg.Set('none', '', 0);
        Row.Init();
        Row."Code" := 'R1';
        Row.Name := 'start';
        Row.Insert();
        CodeRec."Code" := 'C1';
        CodeRec.Description := 'first';
        CodeRec.Insert();
        CodeRec."Code" := 'C2';
        CodeRec.Description := 'second';
        CodeRec.Insert();
        Line.DocCode := 'R1';
        Line.LineNo := 1;
        Line.Qty := 1;
        Line.Insert();
        Line.LineNo := 2;
        Line.Qty := 2;
        Line.Insert();
        Commit();
        Cfg.Set(Source, Scope, Mode);
        ClearLastError();
    end;

    procedure Names() Result: Text
    var
        Row: Record "CEM Row";
        Line: Record "CEM Line";
    begin
        if Row.FindSet() then
            repeat
                Result += Row."Code" + '=' + Row.Name + '/' + Row.Ref + ' ';
            until Row.Next() = 0;
        Result += '(' + Format(Row.Count()) + ' rows)';
        if Line.FindSet() then
            repeat
                Result += ' L' + Format(Line.LineNo) + '=' + Format(Line.Qty);
            until Line.Next() = 0;
    end;
}
