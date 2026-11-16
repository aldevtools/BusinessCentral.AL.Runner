//! A subscriber of AutoFormatTranslate (UI Helper Triggers) outside Microsoft's apps that can set its result changes how BC shows a
//! value of a pinned AutoFormatType (1, 2, 4, 5, 10, 11); the runtime raises no such event, so the format is refused (al-runner
//! issue 8011, review of PR 8015, the reviewer's afprobe6). Outside a test run every subscriber counts as one the test can see.
//! A subscriber that never writes its result, an unbound manual one, and type 0 keep main's text.

/// A page, a report and a codeunit over a type 1 field and a type 0 field; `SUB` is the subscriber codeunit.
fn src(sub: &str) -> String {
    format!(
        r#"
table 69480 "AfW Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; T1; Decimal) {{ AutoFormatType = 1; AutoFormatExpression = ''; }} field(3; T0; Decimal) {{ AutoFormatType = 0; }} field(4; B; Blob) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
page 69481 "AfW Card"
{{
    PageType = Card; SourceTable = "AfW Row"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ field(T1; Rec.T1) {{ }} field(T10; Rec.T1) {{ AutoFormatType = 10; AutoFormatExpression = '<Precision,3:3><Standard Format,0>'; }} field(T0; Rec.T0) {{ }} }} }}
}}
report 69482 "AfW Rep"
{{
    UsageCategory = None;
    dataset {{ dataitem(R; "AfW Row") {{ column(ColT1; T1) {{ }} }} }}
}}
{sub}
codeunit 69484 C
{{
    local procedure Seed()
    var R: Record "AfW Row";
    begin
        R.Id := 1;
        R.T1 := 3456.67843;
        R.T0 := 3456.67843;
        R.Insert();
    end;

    procedure PageT1(): Text
    var P: TestPage "AfW Card";
    begin
        Seed();
        P.OpenView();
        exit(P.T1.Value());
    end;

    procedure PageT10(): Text
    var P: TestPage "AfW Card";
    begin
        Seed();
        P.OpenView();
        exit(P.T10.Value());
    end;

    procedure PageT0(): Text
    var P: TestPage "AfW Card";
    begin
        Seed();
        P.OpenView();
        exit(P.T0.Value());
    end;

    procedure CodeT1(): Text
    var R: Record "AfW Row";
    begin
        R.T1 := 3456.67843;
        exit(Format(R.T1));
    end;

    procedure SubstT1(): Text
    var R: Record "AfW Row";
    begin
        R.T1 := 3456.67843;
        exit(StrSubstNo('%1', R.T1));
    end;

    procedure ReportT1(): Text
    var
        Tmp: Record "AfW Row" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Cols: Text;
    begin
        Seed();
        Tmp.B.CreateOutStream(OutS);
        Report.SaveAs(Report::"AfW Rep", '', ReportFormat::Xml, OutS);
        Tmp.B.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('<Column ') then
                Cols += Line.Trim();
        end;
        exit(Cols);
    end;

    procedure BoundPageT1(): Text
    var
        S: Codeunit "AfW Sub";
        P: TestPage "AfW Card";
    begin
        BindSubscription(S);
        Seed();
        P.OpenView();
        exit(P.T1.Value());
    end;
}}
"#
    )
}

/// The reviewer's afprobe6 subscriber: it sets Translation for types 1 and 10.
const WRITER: &str = r#"
codeunit 69483 "AfW Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure OneDecimal(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        if AutoFormatType in [1, 10] then
            Translation := '<Precision,1:1><Standard Format,0>';
    end;
}
"#;

fn refused(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) {
    let e = format!("{r:?}");
    assert!(
        e.contains("Unsupported") && e.contains("AfW Sub") && e.contains("AutoFormatTranslate"),
        "{e}"
    );
}

#[test]
fn a_writer_refuses_a_pinned_type_on_a_page_in_code_and_in_a_report_column() {
    let s = src(WRITER);
    for routine in ["PageT1", "PageT10", "CodeT1", "SubstT1", "ReportT1"] {
        refused(al_testkit::run_in(&s, "C", routine));
    }
}

/// Type 0 has no new refusal: Format of a plain field raises nothing (events/platform-triggers/P114GlobalTriggerEvents.al:341-347).
#[test]
fn a_writer_leaves_type_0_as_main_shows_it() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src(WRITER), "C", "PageT0")),
        "3,456.68"
    );
}

/// A subscriber that only reads its arguments (the corpus fixtures events/platform-triggers/P114GlobalTriggerEvents.al:104-109,
/// P118SysEvents.al:163-168 log and never write Translation) keeps main's text.
#[test]
fn a_subscriber_that_never_writes_its_result_keeps_main_text() {
    let reader = r#"
codeunit 69483 "AfW Sub"
{
    var Seen: Text;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure Log(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        Seen += Format(AutoFormatType) + AutoFormatExpr + Translation;
    end;
}
"#;
    let s = src(reader);
    let got: Vec<String> = ["PageT1", "PageT10", "CodeT1", "SubstT1"]
        .iter()
        .map(|r| al_testkit::text(al_testkit::run_in(&s, "C", r)))
        .collect();
    assert_eq!(got, ["3,456.68", "3,456.678", "3,456.68", "3,456.68"]);
}

/// A writer that hands its result on by `var` is a writer too.
#[test]
fn a_subscriber_that_passes_its_result_by_var_is_a_writer() {
    let by_var = r#"
codeunit 69483 "AfW Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure Hand(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        SetIt(Translation);
    end;

    local procedure SetIt(var T: Text[80])
    begin
        T := '<Precision,1:1><Standard Format,0>';
    end;
}
"#;
    refused(al_testkit::run_in(&src(by_var), "C", "PageT1"));
}

