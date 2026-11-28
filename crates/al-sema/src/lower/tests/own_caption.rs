//! A page method named like one of the page's own procedures: `Caption` (al-runner issues 8193 and 8444), as BC 29.0
//! pinned it (pages/lifecycle/CurrPageCaptionOwn*.al). The pinned forms run unmarked.
use super::*;

const ROW: &str = "table 50100 T { fields { field(1; No; Code[20]) { } } }";
const MARK: &str = "Assume(";

fn at(ws: &Workspace, name: &str) -> usize {
    ws.objects.iter().position(|o| o.name.as_str() == name).unwrap()
}

/// The lowered body of routine `r` of object `name`.
fn body(ws: &Workspace, l: &Lowered, name: &str, r: &str) -> String {
    let o = &l.program.objects[at(ws, name)];
    format!("{:?}", o.routines.iter().find(|x| x.name == r).unwrap().body)
}

/// The refusals of object `name`.
fn refused(ws: &Workspace, l: &Lowered, name: &str) -> Vec<String> {
    l.unsupported[at(ws, name)]
        .iter()
        .flatten()
        .map(|r| format!("{r:?}"))
        .collect()
}

fn no_refusals(l: &Lowered) {
    let reasons: Vec<&Refusal> = l.unsupported.iter().flatten().flatten().collect();
    assert!(reasons.is_empty(), "{reasons:?}");
}

/// A Card page `name` with `body` and the procedure `own`.
fn card(name: &str, body: &str, own: &str) -> String {
    format!("page 50100 {name} {{ PageType = Card; SourceTable = T; Caption = 'Static'; var S: Text; {body} {own} }}")
}

/// al-runner issue 8444: a List page that declares its own `Caption(): Text` reads `CurrPage.Caption` and
/// `CurrPage.Caption()` in OnOpenPage. BC reads the platform caption (CurrPageCaptionOwnList.al:49-50, :90-92).
#[test]
fn currpage_caption_reads_beside_an_own_caption_procedure_read_the_platform_caption() {
    let src = format!(
        "{ROW}
page 50100 P {{
    PageType = List;
    SourceTable = T;
    Caption = 'Static';
    var A: Text; B: Text;
    trigger OnOpenPage() begin A := CurrPage.Caption; B := CurrPage.Caption(); end;
    procedure Caption(): Text begin exit('X '); end;
}}"
    );
    let (ws, l) = lower_src(&src);
    no_refusals(&l);
    let b = body(&ws, &l, "P", "OnOpenPage");
    assert_eq!(b.matches("Page.Caption").count(), 2, "{b}");
    assert!(!b.contains("Call {"), "{b}");
    assert!(!b.contains(MARK), "{b}");
}

/// al-runner issue 8193: the List page of Base Application's page 5911 assigns `CurrPage.Caption := Caption() + S` in
/// OnAfterGetCurrRecord: the platform setter around a call of the own procedure (CurrPageCaptionOwnList.al:55, :90). Only
/// the called procedure differs from a page whose procedure has another name.
#[test]
fn currpage_caption_assignment_beside_an_own_caption_procedure_is_the_platform_setter() {
    let src = format!(
        "{ROW}
page 50100 P {{
    PageType = List; SourceTable = T; Caption = 'Entries'; var S: Text;
    trigger OnAfterGetCurrRecord() begin CurrPage.Caption := Caption() + S; end;
    procedure Caption(): Text begin exit('X '); end;
}}
page 50101 Q {{
    PageType = List; SourceTable = T; Caption = 'Entries'; var S: Text;
    trigger OnAfterGetCurrRecord() begin CurrPage.Caption := MakeCaption() + S; end;
    procedure MakeCaption(): Text begin exit('X '); end;
}}"
    );
    let (ws, l) = lower_src(&src);
    no_refusals(&l);
    let p = body(&ws, &l, "P", "OnAfterGetCurrRecord");
    let q = body(&ws, &l, "Q", "OnAfterGetCurrRecord");
    assert!(p.contains("Page.Caption") && !p.contains(MARK), "{p}");
    assert!(!q.contains(MARK), "{q}");
    let setter_part = |s: &str| s.split("Call {").next().unwrap().to_string();
    assert_eq!(setter_part(&p), setter_part(&q));
}

