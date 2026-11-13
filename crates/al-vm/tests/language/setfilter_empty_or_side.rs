//! SetFilter with an empty side of `|` on a Text, Code or Integer field raises Filter:CSide with the filter as written; an empty
//! substituted value is a valid term (corpus PR 1382, record/filters/SetFilterEmptyTerm.al:123-222, flagged through
//! `setfilter-empty-or-side`; record/crud/RcAFilters.al:376, :378, :648 pin `|10`, `10|` and `||`). Neighbors keep main's
//! outcome: `&` sides, shapes with more terms, a blank, an empty substituted value next to a `|` and other field types.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69991 "SE Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[50]) { } field(3; Cd; Code[20]) { } field(4; Num; Integer) { } field(5; Amt; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69993 "SE Flow"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; TxF; Text[30]) { FieldClass = FlowFilter; }
        field(3; IntF; Integer) { FieldClass = FlowFilter; }
        field(4; Rel; Integer) { TableRelation = "SE Row"; }
        field(5; Short; Text[3]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69994 "SE Temp"
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; Num; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69995 "SE Capt"
{
    Caption = 'Fancy Table';
    fields { field(1; Id; Integer) { } field(2; Num; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69992 "SE C"
{
    local procedure Seed()
    var Row: Record "SE Row";
    begin
        Row.DeleteAll();
        Add(1, 'A', 1); Add(2, 'B', 2); Add(3, '', 0); Add(4, 'AB', 5);
    end;

    local procedure Add(Id: Integer; Value: Text; Num: Integer)
    var Row: Record "SE Row";
    begin
        Row.Id := Id; Row.Name := Value; Row.Cd := Value; Row.Num := Num; Row.Insert();
    end;

    [TryFunction]
    local procedure TrySet(var Row: Record "SE Row"; Which: Integer)
    var RR: RecordRef; FR: FieldRef;
    begin
        case Which of
            1: Row.SetFilter(Name, '%1|', 'A');
            2: Row.SetFilter(Name, '|%1', 'A');
            3: Row.SetFilter(Name, '%1|%2', 'A', '');
            4: Row.SetFilter(Name, '%1|%2', '', 'A');
            5: Row.SetFilter(Name, '%1', '');
            6: Row.SetFilter(Name, 'A|');
            7: Row.SetFilter(Name, '|A');
            8: Row.SetFilter(Name, 'A||B');
            9: Row.SetFilter(Name, '|');
            10: Row.SetFilter(Cd, '%1|', 'A');
            11: Row.SetFilter(Cd, 'A||B');
            12: Row.SetFilter(Cd, '|A');
            13: Row.SetFilter(Num, '%1|', 1);
            14: Row.SetFilter(Num, '|%1', 1);
            15: Row.SetFilter(Num, '%1|%2', 1, 0);
            16: Row.SetFilter(Num, '1||2');
            17: Row.SetFilter(Name, '||');
            18: Row.SetFilter(Num, '|10');
            19: Row.SetFilter(Num, '10|');
            // Neighbors that keep main's outcome.
            20: Row.SetFilter(Name, 'A|B||C');
            21: Row.SetFilter(Name, ' A|');
            22: Row.SetFilter(Name, '%1|', '');
            23: Row.SetFilter(Amt, '1|');
            24: Row.SetFilter(Name, 'A|&B');
            25: Row.SetFilter(Name, '(A|)');
            26: Row.SetFilter(Name, 'A*|');
            27: begin RR.Open(Database::"SE Row"); FR := RR.Field(2); FR.SetFilter('A|'); end;
            28: begin Row.FilterGroup(2); Row.SetFilter(Name, 'A|'); end;
            // A literal that does not fit an Integer: main's evaluate error.
            29: Row.SetFilter(Num, '99999999999|');
            30: Row.SetFilter(Num, '2147483648|');
            31: Row.SetFilter(Num, '99999999999||1');
            // `|%1` on a Code field is not measured.
            32: Row.SetFilter(Cd, '|%1', 'A');
            // A filter the record already holds: not measured.
            33: begin Row.SetFilter(Num, '2'); Row.SetFilter(Num, '|10'); end;
            34: begin Row.SetRange(Id, 1); Row.SetFilter(Name, 'A|'); end;
        end;
    end;

    [TryFunction]
    local procedure TryOther(Which: Integer)
    var Fl: Record "SE Flow"; TmpRow: Record "SE Row" temporary; Tp: Record "SE Temp"; Ca: Record "SE Capt";
    begin
        case Which of
            41: Fl.SetFilter(TxF, '||');
            42: Fl.SetFilter(IntF, '|10');
            43: Fl.SetFilter(IntF, '10|');
            44: TmpRow.SetFilter(Num, '|10');
            45: Tp.SetFilter(Num, '|10');
            46: Ca.SetFilter(Num, '|10');
            47: Fl.SetFilter(Rel, '|10');
            48: Fl.SetFilter(Short, 'ABCDE|');
        end;
    end;

    procedure G(Which: Integer): Text
    begin
        ClearLastError();
        if not TryOther(Which) then
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        exit('no error');
    end;

    procedure F(Which: Integer): Text
    var Row: Record "SE Row"; Ids: Text;
    begin
        Seed();
        ClearLastError();
        if not TrySet(Row, Which) then
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        if Row.FindSet() then
            repeat Ids += Format(Row.Id) + ','; until Row.Next() = 0;
        exit('filter=' + Row.GetFilters() + ' ids=' + Ids);
    end;
}
"#;

fn call(which: i32) -> Result<Value, Signal> {
    take_hits();
    al_testkit::run_with(SRC, "SE C", "F", vec![Value::Integer(which)]).map(|(v, _)| v)
}

fn text(which: i32) -> String {
    al_testkit::text(call(which))
}

fn err(filter: &str, field: &str, side: &str) -> String {
    format!(
        "The filter \"{filter}\" is not valid for the {field} field on the SE Row table. The {side} side of '|' operators cannot be empty.|Filter:CSide"
    )
}

fn flagged() -> bool {
    take_hits().contains(&Assumption::SetfilterEmptyOrSide)
}

#[test]
fn an_empty_side_written_in_the_filter_raises_the_filter_error_flagged() {
    for (which, filter, field, side) in [
        (1, "%1|", "Name", "right"),
        (2, "|%1", "Name", "left"),
        (6, "A|", "Name", "right"),
        (7, "|A", "Name", "left"),
        (8, "A||B", "Name", "right"),
        (9, "|", "Name", "left"),
        (10, "%1|", "Cd", "right"),
        (11, "A||B", "Cd", "right"),
        (12, "|A", "Cd", "left"),
        (13, "%1|", "Num", "right"),
        (14, "|%1", "Num", "left"),
        (16, "1||2", "Num", "right"),
    ] {
        assert_eq!(text(which), err(filter, field, side), "case {which}");
        assert!(flagged(), "case {which} must mark its assumption");
    }
}

#[test]
fn the_pinned_shapes_raise_the_filter_error_unflagged() {
    for (which, filter, field, side) in [
        (17, "||", "Name", "left"),
        (18, "|10", "Num", "left"),
        (19, "10|", "Num", "right"),
    ] {
        assert_eq!(text(which), err(filter, field, side), "case {which}");
        assert!(!flagged(), "case {which} is pinned");
    }
}

#[test]
fn an_empty_substituted_value_is_a_valid_term() {
    assert_eq!(text(3), "filter=Name: A|'' ids=1,3,");
    assert_eq!(text(4), "filter=Name: ''|A ids=1,3,");
    assert_eq!(text(5), "filter=Name: '' ids=3,");
    assert_eq!(text(15), "filter=Num: 1|0 ids=1,3,");
}

#[test]
fn the_neighbor_shapes_keep_mains_outcome() {
    for which in [20, 21, 22, 23, 24, 25, 26, 27, 28] {
        let r = call(which);
        assert!(!flagged(), "case {which} must not reach the new path");
        assert!(matches!(r, Err(Signal::Unsupported(_))), "case {which}: {r:?}");
    }
}

/// A digit literal that does not fit an Integer keeps main's evaluate error (round-1 review of al-runner PR 7985, cells S029,
/// Z108, Z109).
#[test]
fn an_integer_literal_out_of_range_keeps_mains_evaluate_error() {
    for (which, filter, value) in [
        (29, "99999999999|", "99999999999"),
        (30, "2147483648|", "2147483648"),
        (31, "99999999999||1", "99999999999"),
    ] {
        assert_eq!(
            text(which),
            format!(
                "The filter \"{filter}\" is not valid for the Num field on the SE Row table. The value \"{value}\" can't be evaluated into type Integer.|Filter:CSide"
            ),
            "case {which}"
        );
        assert!(!flagged(), "case {which} must not reach the new path");
    }
}

/// The pins and the probe set the filter on a record that held no filter (RcAFilters.al:361-378, SetFilterEmptyTerm.al:98-104):
/// a record with a filter keeps main's refusal (round-1 review of al-runner PR 7985, cells S900, S901).
#[test]
fn a_record_that_holds_a_filter_keeps_mains_refusal() {
    for which in [33, 34] {
        let r = call(which);
        assert!(!flagged(), "case {which}");
        assert!(matches!(r, Err(Signal::Unsupported(_))), "case {which}: {r:?}");
    }
}

/// `|%1` on a Code field was not measured (SetFilterEmptyTerm.al:161 has only `%1|`): main's refusal.
#[test]
fn a_leading_pipe_placeholder_on_code_keeps_mains_refusal() {
    let r = call(32);
    assert!(!flagged());
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

/// The pinned and measured shapes are a stored field without TableRelation of a non-temporary record of a table that states no
/// TableType and no Caption, and a literal that fits the field. FlowFilter fields, a temporary record, TableType = Temporary,
/// a table Caption, a TableRelation and a literal longer than the field keep main's refusal (round-1 review of al-runner PR
/// 7985, cells S068, S069, S070, S098, S100, S102, S073, S065).
#[test]
fn shapes_outside_the_measured_record_keep_mains_refusal() {
    for which in [41, 42, 43, 44, 45, 46, 47, 48] {
        take_hits();
        let r = al_testkit::run_with(SRC, "SE C", "G", vec![Value::Integer(which)]).map(|(v, _)| v);
        assert!(!flagged(), "case {which} must not reach the new path");
        assert!(matches!(r, Err(Signal::Unsupported(_))), "case {which}: {r:?}");
    }
}
