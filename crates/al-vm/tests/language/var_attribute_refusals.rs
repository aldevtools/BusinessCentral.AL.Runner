//! Variable attributes the runtime does not implement are refused where the variable is used, never ignored
//! (hard rule 4): `[RunOnClient]` and `[WithEvents]` on DotNet variables (al-language-onprem-dotnet/dotnet/ac/AcDnAttributes.al:50,
//! :86, :145-146) and the read of `SecurityFiltering()` where the value is not pinned: on a global record of an object that
//! is no codeunit declared with `[SecurityFiltering(...)]` (al-language/reports/runtime/ACAttributes.al:176-191 pins locals,
//! :427-480 a codeunit's globals), and after a TransferFields into the record.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        // The constructor of a [RunOnClient] variable raises the pinned error, not a refusal
        // (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:122-150, dotnet/ac/AcDnAttributes.al:131-147).
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

// A local record declared with `[SecurityFiltering(x)]` starts with x (al-language/reports/runtime/ACAttributes.al:176-179,
// :187-190; its counts are unchanged, :184-186). The value stays with the variable: Clear goes back to it (:373-374), Reset
// keeps it (:375-376), Copy keeps the target's (:385-387), a `var` parameter carries the argument's (:397-398), and a
// codeunit's global keeps its declared value too (:432, :451-458). What TransferFields leaves is not pinned, nor is the
// attribute on a global of a table: those reads are refused.
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
        procedure N(): Text var U: Record U; begin exit(U.L2()); end; \
        procedure M6(): Text var [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; RR: RecordRef; begin RR.Open(50100); RR.SetTable(R); exit(Format(R.SecurityFiltering())); end; \
        procedure M7(): Text var [SecurityFiltering(SecurityFilter::Ignored)] R: Record T; begin exit(ClearByValue(R)); end; \
        procedure M8(): Text var P: Record T; begin exit(ClearByValue(P)); end; \
        procedure M9(): Text var [SecurityFiltering(SecurityFilter::Ignored)] R: Record T; begin exit(ReadByValue(R)); end; \
        procedure ClearByValue(X: Record T): Text begin Clear(X); exit(Format(X.SecurityFiltering())); end; \
        procedure ReadByValue(X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; \
        procedure Pass(var X: Record T) begin end; \
        procedure Two(N: Integer; var X: Record T) begin end; } \
        table 50101 U { fields { field(1; Id; Integer) { } } var [SecurityFiltering(SecurityFilter::Filtered)] GU: Record T; \
        procedure L2(): Text begin exit(Format(GU.SecurityFiltering())); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "FilteredIgnoredValidated"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "G")), "Validated");
    assert!(matches!(al_testkit::run_in(src, "C", "H"), Ok(Value::Integer(0))));
    for p in ["K", "L", "M1", "M2", "M3", "M4"] {
        assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", p)), "Filtered", "{p}");
    }
    refused(al_testkit::run_in(src, "C", "M5"), "TransferFields");
    refused(al_testkit::run_in(src, "C", "N"), "SecurityFiltering");
    // RecordRef.SetTable into the record: whether SecurityFiltering passes is not pinned (ReadIsolation does,
    // recordref-fieldref/recordref/P92ReadIsolation.al:ReadIsolation_FromRecordAndToRecord), so the read refuses.
    refused(al_testkit::run_in(src, "C", "M6"), "SetTable");
    // A by-value parameter reads the argument's value (ACAttributes.al:397, :411-414); Clear of it goes back to a value
    // the corpus does not pin when the argument was declared with another value than Validated, so the read refuses.
    refused(al_testkit::run_in(src, "C", "M7"), "Clear");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M8")), "Validated");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M9")), "Ignored");
}
