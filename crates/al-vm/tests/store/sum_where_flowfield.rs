//! A sum, -sum, average, min or max FlowField whose WHERE tests a FlowField of the summed table (gap G14, the Bank Acc.
//! Reconciliation "Total Outstd Bank Transactions" shape). The tables are those of corpus
//! tests/al-language/record/lt-y17-sum-where-flowfield/Y17SumWhereFlowField.al:9-115; each test cites the corpus test it mirrors.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const TABLES: &str = r#"
table 91811 "LT Y17 SW Detail"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Owner Entry No."; Integer) { }
        field(3; "Doc Date"; Date) { }
        field(4; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 91812 "LT Y17 SW Entry"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Acct No."; Code[20]) { }
        field(3; Open; Boolean) { }
        field(4; Amount; Decimal) { }
        field(5; "Posting Date"; Date) { }
        field(10; "Detail Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("LT Y17 SW Detail" where("Owner Entry No." = field("Entry No.")));
        }
        field(11; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(12; "Detail Count In Period"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("LT Y17 SW Detail" where("Owner Entry No." = field("Entry No."), "Doc Date" = field("Date Filter")));
        }
        field(13; "Detail Amount"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Detail".Amount where("Owner Entry No." = field("Entry No.")));
        }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
        key(Acct; "Acct No.", Open) { SumIndexFields = Amount; }
    }
}

table 91813 "LT Y17 SW Header"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Acct No."; Code[20]) { }
        field(10; "Sum No Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = const(0)));
        }
        field(11; "Sum With Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = filter(> 0)));
        }
        field(12; "Neg Sum No Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = - sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = const(0)));
        }
        field(13; "Avg With Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = average("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = filter(> 0)));
        }
        field(14; "Min With Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = min("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = filter(> 0)));
        }
        field(15; "Max With Detail"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = max("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), Open = const(true), "Detail Count" = filter(> 0)));
        }
        field(16; "Sum In Period"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), "Detail Count In Period" = filter(> 0)));
        }
        field(17; "Sum Detail Amount NonZero"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), "Detail Amount" = filter(<> 0)));
        }
        field(18; "Sum No Detail All"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("LT Y17 SW Entry".Amount where("Acct No." = field("Acct No."), "Detail Count" = const(0)));
        }
    }

    keys
    {
        key(PK; "Acct No.") { Clustered = true; }
    }
}

"#;

const SEED: &str = r#"
    local procedure Seed()
    begin
        AddEntry(5, 'A', true, 40, 20240105D);
        AddEntry(3, 'A', true, 7.5, 20240103D);
        AddEntry(1, 'A', true, 10, 20240101D);
        AddEntry(4, 'A', false, 100, 20240104D);
        AddEntry(2, 'A', true, -25, 20240102D);
        AddEntry(6, 'B', true, 999, 20240106D);
        AddDetail(3, 3, 20240105D, 0);
        AddDetail(1, 2, 20240105D, 5);
        AddDetail(2, 3, 20240210D, 0);
        AddDetail(4, 6, 20240105D, 1);
        AddHeader('C');
        AddHeader('A');
        AddHeader('B');
    end;
    local procedure AddEntry(No: Integer; Acct: Code[20]; IsOpen: Boolean; Amt: Decimal; PostingDate: Date)
    var O: Record "LT Y17 SW Entry";
    begin
        O."Entry No." := No; O."Acct No." := Acct; O.Open := IsOpen; O.Amount := Amt; O."Posting Date" := PostingDate; O.Insert();
    end;
    local procedure AddDetail(No: Integer; Owner: Integer; DocDate: Date; Amt: Decimal)
    var D: Record "LT Y17 SW Detail";
    begin
        D."Entry No." := No; D."Owner Entry No." := Owner; D."Doc Date" := DocDate; D.Amount := Amt; D.Insert();
    end;
    local procedure AddHeader(Acct: Code[20])
    var H: Record "LT Y17 SW Header";
    begin
        H."Acct No." := Acct; H.Insert();
    end;
"#;

