// The scans of production code (readable_messages, no_version_literals) stop at the first `#[cfg(test)]` of a file,
// so a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use crate::workspace::AppSource;

fn lower_src(src: &str) -> (Workspace, Lowered) {
    lower_files(&[("f.al", src)], false)
}

/// Like `lower_src`, with source positions (the way the runner compiles).
fn lower_pos(src: &str) -> (Workspace, Lowered) {
    lower_files(&[("f.al", src)], true)
}

fn lower_files(files: &[(&str, &str)], positions: bool) -> (Workspace, Lowered) {
    let ws = Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
        }],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let lowered = lower_with_positions(&ws, &members, &checked, positions);
    (ws, lowered)
}

/// Tests-ERM (Microsoft's BaseApp bucket): a page field whose source is a lone quoted name with
/// a hyphen used to reach the synthetic source routine unquoted (`exit(Assign-to No.)`), which
/// parses as `Assign-` with no right operand. Lowering indexed `kids[1]` and panicked, which
/// ended the whole run. The quotes now stay (the next test); an expression with a missing
/// operand is still refused, not a panic.
#[test]
fn a_binary_expression_with_a_missing_operand_is_refused_not_a_panic() {
    let (_, l) = lower_files(
        &[(
            "f.al",
            "table 50100 T { fields { field(1; \"Assign-to No.\"; Code[20]) { } } }
page 50100 P { SourceTable = T; layout { area(Content) { field(X; Assign-to No.) { } } } }",
        )],
        true,
    );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert!(reasons.iter().any(|r| r.contains("missing operand")), "{reasons:?}");
}

/// A statement the parser recovered from an error can lack its parts (an `if` with no condition: `If: Integer;` in a var
/// section once parsed as `if :`). Lowering refuses such a statement; it must never panic (the panic is caught, but it
/// is a bug, and the reason says "internal error").
#[test]
fn a_statement_with_missing_parts_is_refused_not_a_panic() {
    for body in [
        "if : Integer;",
        "if := 1;",
        "if : Integer;",
        "while : Integer;",
        "for : Integer;",
        "foreach : Integer;",
        "with : Integer;",
        "case : Integer;",
        "while do;",
        "repeat until;",
        "for to do;",
        "foreach in do;",
        "with do;",
        "case of end;",
        "X := ;",
    ] {
        let src = format!(
            "codeunit 50100 C {{ Subtype = Test; [Test] procedure P() var X: Integer; begin {body} end;
            [Test] procedure Q() {body} begin end; [Test] procedure R() var {body} begin end; }}"
        );
        // A panic while lowering a routine is caught and becomes this reason (shared.rs); it is still a bug.
        let (_, l) = lower_pos(&src);
        let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
        assert!(
            !reasons.iter().any(|r| r.contains("internal error")),
            "`{body}`: {reasons:?}"
        );
    }
}

#[test]
fn a_page_field_with_a_lone_quoted_source_name_lowers() {
    let (_, l) = lower_files(
        &[(
            "f.al",
            "table 50100 T { fields { field(1; \"Assign-to No.\"; Code[20]) { } } }
page 50100 P { SourceTable = T; layout { area(Content) { field(\"Assign-to No.\"; \"Assign-to No.\") { } } } }",
        )],
        true,
    );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert!(reasons.is_empty(), "{reasons:?}");
}

/// ADR-023: the slot of a base variable is computed from the base object's syntax, so the
/// extension lowers without the lowered base. It must equal the slot the base's own lowering
/// gives the variable (a page lays out its variables in declaration order; a report puts its
/// data items and the request-page flag first).
#[test]
fn the_slot_of_a_base_variable_computed_from_syntax_is_the_one_the_base_lowers_to() {
    let (ws, l) = lower_files(
            &[(
                "f.al",
                "table 50100 T { fields { field(1; A; Integer) { } } keys { key(PK; A) { } } }
page 50100 P { SourceTable = T; var First: Integer; protected var Second: Text; Lbl: Label 'x'; var Third: Record T; }
report 50101 R { dataset { dataitem(I1; T) { dataitem(I2; T) { } } dataitem(I3; T) { } } var RFirst: Integer; protected var RSecond: Text; }",
            )],
            false,
        );
    let find = |name: &str| ws.objects.iter().position(|o| o.name == name).unwrap();
    let (page, report) = (find("P"), find("R"));
    for (oi, items, names) in [
        (page, None, &["First", "Second", "Lbl", "Third"][..]),
        (report, Some(3), &["RFirst", "RSecond"][..]),
    ] {
        let ast = ws.object_ast(oi);
        for n in names {
            let want = l.program.objects[oi].globals.iter().position(|g| g.name == *n);
            assert_eq!(crate::base_access::slot_of(&ast, items, n), want, "{n}");
        }
    }
}

/// Base Application's chart pages pass their control add-in to a `var` parameter
/// (`BusinessChartBuffer.UpdateChart(CurrPage.BusinessChart)`), often on a path a flag guards. Control add-ins are not
/// modelled: the call refuses when it runs, naming the control add-in, and the routine's other paths run.
#[test]
fn a_control_add_in_passed_as_an_argument_refuses_when_the_call_runs() {
    let (_, l) = lower_files(
        &[(
            "f.al",
            "controladdin MyChart { }
codeunit 50100 Buf { procedure UpdateChart(var Chart: ControlAddIn \"MyChart\") begin end; procedure Show(Chart: ControlAddIn \"MyChart\") begin end; }
page 50100 P { PageType = Card; layout { area(Content) { usercontrol(BusinessChart; MyChart) { } } }
  trigger OnOpenPage() var B: Codeunit Buf; begin if Ready then B.UpdateChart(CurrPage.BusinessChart); if Ready then B.Show(CurrPage.BusinessChart); end;
  var Ready: Boolean; }",
        )],
        true,
    );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert!(reasons.is_empty(), "{reasons:?}");
    let page = l.program.objects.iter().find(|o| o.name == "P").expect("page P");
    let body = format!(
        "{:?}",
        page.routines
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case("OnOpenPage"))
            .expect("OnOpenPage")
            .body
    );
    assert_eq!(body.matches("Refused(").count(), 2, "{body}");
    assert!(body.contains("control add-in"), "{body}");
}

/// Tests-SCM: a page extension's field `field(X; Item."Qty. on Component Lines")` names a
/// `protected var` of the page it extends. It lowers (ADR-023; corpus
/// handlers/TestPageExtensionBaseAccessContracts.al:326), and a name the base does not make
/// visible is refused with the compiler's text (AL0161).
#[test]
fn an_extension_naming_a_variable_of_its_base_page_lowers_and_a_private_one_is_refused() {
    let (_, l) = lower_files(
            &[(
                "f.al",
                "table 50100 T { fields { field(1; \"Qty. A\"; Integer) { } } }
page 50100 Base { SourceTable = T; layout { area(Content) { field(A; Rec.\"Qty. A\") { } } } protected var Item: Record T; var Hidden: Integer; }
pageextension 50101 Ext extends Base { layout { addlast(Content) { field(\"X\"; Item.\"Qty. A\") { } } } }
pageextension 50102 Ext2 extends Base { trigger OnOpenPage() begin Hidden := 1; end; }",
            )],
            true,
        );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0].contains("'Hidden' is inaccessible due to its protection level"),
        "{reasons:?}"
    );
}

/// Tests-ERM: the Manufacturing report extensions call `Window.Open(..)` and
/// `ExchangeAmtLCYToFCY(..)`, a `protected var` and a `protected procedure` of the report they
/// extend (Microsoft's "Adjust Add. Reporting Currency"). They lower (corpus
/// reportextensiontrigger/TestReportExtensionBaseMembersContracts.al:256); a `local` procedure
/// is refused with the compiler's text.
#[test]
fn a_report_extension_naming_a_member_of_its_base_report_lowers_and_a_local_one_is_refused() {
    let (_, l) = lower_files(
            &[(
                "f.al",
                "table 50100 T { fields { field(1; Id; Integer) { } } }
report 50100 Base { dataset { dataitem(Item; T) { } } protected var Window: Dialog; protected procedure Conv(A: Decimal): Decimal begin exit(A); end; local procedure Priv() begin end; }
reportextension 50101 Ext extends Base
{
    dataset { modify(Item) { trigger OnPreDataItem() begin Window.Open('x'); end; } }
    trigger OnPreReport() begin Message(Format(Conv(1))); Priv(); end;
}",
            )],
            true,
        );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0].contains("'Priv()' is inaccessible due to its protection level"),
        "{reasons:?}"
    );
}

/// A report extension declares a `var` in its `requestpage`, with the name of a variable of its base report. Its request
/// page triggers see their own variable, not the base's private one (AL0161) and not nothing (corpus
/// report/lt-ab-request-page-triggers/ABRequestPageExt.al:63).
#[test]
fn a_request_page_variable_is_seen_by_the_request_page_triggers_of_a_report_extension() {
    let (_, l) = lower_files(
        &[(
            "f.al",
            "report 50100 Base { ProcessingOnly = true; requestpage { trigger OnOpenPage() begin Log := 1; end; var Log: Integer; } var Log: Integer; }
reportextension 50101 Ext extends Base
{
    requestpage { trigger OnOpenPage() begin Log := 2; end; var Log: Integer; }
}",
        )],
        true,
    );
    let reasons: Vec<&String> = l.unsupported.iter().flatten().flatten().collect();
    assert!(reasons.is_empty(), "{reasons:?}");
}

const TWO_TESTS: &str = "codeunit 50100 \"Wanted\"
{
    Subtype = Test;
    [Test]
    procedure T1()
    var
        x: Integer;
    begin
        x := 1;
    end;
}
codeunit 50101 \"Left Out\"
{
    Subtype = Test;
    [Test]
    procedure T2()
    var
        x: Integer;
    begin
        x := 1;
    end;
}
codeunit 50102 \"Helper\"
{
    procedure H()
    var
        x: Integer;
    begin
        x := 1;
    end;
}
codeunit 50103 \"Listening Test\"
{
    Subtype = Test;
    [Test]
    procedure T3()
    var
        x: Integer;
    begin
        x := 1;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::Helper, 'OnX', '', false, false)]
    local procedure OnX()
    begin
    end;
}";

/// A test selection (`Roots::only_tests`) roots the check at the selected test codeunits:
/// the others are stubs that refuse, and helpers and event listeners stay compiled.
#[test]
fn a_selection_compiles_only_the_selected_test_codeunits() {
    let app = || AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".to_string(), TWO_TESTS.to_string())],
    };
    let reasons = |only: Option<Vec<(usize, u32)>>| {
        let (ws, l) = compile_apps_with(
            vec![app()],
            vec![crate::workspace::AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                baseline: false,
                only_tests: only,
                ..Default::default()
            },
        );
        let of = |id: u32| {
            let oi = ws.objects.iter().position(|o| o.id == Some(id)).expect("object");
            l.unsupported[oi].iter().flatten().count()
        };
        (of(50100), of(50101), of(50102), of(50103))
    };
    assert_eq!(
        reasons(None),
        (0, 0, 0, 0),
        "without a selection every codeunit compiles"
    );
    assert_eq!(
        reasons(Some(vec![(0, 50100)])),
        (0, 1, 0, 0),
        "only the codeunit that is not selected, not a helper and not a listener, is left out"
    );
}

/// With `Roots::reach_only` a normal codeunit's procedure that no selected test (and no root that
/// runs by itself: a trigger, a subscriber, a codeunit with a Subtype) names is a stub that
/// refuses, so a pure test is not compiled together with the data-touching code beside it.
#[test]
fn reach_only_compiles_the_procedures_the_selected_tests_name() {
    const SRC: &str = "codeunit 50100 \"Helper\" {
            procedure Used(): Integer begin exit(Deeper()); end;
            local procedure Deeper(): Integer begin exit(1); end;
            procedure Unused() begin Message('x'); end;
            local procedure AlsoUnused() begin Message('y'); end;
            trigger OnRun() begin Message('z'); end;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Other\", 'OnX', '', false, false)]
            local procedure Listen() begin Message('w'); end;
        }
        codeunit 50101 \"Other\" { [IntegrationEvent(false, false)] procedure OnX() begin end; }
        codeunit 50102 \"T\" { Subtype = Test; var H: Codeunit \"Helper\";
            [Test] procedure P() begin if H.Used() <> 1 then Error('x'); end; }
        codeunit 50103 \"Other Test\" { Subtype = Test; var H: Codeunit \"Helper\";
            [Test] procedure Q() begin H.Unused(); end; }";
    let compile = |reach_only: bool| {
        let (ws, l) = compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".to_string(), SRC.to_string())],
            }],
            vec![crate::workspace::AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                baseline: false,
                only_tests: Some(vec![(0, 50102)]),
                reach_only,
                ..Default::default()
            },
        );
        let oi = ws.objects.iter().position(|o| o.id == Some(50100)).expect("helper");
        let refused: Vec<String> = l.program.objects[oi]
            .routines
            .iter()
            .zip(&l.unsupported[oi])
            .filter(|(_, why)| !why.is_empty())
            .map(|(r, _)| r.name.clone())
            .collect();
        refused
    };
    assert_eq!(
        compile(false),
        Vec::<String>::new(),
        "without reach_only the whole codeunit compiles"
    );
    assert_eq!(
        compile(true),
        vec!["Unused".to_string(), "AlsoUnused".to_string()],
        "Used and Deeper are named from the test, the trigger and the subscriber run by themselves"
    );
}

