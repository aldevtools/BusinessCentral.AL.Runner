//! SetFilter on a Text field with a quote that is not closed (corpus master record/crud/RcAFilters.al:996-1027,
//! SetFilter_Text_UnterminatedQuote_ErrorsOnlyWhenTheQuoteStartsTheFilter; al-runner issue 7462): only a quote that starts the
//! filter and is not closed raises the filter error (code Filter:CSide); after `@`, inside a wildcard filter or after other
//! text it is accepted and GetFilter shows the text as given.

use al_testkit::text;

const SRC: &str = r#"
table 50490 "FUQ Main" { fields { field(1; "Entry No."; Integer) { } field(2; Tx; Text[100]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50490 C
{
    [TryFunction] local procedure TrySet(Expr: Text; var R: Record "FUQ Main" temporary)
    begin
        R.SetFilter(Tx, Expr);
    end;
    local procedure P(Expr: Text): Text
    var R: Record "FUQ Main" temporary; Ok: Boolean; S: Text;
    begin
        Ok := TrySet(Expr, R);
        S := Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|filter=' + R.GetFilter(Tx);
        ClearLastError();
        exit(S);
    end;
    procedure F(): Text
    begin
        exit(P('''') + '/' + P('@''') + '/' + P('@*''*') + '/' + P('*''*') + '/' + P('abc''') + '/' + P('@abc''') + '/'
            + P('''abc') + '/' + P('@''abc') + '/' + P('''''') + '/' + P('''''''') + '/' + P('@''''') + '/' + P('@'''''''));
    end;
}
"#;

#[test]
fn only_a_leading_unclosed_quote_raises() {
    let err = |f: &str| {
        format!(
            "No|The filter \"{f}\" is not valid for the Tx field on the FUQ Main table. An single quote (') is missing.|Filter:CSide|filter="
        )
    };
    let ok = |f: &str| format!("Yes|||filter={f}");
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        [
            err("'"),
            ok("@'"),
            ok("@*'*"),
            ok("*'*"),
            ok("abc'"),
            ok("@abc'"),
            err("'abc"),
            ok("@'abc"),
            ok("''"),
            err("'''"),
            ok("@''"),
            ok("@'''"),
        ]
        .join("/")
    );
}

const READS: &str = r#"
table 50491 "FUQ Row" { fields { field(1; "Entry No."; Integer) { } field(2; Tx; Text[100]) { } field(3; Amount; Decimal) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50491 C
{
    local procedure Filtered(var R: Record "FUQ Row" temporary; Expr: Text)
    begin
        R."Entry No." := 1; R.Tx := ''''; R.Amount := 1; R.Insert();
        R.SetFilter(Tx, Expr);
    end;
    procedure FindFirst(): Boolean var R: Record "FUQ Row" temporary; begin Filtered(R, '@'''); exit(R.FindFirst()); end;
    procedure FindSet(): Boolean var R: Record "FUQ Row" temporary; begin Filtered(R, '@''abc'); exit(R.FindSet()); end;
    procedure Count(): Integer var R: Record "FUQ Row" temporary; begin Filtered(R, '@'''''''); exit(R.Count()); end;
    procedure IsEmpty(): Boolean var R: Record "FUQ Row" temporary; begin Filtered(R, '@'''); exit(R.IsEmpty()); end;
    procedure CalcSums(): Decimal var R: Record "FUQ Row" temporary; begin Filtered(R, '@'''); R.CalcSums(Amount); exit(R.Amount); end;
    procedure RefCount(): Integer var R: Record "FUQ Row" temporary; RecRef: RecordRef; begin Filtered(R, '@'''); RecRef.GetTable(R); exit(RecRef.Count()); end;
    procedure GetFilterOnly(): Text var R: Record "FUQ Row" temporary; begin Filtered(R, '@'''); exit(R.GetFilter(Tx)); end;
}
"#;

/// Which rows such a filter keeps is not pinned (docs/probes-needed.md, "SetFilter with an unclosed quote after `@`"): a read
/// that selects rows through it is refused; setting it and reading it back is not.
#[test]
fn a_read_through_an_unclosed_quote_filter_is_refused() {
    for routine in ["FindFirst", "FindSet", "Count", "IsEmpty", "CalcSums", "RefCount"] {
        match al_testkit::run_in(READS, "C", routine) {
            Err(al_builtins::native::Signal::Unsupported(why)) => {
                assert!(why.contains("matching not pinned"), "{routine}: {why}")
            }
            other => panic!("{routine}: {other:?}"),
        }
    }
    assert_eq!(text(al_testkit::run_in(READS, "C", "GetFilterOnly")), "@'");
}

const FLOW: &str = r#"
table 50492 "FUQ Owner" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[50]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50493 "FUQ Item"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Owner Code"; Code[10]) { }
        field(3; "Owner Name"; Text[50]) { FieldClass = FlowField; CalcFormula = lookup("FUQ Owner".Name where(Code = field("Owner Code"))); }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 50492 C
{
    procedure FlowFieldFilter(): Boolean
    var O: Record "FUQ Owner"; I: Record "FUQ Item";
    begin
        O.Code := 'A'; O.Name := ''''; O.Insert();
        I."Entry No." := 1; I."Owner Code" := 'A'; I.Insert();
        I.SetFilter("Owner Name", '@''');
        exit(I.FindFirst());
    end;
}
"#;

/// A filter on a FlowField is checked row by row against the calculated value; with an unclosed quote after `@` that check
/// is refused too, not answered as "no row".
#[test]
fn a_flowfield_filter_with_an_unclosed_quote_is_refused() {
    match al_testkit::run_in(FLOW, "C", "FlowFieldFilter") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("matching not pinned"), "{why}"),
        other => panic!("{other:?}"),
    }
}
