//! The request page of a report that two run-changing reportextensions of two apps with no dependency path between them extend,
//! each with OnOpenPage and OnClosePage and one field after the same base field (al-runner issue 7358; corpus
//! reports/extensions/IndependentAppsReportExtensions.al:105-111, :135-141): the triggers run base first, then in the run
//! order, and the parameters text lists the report's fields, then the extensions' in that order. A flagged pass (assumption
//! `report-ext-independent-app-order`). Every neighbor shape keeps main's refusal.

use super::reportext_xapp_order::run_marked;
use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

/// What the generated objects differ in from the pinned shape.
#[derive(Clone, Copy, Default)]
struct Shape {
    /// The triggers of each extension's request page (default: OnOpenPage and OnClosePage that log).
    triggers: Option<&'static str>,
    /// More in the extension's field (default: nothing).
    field_extra: &'static str,
    /// The base field D's extension adds after (default: BaseOpt).
    d_anchor: Option<&'static str>,
    /// The log codeunit is not SingleInstance.
    plain_log: bool,
    /// The report's request page states SaveValues.
    save_values: bool,
    /// D's app depends on B's app.
    path: bool,
    /// The report's app also has a reportextension that only extends the request page.
    third: bool,
    /// The layout of the report's request page (default: the one field BaseOpt, as in the pin).
    base_layout: Option<&'static str>,
    /// The triggers of the report's request page (default: OnOpenPage and OnClosePage that log).
    base_triggers: Option<&'static str>,
    /// Properties of the report's request page (default: none).
    base_props: &'static str,
    /// The type of each extension's field variable (default: Boolean).
    ext_type: Option<&'static str>,
    /// An attribute on the log codeunit's procedure Add (default: none).
    add_attribute: &'static str,
    /// The body of the log codeunit's procedure Add (default: append the entry).
    add_body: Option<&'static str>,
    /// The test procedure's body (default: `PINNED_TEST`).
    test: Option<&'static str>,
    /// The request page handler's body (default: OK).
    handler: Option<&'static str>,
}

const LOGGED: &str = "trigger OnOpenPage() begin Log.Add('{tag}:rp open'); end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end;";

/// The pinned base request page: one field, ApplicationArea and Caption (R7358XaIndependentReports.al:88-111).
const BASE_LAYOUT: &str = "area(Content) { field(BaseOpt; BaseOptValue) { ApplicationArea = All; Caption = 'Base'; } }";

fn base(s: Shape) -> String {
    let single = if s.plain_log { "" } else { "SingleInstance = true;" };
    let save = if s.save_values {
        "SaveValues = true;"
    } else {
        s.base_props
    };
    let layout = s.base_layout.unwrap_or(BASE_LAYOUT);
    let triggers = s
        .base_triggers
        .unwrap_or("trigger OnOpenPage() begin Log.Add('base:rp open'); end; trigger OnClosePage() begin Log.Add('base:rp close'); end;");
    let attribute = s.add_attribute;
    let add = s.add_body.unwrap_or("L += E + ';';");
    let third = if s.third {
        r#"reportextension 63103 "XR Ext C" extends "XR Report" { requestpage { layout { addlast(Content) { field(CField; CValue) { } } } } var CValue: Boolean; }"#
    } else {
        ""
    };
    format!(
        r#"
table 63100 "XR Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 63101 "XR Log" {{ {single} var L: Text; {attribute} procedure Add(E: Text) begin {add} end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }}
report 63102 "XR Report"
{{
    dataset {{ dataitem(Row; "XR Row") {{ }} }}
    requestpage
    {{
        {save}
        layout {{ {layout} }}
        {triggers}
    }}
    var Log: Codeunit "XR Log"; BaseOptValue: Boolean; OtherOptValue: Boolean;
}}
{third}
"#
    )
}

fn ext(id: u32, tag: &str, anchor: &str, s: Shape) -> String {
    let triggers = s.triggers.unwrap_or(LOGGED).replace("{tag}", tag);
    let extra = s.field_extra;
    let ty = s.ext_type.unwrap_or("Boolean");
    format!(
        r#"
reportextension {id} "XR Ext {tag}" extends "XR Report"
{{
    dataset {{ modify(Row) {{ trigger OnBeforePreDataItem() begin Log.Add('{tag}:pre'); end; }} }}
    requestpage
    {{
        layout {{ addafter({anchor}) {{ field({tag}Field; {tag}Value) {{ ApplicationArea = All; Caption = '{tag}'; {extra} }} }} }}
        {triggers}
    }}
    var Log: Codeunit "XR Log"; {tag}Value: {ty};
}}"#
    )
}

/// The pinned call: `Report.RunRequestPage(id)`, the handler calls OK only (IndependentAppsReportExtensions.al:73-81,
/// :143-153).
const PINNED_TEST: &str = r#"
        Log.Take();
        P := Report.RunRequestPage(Report::"XR Report");
        Got := Log.Take();
        if Got <> 'base:rp open;B:rp open;D:rp open;base:rp close;B:rp close;D:rp close;' then
            Error('log: %1', Got);
        if StrPos(P, '<Field name="BaseOptValue">false</Field><Field name="BValue">false</Field><Field name="DValue">false</Field>') = 0 then
            Error('options: %1', P);"#;

