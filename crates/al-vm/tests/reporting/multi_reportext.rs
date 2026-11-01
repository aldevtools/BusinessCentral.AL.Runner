//! Several reportextensions that change one report run (ADR-025). Each test names the corpus test
//! that pins it (tests/al-language/reportextensiontrigger/).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 69010 "MR Parent" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 69011 "MR Child" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69015 "MR Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(Entry: Text) begin Log += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
}
"#;

fn compile(apps: Vec<(&str, String)>) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    let mut out = Vec::new();
    for (i, (name, src)) in apps.into_iter().enumerate() {
        out.push(AppSource {
            id: name.into(),
            name: name.into(),
            dependencies: if i == 0 { vec![] } else { vec![out_name(&out)] },
            files: vec![("f.al".into(), src)],
        });
    }
    al_sema::lower::compile(out, &Default::default())
}

fn out_name(v: &[AppSource]) -> String {
    v.last().map(|a| a.id.clone()).unwrap_or_default()
}

fn run(src: &str, body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLES}\n{src}\ncodeunit 69016 C {{ {body} }}");
    let (ws, l) = compile(vec![("app0", src)]);
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
        .position(|r| r.name == "F")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn reasons(src: &str, apps: usize) -> Vec<String> {
    let src = format!("{TABLES}\n{src}");
    let (_, l) = if apps == 1 {
        compile(vec![("app0", src)])
    } else {
        // The extensions go to a second app that depends on the first (base + tables in app0).
        let (a, b) = src.split_once("//--app1\n").expect("a marker");
        compile(vec![("app0", a.into()), ("app1", b.into())])
    };
    let mut out: Vec<String> = l.unsupported.iter().flatten().flatten().cloned().collect();
    for o in &l.program.objects {
        if let Some(r) = &o.report {
            out.extend(r.unsupported.iter().cloned());
        }
    }
    out
}

const RUN: &str = "procedure F(): Text var P: Record \"MR Parent\"; Ch: Record \"MR Child\"; L: Codeunit \"MR Log\";
    begin P.Id := 1; P.Insert(); Ch.Id := 1; Ch.ParentId := 1; Ch.Insert(); L.Take();
    Report.Run(Report::\"MR Base\", false); exit(L.Take()); end;";

// reportextensiontrigger/TestReportExtensionBaseMembersContracts.al:256: the base first, then the
// extensions in object id order (68893 Ext, 68894 Zeta, 68895 Alpha: Alpha runs last).
#[test]
fn triggers_run_base_first_then_the_extensions_in_object_id_order() {
    let src = r#"
report 69100 "MR Base"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(Parent; "MR Parent") { } }
    trigger OnPreReport() var L: Codeunit "MR Log"; begin L.Add('base'); end;
    trigger OnPostReport() var L: Codeunit "MR Log"; begin L.Add('base-post'); end;
}
reportextension 69105 "MR Alpha" extends "MR Base"
{
    trigger OnPreReport() var L: Codeunit "MR Log"; begin L.Add('alpha'); end;
    trigger OnPostReport() var L: Codeunit "MR Log"; begin L.Add('alpha-post'); end;
}
reportextension 69103 "MR Ext" extends "MR Base"
{
    trigger OnPreReport() var L: Codeunit "MR Log"; begin L.Add('ext'); end;
    trigger OnPostReport() var L: Codeunit "MR Log"; begin L.Add('ext-post'); end;
}
reportextension 69104 "MR Zeta" extends "MR Base"
{
    trigger OnPreReport() var L: Codeunit "MR Log"; begin L.Add('zeta'); end;
    trigger OnPostReport() var L: Codeunit "MR Log"; begin L.Add('zeta-post'); end;
}"#;
    assert_eq!(
        text(run(src, RUN)),
        "base;ext;zeta;alpha;base-post;ext-post;zeta-post;alpha-post;"
    );
}

