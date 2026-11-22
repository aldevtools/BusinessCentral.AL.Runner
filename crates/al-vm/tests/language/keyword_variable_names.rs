//! A variable named by a keyword. The latest AL compiler (30.0.42.60748, 2026-10-21) accepts `If: Integer;` (and most
//! other keywords, see `VAR_NAME_KEYWORDS` in al-syntax) as a variable name; the variable is then used quoted. Using it
//! unquoted (`If := 1;`) is a compile error there (AL0224), and a parse error here.

#[test]
fn a_variable_named_by_a_keyword_is_used_quoted() {
    let src = "codeunit 50100 C { procedure F(): Integer var If: Integer; While: Integer; begin \"If\" := 5; \"While\" := 2; exit(\"If\" * \"While\"); end; }";
    assert_eq!(al_testkit::int(al_testkit::run_in(src, "C", "F")), 10);
}

/// al-runner issue 8629: a variable or parameter named Local is used unquoted, in code (the AL compiler of every
/// supported major compiles it, and a BC container runs it; corpus PR 1494, `LocalAsName.al`).
#[test]
fn a_variable_named_local_is_used_unquoted() {
    let src = "codeunit 50100 C {
        procedure F(): Integer var Other, Local: Integer; begin Other := 1; Local := 20; Local += 1; exit(Other + Local); end;
        procedure G(Local: Integer): Integer begin Local := Local * 2; exit(Local); end;
        procedure H(): Integer begin exit(G(21)); end;
        procedure L(): Integer var Other, Local: List of [Integer]; begin Local.Add(4); Local.Add(5); exit(Local.Count() * 10 + Local.Get(2)); end;
        procedure S(): Integer var Other, Local: Integer; begin Other := 0; for Local := 1 to 3 do Other += Local; exit(Other * 10 + Local); end; }";
    for (procedure, want) in [("F", 22), ("H", 42), ("L", 25), ("S", 63)] {
        assert_eq!(
            al_testkit::int(al_testkit::run_in(src, "C", procedure)),
            want,
            "{procedure}"
        );
    }
}

/// al-runner issue 8629 review: a procedure named Local, called unquoted as a statement, in an expression, as an argument
/// and through a codeunit variable, with and without parentheses. The AL compilers of every supported major compile each form
/// (corpus PR 1494, `LocalAsName.al`).
#[test]
fn a_procedure_named_local_is_called_unquoted() {
    let src = "codeunit 50100 P {
        var Calls: Integer;
        procedure Local(): Integer begin Calls += 1; exit(40 + Calls); end;
        procedure CallForms(): Text var X: Integer; Log: Text; begin
            Calls := 0;
            Local();
            Local;
            X := Local();
            Log := Format(X);
            X := Local;
            Log += ',' + Format(X);
            Log += ',' + Format(Local() + 1);
            if Local() > 0 then Log += ',if';
            Log += ',' + Format(Abs(Local()));
            Log += ',' + Format(ExitCall());
            Log += ',' + Format(ExitBare());
            Log += ',' + Format(Calls);
            exit(Log);
        end;
        local procedure ExitCall(): Integer begin exit(Local()); end;
        local procedure ExitBare(): Integer begin exit(Local); end; }
    codeunit 50101 C {
        procedure Other(): Text var Proc: Codeunit P; begin exit(Proc.CallForms() + ';' + Format(Proc.Local()) + ',' + Format(Proc.Local)); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "Other")),
        "43,44,46,if,47,48,49,9;50,51"
    );
}