fn test(s: Shape) -> String {
    let body = s.test.unwrap_or(PINNED_TEST);
    let handler = s.handler.unwrap_or("RP.OK().Invoke();");
    format!(
        r#"
codeunit 63190 T
{{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run()
    var Log: Codeunit "XR Log"; R: Report "XR Report"; P: Text; Got: Text;
    begin
        {body}
    end;
    [RequestPageHandler] procedure H(var RP: TestRequestPage "XR Report")
    var Log: Codeunit "XR Log";
    begin
        {handler}
    end;
}}"#
    )
}

/// Runs the test with app B (`b_id`) and app D (`d_id`); app ids and names sort B before D.
fn run_shape(b_id: u32, d_id: u32, s: Shape) -> (Outcome, Vec<Assumption>) {
    let a = base(s);
    let b = ext(b_id, "B", "BaseOpt", s);
    let d = ext(d_id, "D", s.d_anchor.unwrap_or("BaseOpt"), s);
    let t = test(s);
    let d_deps: &[usize] = if s.path { &[0, 1] } else { &[0] };
    let names: Vec<String> = (0..4).map(|i| format!("app{i}")).collect();
    let (o, hits) = run_marked(&[(&a, &[]), (&b, &[0]), (&d, d_deps), (&t, &[0, 1, 2])], &names);
    let [(_, outcome)] = &o[..] else { panic!("{o:?}") };
    let hits = hits.into_iter().flat_map(|(_, h)| h).collect();
    (outcome.clone(), hits)
}

fn refused(o: &Outcome, why: &str) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains(why))
}

/// A base request page with a second field after the anchor.
const TWO_BASE_FIELDS: &str = "area(Content) { field(BaseOpt; BaseOptValue) { ApplicationArea = All; Caption = 'Base'; } field(OtherOpt; OtherOptValue) { ApplicationArea = All; } }";

const TRIGGER_REFUSAL: &str = "changes the run and extends the request page: a trigger on the extension's request page";

#[test]
fn the_pinned_shape_runs_base_then_run_order_whatever_the_extension_ids() {
    // B's extension has the lower id and then the higher one: the order is the apps', not the ids'.
    for (b_id, d_id) in [(63110, 63120), (63130, 63111)] {
        let (o, hits) = run_shape(b_id, d_id, Shape::default());
        assert_eq!(o, Outcome::Pass, "{b_id} {d_id}");
        assert!(hits.contains(&Assumption::ReportExtIndependentAppOrder), "{hits:?}");
    }
}

#[test]
fn other_request_page_triggers_keep_the_refusal() {
    for triggers in [
        "trigger OnOpenPage() begin Log.Add('{tag}:rp open'); end;",
        "trigger OnOpenPage() begin Log.Add('{tag}:rp open'); end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end; trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end;",
        "trigger OnOpenPage() begin {tag}Value := true; end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end;",
        "trigger OnOpenPage() begin Log.Add('{tag}:rp open'); Log.Add('again'); end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end;",
        "trigger OnOpenPage() var X: Integer; begin Log.Add('{tag}:rp open'); end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end;",
    ] {
        let (o, _) = run_shape(
            63110,
            63120,
            Shape {
                triggers: Some(triggers),
                ..Default::default()
            },
        );
        assert!(refused(&o, TRIGGER_REFUSAL), "{triggers}: {o:?}");
    }
}

#[test]
fn neighbors_of_the_pinned_shape_keep_the_refusal() {
    let shapes = [
        (
            "a field trigger",
            Shape {
                field_extra: "trigger OnValidate() begin end;",
                ..Default::default()
            },
        ),
        (
            "a field property",
            Shape {
                field_extra: "Visible = true;",
                ..Default::default()
            },
        ),
        (
            "another anchor",
            Shape {
                base_layout: Some(TWO_BASE_FIELDS),
                d_anchor: Some("OtherOpt"),
                ..Default::default()
            },
        ),
        (
            "a log codeunit that is not SingleInstance",
            Shape {
                plain_log: true,
                ..Default::default()
            },
        ),
        (
            "a dependency path between the apps",
            Shape {
                path: true,
                ..Default::default()
            },
        ),
        (
            "a third extension of the request page",
            Shape {
                third: true,
                ..Default::default()
            },
        ),
    ];
    for (what, s) in shapes {
        let (o, _) = run_shape(63110, 63120, s);
        assert!(refused(&o, TRIGGER_REFUSAL), "{what}: {o:?}");
    }
    let (o, _) = run_shape(
        63110,
        63120,
        Shape {
            save_values: true,
            ..Default::default()
        },
    );
    assert!(refused(&o, "a request page with SaveValues"), "SaveValues: {o:?}");
}

