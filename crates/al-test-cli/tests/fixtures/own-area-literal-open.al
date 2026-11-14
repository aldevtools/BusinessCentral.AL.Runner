codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }

codeunit 50410 "Own Area Log"
{
    SingleInstance = true;
    var
        Calls: Text;
        Expr: Text;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Calls;
        Calls := '';
        if Result = '' then
            exit('none');
        exit(Result);
    end;

    procedure SetExpr(NewExpr: Text)
    begin
        Expr := NewExpr;
    end;

    procedure GetExpr(): Text
    begin
        exit(Expr);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if not CaptionArea.StartsWith('TST') then
            exit;
        Calls += '(' + CaptionArea + '|' + CaptionExpr + ')';
        Caption := 'R[' + CaptionExpr + ']';
        Resolved := true;
    end;
}

table 50411 "Own Area Literal Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Dimension 1"; Decimal) { CaptionClass = 'TSTL,2,1'; }
        field(3; "Not Shown"; Decimal) { CaptionClass = 'TSTM,2,1'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50412 "Own Area Expr Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dyn; Decimal) { Caption = 'Dyn Caption'; CaptionClass = DynCc(); }
    }
    keys { key(PK; Id) { Clustered = true; } }
    var
        Log: Codeunit "Own Area Log";

    local procedure DynCc(): Text
    begin
        exit(Log.GetExpr());
    end;
}

page 50413 "Own Area Literal Card"
{
    PageType = Card;
    SourceTable = "Own Area Literal Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
            field("Dimension 1"; Rec."Dimension 1") { ApplicationArea = All; }
        }
    }
}

page 50414 "Own Area Expr Card"
{
    PageType = Card;
    SourceTable = "Own Area Expr Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
            field(Dyn; Rec.Dyn) { ApplicationArea = All; }
        }
    }
}

page 50416 "Own Area Expr List"
{
    PageType = List;
    SourceTable = "Own Area Expr Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Dyn; Rec.Dyn) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 50415 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "Own Area Log";

    [Test]
    procedure LiteralReads()
    var
        Row: Record "Own Area Literal Row";
        Card: TestPage "Own Area Literal Card";
        Got: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        Log.Take();
        Card.OpenEdit();
        Got := 'open=' + Log.Take();
        Got += ' r1=[' + Card."Dimension 1".Caption() + '] ev=' + Log.Take();
        Got += ' r2=[' + Card."Dimension 1".Caption() + '] ev=' + Log.Take();
        Card.Close();
        Error(Got);
    end;

    [Test]
    procedure ExpressionRead()
    var
        Row: Record "Own Area Expr Row";
        Card: TestPage "Own Area Expr Card";
    begin
        Row.Id := 1;
        Row.Insert();
        Log.SetExpr('TSTX,2,1');
        Card.OpenEdit();
        Error('open=%1 cap=[%2]', Log.Take(), Card.Dyn.Caption());
    end;

    [Test]
    procedure ListExpressionRead()
    var
        Row: Record "Own Area Expr Row";
        List: TestPage "Own Area Expr List";
        Got: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        Log.SetExpr('TSTY,2,1');
        Log.Take();
        List.OpenView();
        Got := 'open=' + Log.Take();
        Got += ' r1=[' + List.Dyn.Caption() + '] ev=' + Log.Take();
        Got += ' r2=[' + List.Dyn.Caption() + '] ev=' + Log.Take();
        List.Close();
        Error(Got);
    end;
}
