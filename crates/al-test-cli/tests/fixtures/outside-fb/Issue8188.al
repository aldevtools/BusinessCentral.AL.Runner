// al-runner issue 8188: the reporter's repro (green on real BC 29.1, 2/2), app and test app in one file.
table 50100 "Repro Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 50110 "Repro Log"
{
    SingleInstance = true;

    var
        Buffer: Text;

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
}

page 50100 "Repro Rows"
{
    PageType = List;
    SourceTable = "Repro Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "Repro Peek")
            {
                SubPageLink = "No." = field("No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec."No." + ')');
    end;

    var
        Log: Codeunit "Repro Log";
}

page 50101 "Repro Peek"
{
    PageType = CardPart;
    SourceTable = "Repro Row";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('F.AGR(' + Rec."No." + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('F.AGCR(' + Rec."No." + ')');
    end;

    var
        Log: Codeunit "Repro Log";
}

codeunit 50120 "Repro FactBox Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "Repro Log";
        Expected: List of [Text];
        StepNo: Integer;

    local procedure Initialize(RowBefore: Boolean; Steps: Text)
    var
        Row: Record "Repro Row";
        Step: Text;
    begin
        Row.DeleteAll();
        if RowBefore then begin
            Row."No." := 'A';
            Row.Name := 'Alpha';
            Row.Insert();
        end;
        Log.Take();
        Clear(Expected);
        StepNo := 0;
        foreach Step in Steps.Split(' | ') do
            Expected.Add(Step);
    end;

    local procedure Check(Name: Text; Actual: Text)
    begin
        StepNo += 1;
        if Expected.Get(StepNo) <> Name + '=' + Actual then
            Error('Step %1: expected <%2>, got <%3>.', StepNo, Expected.Get(StepNo), Name + '=' + Actual);
    end;

    local procedure Bool(Value: Boolean): Text
    begin
        if Value then
            exit('T');
        exit('F');
    end;

    [Test]
    procedure EmptyView_PartRunsOnAfterGetCurrRecordOnTheBlankLine()
    var
        Rows: TestPage "Repro Rows";
    begin
        Initialize(false, 'open=F.AGCR(); | part= | setno=F.AGCR(); | part2= | setname=F.AGR(B);F.AGCR(B); | part3= | close=');
        Rows.OpenEdit();
        Check('open', Log.Take());
        Check('part', Rows.Peek.Name.Value());
        Rows."No.".SetValue('B');
        Check('setno', Log.Take());
        Check('part2', Rows.Peek.Name.Value());
        Rows.Name.SetValue('N');
        Check('setname', Log.Take());
        Check('part3', Rows.Peek.Name.Value());
        Rows.Close();
        Check('close', Log.Take());
    end;

    [Test]
    procedure RowBefore_New_PartKeepsTheRowUntilTheNextSetValue()
    var
        Rows: TestPage "Repro Rows";
    begin
        Initialize(true, 'open=P.AGR(A);F.AGR(A);F.AGCR(A); | part=Alpha | new= | partnew=Alpha | setno=F.AGCR();F.AGCR(); | part2=' +
            ' | setname=F.AGR(B);F.AGCR(B); | part3= | prev=T/A | prevlog=F.AGR(A);F.AGCR(A); | part4=Alpha' +
            ' | next=T/B | nextlog=F.AGR(B);F.AGCR(B); | part5=N | next2=T/ | next2log=F.AGCR(); | part6= | close=');
        Rows.OpenEdit();
        Check('open', Log.Take());
        Check('part', Rows.Peek.Name.Value());
        Rows.New();
        Check('new', Log.Take());
        Check('partnew', Rows.Peek.Name.Value());
        Rows."No.".SetValue('B');
        Check('setno', Log.Take());
        Check('part2', Rows.Peek.Name.Value());
        Rows.Name.SetValue('N');
        Check('setname', Log.Take());
        Check('part3', Rows.Peek.Name.Value());
        Check('prev', Bool(Rows.Previous()) + '/' + Rows."No.".Value());
        Check('prevlog', Log.Take());
        Check('part4', Rows.Peek.Name.Value());
        Check('next', Bool(Rows.Next()) + '/' + Rows."No.".Value());
        Check('nextlog', Log.Take());
        Check('part5', Rows.Peek.Name.Value());
        Check('next2', Bool(Rows.Next()) + '/' + Rows."No.".Value());
        Check('next2log', Log.Take());
        Check('part6', Rows.Peek.Name.Value());
        Rows.Close();
        Check('close', Log.Take());
    end;
}
