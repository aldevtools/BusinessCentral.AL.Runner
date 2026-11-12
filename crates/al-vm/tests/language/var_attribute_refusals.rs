//! Variable attributes the runtime does not implement are refused where the variable is used, never ignored
//! (hard rule 4): `[RunOnClient]` and `[WithEvents]` on DotNet variables (al-language-onprem-dotnet/dotnet/ac/AcDnAttributes.al:50,
//! :86, :145-146) and the read of `SecurityFiltering()` where the value is not pinned: on a global record declared with
//! `[SecurityFiltering(...)]` of an object that is no codeunit, table, page or report, on a global array of records
//! (al-language/reports/runtime/ACAttributes.al:176-191 pins locals, :427-480 a codeunit's globals;
//! al-language/reports/runtime/SFSecurityFilteringFlow.al:406-459 a table's, a page's and a report's globals and a local
//! array), and on a copy the corpus does not measure.

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

// A global WithEvents DotNet variable of a codeunit that declares no trigger for it runs as a plain DotNet variable, marked
// with assumption with-events-no-trigger (Microsoft's Sales and Inventory Forecast Tests, 139540 TestMissingApiUriOpenSetup).
// A trigger for the variable, and a page's WithEvents variable, keep the refusal.
#[test]
fn a_with_events_variable_no_trigger_handles_is_assumed_and_the_neighbours_stay_refused() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let src = format!(
        "{DOTNET} codeunit 50100 C {{ var [WithEvents] Tm: DotNet T; [WithEvents] Other: DotNet T; N: Integer; \
         trigger Other::Disposed(sender: Variant; e: DotNet A) begin N += 1; end; \
         procedure F(): Boolean begin exit(IsNull(Tm)); end; \
         procedure G(): Boolean begin exit(IsNull(Other)); end; }}"
    );
    take_hits();
    assert!(matches!(al_testkit::run_in(&src, "C", "F"), Ok(Value::Boolean(true))));
    assert!(take_hits().contains(&Assumption::WithEventsNoTrigger));
    refused(al_testkit::run_in(&src, "C", "G"), "WithEvents");
    let page = format!(
        "{DOTNET} page 50100 P {{ var [WithEvents] Tm: DotNet T; procedure F(): Boolean begin exit(IsNull(Tm)); end; }}"
    );
    refused(al_testkit::run_in(&page, "P", "F"), "WithEvents");
}

// A local record declared with `[SecurityFiltering(x)]` starts with x (al-language/reports/runtime/ACAttributes.al:176-179,
// :187-190; its counts are unchanged, :184-186). The value stays with the variable: Clear goes back to it (:373-374), Reset
// keeps it (:375-376), Copy keeps the target's (:385-387), a `var` parameter carries the argument's (:397-398), and a
// codeunit's global keeps its declared value too (:432, :451-458), and so does a table's global
// (al-language/reports/runtime/SFSecurityFilteringFlow.al:414, :420). A global array of records is not pinned: that read is
// refused.
#[test]
fn a_local_record_declared_with_security_filtering_starts_with_the_declared_value() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } } } \
        codeunit 50100 C { var [SecurityFiltering(SecurityFilter::Filtered)] GR: Record T; [SecurityFiltering(SecurityFilter::Filtered)] GA: array[2] of Record T; \
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
        procedure NA(): Text begin exit(Format(GA[1].SecurityFiltering())); end; \
        procedure A1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; begin exit(ReadArray(Rows)); end; \
        procedure A2(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; begin Rows[1].SecurityFiltering(SecurityFilter::Ignored); exit(ReadArray(Rows)); end; \
        procedure ReadArray(X: array[2] of Record T): Text begin exit(Format(X[1].SecurityFiltering())); end; \
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
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "N")), "Filtered");
    refused(al_testkit::run_in(src, "C", "NA"), "SecurityFiltering");
    // The whole array passed by value is not measured (SFSecurityFilteringFlow.al:429-459 reads its elements only directly;
    // ACAttributes.al:397, :411-414 pin a Record parameter, not an array parameter): refused, whatever its elements hold.
    refused(al_testkit::run_in(src, "C", "A1"), "SecurityFiltering");
    refused(al_testkit::run_in(src, "C", "A2"), "SecurityFiltering");
    // TransferFields and RecordRef.SetTable into the record keep its value, and Clear of a by-value copy goes back to the
    // argument's declared value (SFSecurityFilteringFlow.al, see the next test).
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M5")), "Filtered");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M6")), "Filtered");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M7")), "Ignored");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M8")), "Validated");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "M9")), "Ignored");
}

