// Unordered apps tied by data only, no meet (ADR-079 decisions 2, 3 and 6).
codeunit 51390 "FP data Tests"
{
    Subtype = Test;

    [Test]
    procedure SourceRowExists()
    var
        R: Record "FP Data Source";
    begin
        R.Get('Y');
    end;

    [Test]
    procedure ReaderWhoseRowDependsOnTheOrder()
    var
        R: Record "FP Data Differs";
    begin
        R.Get('X');
    end;

    [Test]
    procedure ReaderWhoseRowDoesNotDependOnTheOrder()
    var
        R: Record "FP Data Same";
    begin
        R.Get('X');
    end;

    [Test]
    procedure PairWritingOneRowWithDifferentValues()
    var
        R: Record "FP Data Tangle";
    begin
        R.Get('ROW');
    end;

    [Test]
    procedure PairWritingOneRowWithTheSameValue()
    var
        R: Record "FP Data Equal";
    begin
        R.Get('ROW');
        if R.Value <> 'SAME' then Error('value %1', R.Value);
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