/// Beside `Caption(NewCaption: Text)` the setter forms are the platform setter, not the procedure:
/// `CurrPage.Caption := 'A'` in OnOpenPage, `CurrPage.Caption('X')` and the bare `Caption('Bare')` in OnAction
/// (CurrPageCaptionOwnSignature.al:219,229,240, Called stays '' at :350).
#[test]
fn setter_forms_beside_an_own_caption_with_a_parameter_are_the_platform_setter() {
    let page = card(
        "P",
        "actions { area(Processing) { action(A) { trigger OnAction() begin CurrPage.Caption('X'); Caption('Bare'); end; } } }
         trigger OnOpenPage() begin CurrPage.Caption := 'A'; end;",
        "procedure Caption(NewCaption: Text) begin S := NewCaption; end;",
    );
    let (ws, l) = lower_src(&format!("{ROW}\n{page}"));
    no_refusals(&l);
    for (r, n) in [("OnOpenPage", 1), ("OnAction", 2)] {
        let b = body(&ws, &l, "P", r);
        assert_eq!(b.matches("Page.Caption").count(), n, "{b}");
        assert!(!b.contains("Call {") && !b.contains(MARK), "{b}");
    }
}

/// A Page variable in a test method: `P.Caption('Out')` is the platform setter beside a public `Caption(): Text`
/// (CurrPageCaptionOwnPublic.al:396), `P.Caption()` calls the procedure as before (:394), unmarked.
#[test]
fn page_variable_setter_is_the_platform_setter_and_the_getter_calls_the_procedure() {
    let page = card("P", "", "procedure Caption(): Text begin exit('Own'); end;");
    let src = format!(
        "{ROW}\n{page}
codeunit 50101 C {{
    Subtype = Test;
    [Test] procedure SetIt() var P: Page P; S: Text; begin P.Caption('Out'); end;
    [Test] procedure GetIt() var P: Page P; S: Text; begin S := P.Caption(); S := P.Caption; end;
}}"
    );
    let (ws, l) = lower_src(&src);
    no_refusals(&l);
    let set = body(&ws, &l, "C", "SetIt");
    assert!(set.contains("Page.Caption") && !set.contains(MARK), "{set}");
    let get = body(&ws, &l, "C", "GetIt");
    assert_eq!(get.matches("Call {").count(), 2, "{get}");
    assert!(!get.contains(MARK), "{get}");
}

/// The base page's bare `Caption()` beside a procedure only its page extension declares reads the platform caption
/// (CurrPageCaptionOwnExtension.al:95, :217).
#[test]
fn bare_caption_in_the_base_page_beside_an_extension_procedure_reads_the_platform_caption() {
    let page = card("P", "trigger OnOpenPage() begin S := Caption(); end;", "");
    let src = format!(
        "{ROW}\n{page}\npageextension 50100 X extends P {{ procedure Caption(): Text begin exit('ExtOwn'); end; }}"
    );
    let (ws, l) = lower_src(&src);
    no_refusals(&l);
    let b = body(&ws, &l, "P", "OnOpenPage");
    assert!(
        b.contains("Page.Caption") && !b.contains("Call {") && !b.contains(MARK),
        "{b}"
    );
}

