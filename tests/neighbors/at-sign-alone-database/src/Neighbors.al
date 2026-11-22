// Neighbors of "@ alone" (al-runner issue 8637) past BQ Tests 9b and 9c: <>@ alone on a Code field of a record in the database
// (BC on a local 29.1 container keeps the rows whose value is not empty; the runner refuses it, as main does).
codeunit 97534 "BQ At Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    [Test]
    procedure AtSignAloneNegated_CodeField_SourceRows()
    var
        Source: Record "BQ Kind Source";
        G: Code[20];
    begin
        G := NewGroup();
        Source."Group" := G;
        Source.Id := 1;
        Source."Code Kind" := 'K';
        Source.Insert();
        Source.Id := 2;
        Source."Code Kind" := '';
        Source.Insert();
        Source.SetRange("Group", G);
        Source.SetFilter("Code Kind", '<>@');
        Assert.AreEqual(1, Source.Count(), '<>@ on Code');
    end;
}
