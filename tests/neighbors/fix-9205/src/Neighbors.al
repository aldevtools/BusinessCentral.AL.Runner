// Neighbors of al-runner issue 9205: a Boolean into a Code or Text field of the lengths corpus PR 1575 measures
// (SrcFilterBoolIntoText.al:389-629), the literal routes (:645, :656), lengths it does not measure (Code[4], Text[7]) and the
// literal into Record.SetRange (not measured). Each test raises an error that carries the filter.
table 51201 "BTL Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cod3; Code[3]) { }
        field(3; Cod10; Code[10]) { }
        field(4; Cod50; Code[50]) { }
        field(5; Cod250; Code[250]) { }
        field(6; Txt2; Text[2]) { }
        field(7; Txt3; Text[3]) { }
        field(8; Txt100; Text[100]) { }
        field(9; Txt2048; Text[2048]) { }
        field(10; Cod20; Code[20]) { }
        field(11; Cod4; Code[4]) { }
        field(12; Txt7; Text[7]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 51201 "BTL Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Range(FieldNo: Integer; X: Boolean)
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(FieldNo);
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    local procedure Filter(FieldNo: Integer; X: Boolean)
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(FieldNo);
        F.SetFilter('%1', X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseRangeCod3()
    begin
        Range(2, false);
    end;

    [Test]
    procedure FalseFilterCod3()
    begin
        Filter(2, false);
    end;

    [Test]
    procedure TrueRangeCod3()
    begin
        Range(2, true);
    end;

    [Test]
    procedure TrueFilterCod3()
    begin
        Filter(2, true);
    end;

    [Test]
    procedure FalseRangeCod10()
    begin
        Range(3, false);
    end;

    [Test]
    procedure FalseFilterCod10()
    begin
        Filter(3, false);
    end;

    [Test]
    procedure TrueRangeCod10()
    begin
        Range(3, true);
    end;

    [Test]
    procedure TrueFilterCod10()
    begin
        Filter(3, true);
    end;

    [Test]
    procedure FalseRangeCod50()
    begin
        Range(4, false);
    end;

    [Test]
    procedure FalseFilterCod50()
    begin
        Filter(4, false);
    end;

    [Test]
    procedure TrueRangeCod50()
    begin
        Range(4, true);
    end;

    [Test]
    procedure TrueFilterCod50()
    begin
        Filter(4, true);
    end;

    [Test]
    procedure FalseRangeCod250()
    begin
        Range(5, false);
    end;

    [Test]
    procedure FalseFilterCod250()
    begin
        Filter(5, false);
    end;

    [Test]
    procedure TrueRangeCod250()
    begin
        Range(5, true);
    end;

    [Test]
    procedure TrueFilterCod250()
    begin
        Filter(5, true);
    end;

    [Test]
    procedure FalseRangeTxt2()
    begin
        Range(6, false);
    end;

    [Test]
    procedure FalseFilterTxt2()
    begin
        Filter(6, false);
    end;

    [Test]
    procedure TrueRangeTxt2()
    begin
        Range(6, true);
    end;

    [Test]
    procedure TrueFilterTxt2()
    begin
        Filter(6, true);
    end;

    [Test]
    procedure FalseRangeTxt3()
    begin
        Range(7, false);
    end;

    [Test]
    procedure FalseFilterTxt3()
    begin
        Filter(7, false);
    end;

    [Test]
    procedure TrueRangeTxt3()
    begin
        Range(7, true);
    end;

    [Test]
    procedure TrueFilterTxt3()
    begin
        Filter(7, true);
    end;

    [Test]
    procedure FalseRangeTxt100()
    begin
        Range(8, false);
    end;

    [Test]
    procedure FalseFilterTxt100()
    begin
        Filter(8, false);
    end;

    [Test]
    procedure TrueRangeTxt100()
    begin
        Range(8, true);
    end;

    [Test]
    procedure TrueFilterTxt100()
    begin
        Filter(8, true);
    end;

    [Test]
    procedure FalseRangeTxt2048()
    begin
        Range(9, false);
    end;

    [Test]
    procedure FalseFilterTxt2048()
    begin
        Filter(9, false);
    end;

    [Test]
    procedure TrueRangeTxt2048()
    begin
        Range(9, true);
    end;

    [Test]
    procedure TrueFilterTxt2048()
    begin
        Filter(9, true);
    end;

    [Test]
    procedure FalseRangeCod4()
    begin
        Range(11, false);
    end;

    [Test]
    procedure FalseFilterCod4()
    begin
        Filter(11, false);
    end;

    [Test]
    procedure TrueRangeCod4()
    begin
        Range(11, true);
    end;

    [Test]
    procedure TrueFilterCod4()
    begin
        Filter(11, true);
    end;

    [Test]
    procedure FalseRangeTxt7()
    begin
        Range(12, false);
    end;

    [Test]
    procedure FalseFilterTxt7()
    begin
        Filter(12, false);
    end;

    [Test]
    procedure TrueRangeTxt7()
    begin
        Range(12, true);
    end;

    [Test]
    procedure TrueFilterTxt7()
    begin
        Filter(12, true);
    end;

    [Test]
    procedure FalseLiteralRangeCod20()
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(10);
        F.SetRange(false);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseLiteralFilterCod20()
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(10);
        F.SetFilter('%1', false);
        Error('F:[%1]', F.GetFilter());
    end;

    // Not measured: refused.
    [Test]
    procedure FalseLiteralRecordSetRangeCod20()
    var
        Rec: Record "BTL Row";
    begin
        Rec.SetRange(Cod20, false);
        Error('F:[%1]', Rec.GetFilter(Cod20));
    end;

    // Not measured: a temporary table.
    [Test]
    procedure FalseTemporaryCod3()
    var
        Rec: Record "BTL Row" temporary;
        R: RecordRef;
        F: FieldRef;
    begin
        R.GetTable(Rec);
        F := R.Field(2);
        F.SetRange(false);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure TrueLiteralRangeCod20()
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(10);
        F.SetRange(true);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure TrueLiteralFilterCod20()
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTL Row");
        F := R.Field(10);
        F.SetFilter('%1', true);
        Error('F:[%1]', F.GetFilter());
    end;

    // Not measured: refused.
    [Test]
    procedure TrueLiteralRecordSetRangeCod20()
    var
        Rec: Record "BTL Row";
    begin
        Rec.SetRange(Cod20, true);
        Error('F:[%1]', Rec.GetFilter(Cod20));
    end;

    // Not measured: a temporary table.
    [Test]
    procedure TrueTemporaryCod3()
    var
        Rec: Record "BTL Row" temporary;
        R: RecordRef;
        F: FieldRef;
    begin
        R.GetTable(Rec);
        F := R.Field(2);
        F.SetRange(true);
        Error('F:[%1]', F.GetFilter());
    end;

}
