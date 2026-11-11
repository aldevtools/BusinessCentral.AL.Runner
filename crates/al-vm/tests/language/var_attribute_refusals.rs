//! Variable attributes the runtime does not implement are refused where the variable is used, never ignored
//! (hard rule 4): `[RunOnClient]` and `[WithEvents]` on DotNet variables (al-language-onprem-dotnet/dotnet/ac/AcDnAttributes.al:50,
//! :86, :145-146) and the read of `SecurityFiltering()` where the value is not pinned: on a global record of an object that
//! is no codeunit declared with `[SecurityFiltering(...)]` (al-language/reports/runtime/ACAttributes.al:176-191 pins locals,
//! :427-480 a codeunit's globals).

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
// codeunit's global keeps its declared value too (:432, :451-458). The attribute on a global of a table is not pinned: that
// read is refused.
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
    refused(al_testkit::run_in(src, "C", "N"), "SecurityFiltering");
    // TransferFields and RecordRef.SetTable into the record keep its value, and Clear of a by-value copy goes back to the
    // argument's declared value: measured by the queued probe (see the next test), flagged until it lands.
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M5")), "Filtered");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M6")), "Filtered");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M7")), "Ignored");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M8")), "Validated");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M9")), "Ignored");
}

// What SecurityFiltering() reads after a TransferFields into the record, a RecordRef.SetTable into it, and on a by-value
// parameter, measured on BC 27.5, 28.5 and 29.0 by the queued corpus probe al-language/reports/runtime/SFSecurityFilteringFlow.al
// (corpus PR 1310, not merged yet; line numbers below are that file's). Until it lands each such read marks the assumption
// `security-filtering-flow` (a flagged pass). Each cell: the routine, the value BC reads, and whether the read is flagged.
#[test]
fn security_filtering_through_transfer_fields_set_table_and_by_value_copies() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let src = "table 50100 T { fields { field(1; Id; Integer) { } } } \
        codeunit 50100 C { \
        procedure T1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; P: Record T; begin F.TransferFields(P); exit(Format(F.SecurityFiltering())); end; \
        procedure T2(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; begin F.TransferFields(I); exit(Format(F.SecurityFiltering())); end; \
        procedure T3(): Text var P: Record T; [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; begin P.TransferFields(I); exit(Format(P.SecurityFiltering())); end; \
        procedure T4(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; P: Record T; begin P.SecurityFiltering(SecurityFilter::Disallowed); F.TransferFields(P); exit(Format(F.SecurityFiltering())); end; \
        procedure T5(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; P: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); F.TransferFields(P); exit(Format(F.SecurityFiltering())); end; \
        procedure T6(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; P: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); F.TransferFields(P); Clear(F); exit(Format(F.SecurityFiltering())); end; \
        procedure T7(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; P: Record T; begin P.SecurityFiltering(SecurityFilter::Ignored); P.TransferFields(F, false); exit(Format(P.SecurityFiltering())); end; \
        procedure S1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; RR: RecordRef; begin RR.GetTable(I); RR.SetTable(F); exit(Format(F.SecurityFiltering())); end; \
        procedure S2(): Text var P: Record T; [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; RR: RecordRef; begin RR.GetTable(I); RR.SetTable(P); exit(Format(P.SecurityFiltering())); end; \
        procedure S3(): Text var P: Record T; RR: RecordRef; begin RR.GetTable(P); RR.SecurityFiltering(SecurityFilter::Disallowed); RR.SetTable(P); exit(Format(P.SecurityFiltering())); end; \
        procedure S4(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; RR: RecordRef; begin RR.GetTable(F); RR.SecurityFiltering(SecurityFilter::Ignored); RR.SetTable(F); exit(Format(F.SecurityFiltering())); end; \
        procedure S5(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; RR: RecordRef; begin RR.GetTable(F); RR.SecurityFiltering(SecurityFilter::Ignored); RR.SetTable(F); Clear(F); exit(Format(F.SecurityFiltering())); end; \
        procedure R1(): Text var RR: RecordRef; begin RR.Open(50100); exit(Format(RR.SecurityFiltering())); end; \
        procedure R2(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; RR: RecordRef; begin RR.GetTable(F); exit(Format(RR.SecurityFiltering())); end; \
        procedure R3(): Text var [SecurityFiltering(SecurityFilter::Ignored)] I: Record T; begin exit(ViaVariant(I)); end; \
        procedure ViaVariant(V: Variant): Text var RR: RecordRef; begin RR.GetTable(V); exit(Format(RR.SecurityFiltering())); end; \
        procedure P1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin exit(ClearByValue(F) + '/' + Format(F.SecurityFiltering())); end; \
        procedure P3(): Text var P: Record T; begin P.SecurityFiltering(SecurityFilter::Ignored); exit(ClearByValue(P)); end; \
        procedure P4(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); exit(ReadByValue(F) + '/' + ReadVar(F)); end; \
        procedure P6(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); exit(ClearByValue(F) + '/' + Format(F.SecurityFiltering())); end; \
        procedure P8(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); exit(ClearVar(F) + '/' + Format(F.SecurityFiltering())); end; \
        procedure P9(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin exit(NestedClearByValue(F)); end; \
        procedure P10(): Text var [SecurityFiltering(SecurityFilter::Filtered)] F: Record T; begin F.SecurityFiltering(SecurityFilter::Ignored); exit(ReadTemporary(F) + '/' + ClearTemporary(F)); end; \
        procedure ReadByValue(X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; \
        procedure ReadVar(var X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; \
        procedure ClearByValue(X: Record T): Text begin Clear(X); exit(Format(X.SecurityFiltering())); end; \
        procedure ClearVar(var X: Record T): Text begin Clear(X); exit(Format(X.SecurityFiltering())); end; \
        procedure NestedClearByValue(X: Record T): Text begin exit(ClearByValue(X)); end; \
        procedure ReadTemporary(X: Record T temporary): Text begin exit(Format(X.SecurityFiltering())); end; \
        procedure ClearTemporary(X: Record T temporary): Text begin Clear(X); exit(Format(X.SecurityFiltering())); end; }";
    let mut b = al_testkit::Built::from_source(src);
    let cells: &[(&str, &str, bool)] = &[
        ("T1", "Filtered", true),   // :185-186, :202
        ("T2", "Filtered", true),   // :187-188, :203
        ("T3", "Validated", true),  // :189-190, :204
        ("T4", "Filtered", true),   // :191-193, :205
        ("T5", "Ignored", true),    // :194-196, :206
        ("T6", "Filtered", false),  // :197-198, :207 (Clear goes back to the declared value: ACAttributes.al:388-396)
        ("T7", "Ignored", true),    // :199-201, :208
        ("S1", "Filtered", true),   // :236-238, :261
        ("S2", "Validated", true),  // :239-240, :262
        ("S3", "Validated", true),  // :242-245, :263
        ("S4", "Filtered", true),   // :246-250, :265
        ("S5", "Filtered", false),  // :251-252, :266
        ("R1", "Validated", false), // :223-224, :256
        ("R2", "Validated", true),  // :226-227, :257
        // Find Record Management.FindLastEntryIgnoringSecurityFilter reads it on a RecordRef taken through a Variant from
        // Sales-Post's `[SecurityFiltering(SecurityFilter::Ignored)] GLEntry` (GLEntry.GetLastEntryNo in LockTables).
        ("R3", "Validated", true),          // :228-229, :258
        ("P1", "Filtered/Filtered", true),  // :279-280, :294-295
        ("P3", "Validated", false),         // :283-284, :298 (Clear of a copy of a plain record: Validated either way)
        ("P4", "Filtered/Ignored", true),   // :286-288, :300-301
        ("P6", "Filtered/Ignored", true),   // :289-290, :302-303
        ("P8", "Filtered/Filtered", false), // :291-292, :304-305 (a var parameter is the caller's variable)
        ("P9", "Filtered", true),           // :293, :306
        ("P10", "Filtered/Filtered", true), // :341-349, :372-373
    ];
    let mut wrong = Vec::new();
    for (routine, want, flagged) in cells {
        take_hits();
        let got = match b.run_in("C", routine) {
            Ok(Value::Text(s)) => s.to_string(),
            other => format!("{other:?}"),
        };
        let hit = take_hits().contains(&Assumption::SecurityFilteringFlow);
        if (got.as_str(), hit) != (*want, *flagged) {
            wrong.push(format!(
                "{routine}: got {got} flagged={hit}, want {want} flagged={flagged}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