fn run(vars: &str, body: &str) -> Result<Value, Signal> {
    let src =
        format!("{TABLES}\ncodeunit 50100 C {{ {SEED} procedure F(): Text var {vars} begin Seed(); {body} end; }}");
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn sum_keeps_the_rows_whose_count_flowfield_passes() {
    // Y17SumWhereFlowField.al:197 CalcSumWhereFlowField_Basic.
    let r = run(
        "H: Record \"LT Y17 SW Header\"; T: Text; A: Code[20];",
        "foreach A in 'A,B,C'.Split(',') do begin H.Get(A); H.CalcFields(\"Sum No Detail\", \"Sum With Detail\"); \
         T += Format(H.\"Sum No Detail\", 0, 9) + '/' + Format(H.\"Sum With Detail\", 0, 9) + ';'; end; exit(T);",
    );
    assert_eq!(text(r), "50/-17.5;0/999;0/0;");
}

#[test]
fn negated_sum_average_min_and_max_over_the_passing_rows() {
    // Y17SumWhereFlowField.al:227 CalcSumWhereFlowField_OtherMethods.
    let r = run(
        "H: Record \"LT Y17 SW Header\"; T: Text; A: Code[20];",
        "foreach A in 'A,C'.Split(',') do begin H.Get(A); \
         H.CalcFields(\"Neg Sum No Detail\", \"Avg With Detail\", \"Min With Detail\", \"Max With Detail\", \"Sum No Detail All\"); \
         T += Format(H.\"Neg Sum No Detail\", 0, 9) + '/' + Format(H.\"Avg With Detail\", 0, 9) + '/' + \
         Format(H.\"Min With Detail\", 0, 9) + '/' + Format(H.\"Max With Detail\", 0, 9) + '/' + Format(H.\"Sum No Detail All\", 0, 9) + ';'; end; \
         exit(T);",
    );
    assert_eq!(text(r), "-50/-8.75/-25/7.5/150;0/0/0/0/0;");
}

#[test]
fn a_filter_on_such_a_sum_selects_the_headers() {
    // Y17SumWhereFlowField.al:272 CalcSumWhereFlowField_FilterOnHeader.
    let r = run(
        "H: Record \"LT Y17 SW Header\"; T: Text;",
        "H.SetFilter(\"Sum With Detail\", '<>0'); if H.FindSet() then repeat T += H.\"Acct No.\" + ';'; until H.Next() = 0; \
         H.Reset(); H.SetRange(\"Sum With Detail\", 0); exit(T + Format(H.Count()));",
    );
    assert_eq!(text(r), "A;B;1");
}

#[test]
fn a_condition_flowfield_with_an_unset_flowfilter_and_a_sum_condition() {
    // Y17SumWhereFlowField.al:303 CalcSumWhereFlowField_ConditionWithFlowFilter.
    let r = run(
        "H: Record \"LT Y17 SW Header\";",
        "H.Get('A'); H.CalcFields(\"Sum In Period\", \"Sum Detail Amount NonZero\"); \
         exit(Format(H.\"Sum In Period\", 0, 9) + '/' + Format(H.\"Sum Detail Amount NonZero\", 0, 9));",
    );
    assert_eq!(text(r), "-17.5/-25");
}

#[test]
fn a_negated_count_with_such_a_condition_stays_refused() {
    // Only a sign on sum is pinned (Y17SumWhereFlowField.al:227).
    let src = format!(
        "{TABLES}\ntable 91815 \"LT Y17 SW Neg\" {{ fields {{ field(1; K; Code[20]) {{ }} \
         field(2; N; Integer) {{ FieldClass = FlowField; CalcFormula = - count(\"LT Y17 SW Entry\" where(\"Detail Count\" = const(0))); }} }} }}\n\
         codeunit 50100 C {{ procedure F(): Text var X: Record \"LT Y17 SW Neg\"; begin X.CalcFields(N); exit(Format(X.N)); end; }}"
    );
    let r = al_testkit::run_in(&src, "C", "F");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
