//! al-runner issue 5757: a procedure that returns a Record and leaves by a bare `exit;` or falls off its end returns a record
//! with the InitValue values; a named return value returns what was assigned to it; the receiving variable keeps its filters
//! (misc/lt-inferred-behaviors/P69ProcResultField.al:424-480 RN_BareExit_ReturnsRecord, RN_NamedBareExit_And_NoExit_ReturnRecord).

const SRC: &str = r#"
table 50950 "Ret Row" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[20]) { } field(3; Qty; Decimal) { InitValue = 7; } } keys { key(PK; Code) { } } }
codeunit 50100 C
{
    local procedure Bare(C: Code[10]): Record "Ret Row"
    var R: Record "Ret Row";
    begin
        if C = '' then exit;
        R.Get(C); exit(R);
    end;
    local procedure Named(C: Code[10]) Result: Record "Ret Row"
    begin
        Result.Name := 'named';
        if C = '' then exit;
        Result.Get(C);
    end;
    local procedure NoExit(): Record "Ret Row" var R: Record "Ret Row"; begin R.Get('A'); end;
    procedure F(): Text
    var R: Record "Ret Row"; S: Text;
    begin
        R.Code := 'A'; R.Name := 'alpha'; R.Qty := 2; R.Insert();
        R.SetRange(Code, 'A');
        R := Bare('');
        S := '[' + R.Code + ']' + R.Name + '|' + Format(R.Qty) + '|' + R.GetFilters() + '|' + Format(Bare('').Qty);
        R := Named('');
        S += '#' + R.Name + '|' + Format(R.Qty);
        R := NoExit();
        exit(S + '#[' + R.Code + ']' + Format(R.Qty) + '#' + Bare('A').Name);
    end;
}
"#;

#[test]
fn a_record_returned_by_bare_exit_holds_the_init_values() {
    assert_eq!(
        al_testkit::text(al_testkit::run(SRC, "F")),
        "[]|7|Code: A|7#named|7#[]7#alpha"
    );
}