/// Shapes the probe did not measure refuse: another page type, two procedures named Caption, another spelling, a quoted
/// member, a compound assignment, the assignment in OnInit or in a field's OnValidate, a bare read without parentheses
/// in the base page beside an extension's procedure, `CurrPage.Caption` in a request page beside a report procedure.
#[test]
fn unmeasured_caption_shapes_refuse() {
    let own = "procedure Caption(): Text begin exit('Own'); end;";
    let cases = [
        format!(
            "page 50100 P {{ PageType = Document; SourceTable = T; var S: Text; trigger OnOpenPage() begin S := CurrPage.Caption; end; {own} }}"
        ),
        card(
            "P",
            "trigger OnOpenPage() begin S := CurrPage.Caption; end;",
            &format!("{own} procedure Caption(I: Integer) begin end;"),
        ),
        card("P", "trigger OnOpenPage() begin S := CURRPAGE.CAPTION; end;", own),
        card("P", "trigger OnOpenPage() begin S := CurrPage.\"Caption\"; end;", own),
        card("P", "trigger OnOpenPage() begin CurrPage.Caption += 'x'; end;", own),
        card("P", "trigger OnInit() begin CurrPage.Caption := 'x'; end;", own),
        card(
            "P",
            "layout { area(Content) { field(S; S) { trigger OnValidate() begin CurrPage.Caption := 'x'; end; } } }",
            own,
        ),
        format!(
            "{} pageextension 50100 X extends P {{ {own} }}",
            card("P", "trigger OnOpenPage() begin S := Caption; end;", "")
        ),
        format!(
            "{} pageextension 50100 X extends P {{ {own} }} pageextension 50101 Y extends P {{ trigger OnOpenPage() var V: Text; begin V := CurrPage.Caption; end; }}",
            card("P", "", "")
        ),
        format!(
            "report 50100 P {{ requestpage {{ trigger OnOpenPage() begin CurrPage.Caption := 'x'; end; }} {own} }}"
        ),
    ];
    for page in cases {
        let (_, l) = lower_src(&format!("{ROW}\n{page}"));
        let r: Vec<String> = l
            .unsupported
            .iter()
            .flatten()
            .flatten()
            .map(|r| format!("{r:?}"))
            .collect();
        assert!(
            r.iter().any(|r| r.contains("beside a page procedure named Caption")),
            "{page}\n{r:?}"
        );
    }
}

/// Neighbors keep main's result: a page without a procedure named Caption, the bare `Caption()` that calls the own
/// procedure (CurrPageCaptionOwnPublic.al:158 with :307), and an own `Editable()` beside `CurrPage.Editable := ...`,
/// which stays refused (no case measures it).
#[test]
fn neighbors_keep_their_resolution() {
    let src = format!(
        "{ROW}
{}
{}
page 50102 E {{ SourceTable = T; trigger OnOpenPage() begin CurrPage.Editable := false; end; procedure Editable(): Boolean begin exit(true); end; }}",
        card("N", "trigger OnOpenPage() begin S := CurrPage.Caption; CurrPage.Caption := 'A'; end;", "").replace("50100", "50101"),
        card("P", "trigger OnOpenPage() begin S := Caption(); S := Caption; end;", "procedure Caption(): Text begin exit('Own'); end;"),
    );
    let (ws, l) = lower_src(&src);
    let n = body(&ws, &l, "N", "OnOpenPage");
    assert!(n.contains("Page.Caption") && !n.contains(MARK), "{n}");
    let p = body(&ws, &l, "P", "OnOpenPage");
    assert!(p.contains("Call {") && !p.contains(MARK), "{p}");
    assert!(refused(&ws, &l, "N").is_empty() && refused(&ws, &l, "P").is_empty());
    let e = refused(&ws, &l, "E");
    assert!(e.iter().any(|r| r.contains("too many arguments")), "{e:?}");
}

/// An assignment beside an own `Caption(NewCaption: Text)`: alc reads `X := v` as the call `X(v)`
/// (record/ProbePropertyAssignmentForms.al:375-377), and no cell measures which one BC calls then. Bare `Caption :=` in
/// OnOpenPage and OnAction and `P.Caption :=` in a test method refuse; main called the own procedure.
#[test]
fn assignment_beside_an_own_caption_with_a_parameter_refuses() {
    let own = "procedure Caption(NewCaption: Text) begin S := NewCaption; end;";
    let cases = [
        card("P", "trigger OnOpenPage() begin Caption := 'Bare'; end;", own),
        card(
            "P",
            "actions { area(Processing) { action(A) { trigger OnAction() begin Caption := 'Bare'; end; } } }",
            own,
        ),
        format!(
            "{}\ncodeunit 50101 C {{ Subtype = Test; [Test] procedure SetIt() var P: Page P; begin P.Caption := 'Out'; end; }}",
            card("P", "", own)
        ),
    ];
    for page in cases {
        assert_caption_refusal(&page);
    }
}