/// With `Roots::lean` (and `reach_only`) a workspace object that no selected test names, directly or through the objects
/// it names (by name, or by id in an integer literal), is left out of the compile: its routines refuse with
/// `check::LEFT_OUT`. A table with its triggers, the page on it, a subscriber and an install codeunit beside a pure test
/// are not compiled with it; what the test names is.
#[test]
fn lean_leaves_out_the_workspace_objects_no_selected_test_names() {
    const SRC: &str = "table 50110 \"Row\" { fields { field(1; \"No.\"; Integer) { } }
            trigger OnInsert() begin Message('i'); end; }
        page 50111 \"Rows\" { SourceTable = \"Row\"; trigger OnOpenPage() begin Message('o'); end; }
        codeunit 50112 \"Listener\" {
            [EventSubscriber(ObjectType::Table, Database::\"Row\", 'OnAfterInsertEvent', '', false, false)]
            local procedure L(var Rec: Record \"Row\") begin Message('l'); end; }
        codeunit 50113 \"Seed\" { Subtype = Install; trigger OnInstallAppPerCompany() begin Message('n'); end; }
        codeunit 50100 \"Helper\" { var Lib: Codeunit \"Lib\"; procedure Used(): Integer begin exit(Lib.Two() - 1); end; }
        codeunit 50101 \"Lib\" { procedure Two(): Integer begin exit(2); end; }
        codeunit 50104 \"By Id\" { procedure Three(): Integer begin exit(3); end; }
        codeunit 50102 \"T\" { Subtype = Test; var H: Codeunit \"Helper\";
            [Test] procedure P() var n: Integer; begin n := 50104; if H.Used() <> 1 then Error('x'); end; }
        codeunit 50103 \"Other Test\" { Subtype = Test; [Test] procedure Q() var r: Record \"Row\"; begin r.Insert(); end; }";
    let compile = |lean: bool| {
        let (ws, l) = compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".to_string(), SRC.to_string())],
            }],
            vec![crate::workspace::AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                baseline: false,
                only_tests: Some(vec![(0, 50102)]),
                reach_only: true,
                lean,
                ..Default::default()
            },
        );
        let ids = |pick: &dyn Fn(usize) -> bool| -> Vec<u32> {
            let mut v: Vec<u32> = (0..ws.objects.len())
                .filter(|&oi| pick(oi))
                .filter_map(|oi| ws.objects[oi].id)
                .collect();
            v.sort_unstable();
            v
        };
        let by_reason = ids(&|oi| {
            let refusals: Vec<&String> = l.unsupported[oi].iter().flatten().collect();
            !refusals.is_empty() && refusals.iter().all(|why| why.as_str() == crate::check::LEFT_OUT)
        });
        assert_eq!(
            by_reason,
            ids(&|oi| l.left_out[oi]),
            "Lowered::left_out names the same objects"
        );
        by_reason
    };
    assert_eq!(
        compile(false),
        Vec::<u32>::new(),
        "without lean every object beside the tests compiles"
    );
    assert_eq!(
        compile(true),
        vec![50110, 50111, 50112, 50113],
        "the table, its page, the subscriber and the installer are named by no selected test"
    );
}

/// (line, column) of the position a `Stmt::Pos` marker names.
fn at(l: &Lowered, s: &ir::Stmt) -> (u32, u32) {
    let ir::Stmt::Pos(id) = s else {
        panic!("not a position marker: {s:?}")
    };
    let p = l.program.pos(*id).expect("a position in the table");
    (p.line, p.col)
}

fn routine<'a>(l: &'a Lowered, name: &str) -> (usize, &'a ir::Routine) {
    l.program.objects[0]
        .routines
        .iter()
        .enumerate()
        .find(|(_, r)| r.name == name)
        .expect("routine")
}

const POS_SRC: &str = "codeunit 50100 C
{
    [Scope('OnPrem')]
    procedure P()
    var
        x: Integer;
    begin
        // a comment before the statement
        x := 1;
        if x = 1 then
            x := 2;
        x :=
            3;
    end;

    trigger OnRun()
    begin
        P();
    end;
}
";

#[test]
fn statements_carry_their_line_and_column() {
    let (_, l) = lower_pos(POS_SRC);
    let (_, p) = routine(&l, "P");
    let b = &p.body;
    assert_eq!(b.len(), 6, "{b:?}");
    assert_eq!(at(&l, &b[0]), (9, 9));
    assert!(matches!(b[1], ir::Stmt::Assign(..)));
    assert_eq!(at(&l, &b[2]), (10, 9));
    let ir::Stmt::If(_, then_s, _) = &b[3] else {
        panic!("{:?}", b[3])
    };
    assert_eq!(at(&l, &then_s[0]), (11, 13));
    // A statement that wraps starts on its first line and ends on its last.
    assert_eq!(at(&l, &b[4]), (12, 9));
    let ir::Stmt::Pos(id) = b[4] else { panic!() };
    assert_eq!(l.program.pos(id).unwrap().end_line, 13);
}

#[test]
fn a_routine_position_is_its_keyword_not_its_attribute() {
    let (_, l) = lower_pos(POS_SRC);
    let (oi, p) = routine(&l, "P");
    let pos = l.program.pos(p.pos).expect("routine position");
    assert_eq!(
        (pos.line, pos.col),
        (4, 5),
        "the `procedure` line, not the [Scope] line"
    );
    let (_, t) = routine(&l, "OnRun");
    assert_eq!(l.program.pos(t.pos).map(|p| p.line), Some(16));
    // A statement's position names its object and routine.
    let ir::Stmt::Pos(id) = p.body[0] else { panic!() };
    let sp = l.program.pos(id).unwrap();
    assert_eq!((sp.object as usize, sp.routine), (oi, 0));
    assert_eq!(l.program.sources[sp.file as usize].path, "f.al");
    assert_eq!(l.program.sources[sp.file as usize].app, 0);
}

#[test]
fn without_positions_the_bodies_have_no_markers() {
    let (_, l) = lower_src(POS_SRC);
    let (_, p) = routine(&l, "P");
    assert!(p.body.iter().all(|s| !matches!(s, ir::Stmt::Pos(_))));
    assert!(l.program.positions.is_empty());
    assert_eq!(p.pos, ir::NO_POS);
}

// The page expansion appends routines at the end of the page, which moves the lines of
// every object that follows it in the file. Positions use the author's lines.
#[test]
fn expanding_a_page_does_not_move_the_lines_of_the_next_object() {
    let src = "page 50100 P
{
    layout { area(Content) { field(A; Rec.X) { Visible = not Hide; } } }
    var Hide: Boolean;
}

codeunit 50101 C2
{
    procedure Q()
    begin
        Hide();
    end;
    local procedure Hide() begin end;
}
";
    let (_, l) = lower_pos(src);
    let c2 = l.program.objects.iter().find(|o| o.name == "C2").unwrap();
    let q = c2.routines.iter().find(|r| r.name == "Q").unwrap();
    assert_eq!(l.program.pos(q.pos).map(|p| p.line), Some(9));
    let ir::Stmt::Pos(id) = q.body[0] else {
        panic!("{:?}", q.body)
    };
    assert_eq!(l.program.pos(id).map(|p| p.line), Some(11));
    // The made-up routine has no source position.
    let page = l.program.objects.iter().find(|o| o.name == "P").unwrap();
    let synthetic = page
        .routines
        .iter()
        .find(|r| crate::pages::is_synthetic(&r.name))
        .unwrap();
    assert_eq!(synthetic.pos, ir::NO_POS);
    assert!(synthetic.body.iter().all(|s| !matches!(s, ir::Stmt::Pos(_))));
}

const DOTNET: &str = "dotnet { assembly(\"netstandard\") { Culture = 'neutral'; PublicKeyToken = 'cc7b13ffcd2ddd51'; type(\"System.Convert\"; \"Convert\") { } type(\"System.IO.MemoryStream\"; \"MemoryStream\") { } } }\n";

