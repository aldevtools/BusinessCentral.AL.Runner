// al-runner issue 8971: a memberless `var` Option parameter handed on to a `var` Option parameter with members. The run
// lets it through when the value carries exactly the callee's list and comes from a measured origin (assumption
// var-option-carried-members, corpus PR 1569); every neighbor below flips one fact of that condition.
table 62500 "OC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = " ",Skip,Update; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    procedure Confirm(var Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::Update;
        exit(Seen);
    end;

    [TryFunction]
    procedure TryConfirm(var Choice: Option " ",Skip,Update)
    begin
        Choice := Choice::Update;
    end;

    procedure ConfirmAbc(var Choice: Option A,B,C): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::C;
        exit(Seen);
    end;

    // An origin in a table procedure: no cell has one.
    procedure TableOrigin(): Text
    var
        Choice: Option " ",Skip,Update;
        Seen: Text;
    begin
        Choice := Choice::Skip;
        Seen := HopInTable(Choice);
        exit(Seen + '>' + Format(Choice));
    end;

    local procedure HopInTable(var V: Option): Text
    begin
        exit(Confirm(V));
    end;
}

interface "OC Hop"
{
    procedure Hop(var V: Option; var Row: Record "OC Row"): Text;
}

codeunit 62501 "OC Hop Impl" implements "OC Hop"
{
    procedure Hop(var V: Option; var Row: Record "OC Row"): Text
    begin
        exit(Row.Confirm(V));
    end;
}

codeunit 62502 "OC Events"
{
    procedure RaiseNoSender(var V: Option; var Row: Record "OC Row")
    begin
        OnNoSender(V, Row);
    end;

    procedure RaiseBusiness(var V: Option; var Row: Record "OC Row")
    begin
        OnBusiness(V, Row);
    end;

    procedure RaiseWithSender(var V: Option; var Row: Record "OC Row")
    begin
        OnWithSender(V, Row);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnNoSender(var V: Option; var Row: Record "OC Row")
    begin
    end;

    [BusinessEvent(true)]
    local procedure OnBusiness(var V: Option; var Row: Record "OC Row")
    begin
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record "OC Row")
    begin
    end;
}

codeunit 62503 "OC Subscribers"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnNoSender', '', false, false)]
    local procedure HandleNoSender(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnBusiness', '', false, false)]
    local procedure HandleBusiness(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnWithSender', '', false, false)]
    local procedure HandleWithSender(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record "OC Row")
    begin
        Row.Confirm(V);
    end;
}

codeunit 62504 "OC Carried Tests"
{
    Subtype = Test;

    var
        GlobalChoice: Option " ",Skip,Update;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    local procedure Hop(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.Confirm(V));
    end;

    local procedure HopAbc(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.ConfirmAbc(V));
    end;

    local procedure HopToTry(var V: Option): Boolean
    var
        Row: Record "OC Row";
    begin
        exit(Row.TryConfirm(V));
    end;

    [TryFunction]
    local procedure TryHop(var V: Option)
    var
        Row: Record "OC Row";
    begin
        Row.Confirm(V);
    end;

    local procedure ByValueOrigin(Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Hop(Choice);
        exit(Seen + '>' + Format(Choice));
    end;

    // The measured shape: a local of the `var` section with the callee's own list (VarHop_Same_*).
    [Test]
    procedure Same_LocalOrigin()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Hop(Choice) + '>' + Format(Choice), 'Skip|1>Update');
    end;

    // Another list, the same in origin and callee: the list's content is not read.
    [Test]
    procedure SameOtherList_LocalOrigin()
    var
        Choice: Option A,B,C;
    begin
        Choice := Choice::B;
        Check(HopAbc(Choice) + '>' + Format(Choice), 'B|1>C');
    end;

    // A memberless origin: the callee reads the value's list (measured, not modelled): refused.
    [Test]
    procedure MemberlessOrigin()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(Hop(Choice) + '>' + Format(Choice), '1|1>2');
    end;

    // Other members in the origin: refused (measured, not modelled).
    [Test]
    procedure OtherMembersOrigin()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Second;
        Check(Hop(Choice) + '>' + Format(Choice), 'Second|1>Third');
    end;

    [Test]
    procedure GlobalOrigin()
    begin
        GlobalChoice := GlobalChoice::Skip;
        Check(Hop(GlobalChoice) + '>' + Format(GlobalChoice), 'Skip|1>Update');
    end;

    [Test]
    procedure FieldOrigin()
    var
        Row: Record "OC Row";
    begin
        Row.Opt := Row.Opt::Skip;
        Check(Hop(Row.Opt) + '>' + Format(Row.Opt), 'Skip|1>Update');
    end;

    [Test]
    procedure ByValueParameterOrigin()
    begin
        Check(ByValueOrigin(1), 'Skip|1>Update');
    end;

    [Test]
    procedure TableProcedureOrigin()
    var
        Row: Record "OC Row";
    begin
        Check(Row.TableOrigin(), 'Skip|1>Update');
    end;

    // The callee is a TryFunction.
    [Test]
    procedure TryFunctionCallee()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        HopToTry(Choice);
        Check(Format(Choice), 'Update');
    end;

    // The last hop is a TryFunction.
    [Test]
    procedure TryFunctionHop()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        TryHop(Choice);
        Check(Format(Choice), 'Update');
    end;

    // A memberless `var` parameter of an interface method hands it on.
    [Test]
    procedure InterfaceMethodHop()
    var
        Choice: Option " ",Skip,Update;
        Impl: Codeunit "OC Hop Impl";
        H: Interface "OC Hop";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        H := Impl;
        Check(H.Hop(Choice, Row) + '>' + Format(Choice), 'Skip|1>Update');
    end;

    // The event contract of the batch cells: IntegrationEvent(true, false).
    [Test]
    procedure Event_WithSender()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseWithSender(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    [Test]
    procedure Event_NoSender()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseNoSender(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    [Test]
    procedure Event_Business()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseBusiness(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    // Main's paths, unchanged: a local with other members straight to the callee, and a chain of memberless parameters.
    [Test]
    procedure Main_DirectOtherMembers()
    var
        Choice: Option First,Second,Third;
        Row: Record "OC Row";
    begin
        Choice := Choice::Second;
        Check(Row.Confirm(Choice) + '>' + Format(Choice), 'Skip|1>Third');
    end;

    [Test]
    procedure Main_MemberlessChain()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Inner(Choice), 'Skip|1');
    end;

    local procedure Inner(var V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;
}
