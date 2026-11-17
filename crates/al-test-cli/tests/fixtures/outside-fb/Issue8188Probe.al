// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-subpagelink-property
// Scope: in-scope
// local object: the host's and the part's row triggers, which log, are what is under test
// Fixtures used: shared Assert (60021); own table 97050, log codeunit 97051, pages 97052 and 97053, test codeunit 97054
// Probe (al-runner issue 8188): a List page with OnAfterGetRecord and one CardPart FactBox linked to the row's key
//   (SubPageLink = "No." = field("No.")), the part logging OnAfterGetRecord and OnAfterGetCurrRecord. Readings: the triggers each step
//   runs and what the part shows, at the open of an empty view and of a view with one row, New() below that row, the SetValue of the key
//   and of a field on the new line, Previous(), Next() back to the saved row and Next() onto a new line, and Close().
//   Measured on a local BC 29.1 OnPrem w1 container; BC 27.5 and 28.5 are left to CI.
// BC versions: 27, 28, 29

table 97050 "LT FB Row"
{
    DataClassification = SystemMetadata;

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

page 97052 "LT FB Rows"
{
    PageType = List;
    SourceTable = "LT FB Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "LT FB Peek")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec."No." + ')');
    end;

    var
        Log: Codeunit "LT FB Log";
}

page 97053 "LT FB Peek"
{
    PageType = CardPart;
    SourceTable = "LT FB Row";
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
        Log.Add('F.AGR(' + Rec."No." + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('F.AGCR(' + Rec."No." + ')');
    end;

    var
        Log: Codeunit "LT FB Log";
}

codeunit 97051 "LT FB Log"
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

codeunit 97054 "LT FB Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT FB Log";
        Expected: List of [Text];
        StepNo: Integer;

    local procedure Initialize(RowBefore: Boolean; Steps: Text)
    var
        Row: Record "LT FB Row";
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

    // Each step's reading is compared with the next expected `name=value` in order.
    local procedure Check(Name: Text; Actual: Text)
    begin
        StepNo += 1;
        Assert.AreEqual(Expected.Get(StepNo), Name + '=' + Actual, 'step ' + Format(StepNo));
    end;

    local procedure Bool(Value: Boolean): Text
    begin
        if Value then
            exit('T');
        exit('F');
    end;

    [Test]
    procedure ListHostFactBox_EmptyView_PartRunsOnAfterGetCurrRecordOnTheBlankLine()
    var
        Rows: TestPage "LT FB Rows";
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
    procedure ListHostFactBox_RowBefore_New_PartKeepsTheRowUntilTheNextSetValue()
    var
        Rows: TestPage "LT FB Rows";
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