#[test]
fn dotnet_calls_lower_to_dynamic_member_calls_on_the_declared_type() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P(): Integer var Convert: DotNet Convert; I: Integer; begin I := Convert.ToInt32('12'); exit(I); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    assert_eq!(l.program.dotnet_types.len(), 2);
    let d = &l.program.dotnet_types[0];
    assert_eq!(
        (d.alias.as_str(), d.type_name.as_str(), d.assembly.as_str()),
        ("Convert", "System.Convert", "netstandard")
    );
    assert_eq!(d.public_key_token.as_deref(), Some("cc7b13ffcd2ddd51"));
    let (oi, r) = routine(&l, "P");
    assert_eq!(l.unsupported[oi][0], Vec::<String>::new());
    assert_eq!(r.locals[0].ty, RtType::DotNet(0));
    assert_eq!(
        r.body[0],
        ir::Stmt::Assign(
            ir::Place::Local(1),
            ir::Expr::DotNetCoerce(
                RtType::Integer,
                Box::new(ir::Expr::DotNet {
                    ty: Some(0),
                    receiver: Some(Box::new(ir::Expr::Load(ir::Place::Local(0)))),
                    op: ir::DotNetOp::Call("ToInt32".into()),
                    args: vec![ir::Arg::Value(ir::Expr::Const(ir::Const::Text("12".into())))],
                })
            )
        )
    );
}

#[test]
fn dotnet_constructors_properties_and_by_reference_arguments_lower() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P() var M: DotNet MemoryStream; N: Integer; begin M := M.MemoryStream(16); M.Position := 3; N := M.Length; M.Read(N); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let (oi, r) = routine(&l, "P");
    assert_eq!(l.unsupported[oi][0], Vec::<String>::new());
    let ir::Stmt::Assign(_, ir::Expr::Convert(RtType::DotNet(1), ctor)) = &r.body[0] else {
        panic!("{:?}", r.body[0])
    };
    assert!(matches!(
        **ctor,
        ir::Expr::DotNet {
            ty: Some(1),
            op: ir::DotNetOp::Construct,
            ..
        }
    ));
    assert!(matches!(
        &r.body[1],
        ir::Stmt::Expr(ir::Expr::DotNet { op: ir::DotNetOp::Set(n), args, .. }) if n == "Position" && args.len() == 1
    ));
    assert!(matches!(
        &r.body[2],
        ir::Stmt::Assign(_, ir::Expr::DotNetCoerce(RtType::Integer, e))
            if matches!(**e, ir::Expr::DotNet { op: ir::DotNetOp::Get(ref n), .. } if n == "Length")
    ));
    // A plain variable argument is passed by place, so a by-reference parameter can write it back.
    assert!(matches!(
        &r.body[3],
        ir::Stmt::Expr(ir::Expr::DotNet { op: ir::DotNetOp::Call(n), args, .. })
            if n == "Read" && matches!(args[0], ir::Arg::Ref(ir::Place::Local(1)))
    ));
}

#[test]
fn a_dotnet_alias_without_a_declaration_is_a_lowering_reason() {
    let (_, l) = lower_src("codeunit 50100 C { procedure P() var X: DotNet Missing; begin end; }");
    let (oi, _) = routine(&l, "P");
    assert!(
        l.unsupported[oi][0]
            .iter()
            .any(|r| r.contains("DotNet 'Missing'") && r.contains("no dotnet")),
        "{:?}",
        l.unsupported[oi][0]
    );
}

// session/TestEnvironmentUserTenantContracts.al:204-216 reaches `CopyStr(TenantInfo.PreferredLanguage(), 1, 2)`
// and `IsNull(TenantInfo)` in Microsoft's Azure AD Tenant Impl.: a DotNet result passed to a Text
// parameter must fit it exactly (ADR-014 section 4), and IsNull takes a DotNet value.
#[test]
fn a_dotnet_result_passed_to_a_text_parameter_is_coerced_and_isnull_takes_a_dotnet_value() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P(): Text var Convert: DotNet Convert; begin if IsNull(Convert) then exit(''); exit(CopyStr(Convert.ToBase64String('x'), 1, 2)); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let (oi, r) = routine(&l, "P");
    assert_eq!(l.unsupported[oi][0], Vec::<String>::new());
    let dump = format!("{:?}", r.body);
    assert!(dump.contains("DotNetCoerce(Text(None)"), "{dump}");
}

// Tests-ERM Document Service Management.CheckError: `Error(Service.LastError.Message)`. A DotNet result
// fits the Text overload of Error, not the ErrorInfo one (ADR-024).
#[test]
fn a_dotnet_result_passed_to_error_picks_the_text_overload() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P() var Convert: DotNet Convert; begin Error(Convert.ToBase64String('x')); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let (oi, r) = routine(&l, "P");
    assert_eq!(l.unsupported[oi][0], Vec::<String>::new());
    assert!(format!("{:?}", r.body).contains("DotNetCoerce(Text"), "{:?}", r.body);
}

// NPR install code and Base Application's Job Queue Entry compare with `ReportLayoutType::Rdlc` and `::Word`.
// The members are the documentation's (a documentation-only assumption, docs/probes-needed.md).
#[test]
fn report_layout_type_members_lower() {
    let (ws, l) = lower_src(
        "codeunit 50100 C { procedure P(): Boolean begin exit(ReportLayoutType::Rdlc <> ReportLayoutType::Word); end; }",
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let (oi, _) = routine(&l, "P");
    assert_eq!(l.unsupported[oi][0], Vec::<String>::new());
}

// A `[Native]` procedure of a platform app has an empty AL body: the service tier implements it. Running
// the empty body would answer the default value (the corpus run showed `Agent Utilities.IsAgentSession`
// deciding every session is an agent session, because the native `GetCurrentSessionAgentMetadataProviderType`
// "returned" 0 instead of -1). Such a routine is refused when it runs, not when it lowers (al-runner issue 5684:
// an install routine that can reach `Agent Utilities.SetInstructions` but never calls it runs on BC).
#[test]
fn a_native_procedure_with_an_empty_body_refuses_when_it_runs() {
    let (_, l) = lower_src(
        "codeunit 50100 C { [Native] procedure Kind(): Integer begin end; procedure Other(): Integer begin exit(1); end; }",
    );
    let (ki, kind) = routine(&l, "Kind");
    assert!(l.unsupported[0][ki].is_empty(), "{:?}", l.unsupported[0][ki]);
    assert!(
        matches!(&kind.body[..], [ir::Stmt::Expr(ir::Expr::Refused(why))] if why.contains("Native") && why.contains("Kind")),
        "{:?}",
        kind.body
    );
    let (oi, _) = routine(&l, "Other");
    assert!(l.unsupported[0][oi].is_empty());
}

// ADR-024: a DotNet result used as an operand is settled when the code runs. The comparison with
// a Text literal is a CLR string comparison, so it has its own operator.
#[test]
fn a_dotnet_result_used_as_an_operand_is_settled_at_run_time() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P(): Boolean var Convert: DotNet Convert; begin exit(Convert.ToInt32('1') > 0); end; procedure Q(): Boolean var Convert: DotNet Convert; begin exit(Convert.ToBase64String('x') = 'WA=='); end; procedure R(): Integer var Convert: DotNet Convert; I: Integer; begin for I := 1 to Convert.ToInt32('2') - 1 do exit(-Convert.ToInt32('3')); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    for name in ["P", "Q", "R"] {
        let (ri, _) = routine(&l, name);
        assert!(l.unsupported[0][ri].is_empty(), "{name}: {:?}", l.unsupported[0][ri]);
    }
    let (_, p) = routine(&l, "P");
    let dump = format!("{:?}", p.body);
    assert!(dump.contains("DotNetOperand(") && dump.contains("Order)"), "{dump}");
    let (_, q) = routine(&l, "Q");
    assert!(format!("{:?}", q.body).contains("DotNetEq"), "{:?}", q.body);
    // The bound of a for loop is settled too.
    let (_, r) = routine(&l, "R");
    assert!(format!("{:?}", r.body).contains("DotNetOperand("), "{:?}", r.body);
}