const PLACED_BASE: &str = r#"
report 69100 "MR Base"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(Parent; "MR Parent")
        {
            dataitem(ChildA; "MR Child")
            {
                DataItemLink = ParentId = field(Id);
                trigger OnAfterGetRecord() var L: Codeunit "MR Log"; begin L.Add('ChildA'); end;
            }
            trigger OnAfterGetRecord() var L: Codeunit "MR Log"; begin L.Add('Parent'); end;
        }
    }
}"#;

fn item(name: &str) -> String {
    format!(
        "dataitem({name}; \"MR Child\") {{ DataItemLink = ParentId = field(Id); trigger OnAfterGetRecord() var L: Codeunit \"MR Log\"; begin L.Add('{name}'); end; }}"
    )
}

fn placed_ext(id: u32, name: &str, first: &str, before: &str, after: &str) -> String {
    let blk = |kind: &str, anchor: &str, names: &str| {
        if names.is_empty() {
            String::new()
        } else {
            let items: String = names.split(',').map(item).collect::<Vec<_>>().join(" ");
            format!("{kind}({anchor}) {{ {items} }}")
        }
    };
    format!(
        "reportextension {id} \"{name}\" extends \"MR Base\" {{ dataset {{ {} {} {} }} }}",
        blk("addfirst", "Parent", first),
        blk("addbefore", "ChildA", before),
        blk("addafter", "ChildA", after)
    )
}

// reportextensiontrigger/TestReportExtensionSamePlaceContracts.al:251: Parent;G1;F1;F2;X1;X2;Y1;ChildA;Z2;Z1.
// The extension with the lower id (Ext1) adds F1 F2 first, X1 X2 before and Z1 after; the later one (Ext2)
// adds G1 first, Y1 before and Z2 after.
#[test]
fn items_added_at_one_place_sit_next_to_the_anchor_the_later_extension_nearest() {
    let src = format!(
        "{PLACED_BASE}\n{}\n{}",
        placed_ext(69113, "MR Ext1", "F1,F2", "X1,X2", "Z1"),
        placed_ext(69114, "MR Ext2", "G1", "Y1", "Z2")
    );
    assert_eq!(text(run(&src, RUN)), "Parent;G1;F1;F2;X1;X2;Y1;ChildA;Z2;Z1;");
}

// The order does not depend on the order the objects are written in the file.
#[test]
fn the_object_id_decides_not_the_file_order() {
    let src = format!(
        "{PLACED_BASE}\n{}\n{}",
        placed_ext(69114, "MR Ext2", "G1", "Y1", "Z2"),
        placed_ext(69113, "MR Ext1", "F1,F2", "X1,X2", "Z1")
    );
    assert_eq!(text(run(&src, RUN)), "Parent;G1;F1;F2;X1;X2;Y1;ChildA;Z2;Z1;");
}

// Pinned by reportextensiontrigger/TestReportExtensionSamePlaceContracts.al (bump 6): two items one
// extension adds after the same anchor and two extensions adding last under one item are not refused.
// Still refused until a probe pins them (ADR-025): extensions of different apps whose object ids go against the
// dependency path (reportext/lt-y64-xapp-run-order/Y64XappRunOrder.al pins only ids that rise along it).
#[test]
fn same_place_placements_are_accepted_and_cross_app_extensions_are_refused() {
    let two_after = format!(
        "{PLACED_BASE}\nreportextension 69113 \"MR E\" extends \"MR Base\" {{ dataset {{ addafter(ChildA) {{ {} {} }} }} }}",
        item("Z1"),
        item("Z2")
    );
    let r = reasons(&two_after, 1);
    assert!(!r.iter().any(|x| x.contains("same place")), "{r:?}");
    let two_last = format!(
        "{PLACED_BASE}\nreportextension 69113 \"MR E1\" extends \"MR Base\" {{ dataset {{ addlast(Parent) {{ {} }} }} }}\nreportextension 69114 \"MR E2\" extends \"MR Base\" {{ dataset {{ addlast(Parent) {{ {} }} }} }}",
        item("L1"),
        item("L2")
    );
    let r = reasons(&two_last, 1);
    assert!(
        !r.iter()
            .any(|x| x.contains("add data items last under the same data item")),
        "{r:?}"
    );
    let cross = format!(
        "{PLACED_BASE}\nreportextension 69114 \"MR E1\" extends \"MR Base\" {{ trigger OnPreReport() begin end; }}\n//--app1\nreportextension 69113 \"MR E2\" extends \"MR Base\" {{ trigger OnPreReport() begin end; }}"
    );
    let r = reasons(&cross, 2);
    assert!(r.iter().any(|x| x.contains("different apps")), "{r:?}");
}

