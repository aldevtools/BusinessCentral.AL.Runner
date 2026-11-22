// The reporter's repro of al-runner issue 8637: the app and its test in one file.
table 50100 "Repro Cache Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "Group", "Line No.") { Clustered = true; }
    }
}

page 50101 "Repro Cache Rows"
{
    PageType = List;
    SourceTable = "Repro Cache Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Group"; Rec."Group") { }
                field("Line No."; Rec."Line No.") { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

codeunit 50102 "Repro Cache Reader"
{
    trigger OnRun()
    var
        Row: Record "Repro Cache Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'last':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindLast() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'empty':
                begin
                    Row.SetRange("Group", Group);
                    Seen := Format(Row.IsEmpty());
                end;
            'setfilter':
                begin
                    Row.SetFilter("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50120 "Repro Cache Read Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Inserts a line, then reads the group, so the Insert reaches the worker's session.
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "Repro Cache Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "Repro Cache Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    local procedure Run(var Rows: TestPage "Repro Cache Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"Repro Cache Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure Expect(Expected: Text; Actual: Text; What: Text)
    begin
        if Actual <> Expected then
            Error('%1: expected <%2>, got <%3>', What, Expected, Actual);
    end;

    [Test]
    procedure FindLast_ChangedRow_AnswersTheFirstRead()
    var
        Rows: TestPage "Repro Cache Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        Rows.OpenView();
        Expect('a', Run(Rows, 'last', G), 'first FindLast');
        SetRow(G, 1, 'b');
        Expect('a', Run(Rows, 'last', G), 'FindLast again');
    end;

    [Test]
    procedure IsEmpty_InsertedRow_AnswersTheFirstRead()
    var
        Rows: TestPage "Repro Cache Rows";
        G: Code[20];
        E: Code[20];
    begin
        G := NewGroup();
        E := NewGroup();
        AddRow(G, 1, 'a');
        Rows.OpenView();
        Expect('Yes', Run(Rows, 'empty', E), 'first IsEmpty');
        AddRow(E, 1, 'c');
        Expect('Yes', Run(Rows, 'empty', E), 'IsEmpty again');
    end;

    [Test]
    procedure SetFilterFindFirst_ChangedRow_AnswersTheFirstRead()
    var
        Rows: TestPage "Repro Cache Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        Rows.OpenView();
        Expect('a', Run(Rows, 'setfilter', G), 'first FindFirst');
        SetRow(G, 1, 'b');
        Expect('a', Run(Rows, 'setfilter', G), 'FindFirst again');
    end;
}
