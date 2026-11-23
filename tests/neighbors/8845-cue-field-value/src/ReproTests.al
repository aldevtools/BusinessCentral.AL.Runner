codeunit 50145 "Repro Cue Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // The Card shows the row's Name twice: in a group (Name) and in a cuegroup (CueName), both bound to Rec.Name.
    [Test]
    procedure CueTextField_ShowsZero()
    var
        Row: Record "Repro Cue Row";
        Card: TestPage "Repro Cue Card";
        Plain: Text;
        Cue: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();

        Card.OpenView();
        Plain := Card.Name.Value();
        Cue := Card.CueName.Value();
        Card.Close();
        if Plain <> 'Alpha' then
            Error('the Name control shows <%1>', Plain);
        if Cue <> '0' then
            Error('the CueName control in the cuegroup shows <%1>', Cue);
    end;
}