// extensionevents/TestExtensionDeclaredEvents.al:105-117 pins an event a reportextension declares and raises: a subscriber
// bound to the base report fires with the argument. Base Application's "Serv. Get Source Documents" adds data items whose
// triggers raise such `local` events and a local procedure that counts in a variable of the extension (nothing outside the extension can call a `local` procedure, so which instance it runs
// on is the one that runs the trigger).
#[test]
fn a_reportextension_with_local_event_publishers_and_added_data_items_runs() {
    let src = r#"
report 69100 "MR Base"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(Parent; "MR Parent") { trigger OnAfterGetRecord() var L: Codeunit "MR Log"; begin L.Add('Parent'); end; } }
}
reportextension 69105 "MR Ext" extends "MR Base"
{
    dataset
    {
        addlast(Parent)
        {
            dataitem(ExtItem; "MR Child")
            {
                DataItemLink = ParentId = field(Id);
                trigger OnAfterGetRecord() begin OnExtItem(ExtItem.Id); Bump(); end;
            }
        }
    }
    [IntegrationEvent(false, false)]
    local procedure OnExtItem(Value: Integer) begin end;
    local procedure Bump() var L: Codeunit "MR Log"; begin Seen += 1; L.Add('bump' + Format(Seen)); end;
    var Seen: Integer;
}
codeunit 69106 "MR Sub"
{
    [EventSubscriber(ObjectType::Report, Report::"MR Base", 'OnExtItem', '', false, false)]
    local procedure Handle(Value: Integer) var L: Codeunit "MR Log"; begin L.Add('event' + Format(Value)); end;
}"#;
    let body = "procedure F(): Text var L: Codeunit \"MR Log\"; P: Record \"MR Parent\"; C: Record \"MR Child\"; begin
        P.Id := 1; P.Insert(); C.Id := 7; C.ParentId := 1; C.Insert();
        Report.Run(Report::\"MR Base\", false); exit(L.Take()); end;";
    assert_eq!(text(run(src, body)), "Parent;event7;bump1;");
}

// A public procedure of an extension that has global variables and added data items runs on the extension's instance of
// the report variable it is called through: the global keeps its value from one call to the next, a second variable has
// a state of its own, and Clear starts again (reportext/W3RpExtProcedures.al:ExtensionProcedure_StateOnOneVariableAndOnANewVariable).
#[test]
fn a_public_procedure_keeps_the_extensions_globals_on_one_report_variable() {
    let src = r#"
report 69100 "MR Base"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(Parent; "MR Parent") { } }
}
reportextension 69105 "MR Ext" extends "MR Base"
{
    dataset { addlast(Parent) { dataitem(ExtItem; "MR Child") { DataItemLink = ParentId = field(Id); } } }
    var Seen: Integer;
    procedure Public(): Integer begin Seen += 1; exit(Seen); end;
}"#;
    let body = "procedure F(): Text var R: Report \"MR Base\"; O: Report \"MR Base\"; T: Text; begin
        T := Format(R.Public()) + Format(R.Public()) + Format(O.Public()) + Format(R.Public());
        Clear(R); T += Format(R.Public()); exit(T); end;";
    assert_eq!(text(run(src, body)), "12131");
}