// ADR-024: Format takes an object as it is (its ToString()); a scalar parameter takes a settled
// primitive; a `var` parameter and an AL procedure's DotNet-typed object stay refused.
#[test]
fn a_dotnet_value_passed_to_format_is_a_variant_operand() {
    let (ws, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P(): Text var Convert: DotNet Convert; begin exit(Format(Convert.ToBase64String('x')) + Format(Convert)); end; procedure Q(): Integer var Convert: DotNet Convert; begin exit(StrLen(Convert.ToBase64String('x'))); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    for name in ["P", "Q"] {
        let (ri, _) = routine(&l, name);
        assert!(l.unsupported[0][ri].is_empty(), "{name}: {:?}", l.unsupported[0][ri]);
    }
    let (_, p) = routine(&l, "P");
    assert!(format!("{:?}", p.body).contains("Variant)"), "{:?}", p.body);
    let (_, q) = routine(&l, "Q");
    assert!(format!("{:?}", q.body).contains("Strict)"), "{:?}", q.body);
}

const DOTNET_REGISTRY: &str = "dotnet { assembly(\"Microsoft.Dynamics.Nav.NavUserAccount\") { Culture = 'neutral'; PublicKeyToken = '31bf3856ad364e35'; type(\"Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper\"; \"NavUserAccountHelper\") { } type(\"Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper\"; \"NavTenantSettingsHelper\") { } } assembly(\"Microsoft.Dynamics.Nav.Types\") { Culture = 'neutral'; PublicKeyToken = '31bf3856ad364e35'; type(\"Microsoft.Dynamics.Nav.Types.ALConfigSettings\"; \"ALConfigSettings\") { } } }\n";

/// A member the registry provides declares its AL return type (ADR-014), so its result is an
/// operand like any typed value. Base App: `exit(IsSaaS() and UserAccountHelper.IsAzure())`.
#[test]
fn a_registry_member_with_a_declared_type_is_usable_as_an_operand() {
    let (_, l) = lower_src(&format!(
        "{DOTNET_REGISTRY}codeunit 50100 C {{ procedure P(B: Boolean): Boolean var U: DotNet NavUserAccountHelper; begin if U.IsAzure() then exit(true); exit(B and U.IsAzure()); end; procedure Q(): Boolean var T: DotNet NavTenantSettingsHelper; begin exit(T.GetEnvironmentName() <> ''); end; procedure R(): Boolean var S: DotNet ALConfigSettings; begin S := S.Instance(); exit(S.ApiSubscriptionDelayTime() > 0); end; }}"
    ));
    for name in ["P", "Q", "R"] {
        let (ri, _) = routine(&l, name);
        assert!(l.unsupported[0][ri].is_empty(), "{name}: {:?}", l.unsupported[0][ri]);
    }
}

/// The same member names on a type the registry does not know are untyped, so they are settled
/// when the code runs (ADR-024) instead of refused when it is lowered.
#[test]
fn a_member_of_an_unregistered_type_is_untyped_and_settled_at_run_time() {
    let (_, l) = lower_src(&format!(
        "{DOTNET}codeunit 50100 C {{ procedure P(): Boolean var Convert: DotNet Convert; begin exit(Convert.IsAzure() and true); end; }}"
    ));
    let (oi, r) = routine(&l, "P");
    assert!(l.unsupported[oi][0].is_empty(), "{:?}", l.unsupported[oi][0]);
    assert!(format!("{:?}", r.body).contains("DotNetOperand("), "{:?}", r.body);
}

#[test]
fn slots_for_globals_params_and_locals() {
    let (_, l) = lower_src(
        "codeunit 50100 C { var G: Integer; procedure P(A: Text; var B: Decimal) R: Boolean var L: Code[10]; begin end; }",
    );
    let o = &l.program.objects[0];
    assert_eq!(
        o.globals,
        vec![ir::Slot {
            name: "G".into(),
            ty: RtType::Integer
        }]
    );
    let (_, r) = routine(&l, "P");
    assert_eq!(r.params, 2);
    assert_eq!(r.by_var, vec![false, true]);
    let names: Vec<_> = r.locals.iter().map(|s| (s.name.as_str(), s.ty.clone())).collect();
    assert_eq!(
        names,
        vec![
            ("A", RtType::Text(None)),
            ("B", RtType::Decimal),
            ("R", RtType::Boolean),
            ("L", RtType::Code(Some(10)))
        ]
    );
    assert_eq!(r.ret, Some(RtType::Boolean));
    assert_eq!(r.ret_local, Some(2));
}

#[test]
fn assignment_converts_to_the_slot_type() {
    let (_, l) = lower_src("codeunit 50100 C { procedure P() var D: Decimal; begin D := 1; end; }");
    let (_, r) = routine(&l, "P");
    assert_eq!(
        r.body,
        vec![ir::Stmt::Assign(
            ir::Place::Local(0),
            ir::Expr::Convert(RtType::Decimal, Box::new(ir::Expr::Const(ir::Const::Integer(1))))
        )]
        .into()
    );
}

// ADR-031 item 2.2: a value whose type is the slot's own type needs no conversion. Only types whose conversion is
// the identity are dropped: a Text or Code keeps it (the length of a concatenation is checked there), as does every
// pair of different types.
#[test]
fn an_assignment_between_equal_scalar_types_has_no_convert() {
    let (_, l) = lower_src(
        "codeunit 50100 C { procedure P() var A: Integer; B: Integer; C: Decimal; D: Decimal; G: Boolean; H: Boolean; T1: Date; T2: Date; begin A := B; C := D; G := H; T1 := T2; end; }",
    );
    let (_, r) = routine(&l, "P");
    assert_eq!(r.body.len(), 4, "{:?}", r.body);
    for st in &r.body {
        assert!(
            matches!(st, ir::Stmt::Assign(_, ir::Expr::Load(_))),
            "an equal-type assignment must not convert: {st:?}"
        );
    }
}

#[test]
fn an_assignment_between_different_or_length_checked_types_keeps_its_convert() {
    let (_, l) = lower_src(
        "codeunit 50100 C { procedure P() var D: Decimal; I: Integer; S: Text[10]; U: Text[10]; B: BigInteger; begin D := I; S := U; B := I; end; }",
    );
    let (_, r) = routine(&l, "P");
    for st in &r.body {
        assert!(
            matches!(st, ir::Stmt::Assign(_, ir::Expr::Convert(..))),
            "a conversion that can change the value stays: {st:?}"
        );
    }
}

#[test]
fn compound_assignment_reads_the_place() {
    let (_, l) = lower_src("codeunit 50100 C { procedure P() var I: Integer; begin I += 2; end; }");
    let (_, r) = routine(&l, "P");
    let ir::Stmt::Assign(ir::Place::Local(0), ir::Expr::Convert(_, e)) = &r.body[0] else {
        panic!("{:?}", r.body)
    };
    assert_eq!(
        **e,
        ir::Expr::Binary(
            ir::BinOp::Add,
            Box::new(ir::Expr::Load(ir::Place::Local(0))),
            Box::new(ir::Expr::Const(ir::Const::Integer(2)))
        )
    );
}

#[test]
fn builtin_calls_carry_the_overload_id() {
    let (_, l) = lower_src("codeunit 50100 C { procedure P() var T: Text; begin T := Format(1); end; }");
    let (_, r) = routine(&l, "P");
    let ir::Stmt::Assign(_, ir::Expr::Convert(_, e)) = &r.body[0] else {
        panic!()
    };
    assert!(
        matches!(&**e, ir::Expr::Builtin { id, receiver: None, args, .. } if id.name() == "System.Format#1" && args.len() == 1)
    );
}

#[test]
fn procedure_calls_and_call_graph() {
    let (_, l) = lower_src(
        "codeunit 50100 C { procedure Q(var X: Integer) begin end; procedure P() var I: Integer; begin Q(I); end; }",
    );
    let (qi, _) = routine(&l, "Q");
    let (pi, p) = routine(&l, "P");
    assert_eq!(
        p.body,
        vec![ir::Stmt::Expr(ir::Expr::Call {
            object: 0,
            routine: qi,
            instance: None,
            args: vec![ir::Arg::Ref(ir::Place::Local(0))],
            try_call: false,
        })]
        .into()
    );
    assert_eq!(l.calls[0][pi], vec![(0, qi)]);
}

#[test]
fn format_of_an_option_field_with_captions_formats_the_caption() {
    // corpus fieldref/TestPrecompiledFieldOptionCaption.al:47-55
    let (_, l) = lower_src(
        "table 1 T { fields { field(1; K; Code[10]) { } field(2; St; Option) { OptionMembers = Draft,\"Validaton in Process\"; OptionCaption = 'Draft,Validation in Process'; } field(3; Plain; Option) { OptionMembers = A,B; } } }\n\
             codeunit 2 C { procedure P(): Text var R: Record T; begin exit(Format(R.St)); end; procedure Q(): Text var R: Record T; begin exit(Format(R.Plain)); end; procedure W(): Text var R: Record T; begin exit(Format(R.St, 0, 9)); end; }",
    );
    let arg0 = |name: &str| -> Option<ir::Expr> {
        let c = l.program.objects.iter().position(|o| o.name == "C")?;
        let r = l.program.objects[c].routines.iter().find(|r| r.name == name)?;
        let ir::Stmt::Exit(Some(ir::Expr::Builtin { args, .. })) = &r.body[0] else {
            return None;
        };
        let ir::Arg::Value(v) = &args[0] else { return None };
        Some(v.clone())
    };
    let want = RtType::Option(vec!["Draft".into(), "Validation in Process".into()]);
    assert!(
        matches!(arg0("P"), Some(ir::Expr::Convert(t, _)) if t == want),
        "{:?}",
        arg0("P")
    );
    // No caption list: untouched.
    assert!(
        !matches!(arg0("Q"), Some(ir::Expr::Convert(RtType::Option(_), _))),
        "{:?}",
        arg0("Q")
    );
    // Every Format form reads the caption list (text/TestProbeTextDateFormatContracts.al:
    // Format_OptionFieldAndOptionVariable), standard format 9 included.
    let c = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let w = l.program.objects[c]
        .routines
        .iter()
        .position(|r| r.name == "W")
        .unwrap();
    assert!(l.unsupported[c][w].is_empty(), "{:?}", l.unsupported[c][w]);
}

#[test]
fn a_builtin_newer_than_the_target_runtime_makes_the_routine_unsupported() {
    let src = "codeunit 50100 C { procedure P() begin Codeunit.Run('Some Codeunit'); end; procedure Ok() begin end; }";
    let apps = || {
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }]
    };
    let target = |bc: &str| al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let (_, l) = compile_for(apps(), &Default::default(), &target("27.5"));
    let (pi, _) = routine(&l, "P");
    let (oi, _) = routine(&l, "Ok");
    assert_eq!(
        l.unsupported[0][pi],
        vec![
            "AL0666 'Run' is not available in runtime version '16.0'. The supported runtime versions are: '17.0' or greater."
                .to_string()
        ],
        "refused at runtime 16.0"
    );
    assert!(l.unsupported[0][oi].is_empty());
    // At runtime 17.0 the by-name overload exists; the call lowers (the VM refuses it later).
    let (_, l) = compile_for(apps(), &Default::default(), &target("28"));
    assert!(
        !l.unsupported[0][pi].iter().any(|r| r.contains("AL0666")),
        "{:?}",
        l.unsupported[0][pi]
    );
}

#[test]
fn an_app_below_the_runtime_floor_runs_only_the_routines_whose_builtins_the_compiler_accepts() {
    // al-runner issue 5573: an app that declares runtime 13.0 calls NavApp.GetResourceAsText (alc: 14.0 or greater).
    // The AL compiler rejects it (AL0666, catalog/alc-runtimes.txt); the routine is refused, the others run.
    let src = "codeunit 50100 \"Resource Reader\" { procedure Value(): Text begin exit(NavApp.GetResourceAsText('value.txt', TextEncoding::UTF8)); end; procedure Ok(): Text begin exit(UpperCase('x')); end; }";
    let apps = || {
        vec![AppSource {
            id: "6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e11".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }]
    };
    let mut target = al_version::Target::for_bc(al_version::BcVersion::parse("28.5").unwrap());
    target.app_runtimes = vec![(
        "6F1D2F8E-5B0A-4C1E-9A43-0D6B8A2C5E11".into(),
        al_version::Runtime { major: 13, minor: 0 },
    )];
    let (_, l) = compile_for(apps(), &Default::default(), &target);
    let (vi, _) = routine(&l, "Value");
    let (oi, _) = routine(&l, "Ok");
    assert_eq!(l.unsupported[0][vi].len(), 1, "{:?}", l.unsupported[0][vi]);
    assert!(
        l.unsupported[0][vi][0].starts_with("AL0666 'GetResourceAsText' is not available in runtime version '13.0'"),
        "{:?}",
        l.unsupported[0][vi]
    );
    assert!(l.unsupported[0][oi].is_empty(), "{:?}", l.unsupported[0][oi]);
    // Declared at 14.0 the call is cleared.
    target.app_runtimes[0].1 = al_version::Runtime { major: 14, minor: 0 };
    let (_, l) = compile_for(apps(), &Default::default(), &target);
    assert!(l.unsupported[0][vi].is_empty(), "{:?}", l.unsupported[0][vi]);
}

