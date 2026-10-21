//! A global passed by `var` to a procedure while code reaches the same global by its own name. BC passes a `var` argument
//! by reference: when the procedure is the codeunit's own (same instance), the parameter and the global are ONE variable, and
//! a change through either shows in the other (codeunit/lt-var-global-alias/VarGlobalAlias.al:12-118, tests :120-170).
//! The event form is pinned in events/lt-event-var-through-sender/EventVarThroughSender.al. A global passed by var to a
//! procedure of another codeunit instance, reached by name while that call runs, is still refused (not pinned).
//! Found by Microsoft's Tests-ERM (Reminder Header.InsertTextLines passes its global NextLineNo by var and
//! InsertBlankLine adds to the global: "Reminder Line already exists" in 134997 and 134012).

use al_builtins::native::Signal;
use al_testkit::{int, run, run_in, text};

const SRC: &str = r#"
codeunit 50100 Alias
{
    var
        Next: Integer;

    procedure ViaNestedWrite(): Integer
    begin
        Next := 10;
        Bump(Next);
        exit(Next);
    end;

    local procedure Bump(var N: Integer)
    begin
        AddToGlobal();
        N := N + 1;
    end;

    local procedure AddToGlobal()
    begin
        Next := Next + 100;
    end;

    procedure ViaDirectRead(): Integer
    var
        Seen: Integer;
    begin
        Next := 10;
        Seen := ReadGlobal(Next);
        exit(Seen);
    end;

    local procedure ReadGlobal(var N: Integer): Integer
    begin
        N := 20;
        exit(Next);
    end;

    procedure LocalByVar(): Integer
    var
        L: Integer;
    begin
        Next := 10;
        L := 1;
        BumpLocal(L);
        exit(L + Next);
    end;

    local procedure BumpLocal(var N: Integer)
    begin
        AddToGlobal();
        N := N + 1;
    end;

    procedure GlobalByVarUntouched(): Integer
    begin
        Next := 10;
        Twice(Next);
        exit(Next);
    end;

    local procedure Twice(var N: Integer)
    begin
        N := N * 2;
        Helper();
    end;

    local procedure Helper()
    begin
    end;
}
"#;

/// The nested call changes the global by name; the parameter shows it (VarGlobalAlias.al:66-71, :124-131).
#[test]
fn global_by_var_changed_by_a_nested_call_shows_in_the_parameter() {
    assert_eq!(int(run(SRC, "ViaNestedWrite")), 111);
}

/// The callee changes its parameter and reads the global by name in its own body (VarGlobalAlias.al:88-94, :160-169).
#[test]
fn global_by_var_read_by_its_name_in_the_callee_sees_the_parameter() {
    assert_eq!(int(run(SRC, "ViaDirectRead")), 20);
}

/// A local passed by var cannot be reached by another name: nothing to refuse
/// (codeunit/TestScopeIsolationContracts.al:176 pins the write-back).
#[test]
fn local_by_var_runs() {
    assert_eq!(int(run(SRC, "LocalByVar")), 2 + 110);
}

/// A global passed by var that nothing else touches while the callee runs: the copy and the reference agree
/// (codeunit/TestCodeunitRunContracts.al:149 pins that the change reaches the caller).
#[test]
fn global_by_var_not_touched_otherwise_runs() {
    assert_eq!(int(run(SRC, "GlobalByVarUntouched")), 20);
}

