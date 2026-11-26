// The origin fact `option_carried_of` reads: a variable of a procedure's `var` section. Neighbor: the same variable in a
// trigger's `var` section (no cell of corpus PR 1569 hands a value on from a trigger). Refuses.
codeunit 62507 "OC Trigger Origin"
{
    var
        Seen: Text;

    trigger OnRun()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Seen := Hop(Choice) + '>' + Format(Choice);
    end;

    local procedure Hop(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.Confirm(V));
    end;

    procedure GetSeen(): Text
    begin
        exit(Seen);
    end;
}

codeunit 62508 "OC Trigger Tests"
{
    Subtype = Test;

    [Test]
    procedure TriggerOrigin_OnRun()
    var
        Origin: Codeunit "OC Trigger Origin";
    begin
        Origin.Run();
        if Origin.GetSeen() <> 'Skip|1>Update' then
            Error('expected Skip|1>Update, got %1', Origin.GetSeen());
    end;
}
