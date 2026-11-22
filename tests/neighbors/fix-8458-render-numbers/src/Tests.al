codeunit 50110 "N8458 Tests"
{
    Subtype = Test;

    var
        Saves: Codeunit "N8458 Saves";

    // Two saves of one report with the same rows: the two renders have equal entries (al-runner issue 8458).
    [Test]
    procedure TwoSavesOfOneReport_BothKnownNotEmpty()
    var
        First: Text;
        Second: Text;
    begin
        Saves.Rows('ROW', 1);
        First := Saves.Body(Report::"N8458 Word");
        Second := Saves.Body(Report::"N8458 Word");
        Saves.Check(First <> '', 'first body is not empty');
        Saves.Check(Second <> '', 'second body is not empty');
    end;

    // The two bodies of equal renders joined into one text.
    [Test]
    procedure TwoSavesOfOneReport_Joined()
    var
        Joined: Text;
    begin
        Saves.Rows('ROW', 1);
        Joined := Saves.Body(Report::"N8458 Word") + Saves.Body(Report::"N8458 Word");
        Saves.Check(Joined <> '', 'joined body is not empty');
    end;

    // Two reports whose default layouts differ: the entries differ in report and layout.
    [Test]
    procedure TwoLayouts_BothKnownNotEmpty()
    var
        First: Text;
        Second: Text;
    begin
        Saves.Rows('ROW', 1);
        First := Saves.Body(Report::"N8458 Word");
        Second := Saves.Body(Report::"N8458 Other Word");
        Saves.Check(First <> '', 'first layout body is not empty');
        Saves.Check(Second <> '', 'second layout body is not empty');
    end;

    // A body kept by another codeunit, then an equal render saved again here.
    [Test]
    procedure KeptBodyAcrossCodeunits_ThenEqualSave()
    var
        Keep: Codeunit "N8458 Keep";
        Fresh: Text;
    begin
        Codeunit.Run(Codeunit::"N8458 Keeper");
        Saves.Rows('ROW', 1);
        Fresh := Saves.Body(Report::"N8458 Word");
        Saves.Check(Keep.Get() <> '', 'kept body is not empty');
        Saves.Check(Fresh <> '', 'fresh body is not empty');
    end;

    // The body kept by the test before, read by the next test of the codeunit after more equal saves.
    [Test]
    procedure KeptBodyAcrossTests_ReadAfterEqualSaves()
    var
        Keep: Codeunit "N8458 Keep";
    begin
        Saves.Rows('ROW', 1);
        Saves.Body(Report::"N8458 Word");
        Saves.Body(Report::"N8458 Word");
        Saves.Check(Keep.Get() <> '', 'kept body is not empty');
    end;

    // A save with no dataset row (an error, on main too), then a save with one.
    [Test]
    procedure SaveWithNoRows_ThenSaveWithARow()
    var
        WithRow: Text;
    begin
        Saves.Rows('ROW', 0);
        asserterror Saves.Body(Report::"N8458 Word");
        Saves.Rows('ROW', 1);
        WithRow := Saves.Body(Report::"N8458 Word");
        Saves.Check(WithRow <> '', 'body with a row is not empty');
    end;

    // A save with no dataset row, read by itself (it raises the empty-report error on main too).
    [Test]
    procedure SaveWithNoRows_Read()
    var
        Empty: Text;
    begin
        Saves.Rows('ROW', 0);
        Empty := Saves.Body(Report::"N8458 Word");
        Saves.Check(Empty <> '', 'body with no rows is not empty');
    end;

    // Two saves of rows that hold an rgba color: equal entries that carry the rgba column.
    [Test]
    procedure TwoRgbaRows_BothKnownNotEmpty()
    var
        First: Text;
        Second: Text;
    begin
        Saves.Rows('rgba(0, 0, 0, 0.5)', 2);
        First := Saves.Body(Report::"N8458 Word");
        Second := Saves.Body(Report::"N8458 Word");
        Saves.Check(First <> '', 'first rgba body is not empty');
        Saves.Check(Second <> '', 'second rgba body is not empty');
    end;

    // An rgba save next to a plain save of the same report: entries that differ only in the rgba column.
    [Test]
    procedure TwoRgbaRows_NextToAPlainSave()
    var
        Rgba: Text;
        Plain: Text;
    begin
        Saves.Rows('rgba(0, 0, 0, 0.5)', 2);
        Rgba := Saves.Body(Report::"N8458 Word");
        Saves.Rows('ROW', 2);
        Plain := Saves.Body(Report::"N8458 Word");
        Saves.Check(Rgba <> '', 'rgba body is not empty');
        Saves.Check(Plain <> '', 'plain body is not empty');
    end;
}