/// The corpus host of codeunit/lt-var-global-alias/VarGlobalAlias.al:12-118, with the traces its tests assert (:120-170).
const CORPUS: &str = r#"
codeunit 87140 "LT QP Alias Host"
{
    var
        GInt: Integer;
        GTxt: Text;
        Seen: Text;

    procedure GlobalChangedByName(): Text
    begin
        GInt := 1;
        GTxt := 'g1';
        Seen := '';
        P_ReadAfterQ(GInt, GTxt);
        Seen += 'after:' + Format(GInt) + '/' + GTxt;
        exit(Seen);
    end;

    procedure ParamChanged(): Text
    begin
        GInt := 1;
        GTxt := 'g1';
        Seen := '';
        P_ChangeThenQ(GInt, GTxt);
        Seen += 'after:' + Format(GInt) + '/' + GTxt;
        exit(Seen);
    end;

    procedure BothChanged(): Text
    begin
        GInt := 1;
        GTxt := 'g1';
        Seen := '';
        P_Both(GInt, GTxt);
        Seen += 'after:' + Format(GInt) + '/' + GTxt;
        exit(Seen);
    end;

    procedure IntegerOnly(): Text
    begin
        GInt := 5;
        Seen := '';
        P_Int(GInt);
        Seen += 'after:' + Format(GInt);
        exit(Seen);
    end;

    procedure TextOnly(): Text
    begin
        GTxt := 'x';
        Seen := '';
        P_Txt(GTxt);
        Seen += 'after:' + GTxt;
        exit(Seen);
    end;

    local procedure P_ReadAfterQ(var I: Integer; var T: Text)
    begin
        Seen += 'p_in:' + Format(I) + '/' + T + ';';
        Q_SetGlobals(20, 'q20');
        Seen += 'p_after_q:' + Format(I) + '/' + T + ';';
    end;

    local procedure P_ChangeThenQ(var I: Integer; var T: Text)
    begin
        I := 30;
        T := 'p30';
        Seen += 'p_set;';
        Q_ReadGlobals();
    end;

    local procedure P_Both(var I: Integer; var T: Text)
    begin
        Q_SetGlobals(20, 'q20');
        I += 100;
        T += '+p';
        Seen += 'p_after_add:' + Format(I) + '/' + T + ';';
        Q_ReadGlobals();
        Q_SetGlobals(7, 'q7');
        Seen += 'p_end:' + Format(I) + '/' + T + ';';
    end;

    local procedure P_Int(var I: Integer)
    begin
        GInt := GInt + 1;
        Seen += 'p_after_global_inc:' + Format(I) + ';';
        I := I + 10;
        Seen += 'p_global_after_param_inc:' + Format(GInt) + ';';
    end;

    local procedure P_Txt(var T: Text)
    begin
        GTxt += 'G';
        Seen += 'p_after_global_append:' + T + ';';
        T += 'P';
        Seen += 'p_global_after_param_append:' + GTxt + ';';
    end;

    local procedure Q_SetGlobals(NewInt: Integer; NewTxt: Text)
    begin
        GInt := NewInt;
        GTxt := NewTxt;
    end;

    local procedure Q_ReadGlobals()
    begin
        Seen += 'q_sees:' + Format(GInt) + '/' + GTxt + ';';
    end;
}
"#;

#[test]
fn corpus_traces_match_bc() {
    let cases = [
        ("GlobalChangedByName", "p_in:1/g1;p_after_q:20/q20;after:20/q20"),
        ("ParamChanged", "p_set;q_sees:30/p30;after:30/p30"),
        ("BothChanged", "p_after_add:120/q20+p;q_sees:120/q20+p;p_end:7/q7;after:7/q7"),
        ("IntegerOnly", "p_after_global_inc:6;p_global_after_param_inc:16;after:16"),
        ("TextOnly", "p_after_global_append:xG;p_global_after_param_append:xGP;after:xGP"),
    ];
    for (routine, want) in cases {
        assert_eq!(text(run_in(CORPUS, "LT QP Alias Host", routine)), want, "{routine}");
    }
}

/// A global passed by var to a procedure of ANOTHER codeunit, which calls back into the first codeunit to read the
/// global by name: not pinned (only the own-codeunit form and the event form are), so it stays refused.
const OTHER: &str = r#"
codeunit 50110 Owner
{
    var
        Next: Integer;

    procedure Run(): Integer
    var
        Other: Codeunit Callee;
    begin
        Next := 10;
        Other.Take(Next, this);
        exit(Next);
    end;

    procedure ReadNext(): Integer
    begin
        exit(Next);
    end;
}

codeunit 50111 Callee
{
    procedure Take(var N: Integer; O: Codeunit Owner)
    begin
        N := 20;
        if O.ReadNext() = 0 then;
    end;
}
"#;

#[test]
fn global_by_var_to_another_codeunit_read_back_by_name_is_refused() {
    let r = run_in(OTHER, "Owner", "Run");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref why)) if why.contains("by var")),
        "{r:?}"
    );
}
