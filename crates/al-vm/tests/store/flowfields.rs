//! FlowFields, FlowFilters, CalcFields/CalcSums/SetAutoCalcFields and
//! AutoIncrement, end to end: AL source → compile → run on the in-memory store.
//! Each test cites the corpus file that pins the behaviour.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

/// The corpus fixtures ALT Parent / ALT Child (corpus/tests/al-language/_fixtures/tables/ALTParentChild.al).
const PARENT_CHILD: &str = r#"
table 60004 "ALT Parent"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Code"; Code[20]) { }
        field(3; "Child Count"; Integer) { FieldClass = FlowField; CalcFormula = count("ALT Child" where("Parent Entry No." = field("Entry No."))); }
        field(4; "Child Amount"; Decimal) { FieldClass = FlowField; CalcFormula = sum("ALT Child".Amount where("Parent Entry No." = field("Entry No."))); }
        field(5; "First Child Code"; Code[20]) { FieldClass = FlowField; CalcFormula = lookup("ALT Child".Code where("Parent Entry No." = field("Entry No."))); }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60005 "ALT Child"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Parent Entry No."; Integer) { }
        field(3; "Code"; Code[20]) { }
        field(4; "Amount"; Decimal) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
"#;

fn run_with(tables: &str, body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{tables}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("{routine} is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn run(body: &str) -> Result<Value, Signal> {
    run_with(PARENT_CHILD, body, "F")
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Seeds parent 1 with two children (30 and 50) and parent 2 with one (7).
const SEED: &str = "
    procedure Seed()
    var P: Record \"ALT Parent\"; C: Record \"ALT Child\";
    begin
        P.\"Entry No.\" := 1; P.Insert();
        P.\"Entry No.\" := 2; P.Insert();
        C.\"Entry No.\" := 10; C.\"Parent Entry No.\" := 1; C.Amount := 30; C.Code := 'FIRST'; C.Insert();
        C.\"Entry No.\" := 11; C.\"Parent Entry No.\" := 1; C.Amount := 50; C.Code := 'SECOND'; C.Insert();
        C.\"Entry No.\" := 12; C.\"Parent Entry No.\" := 2; C.Amount := 7; C.Code := 'ONLY'; C.Insert();
    end;
";

// corpus record/TestRecordFlowField.al:15,36,76 (count, sum, several fields in one call).
#[test]
fn calcfields_count_and_sum_over_the_children() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\";
        begin
            Seed();
            P.Get(1);
            P.CalcFields(\"Child Count\", \"Child Amount\");
            exit(Format(P.\"Child Count\") + '/' + Format(P.\"Child Amount\"));
        end;"
    ));
    assert_eq!(text(r), "2/80");
}

// corpus record/TestRecordFlowField.al:57 (lookup of the single child's code).
#[test]
fn lookup_returns_the_code_of_the_one_matching_row() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\";
        begin
            Seed();
            P.Get(2);
            P.CalcFields(\"First Child Code\", \"Child Count\");
            exit(P.\"First Child Code\" + Format(P.\"Child Count\"));
        end;"
    ));
    assert_eq!(text(r), "ONLY1");
}

// lookup() over several matching rows reads the first in primary key order (corpus
// record/TestCalcFieldsTruncateUniqueContracts.al:CalcFields_Lookup_OverSeveralRows_ReadsTheFirstInPrimaryKeyOrder).
#[test]
fn lookup_over_rows_that_disagree_reads_the_first_in_primary_key_order() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\";
        begin
            Seed();
            P.Get(1);
            P.CalcFields(\"First Child Code\");
            exit(P.\"First Child Code\");
        end;"
    ));
    assert_eq!(text(r), "FIRST");
}

