// Neighbors of "@ alone" (al-runner issue 8637) past BQ Tests 9b and 9c and the routes of AtSignAloneReads.al (Routes.al):
// <>@ alone on a Text and on a Code field of a record in the database (BC: the K row), and the places of a filter no cell measures
// with @ alone on a Code field, which the runner refuses as main does: a report's DataItemTableView and a query column's
// ColumnFilter (BC: the empty row).
report 97501 "BQ Kind View Rows"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Source; "BQ Kind Source")
        {
            DataItemTableView = where("Code Kind" = filter('@'));

            trigger OnPreDataItem()
            begin
                Source.SetRange("Group", TheGroup);
            end;

            trigger OnAfterGetRecord()
            begin
                Seen += Format(Source.Id) + ',';
            end;
        }
    }

    var
        TheGroup: Code[20];
        Seen: Text;

    procedure SetGroup(G: Code[20])
    begin
        TheGroup := G;
    end;

    procedure SeenIds(): Text
    begin
        exit(Seen);
    end;
}

query 97562 "BQ Kind Column At"
{
    QueryType = Normal;

    elements
    {
        dataitem(Source; "BQ Kind Source")
        {
            column(KindGroup; "Group") { }
            column(Id; Id) { }
            column(CodeKind; "Code Kind")
            {
                ColumnFilter = CodeKind = filter('@');
            }
        }
    }
}

codeunit 97534 "BQ At Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // Rows 1 (K) and 2 (empty) of a new group, in the Code and the Text kind field.
    local procedure NewGroup(): Code[20]
    var
        Source: Record "BQ Kind Source";
    begin
        Source."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Source.Id := 1;
        Source."Code Kind" := 'K';
        Source."Text Kind" := 'K';
        Source.Insert();
        Source.Id := 2;
        Source."Code Kind" := '';
        Source."Text Kind" := '';
        Source.Insert();
        exit(Source."Group");
    end;

    [Test]
    procedure AtSignAloneNegated_TextField_SourceRows()
    var
        Source: Record "BQ Kind Source";
    begin
        Source.SetRange("Group", NewGroup());
        Source.SetFilter("Text Kind", '<>@');
        Assert.AreEqual(1, Source.Count(), '<>@ on Text');
    end;

    [Test]
    procedure AtSignAlone_ReportDataItemTableView_CodeField()
    var
        Rows: Report "BQ Kind View Rows";
        G: Code[20];
    begin
        G := NewGroup();
        Commit();
        Rows.SetGroup(G);
        Rows.UseRequestPage(false);
        Rows.RunModal();
        Assert.AreEqual('2,', Rows.SeenIds(), 'DataItemTableView @ on Code');
    end;

    [Test]
    procedure AtSignAlone_QueryColumnFilter_CodeColumn()
    var
        ColumnAt: Query "BQ Kind Column At";
        Seen: Text;
    begin
        ColumnAt.SetRange(KindGroup, NewGroup());
        ColumnAt.Open();
        while ColumnAt.Read() do
            Seen += Format(ColumnAt.Id) + ',';
        ColumnAt.Close();
        Assert.AreEqual('2,', Seen, 'ColumnFilter @ on Code');
    end;

    [Test]
    procedure AtSignAloneNegated_CodeField_SourceRows()
    var
        Source: Record "BQ Kind Source";
    begin
        Source.SetRange("Group", NewGroup());
        Source.SetFilter("Code Kind", '<>@');
        Assert.AreEqual(1, Source.Count(), '<>@ on Code');
    end;
}

// Neighbors of <>@ alone on a Code field (AtSignAloneReads.al:698, :730 measure Record.SetFilter): the places a filter comes from
// and the readers no cell measures it with, which the runner refuses as main does.
codeunit 97537 "BQ Not At Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    var
        Source: Record "BQ Kind Source";
    begin
        Source."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Source.Id := 1;
        Source."Code Kind" := 'K';
        Source.Insert();
        Source.Id := 2;
        Source."Code Kind" := '';
        Source.Insert();
        exit(Source."Group");
    end;

    [Test]
    procedure NotAtSignAlone_CodeField_FieldRefSetFilter()
    var
        Ref: RecordRef;
        GroupRef: FieldRef;
        KindRef: FieldRef;
    begin
        Ref.Open(Database::"BQ Kind Source");
        GroupRef := Ref.Field(1);
        KindRef := Ref.Field(3);
        GroupRef.SetRange(NewGroup());
        KindRef.SetFilter('<>@');
        Assert.AreEqual(1, Ref.Count(), 'FieldRef <>@ on Code');
    end;

    [Test]
    procedure NotAtSignAlone_CodeField_SetView()
    var
        Source: Record "BQ Kind Source";
    begin
        Source.SetView(StrSubstNo('WHERE(Group=CONST(%1),Code Kind=FILTER(<>@))', NewGroup()));
        Assert.AreEqual(1, Source.Count(), 'SetView <>@ on Code');
    end;

    [Test]
    procedure NotAtSignAlone_CodeField_GetFilter()
    var
        Source: Record "BQ Kind Source";
    begin
        Source.SetFilter("Code Kind", '<>@');
        Assert.AreEqual('<>@', Source.GetFilter("Code Kind"), 'GetFilter <>@ on Code');
    end;
}
