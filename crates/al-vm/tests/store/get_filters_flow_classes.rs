//! GetFilters of a filter on a FlowField and a filter on a FlowFilter field (al-runner issue 8296). In the one shape corpus
//! PR 1430 measured (tests/al-language/record/filters/ByValueRecordView.al:569-616: the FlowFilter, field 6, set first, then
//! the FlowField, field 5) BC lists the FlowFilter first; it runs flagged (assumption
//! get-filters-flowfilter-before-flowfield). Every other shape with a FlowField filter before a FlowFilter filter in field
//! order refuses. Without that pair GetFilters keeps field order.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50340 Line
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; RowId; Integer) { }
        field(3; Day; Integer) { }
        field(4; Amt; Decimal) { }
    }
    keys { key(PK; Id) { } }
}

table 50341 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(5; Total; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum(Line.Amt where(RowId = field(Id), Day = field("Day Filter")));
        }
        field(6; "Day Filter"; Integer) { FieldClass = FlowFilter; }
        field(7; Later; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum(Line.Amt where(RowId = field(Id)));
        }
    }
    keys { key(PK; Id) { } }
}

codeunit 50342 C
{
    procedure Measured(): Text
    var
        Row: Record Row;
    begin
        Row.SetRange("Day Filter", 1);
        Row.SetRange(Total, 0);
        exit(Row.GetFilters());
    end;

    procedure FlowFieldSetFirst(): Text
    var
        Row: Record Row;
    begin
        Row.SetRange(Total, 0);
        Row.SetRange("Day Filter", 1);
        exit(Row.GetFilters());
    end;

    procedure WithANormalField(): Text
    var
        Row: Record Row;
    begin
        Row.SetRange(Name, 'a');
        Row.SetRange("Day Filter", 1);
        Row.SetRange(Total, 0);
        exit(Row.GetFilters());
    end;

    procedure InGroup2(): Text
    var
        Row: Record Row;
    begin
        Row.FilterGroup(2);
        Row.SetRange("Day Filter", 1);
        Row.SetRange(Total, 0);
        exit(Row.GetFilters());
    end;

    procedure ThroughRecordRef(): Text
    var
        Row: Record Row;
        R: RecordRef;
    begin
        Row.SetRange("Day Filter", 1);
        Row.SetRange(Total, 0);
        R.GetTable(Row);
        exit(R.GetFilters());
    end;

    procedure FlowFilterBeforeFlowField(): Text
    var
        Row: Record Row;
    begin
        Row.SetRange(Later, 0);
        Row.SetRange("Day Filter", 1);
        exit(Row.GetFilters());
    end;

    procedure NormalAndFlowField(): Text
    var
        Row: Record Row;
    begin
        Row.SetRange(Total, 0);
        Row.SetRange(Name, 'a');
        exit(Row.GetFilters());
    end;
}
"#;

fn run(routine: &str) -> (String, bool) {
    assumptions::clear_hits();
    let out = text(run_in(SRC, "C", routine));
    let flagged = assumptions::take_hits().contains(&Assumption::GetFiltersFlowfilterBeforeFlowfield);
    (out, flagged)
}

#[test]
fn the_measured_shape_lists_the_flowfilter_first_flagged() {
    assert_eq!(run("Measured"), ("Day Filter: 1, Total: 0".to_string(), true));
}

#[test]
fn other_shapes_with_a_flowfield_filter_before_a_flowfilter_filter_refuse() {
    for routine in ["FlowFieldSetFirst", "WithANormalField", "InGroup2", "ThroughRecordRef"] {
        match run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("order over field classes"), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn without_the_pair_getfilters_keeps_field_order() {
    assert_eq!(
        run("FlowFilterBeforeFlowField"),
        ("Day Filter: 1, Later: 0".to_string(), false)
    );
    assert_eq!(run("NormalAndFlowField"), ("Name: a, Total: 0".to_string(), false));
}