#[test]
fn unimplemented_builtin_makes_the_routine_unsupported() {
    let (_, l) = lower_src("codeunit 50100 C { procedure P() begin SerialNumber(); end; procedure Ok() begin end; }");
    let (pi, _) = routine(&l, "P");
    let (oi, _) = routine(&l, "Ok");
    assert_eq!(
        l.unsupported[0][pi],
        vec!["builtin Database.SerialNumber#1 is not implemented".to_string()]
    );
    assert!(l.unsupported[0][oi].is_empty());
}

#[test]
fn event_publisher_calls_lower_including_isolated_ones() {
    let (_, l) = lower_src(
        "codeunit 50100 C {
                [IntegrationEvent(false, false)] procedure OnA() begin end;
                [BusinessEvent(false)] procedure OnB() begin end;
                [InternalEvent(false)] procedure OnC() begin end;
                [IntegrationEvent(false, false, true)] procedure OnIso() begin end;
                [BusinessEvent(false, true)] procedure OnIsoB() begin end;
                procedure Plain() begin OnA(); OnB(); OnC(); end;
                procedure Iso() begin OnIso(); OnIsoB(); end;
            }",
    );
    let reasons = |n: &str| l.unsupported[0][routine(&l, n).0].clone();
    assert!(reasons("Plain").is_empty(), "{:?}", reasons("Plain"));
    assert!(reasons("Iso").is_empty(), "{:?}", reasons("Iso"));
}

#[test]
fn control_flow_and_asserterror() {
    let (_, l) = lower_src(
        "codeunit 50100 C { procedure P() var I: Integer; begin if I = 0 then I := 1 else exit; asserterror Error('x'); end; }",
    );
    let (pi, p) = routine(&l, "P");
    assert!(l.unsupported[0][pi].is_empty(), "{:?}", l.unsupported[0][pi]);
    assert!(matches!(p.body[0], ir::Stmt::If(_, ref t, ref e) if t.len() == 1 && e == &vec![ir::Stmt::Exit(None)]));
    assert!(matches!(p.body[1], ir::Stmt::AssertError(ref b) if b.len() == 1));
}

