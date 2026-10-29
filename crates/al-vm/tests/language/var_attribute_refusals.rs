//! Variable attributes the runtime does not implement are refused where the variable is used, never ignored
//! (hard rule 4): `[RunOnClient]` and `[WithEvents]` on DotNet variables (al-language-onprem-dotnet/lt-ac/AcDnAttributes.al:50,
//! :86, :145-146) and the read of `SecurityFiltering()` on a global record declared with `[SecurityFiltering(...)]`
//! (al-language/lt-ac/ACAttributes.al:176-191).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        // The constructor of a [RunOnClient] variable raises the pinned error, not a refusal
        // (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:122-150, lt-ac/AcDnAttributes.al:131-147).
        Err(Signal::Error(e)) if needle == "RunOnClient" => {
            assert!(e.code.contains("DotNetFromAlNotSupported"), "{e:?}")
        }
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

const DOTNET: &str = "dotnet { assembly(\"mscorlib\") { type(\"System.Text.StringBuilder\"; B) { } type(\"System.EventArgs\"; A) { } } assembly(\"System\") { type(\"System.Timers.Timer\"; T) { } } }";

#[test]
fn a_global_run_on_client_variable_is_refused_where_it_is_used() {
    let src = format!(
        "{DOTNET} codeunit 50100 C {{ var [RunOnClient] Cb: DotNet B; procedure F(): Integer begin Cb := Cb.StringBuilder(); exit(1); end; procedure G(): Integer begin exit(2); end; procedure K(Use: Boolean): Integer begin if Use then Cb := Cb.StringBuilder(); exit(3); end; }}"
    );
    refused(al_testkit::run_in(&src, "C", "F"), "RunOnClient");
    assert!(matches!(al_testkit::run_in(&src, "C", "G"), Ok(Value::Integer(2))));
    // The statement that uses the variable refuses when it runs; the rest of its routine runs (Base Application declares
    // such variables in routines whose other paths tests reach).
    let r = al_testkit::run_with(&src, "C", "K", vec![Value::Boolean(false)]).map(|(v, _)| v);
    assert!(matches!(r, Ok(Value::Integer(3))), "{r:?}");
    let r = al_testkit::run_with(&src, "C", "K", vec![Value::Boolean(true)]).map(|(v, _)| v);
    refused(r, "RunOnClient");
}

#[test]
fn a_local_run_on_client_variable_is_refused() {
    let src = format!(
        "{DOTNET} codeunit 50100 C {{ procedure F(): Integer var [RunOnClient] Cb: DotNet B; begin Cb := Cb.StringBuilder(); exit(1); end; }}"
    );
    refused(al_testkit::run_in(&src, "C", "F"), "RunOnClient");
}

#[test]
fn a_with_events_variable_is_refused_where_it_is_used() {
    let src = format!(
        "{DOTNET} codeunit 50100 C {{ var [WithEvents] Tm: DotNet T; N: Integer; trigger Tm::Disposed(sender: Variant; e: DotNet A) begin N += 1; end; procedure F(): Integer begin Tm := Tm.Timer(); Tm.Dispose(); exit(N); end; }}"
    );
    refused(al_testkit::run_in(&src, "C", "F"), "WithEvents");
}

// A local record declared with `[SecurityFiltering(x)]` starts with x (al-language/lt-ac/ACAttributes.al:176-179,
// :187-190; its counts are unchanged, :184-186). What `Clear` or an assignment leaves on such a variable is not
// pinned, nor is the attribute on a global: those are refused.
#[test]
fn a_local_record_declared_with_security_filtering_starts_with_the_declared_value() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } } } \
        codeunit 50100 C { var [SecurityFiltering(SecurityFilter::Filtered)] GR: Record T; \
        procedure F(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; [SecurityFiltering(SecurityFilter::Validated)] V: Record T; begin exit(Format(R.SecurityFiltering()) + Format(I.SecurityFiltering()) + Format(V.SecurityFiltering())); end; \
        procedure G(): Text var P: Record T; begin exit(Format(P.SecurityFiltering())); end; \
        procedure H(): Integer var [SecurityFiltering(SecurityFilter::Ignored)] R: Record T; begin exit(R.Count()); end; \
        procedure K(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; begin Clear(R); exit(Format(R.SecurityFiltering())); end; \
        procedure L(): Text begin exit(Format(GR.SecurityFiltering())); end; \
        procedure M1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; begin Pass(R); exit(Format(R.SecurityFiltering())); end; \
        procedure M2(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; begin Two(1, R); exit(Format(R.SecurityFiltering())); end; \
        procedure M3(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; begin R.Reset(); exit(Format(R.SecurityFiltering())); end; \
        procedure M4(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; P: Record T; begin R.Copy(P); exit(Format(R.SecurityFiltering())); end; \
        procedure M5(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; begin R.TransferFields(R); exit(Format(R.SecurityFiltering())); end; \
        procedure Pass(var X: Record T) begin end; \
        procedure Two(N: Integer; var X: Record T) begin end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "FilteredIgnoredValidated"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "G")), "Validated");
    assert!(matches!(al_testkit::run_in(src, "C", "H"), Ok(Value::Integer(0))));
    // Cleared: the read stays refused, as on main.
    refused(al_testkit::run_in(src, "C", "K"), "SecurityFiltering");
    refused(al_testkit::run_in(src, "C", "L"), "SecurityFiltering");
    // Passed to a call, or the receiver or source of Reset, Copy or TransferFields: what the record keeps is not
    // pinned, so the read stays refused.
    for p in ["M1", "M2", "M3", "M4", "M5"] {
        refused(al_testkit::run_in(src, "C", p), "SecurityFiltering");
    }
}
