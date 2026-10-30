//! `TestHandlerContext` and `DataSourceContext` (BC 29) on a variable that no test handler or data source was handed:
//! the values of lt-ac/ACContexts.al (codeunits 77506, 77541 to 77546), and the refusal of what it does not pin.

use al_testkit::{run, text};

fn with(decl: &str, body: &str) -> String {
    format!(
        "codeunit 50101 \"Ctx Probe\"\n{{\n    procedure Probe(): Text\n    var\n        Ctx: {decl};\n        S: Text;\n    begin\n{body}\n        exit(S);\n    end;\n\n    [TryFunction]\n    procedure TryCodeunitId(var Ctx: DataSourceContext; var S: Text)\n    begin\n        S := Format(Ctx.CodeunitId());\n    end;\n}}\n"
    )
}

// ACContexts.al:47-48 (CodeunitId 0), :68 (CodeunitName ''), :89 (ProcedureName ''), :110 (TestCaseName ''),
// :131 (Success No), :151-155 and :167 (Skip returns).
#[test]
fn test_handler_context_members_on_a_fresh_variable() {
    let got = text(run(
        &with(
            "TestHandlerContext",
            "        Ctx.Skip('probe');\n        S := Format(Ctx.CodeunitId()) + '|' + Ctx.CodeunitName() + '|' + Ctx.ProcedureName() + '|' + Ctx.TestCaseName() + '|' + Format(Ctx.Success());",
        ),
        "Probe",
    ));
    assert_eq!(got, "0||||No");
}

// ACContexts.al:22-25: CodeunitId on a fresh DataSourceContext raises this TypeConversion error.
#[test]
fn data_source_context_codeunit_id_on_a_fresh_variable_raises_the_pinned_error() {
    let got = text(run(
        &with(
            "DataSourceContext",
            "        if not TryCodeunitId(Ctx, S) then\n            S := GetLastErrorText() + '|' + GetLastErrorCode();",
        ),
        "Probe",
    ));
    assert_eq!(
        got,
        "The expression System.Func`1[System.Int32] cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavValue value.|TypeConversion"
    );
}

// ACContexts.al:8: AppId on a fresh DataSourceContext is not probed (a test that calls it ends the session).
#[test]
fn data_source_context_app_id_is_refused() {
    let r = run(&with("DataSourceContext", "        S := Format(Ctx.AppId());"), "Probe");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