#[test]
fn a_base_request_page_or_callee_other_than_the_pinned_one_keeps_the_refusal() {
    let shapes = [
        (
            "OnInit and OnQueryClosePage on the base",
            Shape {
                base_triggers: Some(
                    "trigger OnInit() begin Log.Add('base:init'); end; trigger OnOpenPage() begin Log.Add('base:rp open'); end; trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log.Add('base:query'); exit(true); end; trigger OnClosePage() begin Log.Add('base:rp close'); end;",
                ),
                ..Default::default()
            },
        ),
        (
            "no trigger on the base",
            Shape {
                base_triggers: Some(""),
                ..Default::default()
            },
        ),
        (
            "a base trigger that does more than log",
            Shape {
                base_triggers: Some(
                    "trigger OnOpenPage() begin BaseOptValue := true; end; trigger OnClosePage() begin Log.Add('base:rp close'); end;",
                ),
                ..Default::default()
            },
        ),
        (
            "SourceTable on the base",
            Shape {
                base_props: "SourceTable = \"XR Row\";",
                ..Default::default()
            },
        ),
        (
            "SaveValues = false stated on the base",
            Shape {
                base_props: "SaveValues = false;",
                ..Default::default()
            },
        ),
        (
            "a second base field",
            Shape {
                base_layout: Some(TWO_BASE_FIELDS),
                ..Default::default()
            },
        ),
        (
            "the anchor inside a group",
            Shape {
                base_layout: Some(
                    "area(Content) { group(G) { field(BaseOpt; BaseOptValue) { ApplicationArea = All; Caption = 'Base'; } } }",
                ),
                ..Default::default()
            },
        ),
        (
            "a base field without Caption",
            Shape {
                base_layout: Some("area(Content) { field(BaseOpt; BaseOptValue) { ApplicationArea = All; } }"),
                ..Default::default()
            },
        ),
        (
            "Integer extension fields",
            Shape {
                ext_type: Some("Integer"),
                ..Default::default()
            },
        ),
        (
            "Text extension fields",
            Shape {
                ext_type: Some("Text[30]"),
                ..Default::default()
            },
        ),
        (
            "a callee with an attribute",
            Shape {
                add_attribute: "[NonDebuggable]",
                ..Default::default()
            },
        ),
    ];
    let ran: Vec<String> = shapes
        .into_iter()
        .map(|(what, s)| (what, run_shape(63110, 63120, s).0))
        .filter(|(_, o)| !refused(o, TRIGGER_REFUSAL))
        .map(|(what, o)| format!("{what}: {o:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}

/// What the run-time gate (al-vm `pages/request_ok_only.rs`) says for every call, close and error it refuses.
const CALL_REFUSAL: &str = "pinned only: Report.RunRequestPage(id) closed by OK";

#[test]
fn a_call_close_or_error_other_than_the_pinned_one_refuses() {
    let shapes = [
        (
            "the handler cancels",
            Shape {
                handler: Some("RP.Cancel().Invoke();"),
                ..Default::default()
            },
        ),
        (
            "the handler sets an extension field",
            Shape {
                handler: Some("RP.DField.SetValue(true); RP.OK().Invoke();"),
                ..Default::default()
            },
        ),
        (
            "the handler reads an extension field",
            Shape {
                handler: Some("Log.Add(RP.DField.Value); RP.OK().Invoke();"),
                ..Default::default()
            },
        ),
        (
            "the handler reads a caption",
            Shape {
                handler: Some("Log.Add(RP.BField.Caption); RP.OK().Invoke();"),
                ..Default::default()
            },
        ),
        (
            "Report.Run, the handler saves as XML",
            Shape {
                test: Some("Report.Run(Report::\"XR Report\");"),
                handler: Some("RP.SaveAsXml('p.xml', 'd.xml');"),
                ..Default::default()
            },
        ),
        (
            "a second RunRequestPage with the first one's parameters",
            Shape {
                test: Some(
                    "P := Report.RunRequestPage(Report::\"XR Report\"); P := Report.RunRequestPage(Report::\"XR Report\", P);",
                ),
                ..Default::default()
            },
        ),
        (
            "RunRequestPage of a report variable",
            Shape {
                test: Some("P := R.RunRequestPage();"),
                ..Default::default()
            },
        ),
        (
            "the procedure the base OnOpenPage calls raises an error",
            Shape {
                add_body: Some("if E = 'base:rp open' then Error('boom'); L += E + ';';"),
                ..Default::default()
            },
        ),
    ];
    let ran: Vec<String> = shapes
        .into_iter()
        .map(|(what, s)| (what, run_shape(63110, 63120, s).0))
        .filter(|(_, o)| !refused(o, CALL_REFUSAL))
        .map(|(what, o)| format!("{what}: {o:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}

#[test]
fn a_request_page_the_gate_does_not_admit_keeps_its_close_by_cancel() {
    // No trigger on the extensions' request pages: main's path, which a Cancel closes ('' parameters, the run gate stays off).
    let (o, hits) = run_shape(
        63110,
        63120,
        Shape {
            triggers: Some(""),
            handler: Some("RP.Cancel().Invoke();"),
            test: Some(
                "P := Report.RunRequestPage(Report::\"XR Report\"); if P <> '' then Error('parameters: %1', P);",
            ),
            ..Default::default()
        },
    );
    assert_eq!(o, Outcome::Pass);
    assert!(hits.contains(&Assumption::ReportExtIndependentAppOrder), "{hits:?}");
}
