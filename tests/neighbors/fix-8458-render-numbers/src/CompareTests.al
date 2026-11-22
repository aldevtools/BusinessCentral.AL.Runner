// A refused test untrusts the later tests of its codeunit that read what it wrote, so the refused comparisons stand
// in a codeunit of their own and the other shapes still run.
codeunit 50111 "N8458 Compare Tests"
{
    Subtype = Test;

    var
        Saves: Codeunit "N8458 Saves";

    // The two bodies of equal renders compared with each other: a reader of the number that must not answer.
    [Test]
    procedure TwoSavesOfOneReport_ComparedWithEachOther()
    var
        First: Text;
        Second: Text;
    begin
        Saves.Rows('ROW', 1);
        First := Saves.Body(Report::"N8458 Word");
        Second := Saves.Body(Report::"N8458 Word");
        Saves.Check(First = Second, 'equal bodies');
    end;

    // The bodies of two layouts compared with each other.
    [Test]
    procedure TwoLayouts_ComparedWithEachOther()
    var
        First: Text;
        Second: Text;
    begin
        Saves.Rows('ROW', 1);
        First := Saves.Body(Report::"N8458 Word");
        Second := Saves.Body(Report::"N8458 Other Word");
        Saves.Check(First <> Second, 'different bodies');
    end;
}