// corpus record/TestFlowFieldContracts.al:79 (CalcFields returns true) and :98 (no children give 0).
#[test]
fn calcfields_returns_true_and_an_empty_source_gives_zero() {
    let r = run("procedure F(): Text
        var P: Record \"ALT Parent\"; Ok: Boolean;
        begin
            P.\"Entry No.\" := 1; P.Insert();
            P.Get(1);
            Ok := P.CalcFields(\"Child Count\", \"Child Amount\");
            exit(Format(Ok) + Format(P.\"Child Count\") + Format(P.\"Child Amount\"));
        end;");
    assert_eq!(text(r), "Yes00");
}

// corpus record/TestFlowFieldContracts.al:36 (a calculated value is a snapshot, stale until
// calculated again).
#[test]
fn a_calculated_value_is_stale_until_calculated_again() {
    let r = run("procedure F(): Text
        var P: Record \"ALT Parent\"; C: Record \"ALT Child\"; Seen: Text;
        begin
            P.\"Entry No.\" := 1; P.Insert();
            C.\"Entry No.\" := 1; C.\"Parent Entry No.\" := 1; C.Insert();
            P.Get(1);
            P.CalcFields(\"Child Count\");
            Seen := Format(P.\"Child Count\");
            C.\"Entry No.\" := 2; C.Insert();
            Seen += Format(P.\"Child Count\");
            P.CalcFields(\"Child Count\");
            exit(Seen + Format(P.\"Child Count\"));
        end;");
    assert_eq!(text(r), "112");
}

// corpus record/TestRecordNavContracts.al:200 (a record that was never inserted calculates to 0).
#[test]
fn calcfields_on_a_record_that_was_never_inserted_gives_zero() {
    let r = run("procedure F(): Text
        var P: Record \"ALT Parent\";
        begin
            P.\"Entry No.\" := 9999;
            P.CalcFields(\"Child Count\");
            exit(Format(P.\"Child Count\"));
        end;");
    assert_eq!(text(r), "0");
}

// corpus record/TestFlowFieldContracts.al:119,133 (FindFirst and Next calculate per record).
#[test]
fn set_auto_calc_fields_calculates_on_find_first_and_on_next() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\"; Seen: Text;
        begin
            Seed();
            P.SetAutoCalcFields(\"Child Count\", \"Child Amount\");
            P.FindFirst();
            Seen := Format(P.\"Child Count\") + ':' + Format(P.\"Child Amount\");
            P.Next();
            exit(Seen + '/' + Format(P.\"Child Count\") + ':' + Format(P.\"Child Amount\"));
        end;"
    ));
    assert_eq!(text(r), "2:80/1:7");
}

// corpus record/TestSetAutoCalcFieldsOnGet.Codeunit.al:25,38,48,87 (Get, Find('=') and a
// plain Get that does not calculate).
#[test]
fn set_auto_calc_fields_applies_to_get_and_find_equal_but_a_plain_get_does_not_calculate() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\"; Q: Record \"ALT Parent\"; C: Record \"ALT Child\"; Seen: Text;
        begin
            Seed();
            Q.Get(1);
            Seen := Format(Q.\"Child Count\");
            P.SetAutoCalcFields(\"Child Count\");
            P.Get(2);
            Seen += Format(P.\"Child Count\");
            C.\"Entry No.\" := 20; C.\"Parent Entry No.\" := 2; C.Insert();
            P.Get(2);
            Seen += Format(P.\"Child Count\");
            P.Init();
            P.\"Entry No.\" := 1;
            Seen += Format(P.Find('='));
            exit(Seen + Format(P.\"Child Count\"));
        end;"
    ));
    assert_eq!(text(r), "012Yes2");
}

// corpus record/TestFlowFieldContracts.al:156 (a temporary record: the source table has no
// rows, so both readings agree and the value stays 0).
#[test]
fn set_auto_calc_fields_on_a_temporary_record_over_an_empty_source_stays_zero() {
    let r = run("procedure F(): Text
        var P: Record \"ALT Parent\" temporary;
        begin
            P.\"Entry No.\" := 1; P.Insert();
            P.SetAutoCalcFields(\"Child Count\");
            P.FindFirst();
            exit(Format(P.\"Child Count\"));
        end;");
    assert_eq!(text(r), "0");
}

// A FlowField on a temporary record reads the persistent source table (corpus
// record/TestCalcFieldsTruncateUniqueContracts.al:CalcFields_OnATemporaryRecord_ReadsThePersistentSourceTable).
#[test]
fn a_flow_field_on_a_temporary_record_reads_the_persistent_source() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\" temporary;
        begin
            Seed();
            P.\"Entry No.\" := 1; P.Insert();
            P.CalcFields(\"Child Count\");
            exit(Format(P.\"Child Count\"));
        end;"
    ));
    assert_eq!(text(r), "2");
}

// corpus record/TestRecordFlowFieldFilterVisitor.al:30,49 (a filter on a FlowField selects by the
// calculated value).
#[test]
fn a_filter_on_a_flow_field_selects_by_the_calculated_value() {
    let r = run("procedure F(): Text
        var P: Record \"ALT Parent\"; C: Record \"ALT Child\"; Seen: Text;
        begin
            P.\"Entry No.\" := 1; P.Insert();
            P.\"Entry No.\" := 2; P.Insert();
            P.Reset();
            P.SetRange(\"Child Count\", 0);
            Seen := Format(P.Count());
            C.\"Entry No.\" := 101; C.\"Parent Entry No.\" := 1; C.Insert();
            C.\"Entry No.\" := 102; C.\"Parent Entry No.\" := 1; C.Insert();
            Seen += Format(P.Count()) + Format(P.IsEmpty());
            P.FindFirst();
            exit(Seen + Format(P.\"Entry No.\"));
        end;");
    assert_eq!(text(r), "21No2");
}

/// A compact version of the corpus CFM/CFS/CFF fixtures: one source table, one table with a
/// FlowField per formula shape.
const LINES: &str = r#"
table 60450 "FF Line"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Doc No."; Code[20]) { }
        field(3; Amount; Decimal) { }
        field(4; Qty; Integer) { }
        field(5; "Posting Date"; Date) { }
        field(6; Open; Boolean) { }
        field(7; Status; Option) { OptionMembers = Open,Released,Closed; }
        field(8; Elapsed; Duration) { }
        field(9; Account; Code[20]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60451 "FF Head"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(3; Totaling; Text[250]) { }
        field(10; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."))); }
        field(11; Lines; Integer) { FieldClass = FlowField; CalcFormula = count("FF Line" where("Doc No." = field("No."))); }
        field(12; MinAmt; Decimal) { FieldClass = FlowField; CalcFormula = min("FF Line".Amount where("Doc No." = field("No."))); }
        field(13; MaxAmt; Decimal) { FieldClass = FlowField; CalcFormula = max("FF Line".Amount where("Doc No." = field("No."))); }
        field(14; AvgAmt; Decimal) { FieldClass = FlowField; CalcFormula = average("FF Line".Amount where("Doc No." = field("No."))); }
        field(15; AvgQty; Integer) { FieldClass = FlowField; CalcFormula = average("FF Line".Qty where("Doc No." = field("No."))); }
        field(16; BadAvg; Decimal) { FieldClass = FlowField; CalcFormula = average("FF Line".Qty where("Doc No." = field("No."))); }
        field(17; FirstDate; Date) { FieldClass = FlowField; CalcFormula = min("FF Line"."Posting Date" where("Doc No." = field("No."))); }
        field(18; HasLines; Boolean) { FieldClass = FlowField; CalcFormula = exist("FF Line" where("Doc No." = field("No."))); }
        field(19; NoLines; Boolean) { FieldClass = FlowField; CalcFormula = - exist("FF Line" where("Doc No." = field("No."))); }
        field(20; NegTotal; Decimal) { FieldClass = FlowField; CalcFormula = -sum("FF Line".Amount where("Doc No." = field("No."))); }
        field(21; OpenTotal; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), Open = const(true))); }
        field(22; ActiveTotal; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), Status = filter(Open | Released))); }
        field(23; Period; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), "Posting Date" = field("Date Filter"))); }
        field(24; Balance; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), "Posting Date" = field(upperlimit("Date Filter")))); }
        field(25; Totaled; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), Account = field(filter(Totaling)))); }
        field(26; Hours; Duration) { FieldClass = FlowField; CalcFormula = sum("FF Line".Elapsed where("Doc No." = field("No."))); }
        field(27; Unformulated; Decimal) { FieldClass = FlowField; }
        field(28; SelfRef; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where("Doc No." = field("No."), Amount = field(SelfRef))); }
        field(29; CycleA; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where(Amount = field(CycleB))); }
        field(30; CycleB; Decimal) { FieldClass = FlowField; CalcFormula = sum("FF Line".Amount where(Amount = field(CycleA))); }
        field(31; MinQty; Integer) { FieldClass = FlowField; CalcFormula = min("FF Line".Qty where("Doc No." = field("No."))); }
        field(32; LateDate; Date) { FieldClass = FlowField; CalcFormula = max("FF Line"."Posting Date" where("Doc No." = field("No."))); }
        field(33; Closed; Integer) { FieldClass = FlowField; CalcFormula = count("FF Line" where("Doc No." = field("No."), Status = const(Closed))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
"#;

/// D1: 40, -10, 75, 20 (qty 3, 4, 6, 8; one month apart from 2024-01-10); D2: one line of 999.
const LINE_SEED: &str = "
    procedure Add(E: Integer; D: Code[20]; A: Decimal; Q: Integer; P: Date; IsOpen: Boolean; St: Option)
    var L: Record \"FF Line\";
    begin
        L.Init(); L.\"Entry No.\" := E; L.\"Doc No.\" := D; L.Amount := A; L.Qty := Q; L.\"Posting Date\" := P;
        L.Open := IsOpen; L.Status := St; L.Account := 'A' + Format(E mod 2);
        L.Insert();
    end;
    procedure SeedLines()
    var H: Record \"FF Head\"; L: Record \"FF Line\";
    begin
        H.\"No.\" := 'D1'; H.Insert();
        H.\"No.\" := 'D2'; H.Insert();
        H.\"No.\" := 'D3'; H.Insert();
        Add(1, 'D1', 40, 3, 20240110D, true, L.Status::Open);
        Add(2, 'D1', -10, 4, 20240210D, false, L.Status::Released);
        Add(3, 'D1', 75, 6, 20240310D, true, L.Status::Closed);
        Add(4, 'D1', 20, 8, 20240410D, true, L.Status::Open);
        Add(9, 'D2', 999, 99, 20240215D, true, L.Status::Open);
    end;
";

fn run_lines(body: &str) -> Result<Value, Signal> {
    run_with(LINES, &format!("{LINE_SEED}\n{body}"), "F")
}

// corpus record/TestCalcFormulaMinMaxAvgTests.Codeunit.al:140,271 (125 / 4 rows = 31.25, not 31).
#[test]
fn average_into_a_decimal_field_keeps_the_fraction() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\";
        begin
            SeedLines();
            H.Get('D1');
            H.CalcFields(AvgAmt, Total, Lines);
            exit(Format(H.AvgAmt) + '|' + Format(H.Total) + '|' + Format(H.Lines));
        end;",
    );
    assert_eq!(text(r), "31.25|125|4");
}

// corpus record/TestCalcFormulaMinMaxAvgTests.Codeunit.al:97,120,140 (min/max/average over
// Decimal, Integer and Date sources; empty sets give the type default).
#[test]
fn min_max_and_average_over_decimal_integer_and_date_sources() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\"; Seen: Text;
        begin
            SeedLines();
            H.Get('D1');
            H.CalcFields(MinAmt, MaxAmt, AvgAmt, MinQty, AvgQty, FirstDate, LateDate);
            Seen := Format(H.MinAmt) + '|' + Format(H.MaxAmt) + '|' + Format(H.AvgAmt) + '|' + Format(H.MinQty) + '|' + Format(H.AvgQty);
            Seen += '|' + Format(H.FirstDate, 0, 9) + '|' + Format(H.LateDate, 0, 9);
            H.Get('D3');
            H.CalcFields(MinAmt, MaxAmt, AvgAmt, MinQty, FirstDate);
            exit(Seen + '|' + Format(H.MinAmt) + Format(H.MaxAmt) + Format(H.AvgAmt) + Format(H.MinQty) + Format(H.FirstDate = 0D));
        end;",
    );
    // Average Qty = 21 / 4 = 5.25 into an Integer field: 5.
    assert_eq!(text(r), "-10|75|31.25|3|5|2024-01-10|2024-04-10|0000Yes");
}

// corpus record/TestCalcFormulaSignFiltersTests.Codeunit.al:61,150,201 (exist, signed sum and
// signed exist, const and filter conditions ANDed).
#[test]
fn exist_sign_const_and_filter_conditions() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\"; Seen: Text;
        begin
            SeedLines();
            H.Get('D1');
            H.CalcFields(HasLines, NoLines, NegTotal, OpenTotal, ActiveTotal, Closed);
            Seen := Format(H.HasLines) + Format(H.NoLines) + '|' + Format(H.NegTotal) + '|' + Format(H.OpenTotal) + '|' + Format(H.ActiveTotal) + '|' + Format(H.Closed);
            H.Get('D3');
            H.CalcFields(HasLines, NoLines);
            exit(Seen + '|' + Format(H.HasLines) + Format(H.NoLines));
        end;",
    );
    // Open = 40 + 75 + 20; Open|Released = 40 - 10 + 20.
    assert_eq!(text(r), "YesNo|-125|135|50|1|NoYes");
}

// corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:76,107,128,258 (the caller's filter
// on a FlowFilter field narrows a formula, upperlimit keeps only the upper bound, none set
// means no constraint, and the flow filter does not constrain its own table).
#[test]
fn flow_filters_narrow_formulas_and_never_the_record_itself() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\"; Seen: Text;
        begin
            SeedLines();
            H.Get('D1');
            H.CalcFields(Period, Balance);
            Seen := Format(H.Period) + '/' + Format(H.Balance);
            H.SetRange(\"Date Filter\", 20240201D, 20240229D);
            H.CalcFields(Period, Balance, Total);
            Seen += '|' + Format(H.Period) + '/' + Format(H.Balance) + '/' + Format(H.Total);
            H.SetRange(\"Date Filter\");
            H.CalcFields(Period, Balance);
            Seen += '|' + Format(H.Period) + '/' + Format(H.Balance);
            H.Reset();
            H.SetRange(\"Date Filter\", 20990101D, 20991231D);
            Seen += '|' + Format(H.Count()) + Format(H.FindFirst());
            H.Get('D1');
            H.CalcFields(Period);
            exit(Seen + '|' + Format(H.Period));
        end;",
    );
    // Unfiltered 125/125; February only: -10 and, up to the end of February, 40 - 10 = 30;
    // cleared: back to 125; the table itself is not filtered (3 headers) and a window with no
    // lines gives 0.
    assert_eq!(text(r), "125/125|-10/30/125|125/125|3Yes|0");
}

// corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:164,194 (field(filter(X)) reads the
// parent's value as a filter expression; blank imposes nothing).
#[test]
fn field_filter_reads_the_parent_value_as_a_filter_expression() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\"; Seen: Text;
        begin
            SeedLines();
            H.Get('D1');
            H.CalcFields(Totaled);
            Seen := Format(H.Totaled);
            H.Totaling := 'A1';
            H.CalcFields(Totaled);
            Seen += '|' + Format(H.Totaled);
            H.Totaling := 'A0|A1';
            H.CalcFields(Totaled);
            Seen += '|' + Format(H.Totaled);
            H.Totaling := 'A9';
            H.CalcFields(Totaled);
            exit(Seen + '|' + Format(H.Totaled));
        end;",
    );
    // Account is 'A' + (Entry No. mod 2): entries 1 and 3 are A1 (40 + 75), 2 and 4 are A0.
    assert_eq!(text(r), "125|115|125|0");
}

// corpus record/TestCalcFieldsFieldClass.Codeunit.al:76,92,108,124 (only a FlowField with a
// formula can be calculated) and record/TestCalcFormulaValidationTests.Codeunit.al:96 (the
// FlowField and its source must have the same type).
#[test]
fn calcfields_refuses_what_is_not_a_calculable_flow_field() {
    for (field, expected) in [
        ("\"No.\"", "The No. field in the FF Head table must be a FlowField."),
        (
            "\"Date Filter\"",
            "The Date Filter field in the FF Head table must be a FlowField.",
        ),
        (
            "Unformulated",
            "You must define a CalcFormula for the Unformulated FlowField in the FF Head table.",
        ),
        (
            "BadAvg",
            "The following fields must have the same type:\\\\Field: BadAvg <-- Qty\\Table: FF Head <-- FF Line\\Type: Decimal <-- Integer",
        ),
    ] {
        let r = run_lines(&format!(
            "procedure F(): Text
            var H: Record \"FF Head\";
            begin
                SeedLines();
                H.Get('D1');
                H.CalcFields({field});
            end;"
        ));
        match r {
            Err(Signal::Error(e)) => assert_eq!(e.message, expected),
            other => panic!("{field}: {other:?}"),
        }
    }
}

// corpus record/TestCalcFormulaValidationTests.Codeunit.al:211, record/TestCalcFieldsFieldClass.Codeunit.al:204
// (a refused CalcFields calculates none of its fields, not even the valid ones before the bad one).
#[test]
fn a_refused_calcfields_calculates_nothing() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\";
        begin
            SeedLines();
            H.Get('D1');
            asserterror H.CalcFields(Total, Lines, BadAvg);
            exit(Format(H.Total) + Format(H.Lines));
        end;",
    );
    assert_eq!(text(r), "00");
}

// corpus record/TestCalcFormulaFlowFieldValueTests.Codeunit.al:176,205 (a formula that needs
// itself, directly or through another FlowField, is the recursion error).
#[test]
fn formulas_that_need_themselves_raise_the_recursion_error() {
    for field in ["SelfRef", "CycleA", "CycleB"] {
        let r = run_lines(&format!(
            "procedure F(): Text
            var H: Record \"FF Head\";
            begin
                SeedLines();
                H.Get('D1');
                H.CalcFields({field});
            end;"
        ));
        match r {
            Err(Signal::Error(e)) => assert!(
                e.message.contains("This can be caused by recursive function calls"),
                "{e:?}"
            ),
            other => panic!("{field}: {other:?}"),
        }
    }
}

// corpus record/TestRecordFlowField.al:109-160 and TestRecordNavContracts.al:217 (CalcSums over
// the record's own filters, several fields at once, no rows gives 0).
#[test]
fn calcsums_sums_the_filtered_rows_into_the_record() {
    let r = run_lines(
        "procedure F(): Text
        var L: Record \"FF Line\"; Seen: Text;
        begin
            SeedLines();
            L.SetRange(\"Doc No.\", 'D1');
            L.CalcSums(Amount, Qty);
            Seen := Format(L.Amount) + '/' + Format(L.Qty);
            L.SetRange(\"Entry No.\", 1, 2);
            L.CalcSums(Amount);
            Seen += '|' + Format(L.Amount);
            L.SetRange(\"Entry No.\", 99999);
            L.CalcSums(Amount);
            exit(Seen + '|' + Format(L.Amount));
        end;",
    );
    assert_eq!(text(r), "125/21|30|0");
}

// Microsoft test exception (no corpus pin): Bank Acc. Recon. Test.GetTotalOutstanding sets a filter on the
// "Check Ledger Entries" FlowField of the Bank Account Ledger Entry and calls CalcSums. Assumed: the sum is over
// the rows whose FlowField value passes the filter (registry entry calcsums-flowfield-filter).
#[test]
fn calcsums_with_a_filter_on_a_flowfield_sums_the_rows_that_pass() {
    let r = run_with(
        PARENT_CHILD,
        "procedure F(): Text
        var P: Record \"ALT Parent\"; C: Record \"ALT Child\";
        begin
            P.\"Entry No.\" := 1; P.Insert();
            P.\"Entry No.\" := 2; P.Insert();
            P.\"Entry No.\" := 4; P.Insert();
            C.\"Entry No.\" := 1; C.\"Parent Entry No.\" := 2; C.Insert();
            C.\"Entry No.\" := 2; C.\"Parent Entry No.\" := 4; C.Insert();
            C.\"Entry No.\" := 3; C.\"Parent Entry No.\" := 4; C.Insert();
            P.SetFilter(\"Child Count\", '<>0');
            P.CalcSums(\"Entry No.\");
            exit(Format(P.\"Entry No.\"));
        end;",
        "F",
    );
    assert_eq!(text(r), "6");
}

/// Tables with an AutoIncrement key (corpus record/TestAutoIncrement*.al fixtures, compacted).
/// `Payload` steers the OnInsert trigger: 'refuse' raises an error, 'assign' sets the key to 500.
const AUTO: &str = r#"
table 60460 "AI Probe"
{
    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Payload; Text[30]) { }
        field(3; SawInOnInsert; Integer) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
    trigger OnInsert()
    begin
        SawInOnInsert := "Entry No.";
        if Payload = 'refuse' then Error('AI refused the row');
        if Payload = 'assign' then "Entry No." := 500;
    end;
}
table 60461 "AI Big"
{
    fields { field(1; "Entry No."; BigInteger) { AutoIncrement = true; } field(2; Payload; Text[30]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60462 "AI Composite"
{
    fields { field(1; "Entry No."; Integer) { AutoIncrement = true; } field(2; Code; Code[10]) { } }
    keys { key(PK; "Entry No.", Code) { Clustered = true; } }
}
table 60463 "AI Temp Only"
{
    TableType = Temporary;
    fields { field(1; "Entry No."; Integer) { AutoIncrement = true; } field(2; Payload; Text[30]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60464 "AI Unique"
{
    fields { field(1; "Entry No."; Integer) { AutoIncrement = true; } field(2; Code; Code[20]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } key(Uniq; Code) { Unique = true; } }
}
"#;

const AUTO_HELPERS: &str = "
    procedure Zero(): Integer
    var P: Record \"AI Probe\";
    begin
        P.\"Entry No.\" := 0; P.Payload := 'zero'; P.Insert(true);
        exit(P.\"Entry No.\");
    end;
    procedure Explicit(K: Integer): Boolean
    var P: Record \"AI Probe\";
    begin
        P.\"Entry No.\" := K; P.Payload := 'explicit';
        exit(P.Insert(true));
    end;
";

fn run_auto(body: &str) -> Result<Value, Signal> {
    run_with(AUTO, &format!("{AUTO_HELPERS}\n{body}"), "F")
}

// corpus record/TestAutoIncrementContracts.al:25 (a zero key is assigned on Insert), and
// record/TestBCSystemFieldContracts.al:155 / session/TestTransactionContracts.al:104 (the
// counter only moves forward, also after Delete and DeleteAll).
#[test]
fn a_zero_key_is_assigned_from_a_counter_that_only_moves_forward() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            Seen := Format(Zero()) + Format(Zero()) + Format(Zero());
            P.Get(2); P.Delete();
            Seen += '|' + Format(Zero());
            P.DeleteAll();
            exit(Seen + '|' + Format(Zero()) + Format(P.Count()));
        end;",
    );
    assert_eq!(text(r), "123|4|51");
}

// corpus record/TestAutoIncrementContracts.al:40,62,83 (Init keeps the key, so a second Insert
// of the same variable collides until the key is set to 0 again).
#[test]
fn init_keeps_the_assigned_key_so_a_reused_variable_collides_until_it_is_zeroed() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            P.Insert();
            Seen := Format(P.\"Entry No.\");
            P.Init();
            Seen += Format(P.\"Entry No.\");
            asserterror P.Insert();
            Seen += '|' + Format(GetLastErrorText() <> '');
            P.\"Entry No.\" := 0;
            P.Insert();
            exit(Seen + Format(P.\"Entry No.\"));
        end;",
    );
    assert_eq!(text(r), "11|Yes2");
}

// corpus record/TestAutoIncrementExplicitKey.al:43-130 (an explicit key above the counter raises it,
// one at or below leaves it alone, a refused duplicate or a negative key does not move it).
#[test]
fn explicit_keys_raise_the_counter_only_when_they_are_above_it() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            Seen := Format(Zero());
            Explicit(11);
            Seen += '|' + Format(Zero());
            Explicit(2);
            Seen += '|' + Format(Zero());
            Seen += '|' + Format(Explicit(1)) + Format(Zero());
            Explicit(-5);
            Seen += '|' + Format(Zero());
            P.DeleteAll();
            Explicit(1);
            exit(Seen + '|' + Format(Zero()));
        end;",
    );
    // 1; 11 then 12; 2 fills a gap, next 13; 1 is a duplicate (No) and 14; -5 then 15; after
    // DeleteAll an explicit 1 does not lower the counter: 16.
    assert_eq!(text(r), "1|12|13|No14|15|16");
}

// corpus record/TestAutoIncrementExplicitKey.al:63,80 (OnInsert starts with the field 0 and the key
// it assigns is stored and raises the counter); Insert(false) assigns too (record/TestStorageLayerFieldWriteRoutes.al:130).
#[test]
fn on_insert_sees_a_zero_key_and_may_assign_one_and_insert_false_still_assigns() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            Seen := Format(Zero());
            P.Payload := 'assign';
            P.Insert(true);
            Seen += '|' + Format(P.\"Entry No.\") + Format(P.SawInOnInsert);
            Seen += '|' + Format(Zero());
            Clear(P);
            P.Insert(false);
            exit(Seen + '|' + Format(P.\"Entry No.\") + Format(P.SawInOnInsert));
        end;",
    );
    assert_eq!(text(r), "1|5000|501|5020");
}

// corpus record/TestAutoIncrementEdges.al:354 (a refused Insert leaves the key 0 and takes no value)
// and record/TestAutoIncrementExplicitKey.al:154 (the data rolls back, the counter does not).
#[test]
fn an_insert_refused_by_on_insert_takes_no_value_and_the_counter_survives_rollback() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            Commit();
            Seen := Format(Zero());
            P.Payload := 'refuse';
            asserterror P.Insert(true);
            Seen += '|' + Format(P.\"Entry No.\") + '|' + GetLastErrorText();
            Seen += '|' + Format(Zero());
            exit(Seen + '|' + Format(P.Count()));
        end;",
    );
    // The first row is rolled back with the refused one; the counter is not.
    assert_eq!(text(r), "1|0|AI refused the row|2|1");
}

// corpus record/TestAutoIncrementExplicitKey.al:115 (a BigInteger key past Int32).
#[test]
fn a_big_integer_key_follows_the_counter_past_int32() {
    let r = run_with(
        AUTO,
        "procedure F(): Text
        var B: Record \"AI Big\"; Big: BigInteger; Seen: Text;
        begin
            Big := 500000000;
            Big := Big * 10;
            B.Insert(true);
            Clear(B);
            B.\"Entry No.\" := Big;
            B.Insert(true);
            Clear(B);
            B.Insert(true);
            Seen := Format(B.\"Entry No.\" - Big);
            Clear(B);
            B.Insert(true);
            exit(Seen + Format(B.\"Entry No.\" - Big) + Format(B.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "124");
}

// corpus record/TestTempTableContracts.al:26,52,64 (no counter for a temporary record or a table
// with TableType = Temporary: the key stays 0 and two zero keys collide).
#[test]
fn temporary_records_do_not_get_an_auto_increment_key() {
    let r = run_with(
        AUTO,
        "procedure F(): Text
        var T: Record \"AI Temp Only\"; V: Record \"AI Probe\" temporary; Seen: Text;
        begin
            T.Insert();
            Seen := Format(T.\"Entry No.\");
            V.Insert();
            Seen += Format(V.\"Entry No.\");
            V.\"Entry No.\" := 1;
            V.Insert();
            exit(Seen + Format(V.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "002");
}

// corpus record/TestAutoIncrementExplicitKey.al:147,179 and TestAutoIncrementEdges.al:279,296
// (the AutoIncrement value of a key cannot be renamed, the rest of a composite key can).
#[test]
fn renaming_the_auto_increment_value_is_refused_but_the_rest_of_the_key_is_not() {
    let r = run_with(
        AUTO,
        "procedure F(): Text
        var C: Record \"AI Composite\"; Seen: Text; K: Integer;
        begin
            C.Code := 'A';
            C.Insert(true);
            K := C.\"Entry No.\";
            C.Rename(K, 'B');
            Seen := C.Code + Format(C.Get(K, 'B')) + Format(C.Get(K, 'A'));
            asserterror C.Rename(K + 50, 'B');
            exit(Seen + '|' + GetLastErrorText());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "BYesNo|You cannot change the value of an AutoIncrement field.\\\\Field: Entry No.\\Table: AI Composite\\\\Identification fields and values:\\Entry No.='51',Code='B'"
    );
}

// corpus record/TestAutoIncrementExplicitKey.al:131,148,163 and TestAutoIncrementEdges.al:262
// (an Insert refused by a unique secondary key has already taken its value, does not leave it on
// the record, and an explicit key it was refused with still raises the counter).
#[test]
fn an_insert_refused_by_a_unique_secondary_key_consumes_its_value() {
    let r = run_auto(
        "procedure F(): Text
        var U: Record \"AI Unique\"; Seen: Text; K1: Integer;
        begin
            U.Code := 'X';
            Seen := Format(U.Insert(true));
            K1 := U.\"Entry No.\";
            Clear(U);
            U.Code := 'X';
            Seen += Format(U.Insert(true)) + Format(U.\"Entry No.\");
            Clear(U);
            U.Code := 'Y';
            U.Insert(true);
            Seen += '|' + Format(U.\"Entry No.\" - K1);
            Clear(U);
            U.\"Entry No.\" := 50;
            U.Code := 'X';
            Seen += '|' + Format(U.Insert(true)) + Format(U.\"Entry No.\");
            Clear(U);
            U.Code := 'Z';
            U.Insert(true);
            Seen += '|' + Format(U.\"Entry No.\");
            Clear(U);
            U.Code := 'X';
            asserterror U.Insert(true);
            exit(Seen + '|' + Format(U.\"Entry No.\") + '|' + GetLastErrorText());
        end;",
    );
    // 1: Yes; refused: No and the key back at 0; the next row skips one value (2 after 1, so
    // +2); the explicit 50 is refused but stays on the record and raises the counter to 50, so
    // the next row gets 51; the asserterror form leaves the key 0 too.
    assert_eq!(
        text(r),
        "YesNo0|2|No50|51|0|There is already a record in table AI Unique that has the same values in a unique index for the following fields: Code='X'"
    );
}

// Modify breaks a unique secondary key like Insert does (corpus
// record/TestCalcFieldsTruncateUniqueContracts.al:UniqueIndex_Violation_FullTextAndCode_...), but
// rewriting a row with its own values is no violation.
#[test]
fn modify_on_a_unique_secondary_key_raises_only_for_another_row() {
    let r = run_with(
        AUTO,
        "procedure F(): Text
        var U: Record \"AI Unique\"; V: Record \"AI Unique\"; Seen: Text;
        begin
            U.Code := 'X'; U.Insert(true);
            V.Code := 'Y'; V.Insert(true);
            U.Modify();
            V.Code := 'X';
            asserterror V.Modify();
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "There is already a record in table AI Unique that has the same values in a unique index for the following fields: Code='X'|DB:RecordExists"
    );
}

const TRUNC: &str = r#"
table 60470 "TR Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Link; Code[20]) { } field(3; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("TR Row".Link where(Link = field(Link))); } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60471 "TR Guarded"
{
    fields { field(1; "Entry No."; Integer) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 60472 "TR Delete Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"TR Guarded", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure OnBeforeDeleteGuarded(var Rec: Record "TR Guarded"; RunTrigger: Boolean)
    begin
    end;
}
"#;

fn run_trunc(body: &str) -> Result<Value, Signal> {
    run_with(&format!("{AUTO}\n{TRUNC}"), body, "F")
}

// corpus record/TestRecordDelete.al:169 and TestBCSystemFieldContracts.al:180 (Truncate empties
// the table); record/TestAutoIncrementContracts.al:83 (Truncate(true) restarts the counter at 1).
#[test]
fn truncate_empties_the_table_and_only_truncate_true_restarts_the_counter() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text;
        begin
            Zero(); Zero(); Zero();
            P.Truncate();
            Seen := Format(P.IsEmpty());
            Seen += Format(Zero());
            P.Truncate(true);
            exit(Seen + Format(P.IsEmpty()) + Format(Zero()));
        end;",
    );
    // Truncate() keeps the counter (next key 4); Truncate(true) restarts it (next key 1).
    assert_eq!(text(r), "Yes4Yes1");
}

// corpus record/TestAutoIncrementEdges.al:36-170: a rollback to the last commit brings the
// truncated rows back and puts the counter where it stood before the first Truncate of the
// transaction, whatever was taken after it; without a Truncate the counter is not rolled back.
#[test]
fn a_rollback_restores_the_counter_a_truncate_changed() {
    let r = run_auto(
        "procedure F(): Text
        var P: Record \"AI Probe\"; Seen: Text; W: Integer;
        begin
            P.Truncate(true);
            Zero(); Zero(); Zero();
            Commit();
            P.Truncate(true);
            W := Zero();
            P.Payload := 'refuse';
            asserterror P.Insert(true);
            Seen := Format(W) + '|' + Format(P.Count()) + '|' + Format(Zero());
            Commit();
            P.Truncate(false);
            W := Zero();
            asserterror Error('x');
            exit(Seen + '|' + Format(W) + '|' + Format(P.Count()) + '|' + Format(Zero()));
        end;",
    );
    // First round: after Truncate(true) the Insert took 1; the rollback brings back rows 1..3
    // and the counter 3, so the next key is 4 with 4 rows. Second round: Truncate(false) does
    // not reseed; the next key 5 taken after it is handed out again after the rollback, which
    // also restores the rows (1..4, so 4 rows).
    assert_eq!(text(r), "1|3|4|5|4|5");
}

// corpus record/TestRecordTruncateEventAndFilterGuards.al:46,98,113 and
// TestRecordTruncateTryScope.al:34,70 (the refusals are pinned by message; a filter on a normal
// field that keeps every row empties the table).
#[test]
fn truncate_refusals_carry_the_pinned_messages() {
    let guarded = run_trunc(
        "procedure F(): Text
        var G: Record \"TR Guarded\";
        begin
            G.Truncate();
        end;",
    );
    match guarded {
        Err(Signal::Error(e)) => assert_eq!(
            e.message,
            "Truncate is not supported when the OnBeforeDelete and/or OnAfterDelete event is subscribed. Please remove the events subscriptions or use DeleteAll."
        ),
        other => panic!("{other:?}"),
    }
    let flow = run_trunc(
        "procedure F(): Text
        var R: Record \"TR Row\";
        begin
            R.SetRange(Total, 0, 100);
            R.Truncate();
        end;",
    );
    match flow {
        Err(Signal::Error(e)) => assert_eq!(e.message, "Truncate does not support filters on FlowFields."),
        other => panic!("{other:?}"),
    }
    let in_try = run_trunc(
        "[TryFunction] procedure Inner() var R: Record \"TR Row\"; begin R.Truncate(); end;
        procedure Outer() begin Inner(); end;
        [TryFunction] procedure ViaHelper() begin Outer(); end;
        procedure F(): Text begin
            if ViaHelper() then exit('succeeded');
            exit(GetLastErrorText());
        end;",
    );
    assert_eq!(text(in_try), "Truncate is not supported in try functions.");
    let kept = run_trunc(
        "procedure F(): Text
        var R: Record \"TR Row\";
        begin
            R.\"Entry No.\" := 1; R.Link := 'L1'; R.Insert();
            R.SetRange(Link, 'L1');
            R.Truncate();
            R.Reset();
            exit(Format(R.IsEmpty()));
        end;",
    );
    assert_eq!(text(kept), "Yes");
}

// A filter on a normal field limits what Truncate deletes; a temporary record is refused with the
// pinned text (corpus record/TestCalcFieldsTruncateUniqueContracts.al:
// Truncate_WithFilterOnANormalField_RemovesOnlyTheFilteredRows, Truncate_Refusals_Texts).
#[test]
fn truncate_with_a_filter_removes_only_the_filtered_rows_and_a_temporary_record_is_refused() {
    let r = run_trunc(
        "procedure F(): Text
        var R: Record \"TR Row\";
        begin
            R.\"Entry No.\" := 1; R.Link := 'L1'; R.Insert();
            R.\"Entry No.\" := 2; R.Link := 'L2'; R.Insert();
            R.SetRange(Link, 'L1');
            R.Truncate();
            R.Reset();
            exit(Format(R.Count()));
        end;",
    );
    assert_eq!(text(r), "1");
    let t = run_trunc(
        "procedure F(): Text
        var R: Record \"TR Row\" temporary;
        begin
            asserterror R.Truncate();
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;",
    );
    assert_eq!(text(t), "Temporary tables does not support truncation.|DB:Ok");
}

// A FlowField holds no data: what the buffer shows after CalcFields is not written by Insert or
// Modify, so a later Get reads an uncalculated field (corpus record/TestSetAutoCalcFieldsOnGet.Codeunit.al:38).
#[test]
fn calculated_flow_field_values_are_never_stored() {
    let r = run(&format!(
        "{SEED}
        procedure F(): Text
        var P: Record \"ALT Parent\"; Q: Record \"ALT Parent\"; Seen: Text;
        begin
            Seed();
            P.Get(1);
            P.CalcFields(\"Child Count\");
            P.Code := 'X';
            P.Modify();
            Q.Get(1);
            Seen := Format(P.\"Child Count\") + Format(Q.\"Child Count\") + Q.Code;
            P.\"Entry No.\" := 77;
            P.\"Child Count\" := 9;
            P.Insert();
            Q.Get(77);
            exit(Seen + Format(Q.\"Child Count\"));
        end;"
    ));
    assert_eq!(text(r), "20X0");
}

// corpus record/TestCalcFieldsFieldClass.Codeunit.al:140 accepts a Blob in CalcFields, which loads
// its bytes. Streams are implemented now (streams/), so the call succeeds instead of being refused
// (it was a refusal placeholder until then; the Blob semantics are tested in tests/streams.rs).
#[test]
fn calcfields_on_a_blob_field_is_accepted() {
    let r = run_with(
        "table 60480 \"BL Row\" { fields { field(1; K; Code[10]) { } field(2; Data; Blob) { } } }",
        "procedure F()
        var R: Record \"BL Row\";
        begin
            R.CalcFields(Data);
        end;",
        "F",
    );
    assert!(r.is_ok(), "{r:?}");
}

// corpus types/TestEnumBlankOptionFilterTests.Codeunit.al:100-160: SetFilter on an enum field
// names members (case-insensitively, trimmed) or gives an ordinal; the same filter grammar the
// CalcFormula filter() condition uses.
#[test]
fn set_filter_on_an_enum_field_names_members() {
    let r = run_with(
        "enum 60490 \"EN Kind\" { value(0; \" \") { } value(1; \"Initial Entry\") { } value(5; Application) { } }
         table 60491 \"EN Row\" { fields { field(1; K; Integer) { } field(2; Kind; Enum \"EN Kind\") { } } keys { key(PK; K) { } } }",
        "procedure F(): Text
        var R: Record \"EN Row\"; Seen: Text;
        begin
            R.K := 1; R.Kind := R.Kind::\"Initial Entry\"; R.Insert();
            R.K := 2; R.Kind := R.Kind::Application; R.Insert();
            R.K := 3; R.Kind := R.Kind::\" \"; R.Insert();
            R.SetFilter(Kind, 'application');
            Seen := Format(R.Count());
            R.SetFilter(Kind, ''' Initial Entry '' | 5');
            Seen += Format(R.Count());
            R.SetFilter(Kind, '<>'' ''');
            exit(Seen + Format(R.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "122");
}

// corpus record/TestRenamePropagationCond*.al (Record_Rename_ConditionalRelation_ElseArm_Propagates):
// a table named only by the else arm of a conditional TableRelation is related too: its rename
// re-keys the related rows (was refused until propagation existed; corpus
// record/TestRenamePropagationConditional.al:71-111 pins that the else arm carries no condition).
#[test]
fn renaming_a_table_named_by_the_else_arm_of_a_conditional_relation_rewrites_the_related_rows() {
    let r = run_with(
        "table 60500 \"RL Parent A\" { fields { field(1; Code; Code[10]) { } } }
         table 60501 \"RL Parent B\" { fields { field(1; Code; Code[10]) { } } }
         table 60502 \"RL Child\" { fields { field(1; K; Integer) { } field(2; Kind; Option) { OptionMembers = A,B; }
            field(3; Ref; Code[10]) { TableRelation = if (Kind = const(A)) \"RL Parent A\".Code else \"RL Parent B\".Code; } } }",
        "procedure F(): Text
        var B: Record \"RL Parent B\"; C: Record \"RL Child\";
        begin
            B.Code := 'OLD'; B.Insert();
            C.K := 1; C.Kind := C.Kind::A; C.Ref := 'OLD'; C.Insert();
            B.Rename('NEW');
            C.Get(1);
            exit(C.Ref);
        end;",
        "F",
    );
    assert_eq!(text(r), "NEW");
}

// corpus record/TestXRecContracts.al:133 (an OnValidate that nests a Validate on the same record:
// the inner trigger's xRec is the record as it was before the OUTER call) and :166 (a plain
// Validate sees the stored value as xRec).
#[test]
fn a_nested_validate_sees_the_record_as_it_was_before_the_outer_one() {
    let tables = "table 60510 \"NV Row\" { fields {
            field(1; Code; Code[10]) { }
            field(2; Kind; Option) { OptionMembers = First,Second; trigger OnValidate() begin Validate(Ref, ''); end; }
            field(3; Ref; Code[20]) { trigger OnValidate() begin Seen := Format(xRec.Kind) + '|' + xRec.Ref + '|' + Ref; end; }
            field(4; Seen; Text[100]) { } } keys { key(PK; Code) { } } }";
    let r = run_with(
        tables,
        "procedure F(): Text
        var R: Record \"NV Row\";
        begin
            R.Code := 'A'; R.Kind := R.Kind::First; R.Ref := 'X'; R.Insert(true);
            R.Validate(Ref, 'Y');
            exit(R.Seen);
        end;",
        "F",
    );
    assert_eq!(text(r), "First|X|Y");
    let nested = run_with(
        tables,
        "procedure F(): Text
        var R: Record \"NV Row\";
        begin
            R.Code := 'A'; R.Kind := R.Kind::First; R.Ref := 'X'; R.Insert(true);
            R.Validate(Kind, R.Kind::Second);
            exit(R.Seen);
        end;",
        "F",
    );
    assert_eq!(text(nested), "First|X|");
}

// corpus record/TestCalcFormulaValidationTests.Codeunit.al:129 (sum of a Duration source into a Duration
// FlowField: 1 + 2 + 3 + 4 hours) and :154 (average of a Duration source into a Decimal one is refused).
#[test]
fn duration_sources_sum_into_duration_flow_fields() {
    let r = run_lines(
        "procedure Hour(N: Integer): Duration
        begin
            exit(N * 60 * 60 * 1000);
        end;
        procedure F(): Text
        var H: Record \"FF Head\"; L: Record \"FF Line\"; Ten: Duration; None: Duration; Seen: Text;
        begin
            SeedLines();
            L.Get(1); L.Elapsed := Hour(1); L.Modify();
            L.Get(2); L.Elapsed := 2 * Hour(1); L.Modify();
            L.Get(3); L.Elapsed := 3 * Hour(1); L.Modify();
            L.Get(4); L.Elapsed := 4 * Hour(1); L.Modify();
            H.Get('D1');
            H.CalcFields(Hours);
            Ten := 10 * Hour(1);
            Seen := Format(H.Hours = Ten);
            H.Get('D3');
            H.CalcFields(Hours);
            exit(Seen + Format(H.Hours = None));
        end;",
    );
    assert_eq!(text(r), "YesYes");
}

// Mutation-testing additions (docs/status.md debt): checks that were only ever seen green.

const FFX: &str = r#"
enum 60460 "FFX E1" { value(0; A) { } value(1; B) { } }
enum 60461 "FFX E2" { value(0; A) { } value(1; B) { } }
table 60462 "FFX Src"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Grp; Code[10]) { }
        field(3; Txt; Text[30]) { }
        field(4; Cd; Code[30]) { }
        field(5; Big; BigInteger) { }
        field(6; E1; Enum "FFX E1") { }
        field(7; Amount; Decimal) { }
        field(8; Dur; Duration) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 60463 "FFX Head"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; FilterInt; Integer) { FieldClass = FlowFilter; }
        field(3; FilterDec; Decimal) { FieldClass = FlowFilter; }
        field(10; TxtFromCode; Text[30]) { FieldClass = FlowField; CalcFormula = lookup("FFX Src".Cd where(Grp = field("No."))); }
        field(11; CodeFromTxt; Code[30]) { FieldClass = FlowField; CalcFormula = lookup("FFX Src".Txt where(Grp = field("No."))); }
        field(12; EnumSame; Enum "FFX E1") { FieldClass = FlowField; CalcFormula = lookup("FFX Src".E1 where(Grp = field("No."))); }
        field(13; EnumOther; Enum "FFX E2") { FieldClass = FlowField; CalcFormula = lookup("FFX Src".E1 where(Grp = field("No."))); }
        field(14; MinWrong; Integer) { FieldClass = FlowField; CalcFormula = min("FFX Src".Amount where(Grp = field("No."))); }
        field(15; SumBig; BigInteger) { FieldClass = FlowField; CalcFormula = sum("FFX Src".Big where(Grp = field("No."))); }
        field(16; NegCount; Integer) { FieldClass = FlowField; CalcFormula = -count("FFX Src" where(Grp = field("No."))); }
        field(17; ByIntFilter; Decimal) { FieldClass = FlowField; CalcFormula = sum("FFX Src".Amount where(Grp = field("No."), "No." = field(FilterInt))); }
        field(18; ByDecFilter; Decimal) { FieldClass = FlowField; CalcFormula = sum("FFX Src".Amount where(Grp = field("No."), "No." = field(FilterDec))); }
        field(19; LookupAmt; Decimal) { FieldClass = FlowField; CalcFormula = lookup("FFX Src".Amount where(Grp = field("No."))); }
        field(20; SumDur; Duration) { FieldClass = FlowField; CalcFormula = sum("FFX Src".Dur where(Grp = field("No."))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
"#;

const FFX_SEED: &str = "
    procedure Add(N: Integer; G: Code[10]; T: Text[30]; C: Code[30]; B: BigInteger; A: Decimal)
    var S: Record \"FFX Src\";
    begin
        S.\"No.\" := N; S.Grp := G; S.Txt := T; S.Cd := C; S.Big := B; S.Amount := A; S.E1 := S.E1::B;
        S.Insert();
    end;
    procedure Seed()
    var H: Record \"FFX Head\"; Big: BigInteger;
    begin
        H.\"No.\" := 'H'; H.Insert();
        Big := 2147483647; Big := Big * 2;
        Add(1, 'H', 'tx', 'CD', Big, 2.5);
        Add(2, 'H', 'tx', 'CD', 7, 4);
    end;
";

fn run_ffx(body: &str) -> Result<Value, Signal> {
    run_with(FFX, &format!("{FFX_SEED}\n{body}"), "F")
}

fn calc_one(field: &str) -> Result<Value, Signal> {
    run_ffx(&format!(
        "procedure F(): Text
        var H: Record \"FFX Head\";
        begin
            Seed();
            H.Get('H');
            H.CalcFields({field});
            exit(Format(H.{field}));
        end;"
    ))
}

// corpus record/TestCalcFormulaMinMaxAvgTests.Codeunit.al:97 pins min/max/lookup into the very
// same type; Text and Code count as one kind, so does the same enum, other types are refused.
#[test]
fn min_max_and_lookup_accept_text_for_code_and_the_same_enum() {
    assert_eq!(text(calc_one("TxtFromCode")), "CD");
    assert_eq!(text(calc_one("CodeFromTxt")), "TX");
    assert_eq!(text(calc_one("EnumSame")), "B");
}

#[test]
fn min_max_and_lookup_into_another_type_are_refused_and_a_lookup_into_another_enum_keeps_the_member() {
    assert!(unsupported(calc_one("MinWrong")).contains("another type"));
    // corpus record/lt-flowfield-lookup-enum/FlowFieldLookupOtherEnum.al: the looked-up member stays as it is.
    assert_eq!(text(calc_one("EnumOther")), "B");
}

// A sum over a BigInteger source works past the Int32 range.
#[test]
fn sum_over_a_big_integer_source_goes_past_int32() {
    assert_eq!(text(calc_one("SumBig")), "4294967301");
}

// A leading minus on count negates the number of rows (corpus record/TestCalcFormulaSignFiltersTests.Codeunit.al
// pins the sign for sum and exist; count takes the same code path).
#[test]
fn a_negated_count_is_minus_the_row_count() {
    assert_eq!(text(calc_one("NegCount")), "-2");
}

// corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:107: a flow filter of the same kind as
// the source field narrows the formula; one of another type is refused (the match is not pinned).
#[test]
fn a_flow_filter_of_the_source_fields_type_applies_and_another_type_is_refused() {
    let r = run_ffx(
        "procedure F(): Text
        var H: Record \"FFX Head\";
        begin
            Seed();
            H.Get('H');
            H.SetRange(FilterInt, 2);
            H.CalcFields(ByIntFilter);
            exit(Format(H.ByIntFilter));
        end;",
    );
    assert_eq!(text(r), "4");
    let r = run_ffx(
        "procedure F(): Text
        var H: Record \"FFX Head\";
        begin
            Seed();
            H.Get('H');
            H.SetRange(FilterDec, 2);
            H.CalcFields(ByDecFilter);
            exit(Format(H.ByDecFilter));
        end;",
    );
    assert!(unsupported(r).contains("flow filter of another type"));
}

// A lookup over several rows that agree is the value they agree on (disagreeing rows read the
// first in primary key order: lookup_over_rows_that_disagree_reads_the_first_in_primary_key_order).
#[test]
fn lookup_over_rows_that_agree_returns_their_value() {
    let r = run_ffx(
        "procedure F(): Text
        var H: Record \"FFX Head\"; S: Record \"FFX Src\";
        begin
            Seed();
            S.ModifyAll(Amount, 9);
            H.Get('H');
            H.CalcFields(LookupAmt);
            exit(Format(H.LookupAmt));
        end;",
    );
    assert_eq!(text(r), "9");
}

// CalcSums only takes stored number fields; both a stored non-number and a FlowField are refused.
#[test]
fn calcsums_refuses_stored_non_numbers_and_flow_fields() {
    #[allow(clippy::single_element_loop)]
    for field in ["Txt"] {
        let r = run_ffx(&format!(
            "procedure F(): Text
            var S: Record \"FFX Src\";
            begin
                Seed();
                S.CalcSums({field});
            end;"
        ));
        assert!(
            unsupported(r).contains("only stored number fields are implemented"),
            "{field}"
        );
    }
    let r = run_ffx(
        "procedure F(): Text
        var H: Record \"FFX Head\";
        begin
            Seed();
            H.CalcSums(ByIntFilter);
        end;",
    );
    assert!(unsupported(r).contains("only stored number fields are implemented"));
}

/// min and max over the second field of a two-field primary key, with the first field pinned,
/// are the first and last row of the range; the other group's rows and a filter on the field
/// itself leave them right (corpus record/TestDateVirtualTableFlowField.al:94-110 has the same
/// shape over Date: key (Period Type, Period Start)).
#[test]
fn min_and_max_over_a_pinned_key_prefix_ignore_the_rows_of_other_groups() {
    const T: &str = r#"
table 60610 "MM Row"
{
    fields
    {
        field(1; Grp; Integer) { }
        field(2; Seq; Integer) { }
        field(3; "Seq Filter"; Integer) { FieldClass = FlowFilter; }
        field(4; LowSeq; Integer) { FieldClass = FlowField; CalcFormula = min("MM Row".Seq where(Grp = const(1), Seq = field("Seq Filter"))); }
        field(5; HighSeq; Integer) { FieldClass = FlowField; CalcFormula = max("MM Row".Seq where(Grp = const(1), Seq = field("Seq Filter"))); }
        field(6; HighAny; Integer) { FieldClass = FlowField; CalcFormula = max("MM Row".Seq); }
    }
    keys { key(PK; Grp, Seq) { Clustered = true; } }
}
"#;
    let r = run_with(
        T,
        "procedure F(): Text
        var R: Record \"MM Row\"; Q: Record \"MM Row\"; S: Text;
        begin
            R.Grp := 1; R.Seq := 30; R.Insert();
            R.Grp := 2; R.Seq := 99; R.Insert();
            R.Grp := 1; R.Seq := 10; R.Insert();
            R.Grp := 1; R.Seq := 20; R.Insert();
            R.Grp := 0; R.Seq := 5; R.Insert();
            Q.CalcFields(LowSeq, HighSeq, HighAny);
            S := Format(Q.LowSeq) + '/' + Format(Q.HighSeq) + '/' + Format(Q.HighAny);
            Q.SetRange(\"Seq Filter\", 11, 25);
            Q.CalcFields(LowSeq, HighSeq);
            S := S + ' ' + Format(Q.LowSeq) + '/' + Format(Q.HighSeq);
            Q.SetRange(\"Seq Filter\", 31, 40);
            Q.CalcFields(LowSeq, HighSeq);
            exit(S + ' ' + Format(Q.LowSeq) + '/' + Format(Q.HighSeq));
        end;",
        "F",
    );
    assert_eq!(text(r), "10/30/99 20/20 0/0");
}

// Assumption upperlimit-of-a-single-value (Tests-ERM UT_CustBalanceDueCalcOnMaxLimitOfDateFilter): a flow filter that holds one
// value is its own upper limit. Lines of D1 are dated 2024-01-10 (40), 2024-02-10 (-10), 2024-03-10 (75), 2024-04-10 (20).
#[test]
fn upperlimit_of_a_flow_filter_with_one_value_is_that_value() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\";
        begin
            SeedLines();
            H.Get('D1');
            H.SetFilter(\"Date Filter\", Format(20240210D));
            H.CalcFields(Period, Balance);
            exit(Format(H.Period) + '/' + Format(H.Balance));
        end;",
    );
    assert_eq!(text(r), "-10/30");
}

// calcfields/lt-upperlimit-error-code/UpperLimitErrorCode.al:107-111,131: upperlimit() of a filter that is more than one
// range fails with the code DB:NotIntervalFilter.
#[test]
fn upperlimit_of_more_than_one_range_fails_with_not_interval_filter() {
    let r = run_lines(
        "procedure F(): Text
        var H: Record \"FF Head\";
        begin
            SeedLines();
            H.Get('D1');
            H.SetFilter(\"Date Filter\", '%1|%2', 20240210D, 20240310D);
            H.CalcFields(Balance);
            exit('ran');
        end;",
    );
    match r {
        Err(Signal::Error(e)) => {
            assert!(e.message.contains("results in more than one range."), "{}", e.message);
            assert_eq!(e.code, "DB:NotIntervalFilter");
        }
        other => panic!("expected an AL error, got {other:?}"),
    }
}

// Record.CalcSums of a Duration is not pinned (only FieldRef.CalcSum is, FieldRefCalcSumTypes; corpus issue 630 item 3): refused.
#[test]
fn record_calcsums_of_a_duration_field_is_refused_but_fieldref_calcsum_sums_it() {
    const T: &str = r#"
table 60010 "ALT Dur"
{
    fields { field(1; "No."; Integer) { } field(2; D; Duration) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
"#;
    let r = run_with(
        T,
        "procedure F(): Text
        var R: Record \"ALT Dur\";
        begin
            R.\"No.\" := 1; R.D := 5; R.Insert();
            R.CalcSums(D);
            exit('ran');
        end;",
        "F",
    );
    assert!(unsupported(r).contains("CalcSums on D"));
    let r = run_with(
        T,
        "procedure F(): Text
        var R: Record \"ALT Dur\"; RR: RecordRef; FR: FieldRef;
        begin
            R.\"No.\" := 1; R.D := 5; R.Insert();
            RR.GetTable(R); FR := RR.Field(2);
            FR.CalcSum();
            exit(Format(FR.Value));
        end;",
        "F",
    );
    assert_eq!(text(r), "5 milliseconds");
}

// record/lt-temporary-blob/TemporaryBlobAutoCalc.al:Temporary_SetAutoCalcFields_ThenFind: a temporary record with
// SetAutoCalcFields(Blob) loads the Blob on Find, FindFirst, Get and FindSet.
#[test]
fn set_auto_calc_fields_of_a_blob_on_a_temporary_record_loads_it() {
    const T: &str = r#"
table 60011 "ALT TB"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;
    let r = run_with(
        T,
        "procedure F(): Text
        var Row: Record \"ALT TB\" temporary; Fresh: Record \"ALT TB\" temporary; OutS: OutStream; s: Text;
        begin
            Row.Id := 1;
            Row.Data.CreateOutStream(OutS); OutS.WriteText('hello');
            Row.Insert();
            Fresh.Copy(Row, true);
            Fresh.SetAutoCalcFields(Data);
            s := Format(Fresh.FindFirst()) + Format(Fresh.Data.Length());
            Clear(Fresh); Fresh.Copy(Row, true); Fresh.SetAutoCalcFields(Data);
            s += '|' + Format(Fresh.Get(1)) + Format(Fresh.Data.Length());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes5|Yes5");
}

// record/lt-testfield-blob-decimal/TestFieldBlobDecimal.al:TestField_Blob_StoredContent: TestField of a Blob passes after Get
// without CalcFields when the row holds content, and fails once the saved content was cleared.
#[test]
fn test_field_of_a_blob_after_get_without_calcfields_passes_when_the_row_holds_content() {
    const T: &str = r#"
table 60012 "ALT TFB"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;
    let r = run_with(
        T,
        "procedure F(): Text
        var Row: Record \"ALT TFB\"; OutS: OutStream; s: Text;
        begin
            Row.Id := 1; Row.Insert();
            Row.Data.CreateOutStream(OutS); OutS.WriteText('content');
            Row.Modify();
            Row.Get(1);
            Row.TestField(Data);
            s := 'ok';
            Clear(Row.Data); Row.Modify(); Row.Get(1); Row.CalcFields(Data);
            asserterror Row.TestField(Data);
            exit(s + '|' + GetLastErrorText());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "ok|Data must have a value in ALT TFB: Id=1. It cannot be zero or empty."
    );
}

/// al-runner issue 5575: a CalcFormula filter written as one quoted literal with filter operators in it
/// (`filter('10|200..299|400..499')`) sums the matching entries on BC 29, where the runtime read it as one literal value
/// and summed nothing. No corpus test pins how the quoted form is read, so the FlowField is refused; a quoted literal
/// with no operator in it reads the same either way.
#[test]
fn a_calc_formula_filter_quoted_whole_with_operators_is_refused() {
    let src = r#"
table 50110 "Filter Entry" { fields { field(1; "Entry No."; Integer) { } field(2; Category; Code[20]) { } field(3; Amount; Decimal) { } } keys { key(PK; "Entry No.") { } } }
table 50111 "Filter Summary"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("Filter Entry".Amount where(Category = filter('10|200..299|400..499'))); }
        field(3; Plain; Decimal) { FieldClass = FlowField; CalcFormula = sum("Filter Entry".Amount where(Category = filter('250'))); }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50112 C
{
    local procedure Seed()
    var E: Record "Filter Entry";
    begin
        E.DeleteAll();
        E."Entry No." := 1; E.Category := '250'; E.Amount := 17; E.Insert();
    end;

    procedure Union(): Decimal
    var S: Record "Filter Summary";
    begin
        Seed();
        S.CalcFields(Total);
        exit(S.Total);
    end;

    procedure Plain(): Decimal
    var S: Record "Filter Summary";
    begin
        Seed();
        S.CalcFields(Plain);
        exit(S.Plain);
    end;
}
"#;
    match al_testkit::run_in(src, "C", "Union") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("quoted"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(format!("{:?}", al_testkit::run_in(src, "C", "Plain").unwrap()), "Decimal(17)");
}