const TABLE: &str = "table 50900 \"ALT T\"
{
    fields
    {
        field(1; \"No.\"; Code[20]) { }
        field(2; Amount; Decimal) { InitValue = 5; trigger OnValidate() begin Amount := Amount * 2; end; }
        field(3; Counter; Integer) { AutoIncrement = true; }
        field(4; Total; Decimal) { FieldClass = FlowField; }
    }
    keys { key(PK; \"No.\") { } key(ByAmount; Amount) { } }
    procedure Double(X: Integer): Integer begin exit(X * 2); end;
    procedure SetAmount() begin Amount := 1; end;
}";

/// An enum field's InitValue by name when the enum is in another namespace, named by a
/// qualified path: Base App's `Item Journal Line.Type` is
/// `Enum Microsoft.Manufacturing.Capacity."Capacity Type Journal"` with `InitValue = " "`
/// (corpus record/TestRenamePropagationBaseApp.al, Codeunit 60975).
#[test]
fn enum_init_value_with_namespaced_enum() {
    let (_, l) = lower_files(
        &[
            (
                "e.al",
                "namespace A.B; enum 50920 \"ALT Cap\" { value(0; \" \") { } value(1; \"Machine Center\") { } value(2; Work) { } }",
            ),
            (
                "t.al",
                "namespace C.D; table 50921 \"ALT Tab\" { fields { field(1; K; Code[10]) { } \
                     field(2; T; Enum A.B.\"ALT Cap\") { InitValue = \" \"; } \
                     field(3; U; Enum A.B.\"ALT Cap\") { InitValue = Work; } } keys { key(PK; K) { } } }",
            ),
        ],
        false,
    );
    let t = l
        .program
        .objects
        .iter()
        .find_map(|o| o.table.as_ref().filter(|t| t.fields.iter().any(|f| f.name == "T")))
        .expect("table schema");
    let f = |n: &str| t.fields.iter().find(|f| f.name == n).unwrap();
    assert!(matches!(f("T").init, Some(ir::Const::Enum(_, 0))), "{:?}", f("T").init);
    assert!(matches!(f("U").init, Some(ir::Const::Enum(_, 2))), "{:?}", f("U").init);
}

/// A dependency app's TableRelation is fixed when that app compiled, against the apps it saw:
/// a same-named table of a dependent app does not take it over (corpus
/// fieldref/TestRelationTargetNameCollision.al: Customer."Shipping Agent Code" relates to the
/// Base Application table 291, not to the dependent app's table 60990).
#[test]
fn a_dependency_apps_relation_resolves_among_the_apps_it_saw() {
    use crate::workspace::AppKind;
    let src = |id: &str, deps: &[&str], files: &[(&str, &str)]| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
    };
    let dep = src(
        "dep",
        &[],
        &[
            (
                "cust.al",
                "namespace D.Sales; using D.Ship; table 18 Cust { fields { field(1; K; Code[10]) { } \
                     field(2; Agent; Code[10]) { TableRelation = \"Ship Agent\"; } } keys { key(PK; K) { } } }",
            ),
            (
                "ship.al",
                "namespace D.Ship; table 291 \"Ship Agent\" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }",
            ),
        ],
    );
    let main = src(
        "main",
        &["dep"],
        &[
            (
                "m.al",
                "namespace M; using D.Sales; codeunit 1 C { var X: Record Cust; }",
            ),
            (
                "local.al",
                "namespace M; table 60990 \"Ship Agent\" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }",
            ),
        ],
    );
    let ws = Workspace::load_apps(
        vec![main, dep],
        vec![AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let l = lower_with_positions(&ws, &members, &checked, false);
    let cust = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "Cust")
        .expect("Cust is compiled");
    let agent = cust
        .table
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == "Agent")
        .unwrap();
    let target = agent.relation_arms[0].table.map(|t| l.program.objects[t].id);
    assert_eq!(target, Some(Some(291)), "{:?}", agent.relation_arms);
}

// Base Application's `AdjustAddReportingCurrency.Report.al` declares
// `dataitem(Integer; System.Utilities.Integer)`: a data item whose table is written with its namespace.
#[test]
fn a_dataitem_table_written_with_its_namespace_resolves() {
    use crate::workspace::AppKind;
    let src = |id: &str, deps: &[&str], files: &[(&str, &str)]| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
    };
    let dep = src(
        "dep",
        &[],
        &[(
            "ship.al",
            "namespace D.Ship; table 291 \"Ship Agent\" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }",
        )],
    );
    let main = src(
        "main",
        &["dep"],
        &[(
            "r.al",
            "report 50000 R { ProcessingOnly = true; UseRequestPage = false; \
                 dataset { dataitem(Agent; D.Ship.\"Ship Agent\") { } } \
                 trigger OnPreReport() begin end; } \
                 codeunit 50001 C { var Rep: Report R; procedure P() begin end; }",
        )],
    );
    let ws = Workspace::load_apps(
        vec![main, dep],
        vec![AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let l = lower_with_positions(&ws, &members, &checked, false);
    let why: Vec<_> = l
        .unsupported
        .iter()
        .enumerate()
        .flat_map(|(o, rs)| {
            let name = &l.program.objects[o].name;
            rs.iter().flatten().map(move |r| format!("{name}: {r}"))
        })
        .collect();
    assert!(why.is_empty(), "{why:?}");
}

fn table_ir(l: &Lowered) -> &ir::Table {
    l.program.objects[0].table.as_ref().expect("table schema")
}

#[test]
fn table_schema_fields_keys_and_system_fields() {
    let (_, l) = lower_src(TABLE);
    let t = table_ir(&l);
    let names: Vec<_> = t.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(&names[..4], ["No.", "Amount", "Counter", "Total"]);
    assert!(names.contains(&"SystemId"));
    assert_eq!(t.keys, vec![vec![0], vec![1]]);
    assert_eq!(t.fields[1].init, Some(ir::Const::Decimal("5".into())));
    assert!(t.fields[2].auto_increment);
    assert!(!t.fields[3].stored, "FlowFields are not stored");
    assert!(t.fields[1].on_validate.is_some());
    assert!(t.per_company);
}

const FLOW_TABLES: &str = "
enum 50910 \"ALT Kind\" { value(0; \" \") { } value(1; \"Initial Entry\") { } value(5; Application) { } }
table 50911 \"ALT Line\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(2; \"Doc No.\"; Code[20]) { }
        field(3; Amount; Decimal) { }
        field(4; Kind; Enum \"ALT Kind\") { }
        field(5; St; Option) { OptionMembers = Open,Released,Closed; }
        field(6; Open; Boolean) { }
        field(7; \"Table ID\"; Integer) { }
    }
}
table 50912 \"ALT Head\"
{
    fields
    {
        field(1; \"No.\"; Code[20]) { }
        field(2; \"Date Filter\"; Date) { FieldClass = FlowFilter; }
        field(3; Totaling; Text[250]) { }
        field(10; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum(\"ALT Line\".Amount where(\"Doc No.\" = field(\"No.\"))); }
        field(11; Neg; Integer) { FieldClass = FlowField; CalcFormula = -count(\"ALT Line\" where(\"Doc No.\" = field(\"No.\"), Open = const(true), St = const(Closed), \"Table ID\" = const(Database::\"ALT Line\"))); }
        field(12; Quoted; Decimal) { FieldClass = FlowField; CalcFormula = sum(\"ALT Line\".Amount where(Kind = filter(\"Initial Entry\" | Application))); }
        field(13; Limit; Decimal) { FieldClass = FlowField; CalcFormula = sum(\"ALT Line\".Amount where(\"Doc No.\" = field(\"No.\"), Kind = const(\"Initial Entry\"))); }
        field(14; Lim2; Decimal) { FieldClass = FlowField; CalcFormula = sum(\"ALT Line\".Amount where(\"Entry No.\" = field(upperlimit(\"Date Filter\")), \"Doc No.\" = field(filter(Totaling)))); }
        field(15; NoFormula; Decimal) { FieldClass = FlowField; }
        field(16; Outside; Integer) { FieldClass = FlowField; CalcFormula = count(Date where(\"Period Type\" = const(Date))); }
        field(17; BadField; Integer) { FieldClass = FlowField; CalcFormula = count(\"ALT Line\" where(Nope = const(1))); }
        field(18; Both; Integer) { FieldClass = FlowField; CalcFormula = count(\"ALT Line\" where(Open = const(true), Open = const(false))); }
    }
}";

fn flow(l: &Lowered, table: &str, field: &str) -> ir::Field {
    let o = l.program.objects.iter().find(|o| o.name == table).unwrap();
    o.table
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == field)
        .unwrap_or_else(|| panic!("{field}"))
        .clone()
}

#[test]
fn flow_filters_and_flow_fields_are_marked_and_lowered() {
    let (_, l) = lower_src(FLOW_TABLES);
    let df = flow(&l, "ALT Head", "Date Filter");
    assert!(df.flow_filter && !df.stored && df.calc.is_none());
    let t = flow(&l, "ALT Head", "Total");
    assert!(!t.flow_filter && !t.stored);
    // Source table "ALT Line" is object 1 (enum 0, line 1, head 2); Amount is position 2.
    assert_eq!(
        t.calc,
        Some(ir::CalcFormula {
            method: ir::CalcMethod::Sum,
            negate: false,
            table: 1,
            field: Some(2),
            conds: vec![ir::CalcCond {
                field: 1,
                value: ir::CalcValue::Field(0)
            }],
        })
    );
    assert_eq!(t.calc_refusal, None);
}

#[test]
fn calc_formula_consts_are_typed_as_the_source_field() {
    let (_, l) = lower_src(FLOW_TABLES);
    let n = flow(&l, "ALT Head", "Neg");
    let f = n.calc.clone().unwrap_or_else(|| panic!("{:?}", n.calc_refusal));
    assert!(f.negate && f.method == ir::CalcMethod::Count && f.field.is_none());
    let consts: Vec<_> = f.conds[1..].iter().map(|c| c.value.clone()).collect();
    assert_eq!(
        consts,
        vec![
            ir::CalcValue::Const(ir::Const::Boolean(true)),
            ir::CalcValue::Const(ir::Const::Option(
                2,
                vec!["Open".into(), "Released".into(), "Closed".into()]
            )),
            // Database::"ALT Line" is that table's object id.
            ir::CalcValue::Const(ir::Const::Integer(50911)),
        ]
    );
    let k = flow(&l, "ALT Head", "Limit").calc.expect("lowered");
    assert_eq!(k.conds[1].value, ir::CalcValue::Const(ir::Const::Enum(0, 1)));
}

#[test]
fn table_relation_where_object_constants_resolve_to_the_object_id() {
    // corpus record/CdcOwner.Table.al:67,79 (`Database::"CDC Owner"`, `Report::"ALT Simple Report"`).
    let (_, l) = lower_src(
            "table 50911 \"ALT Line\" { fields { field(1; Code; Code[20]) { } field(7; \"Table ID\"; Integer) { } field(8; \"Report ID\"; Integer) { } } }
             report 50950 \"ALT Rep\" { }
             table 50912 \"ALT Head\" { fields {
                field(1; \"No.\"; Code[20]) { }
                field(2; D; Code[20]) { TableRelation = \"ALT Line\".Code where(\"Table ID\" = const(Database::\"ALT Line\")); }
                field(3; R; Code[20]) { TableRelation = \"ALT Line\".Code where(\"Report ID\" = const(Report::\"ALT Rep\")); }
             } }",
        );
    let text = |name: &str| {
        let f = flow(&l, "ALT Head", name);
        match &f.relation_arms[0].wheres.as_ref().expect("wheres understood")[0] {
            ir::RelationWhere::Value { text, .. } => text.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(text("D"), "50911");
    assert_eq!(text("R"), "50950");
}

#[test]
fn table_relation_condition_on_the_blank_enum_member_resolves() {
    // Base Application "Standard Service Line"."No.": `if (Type = const(" ")) "Standard Text" else
    // if (Type = const(Item)) Item ...`. The blank member is the enum value named `" "` (the corpus
    // declares such members: _fixtures/enums/ALTStatus.al:5); trimming the name lost it.
    let (_, l) = lower_src(
            "enum 50913 \"ALT Line Kind\" { value(0; \" \") { } value(1; Item) { } }
             table 50914 \"ALT Text\" { fields { field(1; Code; Code[20]) { } } }
             table 50915 \"ALT Item\" { fields { field(1; \"No.\"; Code[20]) { } } }
             table 50916 \"ALT Std Line\" { fields {
                field(1; Kind; Enum \"ALT Line Kind\") { }
                field(2; \"No.\"; Code[20]) { TableRelation = if (Kind = const(\" \")) \"ALT Text\" else if (Kind = const(Item)) \"ALT Item\"; }
             } }",
        );
    let f = flow(&l, "ALT Std Line", "No.");
    let conds: Vec<_> = f.relation_arms.iter().map(|a| a.conditions[0].value.clone()).collect();
    assert_eq!(conds.len(), 2, "{:?}", f.relation_arms);
    assert!(matches!(conds[0], ir::Const::Enum(_, 0)), "{conds:?}");
    assert!(matches!(conds[1], ir::Const::Enum(_, 1)), "{conds:?}");
}

#[test]
fn table_relation_const_blank_on_an_option_resolves_only_when_the_blank_member_is_first() {
    // record/lt-table-relation-arms/TableRelationArms.al (OptionMembers = " ",A, const(" ") selects the first arm) pins the blank
    // member at ordinal 0; a blank member elsewhere is not pinned and the relation is not understood.
    let src = |members: &str| {
        format!(
            "table 50930 \"ALT One\" {{ fields {{ field(1; Code; Code[20]) {{ }} }} }}
             table 50931 \"ALT Two\" {{ fields {{ field(1; Code; Code[20]) {{ }} }} }}
             table 50932 \"ALT Opt Line\" {{ fields {{
                field(1; Opt; Option) {{ OptionMembers = {members}; }}
                field(2; \"No.\"; Code[20]) {{ TableRelation = if (Opt = const(\" \")) \"ALT One\" else if (Opt = const(A)) \"ALT Two\"; }}
             }} }}"
        )
    };
    let (_, l) = lower_src(&src("\" \",A"));
    let f = flow(&l, "ALT Opt Line", "No.");
    assert_eq!(f.relation_arms.len(), 2, "{:?}", f.relation_arms);
    let (_, l) = lower_src(&src("A,\" \""));
    let f = flow(&l, "ALT Opt Line", "No.");
    assert!(f.relation_arms.is_empty(), "{:?}", f.relation_arms);
}

#[test]
fn table_relation_where_filter_rewrites_double_quoted_names() {
    // Base Application's `TableRelation = Item where(Type = filter(Inventory | "Non-Inventory"))`
    // (Prod. Order Component): the filter grammar quotes with `'`, so the AL quoting is rewritten
    // the way a CalcFormula filter's is (corpus record/TestCalcFormulaQuotedFilterTests.Codeunit.al:9).
    let (_, l) = lower_src(
            "enum 50920 \"ALT Kind\" { value(0; Inventory) { } value(1; \"Non-Inventory\") { } }
             table 50921 \"ALT Item\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Type; Enum \"ALT Kind\") { } } }
             table 50922 \"ALT Comp\" { fields {
                field(1; \"No.\"; Code[20]) { }
                field(2; Item; Code[20]) { TableRelation = \"ALT Item\" where(Type = filter(Inventory | \"Non-Inventory\")); }
             } }",
        );
    let f = flow(&l, "ALT Comp", "Item");
    match &f.relation_arms[0].wheres.as_ref().expect("wheres understood")[0] {
        ir::RelationWhere::Value { text, .. } => assert_eq!(text, "Inventory | 'Non-Inventory'"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn calc_formula_filters_rewrite_double_quotes_and_flow_references_resolve() {
    let (_, l) = lower_src(FLOW_TABLES);
    let q = flow(&l, "ALT Head", "Quoted").calc.expect("lowered");
    assert_eq!(
        q.conds[0].value,
        ir::CalcValue::Filter("'Initial Entry' | Application".into())
    );
    let m = flow(&l, "ALT Head", "Lim2").calc.expect("lowered");
    // "Date Filter" is position 1 of ALT Head, Totaling position 2.
    assert_eq!(m.conds[0].value, ir::CalcValue::UpperLimit(1));
    assert_eq!(m.conds[1].value, ir::CalcValue::FieldFilter(2));
}

#[test]
fn a_record_of_a_table_that_is_not_normal_or_temporary_is_refused() {
    // A table served by a table connection the runtime does not model (ExternalSQL here) is refused.
    // A CRM table is lowered: its first use fails at run time while no connection is registered
    // (session/TestCrmTableNoConnectionContracts.al), and with one it is refused there.
    let (_, l) = lower_src(
        "table 50960 Crm { TableType = ExternalSQL; fields { field(1; Id; Guid) { } } }
             table 50961 Plain { TableType = Normal; fields { field(1; Id; Integer) { } } }
             codeunit 50962 C {
                procedure A() var R: Record Crm; begin R.Init(); end;
                procedure B() var R: Record Plain; begin R.Init(); end;
             }",
    );
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let reasons = |n: &str| {
        let ri = l.program.objects[oi].routines.iter().position(|r| r.name == n).unwrap();
        l.unsupported[oi][ri].clone()
    };
    assert!(
        reasons("A").iter().any(|r| r.contains("TableType = ExternalSQL")),
        "{:?}",
        reasons("A")
    );
    assert!(reasons("B").is_empty(), "{:?}", reasons("B"));
}

#[test]
fn unusable_formulas_carry_a_reason_and_a_missing_formula_carries_none() {
    let (_, l) = lower_src(FLOW_TABLES);
    let nf = flow(&l, "ALT Head", "NoFormula");
    assert!(nf.calc.is_none() && nf.calc_refusal.is_none() && !nf.stored && !nf.flow_filter);
    let out = flow(&l, "ALT Head", "Outside");
    assert!(out.calc.is_none());
    assert!(
        out.calc_refusal.as_deref().unwrap().contains("Date"),
        "{:?}",
        out.calc_refusal
    );
    let bad = flow(&l, "ALT Head", "BadField");
    assert!(
        bad.calc_refusal.as_deref().unwrap().contains("Nope"),
        "{:?}",
        bad.calc_refusal
    );
    let both = flow(&l, "ALT Head", "Both");
    assert!(
        both.calc_refusal.as_deref().unwrap().contains("same source field"),
        "{:?}",
        both.calc_refusal
    );
}

// The runtime stamps SystemId, SystemRowVersion and the created/modified pairs on writes
// (al-vm hostvalues.rs), so a formula may read any of them
// (record/TestCalcFormulaSystemFieldsTests.Codeunit.al:120-169).
#[test]
fn formulas_over_system_fields_are_lowered() {
    let src = "table 50920 L { fields { field(1; K; Integer) { } field(2; Ref; Guid) { } } }
table 50921 H { fields { field(1; K; Integer) { }
    field(2; A; DateTime) { FieldClass = FlowField; CalcFormula = max(L.SystemCreatedAt where(K = field(K))); }
    field(3; B; Guid) { FieldClass = FlowField; CalcFormula = lookup(L.SystemModifiedBy where(K = field(K))); }
    field(4; C; Integer) { FieldClass = FlowField; CalcFormula = count(L where(SystemCreatedBy = field(SystemId))); }
    field(5; D; Integer) { FieldClass = FlowField; CalcFormula = count(L where(Ref = field(SystemId))); }
    field(6; E; BigInteger) { FieldClass = FlowField; CalcFormula = max(L.SystemRowVersion where(K = field(K))); } } }";
    let (_, l) = lower_src(src);
    for name in ["A", "B", "C", "D", "E"] {
        let f = flow(&l, "H", name);
        assert!(f.calc.is_some(), "{name}: {:?}", f.calc_refusal);
    }
}

#[test]
fn routines_reading_system_fields_are_lowered() {
    let src = format!(
            "codeunit 50100 C {{ procedure Stamped() var R: Record \"ALT T\"; D: DateTime; begin D := R.SystemModifiedAt; end;
             procedure Id() var R: Record \"ALT T\"; G: Guid; begin G := R.SystemId; end; }}\n{TABLE}"
        );
    let (_, l) = lower_src(&src);
    let (stamped, _) = routine(&l, "Stamped");
    let (id, _) = routine(&l, "Id");
    let oi = 0;
    assert!(
        l.unsupported[oi][stamped].is_empty(),
        "{:?}",
        l.unsupported[oi][stamped]
    );
    assert!(l.unsupported[oi][id].is_empty(), "{:?}", l.unsupported[oi][id]);
}

// corpus _fixtures/ALTRelationScopeExtension.al: a name in a tableextension's CalcFormula means
// what it means in the extension's file (own namespace first), not in the extended table's.
#[test]
fn a_table_extension_formula_resolves_names_in_the_extensions_file() {
    let files = vec![
            (
                "a.al".to_string(),
                "namespace N1;
                table 70001 Pkg { fields { field(1; Code; Code[10]) { } } }
                table 70000 Base { fields { field(1; Code; Code[10]) { } } }"
                    .to_string(),
            ),
            (
                "b.al".to_string(),
                "namespace N2;
                using N1;
                table 70002 Pkg { fields { field(1; Code; Code[10]) { } } }
                tableextension 70003 Ext extends Base { fields {
                    field(5; Rows; Integer) { FieldClass = FlowField; CalcFormula = count(Pkg where(Code = field(Code))); } } }"
                    .to_string(),
            ),
        ];
    let (_, l) = crate::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files,
        }],
        &Default::default(),
    );
    let local = l.program.objects.iter().position(|o| o.id == Some(70002)).unwrap();
    let f = flow(&l, "Base", "Rows");
    assert_eq!(f.calc.map(|c| c.table), Some(local), "{:?}", f.calc_refusal);
}

#[test]
fn namespace_qualified_object_types_resolve_in_the_workspace() {
    // tableextensiontrigger/TestTableExtTargetNamespace_Tests.al:58 declares
    // `Record ALLanguage.Coverage.TxnLocal."Shipment Method"`, a workspace table.
    let files = vec![
        (
            "t.al".to_string(),
            "namespace A.B; table 70010 \"Ship. Method\" { fields { field(1; Code; Code[10]) { } } }".to_string(),
        ),
        (
            "c.al".to_string(),
            "codeunit 70011 C { procedure P() var R: Record A.B.\"Ship. Method\"; begin end;
                    procedure Q(): Integer begin exit(Database::A.B.\"Ship. Method\"); end; }"
                .to_string(),
        ),
    ];
    let (_, l) = crate::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files,
        }],
        &Default::default(),
    );
    let c = l.program.objects.iter().position(|o| o.id == Some(70011)).unwrap();
    let p = l.program.objects[c]
        .routines
        .iter()
        .position(|r| r.name == "P")
        .unwrap();
    assert!(l.unsupported[c][p].is_empty(), "{:?}", l.unsupported[c][p]);
    // TestTableExtTargetNamespace_Tests.al: `Database::ALLanguage.Coverage.TxnLocal."Shipment Method"`.
    let q = l.program.objects[c]
        .routines
        .iter()
        .position(|r| r.name == "Q")
        .unwrap();
    assert!(l.unsupported[c][q].is_empty(), "{:?}", l.unsupported[c][q]);
    assert!(format!("{:?}", l.program.objects[c].routines[q].body).contains("Integer(70010)"));
}

#[test]
fn option_fields_carry_their_option_members() {
    // types/TestOption.al:Option_Format_ReturnsName -- Format(option=1) is 'Draft'.
    let (_, l) = lower_src(
        "table 50901 K { fields {
                field(1; A; Integer) { }
                field(2; O; Option) { OptionMembers = \" \",Draft,\"In Work\"; }
                field(3; P; Option) { OptionMembers = ,Alpha; }
                field(4; Q; Option) { }
            } }",
    );
    let t = table_ir(&l);
    assert_eq!(
        t.fields[1].ty,
        RtType::Option(vec![" ".into(), "Draft".into(), "In Work".into()])
    );
    assert_eq!(t.fields[2].ty, RtType::Option(vec!["".into(), "Alpha".into()]));
    assert_eq!(t.fields[3].ty, RtType::Option(vec!["".into()]));
}

#[test]
fn audit_system_fields_are_lowered_now_that_the_runtime_stamps_them() {
    // record/TestBCSystemFieldContracts.al reads SystemCreatedAt/By and SystemModifiedAt/By;
    // the session stamps them from the host's clock and identity (al-vm hostvalues.rs).
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{
                procedure Read() var R: Record \"ALT T\"; D: DateTime; begin D := R.SystemModifiedAt; end;
                procedure Filter() var R: Record \"ALT T\"; begin R.SetRange(SystemCreatedBy); end;
                procedure Ok() var R: Record \"ALT T\"; G: Guid; begin G := R.SystemId; end;
            }}"
    );
    let (_, l) = lower_src(&src);
    let find = |n: &str| l.program.objects[1].routines.iter().position(|r| r.name == n).unwrap();
    for n in ["Read", "Filter", "Ok"] {
        assert!(
            l.unsupported[1][find(n)].is_empty(),
            "{n}: {:?}",
            l.unsupported[1][find(n)]
        );
    }
}

#[test]
fn a_removed_key_takes_no_index_position() {
    // corpus recordref/TestKeyMetadataRowVersionRemoved.al:10-20: KeyIndex() skips the
    // Removed key's position while a Pending key stays.
    let (_, l) = lower_src(
            "table 50901 R { fields { field(1; A; Integer) { } field(2; B; Integer) { } field(3; C; Integer) { } }
             keys { key(PK; A) { } key(P; B) { ObsoleteState = Pending; ObsoleteReason = 'x'; ObsoleteTag = '1'; }
                    key(Gone; B) { ObsoleteState = Removed; ObsoleteReason = 'x'; ObsoleteTag = '1'; } key(Live; C) { } } }",
        );
    assert_eq!(table_ir(&l).keys, vec![vec![0], vec![1], vec![2]]);
}

#[test]
fn without_keys_the_first_field_is_the_primary_key() {
    let (_, l) = lower_src("table 50901 K { fields { field(1; A; Integer) { } field(2; B; Integer) { } } }");
    assert_eq!(table_ir(&l).keys, vec![vec![0]]);
}

#[test]
fn record_fields_are_places() {
    let src =
        format!("{TABLE}\ncodeunit 50100 C {{ procedure P() var R: Record \"ALT T\"; begin R.\"No.\" := 'a'; end; }}");
    let (_, l) = lower_src(&src);
    let p = l.program.objects[1].routines.iter().find(|r| r.name == "P").unwrap();
    assert_eq!(p.locals[0].ty, RtType::Record(0, false));
    assert_eq!(
        p.body,
        vec![ir::Stmt::Assign(
            ir::Place::Field(Box::new(ir::Place::Local(0)), 0),
            ir::Expr::Convert(
                RtType::Code(Some(20)),
                Box::new(ir::Expr::Const(ir::Const::Text("a".into())))
            )
        )]
        .into()
    );
}

#[test]
fn implicit_rec_in_table_code() {
    let (_, l) = lower_src(TABLE);
    let o = &l.program.objects[0];
    let rec = o.rec_slot.expect("Rec slot");
    assert_eq!(o.globals[rec].ty, RtType::Record(0, false));
    let p = o.routines.iter().find(|r| r.name == "SetAmount").unwrap();
    assert!(matches!(&p.body[0], ir::Stmt::Assign(ir::Place::Field(b, 1), _) if **b == ir::Place::Global(rec)));
    let v = o.routines.iter().find(|r| r.name == "OnValidate").unwrap();
    assert_eq!(v.field, Some(1));
}

#[test]
fn record_methods_know_whether_their_result_is_used() {
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{ procedure P() var R: Record \"ALT T\"; begin R.Insert(); if R.Get('a') then; end; }}"
    );
    let (_, l) = lower_src(&src);
    let p = l.program.objects[1].routines.iter().find(|r| r.name == "P").unwrap();
    assert!(
        matches!(&p.body[0], ir::Stmt::Expr(ir::Expr::Builtin { id, receiver: Some(r), ret_used: false, .. }) if id.name() == "Record.Insert#1" && **r == ir::Arg::Ref(ir::Place::Local(0))),
        "{:?}",
        p.body[0]
    );
    assert!(
        matches!(&p.body[1], ir::Stmt::If(ir::Expr::Builtin { ret_used: true, .. }, _, _)),
        "{:?}",
        p.body[1]
    );
}

#[test]
fn field_arguments_become_field_positions() {
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{ procedure P() var R: Record \"ALT T\"; begin R.SetRange(Amount, 1); end; }}"
    );
    let (_, l) = lower_src(&src);
    let p = l.program.objects[1].routines.iter().find(|r| r.name == "P").unwrap();
    let ir::Stmt::Expr(ir::Expr::Builtin { args, .. }) = &p.body[0] else {
        panic!("{:?}", p.body)
    };
    assert_eq!(args[0], ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(1))));
}

#[test]
fn table_procedures_run_on_the_record() {
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{ procedure P(): Integer var R: Record \"ALT T\"; begin exit(R.Double(2)); end; }}"
    );
    let (_, l) = lower_src(&src);
    let p = l.program.objects[1].routines.iter().find(|r| r.name == "P").unwrap();
    let ir::Stmt::Exit(Some(ir::Expr::Call {
        object: 0,
        instance: Some(i),
        ..
    })) = &p.body[0]
    else {
        panic!("{:?}", p.body)
    };
    assert_eq!(**i, ir::Expr::Load(ir::Place::Local(0)));
}

#[test]
fn enum_and_option_constants() {
    let (_, l) = lower_src(
        "enum 50900 E { value(0; A) { } value(7; B) { } }\ncodeunit 50100 C { procedure P() var X: Enum E; O: Option P,Q; begin X := E::B; O := O::Q; end; }",
    );
    let r = l.program.objects[1].routines.iter().find(|r| r.name == "P").unwrap();
    assert!(
        matches!(&r.body[0], ir::Stmt::Assign(_, ir::Expr::Convert(_, e)) if **e == ir::Expr::Const(ir::Const::Enum(0, 7)))
    );
    assert!(
        matches!(&r.body[1], ir::Stmt::Assign(_, ir::Expr::Convert(_, e)) if **e == ir::Expr::Const(ir::Const::Option(1, vec!["P".into(), "Q".into()])))
    );
}

const PAGE_SRC: &str = r#"
table 50000 "T" { fields { field(1; "No."; Code[20]) { } field(2; Qty; Integer) { } } keys { key(PK; "No.") { } } }
page 50001 "P"
{
    PageType = Card;
    SourceTable = "T";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(QtyCtl; Rec.Qty) { Visible = Show; Editable = false; }
            group(G) { field(Glob; GlobN) { trigger OnValidate() begin GlobN := 7; end; } }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Act) { trigger OnAction() begin Rec.Qty := 1; end; }
        }
    }
    var
        Show: Boolean;
        GlobN: Integer;
    trigger OnOpenPage() begin Show := true; end;
}
codeunit 50002 "C" { procedure P() var TP: TestPage "P"; begin end; }
"#;

// Corpus handlers/TestPageFieldFormats.Page.al: controls bound to `Rec.Field` and to page
// variables; handlers/TestPageControlPropertyLiveness_Tests.al: expression-valued Visible.
#[test]
fn a_page_lowers_to_a_control_tree_with_bindings_and_triggers() {
    let (_, l) = lower_src(PAGE_SRC);
    let page_oi = l.program.objects.iter().position(|o| o.name == "P").unwrap();
    let o = &l.program.objects[page_oi];
    let pg = o.page.as_ref().expect("a page object has a page");
    let names: Vec<_> = pg.controls.iter().map(|c| (c.name.as_str(), c.kind.clone())).collect();
    assert_eq!(
        names,
        vec![
            ("NoCtl", ir::ControlKind::Field),
            ("QtyCtl", ir::ControlKind::Field),
            ("G", ir::ControlKind::Group),
            ("Glob", ir::ControlKind::Field),
            ("Act", ir::ControlKind::Action),
        ]
    );
    assert_eq!(pg.controls[3].parent, Some(2));
    assert_eq!(pg.controls[0].binding, ir::Binding::Field(0));
    assert_eq!(pg.controls[1].binding, ir::Binding::Field(1));
    let glob_slot = o.globals.iter().position(|g| g.name == "GlobN").unwrap();
    assert_eq!(pg.controls[3].binding, ir::Binding::Var(glob_slot));
    // Literal property stays text, expression becomes a routine of the page.
    assert_eq!(pg.controls[1].prop("editable"), Some(&ir::PropVal::Lit("false".into())));
    let Some(ir::PropVal::Routine(r)) = pg.controls[1].prop("visible") else {
        panic!("{:?}", pg.controls[1].props)
    };
    assert!(o.routines[*r].name.starts_with("__visible_"));
    assert!(pg.controls[3].trigger("onvalidate").is_some());
    assert!(pg.controls[4].trigger("onaction").is_some());
    assert!(pg.triggers.iter().any(|(n, _)| n == "onopenpage"));
    assert!(o.rec_slot.is_some() && o.xrec_slot.is_some());
    assert_eq!(o.globals[pg.curr_page_slot].name, "CurrPage");
    assert!(pg.unsupported.is_empty(), "{:?}", pg.unsupported);
}

const SAVE_VALUES_PAGE: &str = r#"
table 50010 "SV T" { fields { field(1; "No."; Code[20]) { } field(2; Qty; Decimal) { } } }
page 50011 "SV P" {
    SourceTable = "SV T"; SaveValues = true; PageType = StandardDialog;
    layout { area(Content) { field(Glob; GlobN) { } } }
    var GlobN: Text[30];
}
"#;

// Handlers/TestPageSaveValues.al pins SaveValues for a page without extensions. An extension
// that adds only controls bound to `Rec` fields adds no variable-bound control, so what the page
// stores is the same set (docs/probes-needed.md, SaveValues with page extensions).
#[test]
fn save_values_with_an_extension_that_adds_only_record_fields_is_lowered() {
    let src = format!(
        "{SAVE_VALUES_PAGE} pageextension 50012 \"SV X\" extends \"SV P\" {{ layout {{ addlast(Content) {{ field(Q; Rec.Qty) {{ }} }} }} }}"
    );
    let (_, l) = lower_src(&src);
    let pg = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "SV P")
        .unwrap()
        .page
        .clone()
        .unwrap();
    assert!(pg.unsupported.is_empty(), "{:?}", pg.unsupported);
}

// What BC stores for a control an extension binds to its own variable is not pinned: refused.
#[test]
fn save_values_with_an_extension_variable_control_is_refused() {
    let src = format!(
        "{SAVE_VALUES_PAGE} pageextension 50012 \"SV X\" extends \"SV P\" {{ layout {{ addlast(Content) {{ field(V; ExtVar) {{ }} }} }} var ExtVar: Integer; }}"
    );
    let (_, l) = lower_src(&src);
    let pg = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "SV P")
        .unwrap()
        .page
        .clone()
        .unwrap();
    assert!(
        pg.unsupported
            .iter()
            .any(|w| w.contains("SaveValues") && w.contains("variable")),
        "{:?}",
        pg.unsupported
    );
}

#[test]
fn a_testpage_variable_is_typed_by_its_page_and_reaches_the_page_routines() {
    let (_, l) = lower_src(PAGE_SRC);
    let page_oi = l.program.objects.iter().position(|o| o.name == "P").unwrap();
    let cu = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let p = l.program.objects[cu]
        .routines
        .iter()
        .position(|r| r.name == "P")
        .unwrap();
    assert_eq!(
        l.program.objects[cu].routines[p].locals[0].ty,
        RtType::TestPage(page_oi as u32)
    );
    assert!(l.unsupported[cu][p].is_empty(), "{:?}", l.unsupported[cu][p]);
    // The page's triggers are not called from AL, so the call graph gets edges to them.
    let n = l.program.objects[page_oi].routines.len();
    for r in 0..n {
        assert!(l.calls[cu][p].contains(&(page_oi, r)), "no edge to routine {r}");
    }
}

/// ADR-036 step 0: the types of the expressions of a routine, built with the expressions.
#[test]
fn integer_and_boolean_expressions_get_their_types() {
    use al_ir::types::{ExprTy, ExprTypes};
    let (_, l) = lower_src(
        "codeunit 50100 C {
    procedure P(a: Integer; b: Integer): Boolean
    var c: Integer;
    begin
        c := a + b * 2;
        exit((c > 3) and not (a = b));
    end;
}",
    );
    let r = l
        .program
        .objects
        .iter()
        .flat_map(|o| o.routines.iter())
        .find(|r| r.name == "P")
        .unwrap();
    assert_eq!(
        r.expr_tys.len(),
        al_ir::types::count_exprs(&r.body),
        "the table tiles the body"
    );
    let types = ExprTypes::new(r);
    let mut seen = Vec::new();
    for s in &r.body {
        al_ir::types::stmt_roots(s, &mut |e| seen.push(types.of(e)));
    }
    // `c := a + b * 2` holds the sum; `exit(...)` holds the `and`.
    assert_eq!(seen, [ExprTy::Integer, ExprTy::Boolean]);
    let mut all = Vec::new();
    for s in &r.body {
        al_ir::types::stmt_roots(s, &mut |e| al_ir::types::children(e, &mut |c| all.push(types.of(c))));
    }
    assert!(all.iter().all(|t| *t != ExprTy::Unknown), "{all:?}");
}

/// A `FieldRef.Value()` number converts at run time (VariantFieldRefBigIntToText.al) (Base Application's `Value := FieldRef.Value()` into a Text[250] field).
/// A number variable assigned to a Text or Code is a compile error in AL (AL0122) and stays refused by the lowering; a Variant
/// into a Text lowers.
#[test]
fn a_number_variable_into_text_is_refused_but_a_variant_into_text_lowers() {
    let reasons = |body: &str| {
        let (_, l) = lower_src(&format!(
            "codeunit 50100 C {{ procedure P() var I: Integer; B: BigInteger; D: Decimal; V: Variant; T: Text[50]; K: Code[20]; begin {body} end; }}"
        ));
        l.unsupported
            .iter()
            .flatten()
            .flatten()
            .cloned()
            .collect::<Vec<String>>()
    };
    for bad in ["T := I;", "T := B;", "T := D;", "K := B;"] {
        assert!(
            reasons(bad)
                .iter()
                .any(|r| r.contains("only a Variant holding a number converts")),
            "{bad}: {:?}",
            reasons(bad)
        );
    }
    for ok in ["T := V;", "K := V;", "T := Format(B);"] {
        assert!(reasons(ok).is_empty(), "{ok}: {:?}", reasons(ok));
    }
}

/// A selection compiles less of the workspace (`only_tests`, `reach_only`, `lean`), but the AL compiler rejects an app
/// for an error anywhere in it: the errors of what the selection leaves out are still found (al-runner issue 5602, an
/// AL0118 in a procedure no selected test names gave a clean pass).
#[test]
fn a_selection_still_finds_the_compile_errors_of_what_it_leaves_out() {
    const SRC: &str = "codeunit 50100 \"Helper\" {
            procedure Used(): Integer begin exit(1); end;
            procedure Unused(): Integer var I: Integer; begin I := NoSuchProcA(); exit(I); end; }
        codeunit 50101 \"Apart\" { procedure Two(): Integer begin exit(NoSuchProcB()); end; }
        codeunit 50102 \"T\" { Subtype = Test; var H: Codeunit \"Helper\";
            [Test] procedure P() begin if H.Used() <> 1 then Error('x'); end; }
        codeunit 50103 \"Other Test\" { Subtype = Test; [Test] procedure Q() begin NoSuchProcC(); end; }";
    for (reach_only, lean) in [(false, false), (true, false), (true, true)] {
        let (_, l) = compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".to_string(), SRC.to_string())],
            }],
            vec![crate::workspace::AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                baseline: false,
                only_tests: Some(vec![(0, 50102)]),
                reach_only,
                lean,
                ..Default::default()
            },
        );
        let got: Vec<String> = l
            .al_diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect();
        assert_eq!(
            got,
            vec![
                "AL0118 The name 'NoSuchProcA' does not exist in the current context.".to_string(),
                "AL0118 The name 'NoSuchProcB' does not exist in the current context.".to_string(),
                "AL0118 The name 'NoSuchProcC' does not exist in the current context.".to_string(),
            ],
            "reach_only {reach_only}, lean {lean}"
        );
    }
}
