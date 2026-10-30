//! SetFilter on a Text field with a malformed expression (filters/lt-wc/WcInvalidFilterExpr.al, al-runner issue 6319): an
//! unclosed `(` raises "A ')' is missing.", a `)` or `<>` where no operator may stand raises "Did not expect ...", `@` alone keeps
//! every row, `<>@` alone keeps none, `<>@` before a value keeps the rows that do not equal it without regard to case, and
//! `@@` reads the second `@` as a character.

use al_testkit::text;

const SRC: &str = r#"
table 50489 "FIE Value" { fields { field(1; "Entry No."; Integer) { } field(2; Value; Text[100]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50489 C
{
    local procedure Add(var V: Record "FIE Value" temporary; No: Integer; T: Text)
    begin
        V."Entry No." := No; V.Value := T; V.Insert();
    end;
    [TryFunction] local procedure TryFilter(T: Text; var R: Text)
    var V: Record "FIE Value" temporary;
    begin
        Add(V, 1, 'c-a'); Add(V, 2, 'c-b'); Add(V, 3, 'C-A'); Add(V, 4, '((x'); Add(V, 5, '');
        V.SetFilter(Value, T);
        R := V.GetFilter(Value) + '=';
        if V.FindSet() then repeat R += Format(V."Entry No.") + ','; until V.Next() = 0;
    end;
    local procedure P(T: Text): Text
    var R: Text;
    begin
        if TryFilter(T, R) then exit(R);
        exit(GetLastErrorText());
    end;
    procedure Missing(): Text begin exit(P('((*') + '/' + P('(') + '/' + P('((x') + '/' + P('(c-a') + '/' + P('(c-a)')); end;
    procedure NotExpected(): Text begin exit(P('c-a)') + '/' + P('@<>C-A') + '/' + P('@<>')); end;
    procedure AtSign(): Text begin exit(P('@') + '/' + P('@@C-A') + '/' + P('<>@C-A') + '/' + P('<>@c-b') + '/' + P('<>@')); end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

fn invalid(filter: &str, tail: &str) -> String {
    format!("The filter \"{filter}\" is not valid for the Value field on the FIE Value table. {tail}")
}

/// WcInvalidFilterExpr.al:66-90 `((*`, `(`, `((x`, `(c-a`, `(c-a)`.
#[test]
fn an_unclosed_parenthesis_raises_a_missing_parenthesis() {
    let m = "A ')' is missing.";
    assert_eq!(
        run("Missing"),
        [
            invalid("((*", m),
            invalid("(", m),
            invalid("((x", m),
            invalid("(c-a", m),
            "c-a=1,".into()
        ]
        .join("/")
    );
}

/// WcInvalidFilterExpr.al:71-87 `c-a)`, `@<>C-A`, `@<>`.
#[test]
fn a_stray_parenthesis_or_not_equal_raises_did_not_expect() {
    let d = |w: &str| {
        format!(
            "An error occurred while interpreting the filter: Did not expect '{w}'.\\\\For more information, see https://go.microsoft.com/fwlink/?linkid=2158058."
        )
    };
    assert_eq!(
        run("NotExpected"),
        [
            invalid("c-a)", &d(")")),
            invalid("@<>C-A", &d("<>")),
            invalid("@<>", &d("<>"))
        ]
        .join("/")
    );
}

/// WcInvalidFilterExpr.al:84-90 `@`, `@@C-A`; :93-111 `<>@C-A`, `<>@c-b`, `<>@`.
#[test]
fn at_sign_alone_and_doubled_and_after_not_equal() {
    assert_eq!(run("AtSign"), "@=1,2,3,4,5,/@@C-A=/<>@C-A=2,4,5,/<>@c-b=1,3,4,5,/<>@=");
}