/// A manual subscriber writes only while it is bound: unbound it keeps main's text, bound it refuses.
#[test]
fn a_manual_writer_refuses_only_while_bound() {
    let manual = WRITER.replace(
        "codeunit 69483 \"AfW Sub\"\n{",
        "codeunit 69483 \"AfW Sub\"\n{\n    EventSubscriberInstance = Manual;",
    );
    let s = src(&manual);
    assert_eq!(al_testkit::text(al_testkit::run_in(&s, "C", "PageT1")), "3,456.68");
    refused(al_testkit::run_in(&s, "C", "BoundPageT1"));
}

/// A subscriber of OnResolveAutoFormat or OnAfterResolveAutoFormat of the System Application's "Auto Format" codeunit, which
/// its own AutoFormatTranslate subscriber raises, that sets its result is a writer as well. The codeunit here is a stand-in with
/// the events' signatures only.
#[test]
fn a_writer_of_the_auto_format_events_refuses_too() {
    for (event, params, body) in [
        (
            "OnResolveAutoFormat",
            "var Result: Text[80]; var Resolved: Boolean",
            "Result := '<Precision,1:1><Standard Format,0>'; Resolved := true;",
        ),
        (
            "OnAfterResolveAutoFormat",
            "var Result: Text[80]",
            "Result := '<Precision,1:1><Standard Format,0>';",
        ),
    ] {
        let sub = format!(
            r#"
codeunit 45 "Auto Format"
{{
    [IntegrationEvent(false, false)]
    procedure {event}(AutoFormatType: Integer; AutoFormatExpr: Text[80]; {params})
    begin
    end;
}}
codeunit 69483 "AfW Sub"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Auto Format", '{event}', '', false, false)]
    local procedure Resolve(AutoFormatType: Integer; AutoFormatExpr: Text[80]; {params})
    begin
        {body}
    end;
}}
"#
        );
        let e = format!("{:?}", al_testkit::run_in(&src(&sub), "C", "PageT1"));
        assert!(
            e.contains("Unsupported") && e.contains("AfW Sub") && e.contains(event),
            "{e}"
        );
    }
}

/// The writer compiled with every app's publisher set to `publisher` and every app marked a dependency or not.
fn page_t1_with_publisher(
    publisher: &str,
    dependency: bool,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut l = al_testkit::compile(&src(WRITER));
    for app in &mut std::sync::Arc::make_mut(&mut l.program).apps {
        app.publisher = publisher.to_string();
        app.dependency = dependency;
    }
    al_testkit::Built::new(l).run_in("C", "PageT1")
}

/// Only a dependency app published by Microsoft is exempt: the app under test can set its publisher to "Microsoft" too (review
/// of PR 8015, the reviewer's afprobe7), and BC runs its subscriber.
#[test]
fn a_writer_in_the_app_under_test_with_publisher_microsoft_refuses() {
    refused(page_t1_with_publisher("Microsoft", false));
    assert_eq!(al_testkit::text(page_t1_with_publisher("Microsoft", true)), "3,456.68");
    refused(page_t1_with_publisher("Partner", true));
}

/// A subscriber whose writing statement lowering turned into a refusal still writes in BC (the reviewer's afprobe10: the write
/// uses a RunOnClient variable, so its `Assign` is gone from the lowered body).
#[test]
fn a_writer_whose_write_was_lowered_to_a_refusal_refuses() {
    let sub = r#"
dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; B) { } } }
codeunit 69483 "AfW Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure RunOnClientWrite(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    var
        [RunOnClient] S: DotNet B;
    begin
        if AutoFormatType = 1 then
            Translation := Format(S);
    end;
}
"#;
    refused(al_testkit::run_in(&src(sub), "C", "PageT1"));
}

/// A subscriber routine that cannot run (lowering dropped a statement and blocked the routine) counts as a writer: the dropped
/// statement may be the write.
#[test]
fn a_blocked_subscriber_routine_counts_as_a_writer() {
    let reader = r#"
codeunit 69483 "AfW Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure Log(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
    end;
}
"#;
    let l = al_testkit::compile(&src(reader));
    let mut blocked = al_testkit::blocked(&l);
    let oi = l.program.objects.iter().position(|o| o.name == "AfW Sub").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Log")
        .unwrap();
    blocked[oi][ri] = Some("a statement lowering could not read".into());
    let engine = al_vm::Engine::new(l.program.clone(), blocked);
    let mut s = al_vm::Session::new(engine);
    let ci = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[ci]
        .routines
        .iter()
        .position(|r| r.name == "PageT1")
        .unwrap();
    let inst = s.instance(ci);
    refused(s.call(ci, ri, inst, vec![]).map(|(v, _)| v));
}
