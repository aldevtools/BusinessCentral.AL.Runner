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
}

const LOGGED: &str = "trigger OnOpenPage() begin Log.Add('{tag}:rp open'); end; trigger OnClosePage() begin Log.Add('{tag}:rp close'); end;";

fn base(s: Shape) -> String {
    let single = if s.plain_log { "" } else { "SingleInstance = true;" };
    let save = if s.save_values { "SaveValues = true;" } else { "" };
    let third = if s.third {
        r#"reportextension 63103 "XR Ext C" extends "XR Report" { requestpage { layout { addlast(Content) { field(CField; CValue) { } } } } var CValue: Boolean; }"#
    } else {
        ""
    };
    format!(
        r#"
table 63100 "XR Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 63101 "XR Log" {{ {single} var L: Text; procedure Add(E: Text) begin L += E + ';'; end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }}
report 63102 "XR Report"
{{
    dataset {{ dataitem(Row; "XR Row") {{ }} }}
    requestpage
    {{
        {save}
        layout {{ area(Content) {{ field(BaseOpt; BaseOptValue) {{ ApplicationArea = All; }} field(OtherOpt; OtherOptValue) {{ ApplicationArea = All; }} }} }}
        trigger OnOpenPage() begin Log.Add('base:rp open'); end;
        trigger OnClosePage() begin Log.Add('base:rp close'); end;
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
    var Log: Codeunit "XR Log"; {tag}Value: Boolean;
}}"#
    )
}

const TEST: &str = r#"
codeunit 63190 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run()
    var Log: Codeunit "XR Log"; P: Text; Got: Text;
    begin
        Log.Take();
        P := Report.RunRequestPage(Report::"XR Report");
        Got := Log.Take();
        if Got <> 'base:rp open;B:rp open;D:rp open;base:rp close;B:rp close;D:rp close;' then
            Error('log: %1', Got);
        if StrPos(P, '<Field name="BaseOptValue">false</Field><Field name="OtherOptValue">false</Field><Field name="BValue">false</Field><Field name="DValue">false</Field>') = 0 then
            Error('options: %1', P);
    end;
    [RequestPageHandler] procedure H(var RP: TestRequestPage "XR Report") begin RP.OK().Invoke(); end;
}"#;

/// Runs the test with app B (`b_id`) and app D (`d_id`); app ids and names sort B before D.
fn run_shape(b_id: u32, d_id: u32, s: Shape) -> (Outcome, Vec<Assumption>) {
    let a = base(s);
    let b = ext(b_id, "B", "BaseOpt", s);
    let d = ext(d_id, "D", s.d_anchor.unwrap_or("BaseOpt"), s);
    let t = TEST.to_string();
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
