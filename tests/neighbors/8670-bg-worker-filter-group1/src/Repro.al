// The reporter's repro of al-runner issue 8670 (app/Repro.al and test/Tests.Codeunit.al of the issue, in one file).
table 50140 "Repro G1 Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Tag; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50141 "Repro G1 Host"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50142 "Repro G1 Hosts"
{
    PageType = List;
    SourceTable = "Repro G1 Host";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
            }
        }
    }
}

codeunit 50143 "Repro G1 Reader"
{
    trigger OnRun()
    var
        Row: Record "Repro G1 Row";
        Results: Dictionary of [Text, Text];
    begin
        Results.Add('Count', Format(Row.Count(), 0, 9));
        Row.FilterGroup(1);
        Results.Add('Group1', Row.GetFilters());
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50144 "Repro G1 Worker Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure WorkerFreshRecord_DoesNotSeeTheTestSessionsGroup1Filter()
    var
        Hosts: TestPage "Repro G1 Hosts";
        Row: Record "Repro G1 Row";
        All: Record "Repro G1 Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Total: Integer;
        WorkerCount: Integer;
        Seen: Text;
        Group1: Text;
    begin
        Row."No." := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Row.Tag := 7;
        Row.Insert();
        Row."No." := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Row.Tag := 0;
        Row.Insert();
        Total := All.Count();

        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.FilterGroup(0);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"Repro G1 Reader", Parameters, false);
        Hosts.Close();
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.FilterGroup(0);

        Results.Get('Count', Seen);
        Evaluate(WorkerCount, Seen, 9);
        Results.Get('Group1', Group1);
        if (WorkerCount <> Total) or (Group1 <> '') then
            Error('The worker saw %1 of %2 rows and filter group 1 <%3>', WorkerCount, Total, Group1);
    end;
}
