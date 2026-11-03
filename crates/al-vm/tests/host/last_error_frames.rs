//! `Exception.ToString()` of the exception `GetLastErrorObject` answers after an asserterror or in a TryFunction: every line the
//! corpus pins (tests/al-language-onprem-dotnet/lt-y89-tostring-frames/Y89ToStringFrames.al, BC 27, 28, 29; the ValueTask frame
//! from BC 29).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
dotnet { assembly("mscorlib") { type("System.Exception"; FrExc) { } } }
codeunit 50201 "FR Thrower"
{
    procedure Level1() begin Level2(); end;
    local procedure Level2() begin Error('deep'); end;
    procedure Divide() var Zero: Integer; Result: Integer; begin Result := 1 div Zero; end;
}
codeunit 50200 C
{
    [TryFunction] local procedure TryDeep() var T: Codeunit "FR Thrower"; begin T.Level1(); end;
    [TryFunction] local procedure TryDivide() var T: Codeunit "FR Thrower"; begin T.Divide(); end;
    local procedure Trace(): Text var Exc: DotNet FrExc; begin Exc := GetLastErrorObject(); exit(Exc.ToString()); end;
    procedure Plain(): Text begin asserterror Error('plain'); exit(Trace()); end;
    procedure AssertDeep(): Text var T: Codeunit "FR Thrower"; begin asserterror T.Level1(); exit(Trace()); end;
    procedure AssertDivide(): Text var T: Codeunit "FR Thrower"; begin asserterror T.Divide(); exit(Trace()); end;
    procedure TryDeepText(): Text begin if TryDeep() then; exit(Trace()); end;
    procedure TryDivideText(): Text begin if TryDivide() then; exit(Trace()); end;
}
"#;

fn run_on(bc: &str, routine: &str) -> String {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
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
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![]).map(|(v, _)| v) {
        // The corpus counts the lines with `b__` and does not show them (Y89ToStringFrames.al `generated`): one in every trace.
        Ok(Value::Text(t)) => {
            let (generated, kept): (Vec<&str>, Vec<&str>) = t.split('\n').partition(|l| l.contains("b__"));
            assert_eq!(generated.len(), 1, "{t}");
            kept.join("\n")
        }
        Ok(other) => panic!("expected a text, got {other:?}"),
        Err(Signal::Unsupported(why)) => format!("unsupported: {why}"),
        Err(other) => panic!("{other:?}"),
    }
}

const NS: &str = "Microsoft.Dynamics.Nav";
const VT: &str =
    "   at Microsoft.Dynamics.Nav.Types.ValueTaskExtensions.ManualResetValueTaskSource`1.GetResult(Int16 token)";
const END: &str = "--- End of stack trace from previous location ---";
const ASSERT: &str = "   at Microsoft.Dynamics.Nav.Runtime.NavMethodScope.AssertErrorAsync(Func`1 body)";
const TRY: &str =
    "   at Microsoft.Dynamics.Nav.Runtime.NavApplicationObjectBase.TryInvokeAsync(NavSession session, Func`1 method)";

fn frame(cu: u32, name: &str) -> String {
    format!("   at {NS}.BusinessApplication.Codeunit{cu}.{name}()")
}

fn lines(parts: &[&str]) -> String {
    parts.join("\n")
}

fn dialog(msg: &str) -> String {
    format!("{NS}.Types.Exceptions.NavNCLDialogException: {msg}")
}

fn divide() -> [String; 3] {
    [
        format!("{NS}.Types.Exceptions.NavALException: Attempted to divide by zero."),
        " ---> System.DivideByZeroException: Attempted to divide by zero.".into(),
        frame(50201, "Divide"),
    ]
}

#[test]
fn asserterror_of_a_plain_error() {
    // Y89_Asserterror_Plain.
    for bc in ["27.5", "29.0"] {
        assert_eq!(run_on(bc, "Plain"), lines(&[&dialog("plain"), END, ASSERT]), "{bc}");
    }
}

#[test]
fn asserterror_of_an_error_two_procedures_deep() {
    // Y89_Asserterror_Deep.
    let (l2, l1) = (frame(50201, "Level2"), frame(50201, "Level1"));
    assert_eq!(
        run_on("27.5", "AssertDeep"),
        lines(&[&dialog("deep"), &l2, &l1, END, ASSERT])
    );
    assert_eq!(
        run_on("29.0", "AssertDeep"),
        lines(&[&dialog("deep"), &l2, &l1, VT, END, ASSERT])
    );
}

#[test]
fn asserterror_of_a_division() {
    // Y89_Asserterror_Divide.
    let [a, b, c] = divide();
    let inner_end = "   --- End of inner exception stack trace ---";
    assert_eq!(
        run_on("28.5", "AssertDivide"),
        lines(&[&a, &b, &c, inner_end, &frame(50201, "Divide"), END, ASSERT])
    );
    assert_eq!(
        run_on("29.0", "AssertDivide"),
        lines(&[&a, &b, &c, inner_end, &frame(50201, "Divide"), VT, END, ASSERT])
    );
}

#[test]
fn try_function_of_an_error_two_procedures_deep() {
    // Y89_Try_Deep.
    let (l2, l1, t) = (frame(50201, "Level2"), frame(50201, "Level1"), frame(50200, "TryDeep"));
    assert_eq!(
        run_on("27.5", "TryDeepText"),
        lines(&[&dialog("deep"), &l2, &l1, &t, TRY])
    );
    assert_eq!(
        run_on("29.0", "TryDeepText"),
        lines(&[&dialog("deep"), &l2, &l1, VT, &t, TRY])
    );
}

#[test]
fn try_function_of_a_division() {
    // Y89_Try_Divide.
    let [a, b, c] = divide();
    let inner_end = "   --- End of inner exception stack trace ---";
    let t = frame(50200, "TryDivide");
    assert_eq!(
        run_on("28.5", "TryDivideText"),
        lines(&[&a, &b, &c, inner_end, &frame(50201, "Divide"), &t, TRY])
    );
    assert_eq!(
        run_on("29.0", "TryDivideText"),
        lines(&[&a, &b, &c, inner_end, &frame(50201, "Divide"), VT, &t, TRY])
    );
}

/// The generated line is the one thing of these traces no corpus test shows: the runtime marks it as an assumption.
#[test]
fn the_pinned_traces_mark_the_generated_line_as_an_assumption() {
    use al_builtins::assumptions::{Assumption, take_hits};
    for routine in ["Plain", "AssertDeep", "AssertDivide", "TryDeepText", "TryDivideText"] {
        take_hits();
        let _ = run_on("29.0", routine);
        let hits = take_hits();
        assert!(hits.contains(&Assumption::ExceptionToStringGeneratedFrame), "{routine}");
        assert!(!hits.contains(&Assumption::ExceptionToStringStackFrames), "{routine}");
    }
}
