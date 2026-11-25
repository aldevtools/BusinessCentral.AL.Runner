table 97716 "BCE R2 Line"
{
    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Kind; Code[10]) { }
    }
    keys { key(PK; Parent, "Line No.") { } }
}

table 97717 "BCE R2 Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Kind Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(3; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = Count("BCE R2 Line" where(Parent = field("No."), Kind = field("Kind Filter")));
        }
    }
    keys { key(PK; "No.") { } }
}

page 97718 "BCE R2 Lines"
{
    PageType = List;
    SourceTable = "BCE R2 Line";
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } } } }
}

codeunit 97719 "BCE R2 Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Row: Record "BCE R2 Row";
        Ref: RecordRef;
        KeyField: FieldRef;
        CountField: FieldRef;
        G: Code[20];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        G := Parameters.Get('Group');
        case Parameters.Get('Mode') of
            'flowfilter':
                begin
                    Row."No." := G;
                    Row.SetRange("Kind Filter", 'K');
                    Row.CalcFields("Line Count");
                    Seen := Format(Row."Line Count");
                end;
            'tempfieldref':
                begin
                    Ref.Open(Database::"BCE R2 Row", true);
                    KeyField := Ref.Field(1);
                    KeyField.Value := G;
                    CountField := Ref.Field(3);
                    CountField.CalcField();
                    Seen := Format(CountField.Value);
                    Ref.Close();
                end;
            'recordreftruncate':
                begin
                    Row."No." := G;
                    Row.CalcFields("Line Count");
                    Seen := Format(Row."Line Count");
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97720 "BCE Review R2 Tests"
{
    Subtype = Test;

    local procedure AddLine(G: Code[20]; LineNo: Integer)
    var
        Line: Record "BCE R2 Line";
    begin
        Line.Parent := G;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
        Line.Insert();
        Line.SetRange(Parent, G);
        if Line.FindFirst() then;
    end;

    local procedure Read(var Lines: TestPage "BCE R2 Lines"; Mode: Text; G: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Group', G);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCE R2 Reader", Parameters, false);
        exit(Results.Get('Seen'));
    end;

    local procedure FirstThenTruncate(Mode: Text): Text
    var
        Lines: TestPage "BCE R2 Lines";
        Row: Record "BCE R2 Row";
        Ref: RecordRef;
        KeyField: FieldRef;
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row."No." := G;
        Row.Insert();
        AddLine(G, 1);
        Commit();
        Lines.OpenView();
        Seen := Read(Lines, Mode, G);
        if Mode = 'recordreftruncate' then begin
            Ref.Open(Database::"BCE R2 Row");
            KeyField := Ref.Field(1);
            KeyField.SetRange(G);
            Ref.Truncate();
            Ref.Close();
        end else
            Row.Truncate();
        Commit();
        AddLine(G, 0);
        AddLine(G, 3);
        Seen += ' / ' + Read(Lines, Mode, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure CalcFieldsWithFlowFilterRefused()
    begin
        if FirstThenTruncate('flowfilter') <> '1 / 1' then
            Error('unexpected answer');
    end;

    [Test]
    procedure TemporaryFieldRefCalcFieldRefused()
    begin
        if FirstThenTruncate('tempfieldref') <> '1 / 1' then
            Error('unexpected answer');
    end;

    [Test]
    procedure FilteredRecordRefTruncateRefused()
    begin
        if FirstThenTruncate('recordreftruncate') <> '1 / 1' then
            Error('unexpected answer');
    end;

    [Test]
    procedure WithoutTruncateKeepsMainAnswer()
    var
        Lines: TestPage "BCE R2 Lines";
        Row: Record "BCE R2 Row";
        G: Code[20];
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row."No." := G;
        Row.Insert();
        AddLine(G, 1);
        Commit();
        Lines.OpenView();
        if Read(Lines, 'recordreftruncate', G) <> '1' then
            Error('unexpected answer');
        Lines.Close();
    end;
}
