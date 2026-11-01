//! The `with` statement: AL source → compile → run as a test codeunit. Each test mirrors a corpus test (the
//! scenarios of events/lt-global-triggers-statements/P114WithStatement.al and
//! events/lt-with-statement-scopes/WithStatementAmbiguous.al and WithStatementNested.al), with `Check` in place
//! of the Probe Observer fixture.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62500 "WS A"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Qty; Integer) { } field(4; OnlyA; Text[30]) { } }
    keys { key(PK; Id) { } }
    procedure Describe(): Text begin exit('A:' + Name); end;
    procedure OnlyAProc(): Text begin exit('only-a-proc'); end;
    procedure Twice(Value: Integer): Integer begin exit(Value * 2); end;
}
table 62501 "WS B"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Qty; Decimal) { } field(4; OnlyB; Text[30]) { } }
    keys { key(PK; Id) { } }
    procedure Describe(): Text begin exit('B:' + Name); end;
}
table 62502 "WS Other"
{
    fields { field(1; Id; Integer) { } field(2; Name; Integer) { } field(3; Qty; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 62503 "WS Tests"
{
    Subtype = Test;
    var Name: Text[30]; Total: Integer;
    local procedure Describe(): Text begin exit('codeunit-describe'); end;
    local procedure Twice(Value: Integer): Integer begin exit(Value * 100); end;
    local procedure Count(): Integer begin exit(-1); end;
    local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;

    // P114WithStatement.al:P114_WithRecordFieldsAndMethods
    [Test]
    procedure RecordFieldsAndMethods() var Row: Record "WS A"; begin
        Row.DeleteAll();
        with Row do begin
            Init(); Id := 1; Name := 'one'; Qty := 5; Insert();
            Check('A:one', Describe());
            Check('1', Format(Count()));
            Qty := Qty + 1; Modify();
        end;
        Row.Get(1); Check('6', Format(Row.Qty));
    end;

    // P114WithStatement.al:P114_WithSingleStatementAndNested
    [Test]
    procedure SingleStatementAndNested() var Row: Record "WS A"; Other: Record "WS A"; begin
        Row.DeleteAll();
        with Row do
            Id := 7;
        Check('7', Format(Row.Id));
        with Row do begin
            Name := 'outer';
            with Other do begin Name := 'inner'; Qty := 3; end;
            Check('outer', Name);
        end;
        Check('inner', Other.Name); Check('3', Format(Other.Qty));
    end;

    // P114WithStatement.al:P114_WithLocalVariableShadow
    [Test]
    procedure LocalVariableShadow() var Row: Record "WS A"; Qty: Integer; begin
        Qty := 100;
        with Row do begin
            Name := 'shadow';
            Check('0', Format(Qty));
            Qty := 50;
        end;
        Check('100', Format(Qty)); Check('50', Format(Row.Qty));
    end;

    // WithStatementAmbiguous.al:P195_With_GlobalVariableSameNameAsField
    [Test]
    procedure GlobalSameNameAsField() var Row: Record "WS A"; begin
        Row.DeleteAll();
        Name := 'global-name'; Row.Name := 'field-name';
        with Row do begin Check('field-name', Name); Name := 'written-inside'; end;
        Check('written-inside', Row.Name); Check('global-name', Name);
    end;

    // WithStatementAmbiguous.al:P195_With_ProcedureSameNameAsTableProcedure
    [Test]
    procedure ProcedureSameNameAsTableProcedure() var Row: Record "WS A"; begin
        Check('codeunit-describe', Describe());
        with Row do begin Check('A:', Describe()); Check('6', Format(Twice(3))); end;
        Check('A:', Row.Describe()); Check('300', Format(Twice(3)));
    end;

    // WithStatementAmbiguous.al:P195_With_BuiltInMethodSameNameAsCodeunitProcedure
    [Test]
    procedure BuiltInMethodSameNameAsCodeunitProcedure() var Row: Record "WS A"; begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
        Check('-1', Format(Count()));
        with Row do
            Check('2', Format(Count()));
    end;

    // WithStatementAmbiguous.al:P195_NestedWith_DifferentFieldTypes_SameName
    [Test]
    procedure NestedDifferentFieldTypes() var Row: Record "WS A"; Other: Record "WS Other"; begin
        Row.Name := 'text-name'; Other.Name := 42; Row.Qty := 7; Other.Qty := 'text-qty';
        with Row do
            with Other do begin
                Check('42', Format(Name)); Check('text-qty', Format(Qty));
                Total := Name + 1;
            end;
        Check('43', Format(Total));
        with Other do
            with Row do begin Check('text-name', Format(Name)); Check('7', Format(Qty)); end;
    end;

    // WithStatementNested.al:P195_NestedWith_SameFieldName_InnerOrOuter
    [Test]
    procedure NestedInnerOrOuter() var A: Record "WS A"; B: Record "WS B"; begin
        A.Name := 'a-name'; B.Name := 'b-name';
        with A do with B do Check('b-name', Name);
        with B do with A do Check('a-name', Name);
        with A do with B do Check('B:b-name', Describe());
        with B do with A do Check('A:a-name', Describe());
        with A do with B do Check('0', Format(Id));
    end;

    // WithStatementNested.al:P195_NestedWith_AssignmentTarget
    [Test]
    procedure NestedAssignmentTarget() var A: Record "WS A"; B: Record "WS B"; begin
        with A do with B do begin Name := 'assigned'; Qty := 3; end;
        Check('', A.Name); Check('assigned', B.Name); Check('0', Format(A.Qty)); Check('3', Format(B.Qty));
        with B do with A do begin Name := 'reversed'; Qty := 4; end;
        Check('reversed', A.Name); Check('assigned', B.Name); Check('4', Format(A.Qty)); Check('3', Format(B.Qty));
    end;

    // WithStatementNested.al:P195_NestedWith_NameOnlyInOuter_OrOnlyInInner
    [Test]
    procedure NameOnlyInOuterOrInner() var A: Record "WS A"; B: Record "WS B"; begin
        A.OnlyA := 'oa'; B.OnlyB := 'ob';
        with A do with B do begin
            Check('oa', OnlyA); Check('ob', OnlyB); Check('only-a-proc', OnlyAProc());
        end;
        with B do with A do begin Check('ob', OnlyB); Check('oa', OnlyA); end;
    end;

    // WithStatementNested.al:P195_NestedWith_SameRecordTwice_AndSequential
    [Test]
    procedure SameRecordTwiceAndSequential() var A: Record "WS A"; Other: Record "WS A"; begin
        A.Name := 'first'; Other.Name := 'second';
        with A do with A do Name := Name + '-twice';
        Check('first-twice', A.Name);
        with A do begin
            with Other do Name := Name + '-inner';
            Name := Name + '-outer';
        end;
        Check('first-twice-outer', A.Name); Check('second-inner', Other.Name);
        with Other do with A do Name := 'x';
        Check('x', A.Name); Check('second-inner', Other.Name);
    end;

    // WithStatementNested.al:P195_With_TemporaryRecord_AndLoop
    [Test]
    procedure TemporaryRecordAndLoop() var Rows: Record "WS A" temporary; Sum: Integer; begin
        with Rows do begin
            Init(); Id := 1; Qty := 10; Insert();
            Init(); Id := 2; Qty := 20; Insert();
            Check('2', Format(Count())); Check('Yes', Format(IsTemporary()));
            if FindSet() then
                repeat
                    Sum += Qty; Qty := Qty + 1; Modify();
                until Next() = 0;
            Check('30', Format(Sum)); Check('Yes', Format(Get(2))); Check('21', Format(Qty));
            SetRange(Qty, 11); Check('1', Format(Count()));
        end;
        Check('Qty: 11', Rows.GetFilters());
    end;
}
"#;

fn run() -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(OBJECTS, "WS Tests")
}

#[test]
fn the_with_statement_binds_names_to_the_record_first() {
    let o = run();
    assert_eq!(o.len(), 12, "{o:?}");
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}