// What SecurityFiltering() reads after a TransferFields into the record, a RecordRef.SetTable into it, and on a by-value
// parameter, pinned on BC 27.5, 28.5 and 29.0 by the corpus probe al-language/reports/runtime/SFSecurityFilteringFlow.al (line
// numbers below are that file's); the corpus case runs the same shapes, this test keeps them apart, one routine per cell.
#[test]
fn security_filtering_through_transfer_fields_set_table_and_by_value_copies() {
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
    let cells: &[(&str, &str)] = &[
        ("T1", "Filtered"),  // :185-186, :202
        ("T2", "Filtered"),  // :187-188, :203
        ("T3", "Validated"), // :189-190, :204
        ("T4", "Filtered"),  // :191-193, :205
        ("T5", "Ignored"),   // :194-196, :206
        ("T6", "Filtered"),  // :197-198, :207 (Clear goes back to the declared value: ACAttributes.al:388-396)
        ("T7", "Ignored"),   // :199-201, :208
        ("S1", "Filtered"),  // :236-238, :261
        ("S2", "Validated"), // :239-240, :262
        ("S3", "Validated"), // :242-245, :263
        ("S4", "Filtered"),  // :246-250, :265
        ("S5", "Filtered"),  // :251-252, :266
        ("R1", "Validated"), // :223-224, :256
        ("R2", "Validated"), // :226-227, :257
        // Find Record Management.FindLastEntryIgnoringSecurityFilter reads it on a RecordRef taken through a Variant from
        // Sales-Post's `[SecurityFiltering(SecurityFilter::Ignored)] GLEntry` (GLEntry.GetLastEntryNo in LockTables).
        ("R3", "Validated"),          // :228-229, :258
        ("P1", "Filtered/Filtered"),  // :279-280, :294-295
        ("P3", "Validated"),          // :283-284, :298 (Clear of a copy of a plain record: Validated either way)
        ("P4", "Filtered/Ignored"),   // :286-288, :300-301
        ("P6", "Filtered/Ignored"),   // :289-290, :302-303
        ("P8", "Filtered/Filtered"),  // :291-292, :304-305 (a var parameter is the caller's variable)
        ("P9", "Filtered"),           // :293, :306
        ("P10", "Filtered/Filtered"), // :341-349, :372-373
    ];
    let mut wrong = Vec::new();
    for (routine, want) in cells {
        let got = match b.run_in("C", routine) {
            Ok(Value::Text(s)) => s.to_string(),
            other => format!("{other:?}"),
        };
        if got.as_str() != *want {
            wrong.push(format!("{routine}: got {got}, want {want}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// A by-value copy of a copy the corpus does not measure (here an event argument of a record declared Filtered) goes back to the
// declared value, which came with the unmeasured copy: what it reads is not pinned, as after Clear on that copy (`Clear` and a
// by-value copy both go back to the declared value: ACAttributes.al:388-397). Both are refused.
#[test]
fn a_by_value_copy_of_an_unmeasured_copy_is_refused_like_clear() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } } } \
        codeunit 50101 Pub { [IntegrationEvent(false, false)] procedure OnRow(Row: Record T; var Result: Text) begin end; } \
        codeunit 50102 Sub { EventSubscriberInstance = StaticAutomatic; \
        [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnRow', '', false, false)] \
        local procedure OnRow(Row: Record T; var Result: Text) begin \
            Row.SecurityFiltering(SecurityFilter::Ignored); \
            if Result = 'clear' then begin Clear(Row); Result := Format(Row.SecurityFiltering()); end else Result := ReadByValue(Row); end; \
        local procedure ReadByValue(X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; } \
        codeunit 50100 C { \
        procedure ByValue(): Text var Pub: Codeunit Pub; [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; Result: Text; begin Pub.OnRow(R, Result); exit(Result); end; \
        procedure Cleared(): Text var Pub: Codeunit Pub; [SecurityFiltering(SecurityFilter::Filtered)] R: Record T; Result: Text; begin Result := 'clear'; Pub.OnRow(R, Result); exit(Result); end; }";
    refused(al_testkit::run_in(src, "C", "Cleared"), "SecurityFiltering");
    refused(al_testkit::run_in(src, "C", "ByValue"), "SecurityFiltering");
}

// Array shapes the probe al-language/reports/runtime/SFSecurityFilteringFlow.al:429-459 does not ask (it reads the elements of a
// local array declared with the attribute directly, after SecurityFiltering(x), Clear of an element and Clear of the array): an
// element passed by value whose value is not the declared one, an array assignment in either direction, CopyArray in either
// direction, the whole array passed by value, and an element of a global array (whose declared value the runtime does not keep)
// passed through a var parameter. What SecurityFiltering() reads there is not pinned: each read is refused.
#[test]
fn security_filtering_of_array_shapes_outside_the_probe_is_refused() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } } } \
        codeunit 50100 C { var [SecurityFiltering(SecurityFilter::Filtered)] GA: array[2] of Record T; \
        procedure E1(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; begin Rows[1].SecurityFiltering(SecurityFilter::Ignored); exit(ReadByValue(Rows[1])); end; \
        procedure E2(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; Plain: array[2] of Record T; begin Plain := Rows; exit(Format(Plain[1].SecurityFiltering())); end; \
        procedure E3(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; Plain: array[2] of Record T; begin CopyArray(Plain, Rows, 1); exit(Format(Plain[1].SecurityFiltering())); end; \
        procedure E4(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; Plain: array[2] of Record T; begin Rows := Plain; exit(Format(Rows[1].SecurityFiltering())); end; \
        procedure E5(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; Plain: array[2] of Record T; begin CopyArray(Rows, Plain, 1); exit(Format(Rows[1].SecurityFiltering())); end; \
        procedure E6(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; begin exit(ReadArray(Rows)); end; \
        procedure E7(): Text begin exit(ReadVar(GA[1])); end; \
        procedure E8(): Text var [SecurityFiltering(SecurityFilter::Filtered)] Rows: array[2] of Record T; Plain: array[2] of Record T; begin Plain := Rows; Clear(Plain); exit(Format(Plain[1].SecurityFiltering())); end; \
        procedure ReadArray(X: array[2] of Record T): Text begin exit(Format(X[1].SecurityFiltering())); end; \
        procedure ReadByValue(X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; \
        procedure ReadVar(var X: Record T): Text begin exit(Format(X.SecurityFiltering())); end; }";
    let mut b = al_testkit::Built::from_source(src);
    let mut answered = Vec::new();
    for p in ["E1", "E2", "E3", "E4", "E5", "E6", "E7", "E8"] {
        match b.run_in("C", p) {
            Err(Signal::Unsupported(why)) if why.contains("SecurityFiltering") => {}
            other => answered.push(format!("{p}: {other:?}")),
        }
    }
    assert!(answered.is_empty(), "not refused:\n{}", answered.join("\n"));
}
