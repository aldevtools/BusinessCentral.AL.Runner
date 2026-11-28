// A column that reads another data item's field through that item's dotted name, in a report with an admitted own-item dotted
// column (DName). The probe measures one such shape (part 1 at cf6884b9, :86-90 `CParentName`): a child item nested directly
// under the dotted item, a Text field with a Caption, the item and the field in their own case, a column that states exactly
// IncludeCaption = true; labels :224, values :250. In that shape the report's dotted columns stay on and the child column keeps
// main's rule for another item's field. Every other shape keeps the report's dotted columns on main's answer (not run, as on
// main). An expected text is the probe's or the undotted rule, not a pin.

codeunit 51029 "NB DQ2 Other Lib"
{
    procedure Seed()
    var
        Row: Record "NB DQ2 Row";
        Child: Record "NB DQ2 Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Row.Id := 1;
        Row.Name := 'Row one';
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Note := 'C1';
        Child.Insert();
    end;
}

// The measured shape (:90): the child item has a dotted name and a DataItemLink, as in the probe.
report 51011 "NB DQ2 Child Text"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51011 "NB DQ2 Child Text T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ChildReadsDottedParentText()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Child Text", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Child Text", '<Column '));
    end;
}

// The measured shape under a child item whose name holds no dot (the probe's child is "Dq. Child").
report 51012 "NB DQ2 Undotted Kid Text"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem(Kid; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51012 "NB DQ2 Undotted Kid Text T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_UndottedChildReadsDottedParentText()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Undotted Kid Text", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Undotted Kid Text", '<Column '));
    end;
}

// The measured shape under a child item without DataItemLink (the probe's child states one, :88).
report 51013 "NB DQ2 Unlinked Kid Text"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51013 "NB DQ2 Unlinked Kid Text T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_UnlinkedChildReadsDottedParentText()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Unlinked Kid Text", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Unlinked Kid Text", '<Column '));
    end;
}

// A grandchild reads the dotted item: not the measured parent-child relation.
report 51014 "NB DQ2 Grandchild Text"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem(Kid; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                dataitem(Grand; "NB DQ2 Child")
                {
                    DataItemLink = Id = field(Id);

                    column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
                }
            }
        }
    }
}

codeunit 51014 "NB DQ2 Grandchild Text T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_GrandchildReadsDottedText()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Grandchild Text", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Grandchild Text", '<Column '));
    end;
}

// A later top-level item reads the dotted item: not the measured parent-child relation.
report 51015 "NB DQ2 Sibling Text"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }


        }
        dataitem(Next; "NB DQ2 Child")
        {
            column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
        }
    }
}

codeunit 51015 "NB DQ2 Sibling Text T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_SiblingReadsDottedText()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Sibling Text", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Sibling Text", '<Column '));
    end;
}

// The child column states no property (the probe's states exactly IncludeCaption = true).
report 51017 "NB DQ2 Child Text No Prop"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".Name) { }
            }
        }
    }
}

codeunit 51017 "NB DQ2 Child Text No Prop T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ChildReadsDottedTextNoProperty()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Child Text No Prop", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Child Text No Prop", '<Column '));
    end;
}

// The qualifier in another case than the item's name.
report 51018 "NB DQ2 Child Other Case"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "dq. row".Name) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51018 "NB DQ2 Child Other Case T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ChildReadsDottedQualifierOtherCase()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Child Other Case", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Child Other Case", '<Column '));
    end;
}

// The field in another case than its name.
report 51019 "NB DQ2 Child Field Case"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".name) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51019 "NB DQ2 Child Field Case T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ChildReadsDottedFieldOtherCase()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Child Field Case", '<Label '));
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="CParentName">Row one</Column>]', Lib.Lines(Report::"NB DQ2 Child Field Case", '<Column '));
    end;
}

// The dotted parent reads its dotted child's field: not the measured direction.
report 51021 "NB DQ2 Parent Reads Kid"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }
            column(FromKid; "Dq. Kid".Note) { IncludeCaption = true; }

            dataitem("Dq. Kid"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CNote; "Dq. Kid".Note) { }
            }
        }
    }
}

codeunit 51021 "NB DQ2 Parent Reads Kid T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ParentReadsDottedChild()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="FromKidCaption">DQ Child Note cap</Label>]', Lib.Lines(Report::"NB DQ2 Parent Reads Kid", '<Label '));
    end;
}

// The parent's Text field states no Caption (the probe's Name states one, :31).
report 51016 "NB DQ2 Child Text No Cap"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(Admitted; "Dq. X".Id) { IncludeCaption = true; }

            dataitem(Kid; "NB DQ2 Extra")
            {
                DataItemLink = Id = field(Id);

                column(ParentText; "Dq. X".TextNoCap) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51016 "NB DQ2 Child Text No Cap T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_ChildReadsDottedTextNoCaption()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Seed();
        Lib.Check('[<Label name="AdmittedCaption">X Id cap</Label>][<Label name="ParentTextCaption">TextNoCap</Label>]', Lib.Lines(Report::"NB DQ2 Child Text No Cap", '<Label '));
    end;
}

// The qualifier names a global Record variable, not a data item.
report 51020 "NB DQ2 Global Dotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }
            column(G; "Gl. Row".Id) { }


        }
    }

    var
        "Gl. Row": Record "NB DQ2 Row";
}

codeunit 51020 "NB DQ2 Global Dotted T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OtherItem_GlobalDottedRecord()
    var
        Other: Codeunit "NB DQ2 Other Lib";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Other.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ2 Global Dotted", '<Label '));
    end;
}
