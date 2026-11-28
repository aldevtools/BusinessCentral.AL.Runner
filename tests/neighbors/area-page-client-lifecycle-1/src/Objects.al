table 50300 "NCap Row"
{
    fields
    {
        field(1; No; Code[20]) { }
    }
}

table 50301 "NCap Field Row"
{
    fields
    {
        field(1; No; Code[20]) { }
        field(2; Caption; Text[30]) { }
    }
}

// An own Caption(): Text; locals, parameters and a named return value called Caption.
page 50300 "NCap Text Card"
{
    PageType = Card;
    SourceTable = "NCap Row";
    Caption = 'Static';

    procedure Caption(): Text
    begin
        exit('Own');
    end;

    procedure LocalNamedCaption(): Text
    var
        Caption: Text;
    begin
        Caption := 'Loc';
        exit(Caption);
    end;

    procedure ParameterNamedCaption(Caption: Text): Text
    begin
        Caption := Caption + 'P';
        exit(Caption);
    end;

    procedure ReturnNamedCaption() Caption: Text
    begin
        Caption := 'Ret';
    end;

    procedure OwnText(): Text
    var
        S: Text;
    begin
        S := Caption();
        exit(S);
    end;
}

// An own Caption(): Integer: reads that compile (an Integer into an Integer, the platform caption into a Text).
page 50301 "NCap Integer Card"
{
    PageType = Card;
    SourceTable = "NCap Row";
    Caption = 'Static';

    var
        Opened: Text;

    trigger OnOpenPage()
    begin
        Opened := CurrPage.Caption;
    end;

    procedure Caption(): Integer
    begin
        exit(5);
    end;

    procedure OwnInteger(): Integer
    var
        I: Integer;
    begin
        I := Caption();
        exit(I);
    end;

    procedure Five(): Integer
    begin
        exit(5);
    end;

    procedure OtherIntegerIntoInteger(): Integer
    var
        I: Integer;
    begin
        I := Five();
        exit(I);
    end;

    procedure Name(): Text
    begin
        exit('N');
    end;

    [TryFunction]
    procedure TryIt()
    begin
    end;

    procedure NoResult()
    begin
    end;
}

// A source-table field called Caption beside an own Caption(): Text.
page 50302 "NCap Field Card"
{
    PageType = Card;
    SourceTable = "NCap Field Row";
    Caption = 'Static';

    procedure Caption(): Text
    begin
        exit('Own');
    end;

    procedure AssignField(): Text
    begin
        Caption := 'Fld';
        exit(Rec.Caption);
    end;
}
