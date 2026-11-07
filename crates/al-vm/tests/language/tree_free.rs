//! Nodes that used to run on the tree walker from inside the bytecode now have instructions of their own
//! (docs/reviews/2026-10-bytecode-design.md, "Retiring the tree walker"). Each routine has a loop so that it is compiled at
//! its first call.

use al_testkit::int;

fn code(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

fn ops_of(src: &str) -> String {
    let l = al_testkit::compile(src);
    let (oi, ri) = l
        .program
        .objects
        .iter()
        .enumerate()
        .find_map(|(oi, o)| o.routines.iter().position(|r| r.name == "F").map(|ri| (oi, ri)))
        .expect("routine F");
    al_vm::exec_disassemble(&l.program, oi, ri)
        .expect("compiles")
        .join("\n")
}

fn run(src: &str) -> i32 {
    int(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn foreach_over_a_global_and_over_a_routine_result_runs_in_bytecode() {
    let src = code(
        "var G: List of [Integer]; \
         procedure Make(): List of [Integer] var L: List of [Integer]; begin L.Add(10); L.Add(20); exit(L); end; \
         procedure F(): Integer var V: Integer; S: Integer; I: Integer; begin G.Add(1); G.Add(2); G.Add(3); \
         for I := 1 to 2 do begin foreach V in G do S += V; foreach V in Make() do S += V; end; exit(S); end;",
    );
    let ops = ops_of(&src);
    assert!(!ops.contains("TreeExpr"), "{ops}");
    assert!(ops.matches("ForeachInit").count() == 2, "{ops}");
    assert_eq!(run(&src), 2 * (6 + 30));
}

#[test]
fn application_area_and_clear_all_run_in_bytecode() {
    let src = code(
        "var G: Integer; \
         procedure F(): Integer var I: Integer; R: Integer; begin G := 5; for I := 1 to 1 do begin ApplicationArea('#All'); \
         if ApplicationArea() = '#All' then R += 1; ClearAll(); R += G; end; exit(R); end;",
    );
    let ops = ops_of(&src);
    assert!(!ops.contains("TreeExpr"), "{ops}");
    assert!(ops.contains("Special"), "{ops}");
    // ClearAll resets the global G to 0.
    assert_eq!(run(&src), 1);
}

#[test]
fn an_interface_call_with_computed_arguments_resolves_its_target_before_the_arguments_run() {
    let src = "interface IThing { procedure Get(N: Integer): Integer; } \
        codeunit 50101 Impl implements IThing { procedure Get(N: Integer): Integer begin exit(N + 1); end; } \
        codeunit 50100 C { var Calls: Integer; \
          procedure Next(): Integer begin Calls += 1; exit(Calls); end; \
          procedure F(): Integer var T: Interface IThing; U: Interface IThing; I: Integer; Impl: Codeunit Impl; R: Integer; \
          begin for I := 1 to 1 do begin T := Impl; R := T.Get(Next() * 2); asserterror R := U.Get(Next()); end; \
          exit(R * 100 + Calls); end; }";
    let ops = ops_of(src);
    assert!(!ops.contains("TreeExpr"), "{ops}");
    assert!(ops.contains("IfaceResolve"), "{ops}");
    // Get(1 * 2) = 3 -> R = 3; the call on the cleared variable raises before Next() runs, so Calls stays 1.
    assert_eq!(run(src), 301);
}

#[test]
fn a_routine_that_needs_more_registers_than_the_bytecode_can_name_is_refused_not_run_in_part() {
    // 65,600 locals: the executable form names a register with 16 bits. The call is refused with a reason, as an
    // unsupported feature is (hard rule 4); the routine does not run in part and nothing else is affected.
    let mut vars = String::new();
    for i in 0..65_600 {
        vars.push_str(&format!("v{i}: Integer; "));
    }
    let src = code(&format!(
        "procedure Big(): Integer var {vars} begin exit(1); end; procedure F(): Integer begin exit(7); end;"
    ));
    assert_eq!(run(&src), 7, "a routine of the same codeunit still runs");
    let err = al_testkit::run_with(&src, "C", "Big", vec![]).unwrap_err();
    match err {
        al_builtins::native::Signal::Unsupported(why) => assert!(why.contains("65,535 registers"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_for_loop_over_an_empty_range_leaves_the_counter_at_the_start() {
    // language/ControlFlow.al:ForLoop_VariableAfterLoop: 'empty to' reads 5, 'empty downto' reads 4.
    let src = code(
        "procedure F(): Text var I: Integer; B: BigInteger; N: Integer; begin N := 0; I := 1; for I := 5 to 4 do N += 1; \
         B := 9; for B := 4 downto 5 do N += 1; exit(Format(I) + '|' + Format(B) + '|' + Format(N)); end;",
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v)),
        "5|4|0"
    );
}

#[test]
fn a_textconst_without_an_enu_text_and_an_unpinned_order_is_refused() {
    // Without ENU only one language or DEU then FRA is pinned (textconst_languages.rs); FRA before DEU is not.
    let global = code("var G: TextConst FRA = 'Bonjour', DEU = 'Hallo'; procedure F(): Text begin exit(G); end;");
    let local = code("procedure F(): Text var L: TextConst FRA = 'Bonjour', DEU = 'Hallo'; begin exit(L); end;");
    for src in [global, local] {
        match al_testkit::run_with(&src, "C", "F", vec![]) {
            Err(al_builtins::native::Signal::Unsupported(why)) => {
                assert!(
                    why.contains("no ENU text, other than one text or DEU then FRA"),
                    "{why}"
                )
            }
            other => panic!("{other:?}"),
        }
    }
    // With an ENU text it reads as that text, wherever it stands in the list.
    let ok = code("var G: TextConst DEU = 'Hallo', ENU = 'Hello'; procedure F(): Text begin exit(G); end;");
    assert_eq!(
        al_testkit::text(al_testkit::run_with(&ok, "C", "F", vec![]).map(|(v, _)| v)),
        "Hello"
    );
}