/// Calls with an argument count other than one beside an own Caption of that many parameters, and calls with one
/// Integer argument beside `Caption(N: Integer)`: no cell measures them, so they refuse with the gate's reason.
#[test]
fn calls_with_unmeasured_arguments_beside_an_own_caption_refuse() {
    let two = "procedure Caption(A: Integer; B: Integer) begin end;";
    let int = "procedure Caption(N: Integer) begin end;";
    let test_cu = |call: &str| {
        format!("codeunit 50101 C {{ Subtype = Test; [Test] procedure SetIt() var P: Page P; begin {call}; end; }}")
    };
    let cases = [
        card("P", "trigger OnOpenPage() begin Caption(3, 4); end;", two),
        format!("{}\n{}", card("P", "", two), test_cu("P.Caption(7, 8)")),
        card("P", "trigger OnOpenPage() begin Caption(5); end;", int),
        format!("{}\n{}", card("P", "", int), test_cu("P.Caption(9)")),
    ];
    for page in cases {
        assert_caption_refusal(&page);
    }
}

/// Lowers `src` after the row table and asserts the Caption gate refused a site.
fn assert_caption_refusal(src: &str) {
    assert_caption_refusal_in(&format!("{ROW}\n{src}"));
}

/// Lowers `src` (tables included) and asserts the Caption gate refused a site: a routine refused, or a statement
/// lowered to a refusal (as in an action trigger).
fn assert_caption_refusal_in(src: &str) {
    let (_, l) = lower_src(src);
    let bodies = l
        .program
        .objects
        .iter()
        .flat_map(|o| &o.routines)
        .map(|r| format!("{:?}", r.body));
    let r: Vec<String> = l
        .unsupported
        .iter()
        .flatten()
        .flatten()
        .map(|r| format!("{r:?}"))
        .chain(bodies)
        .collect();
    assert!(
        r.iter().any(|r| r.contains("beside a page procedure named Caption")),
        "{src}\n{r:?}"
    );
}

/// A local, a parameter, a source-table field, a page global and a named return value called `Caption` beside an own
/// `Caption(): Text` keep the gate's refusal, in page and in page-extension code: no cell measures which one BC names
/// there (al-runner issue 9505).
#[test]
fn a_variable_or_field_named_caption_beside_an_own_caption_refuses() {
    let own = "procedure Caption(): Text begin exit('Own'); end;";
    let field_row = "table 50100 T { fields { field(1; No; Code[20]) { } field(2; Caption; Text[30]) { } } }";
    for body in [
        "trigger OnOpenPage() begin Caption := 'Fld'; end;",
        "procedure Loc(): Text var Caption: Text; begin Caption := 'Loc'; exit(Caption + Rec.Caption); end;",
    ] {
        assert_caption_refusal_in(&format!("{field_row}\n{}", card("P", body, own)));
    }
    assert_caption_refusal(&format!(
        "page 50100 P {{ PageType = Card; SourceTable = T; Caption = 'Static'; var S: Text; Caption: Text; \
         trigger OnOpenPage() begin Caption := 'Glob'; end; {own} }}"
    ));
    assert_caption_refusal(&card(
        "P",
        "procedure Ret() Caption: Text begin Caption := 'Ret'; end;",
        own,
    ));
    for routine in [
        "procedure Loc(): Text var Caption: Text; begin Caption := 'Loc'; exit(Caption); end;",
        "procedure Par(Caption: Text): Text begin Caption := Caption + 'P'; exit(Caption); end;",
    ] {
        assert_caption_refusal(&card("P", routine, own));
        assert_caption_refusal(&format!(
            "{}\npageextension 50101 E extends P {{ {routine} }}",
            card("P", "", own)
        ));
    }
}
