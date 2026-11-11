//! The save at the close of a modal page a ModalPageHandler left open, when it raises a TestField error (al-runner issue 7400).
//! Pinned only on a Card or List run by a static `Page.RunModal(Id, Rec)` whose return value is used, the error raised in the
//! page's OnModifyRecord, the page without OnQueryClosePage (pages/close-errors/ModalCloseSaveErrorShapes.al:250, :271, :314,
//! :336). A Page variable's RunModal, a static RunModal without a record, the table's OnModify and a page with OnQueryClosePage
//! keep the refusal (review of runner PR 7800). Each shape runs in its own codeunit: a refusal in one test of a codeunit stops the rest of it.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const OBJECTS: &str = r#"
codeunit 50200 "Q Cfg" { SingleInstance = true; var Mode: Text; Log: Text;
  procedure Set(M: Text) begin Mode := M; Log := ''; end; procedure Get(): Text begin exit(Mode); end;
  procedure Add(V: Text) begin Log += V + ';'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }
table 50201 "Q Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; A; Text[30])
        {
            trigger OnValidate()
            var Cfg: Codeunit "Q Cfg";
            begin
                if Cfg.Get() = 'base-rec' then begin
                    Rec.ProcWrite();
                    Cfg.Add('T x=' + xRec."Code" + '/' + xRec.A + '/' + xRec.Note);
                end;
            end;
        }
        field(3; Note; Text[30]) { }
        field(4; Canceled; Boolean) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
    procedure ProcWrite()
    var Cfg: Codeunit "Q Cfg";
    begin
        Cfg.Add('P x=' + xRec."Code" + '/' + xRec.A + '/' + xRec.Note);
        xRec.Note := 'FROMPROC';
    end;
    trigger OnModify()
    var Cfg: Codeunit "Q Cfg";
    begin
        Cfg.Add('TM');
        if Cfg.Get() = 'table-testfield' then
            TestField(Canceled, false);
    end;
}
page 50204 "Q List"
{
    PageType = List; SourceTable = "Q Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { ApplicationArea = All; }
        field(A; Rec.A) { ApplicationArea = All; }
        field(Note; Rec.Note) { ApplicationArea = All; } } } }
    var Cfg: Codeunit "Q Cfg";
    trigger OnModifyRecord(): Boolean
    begin
        Cfg.Add('M');
        if Cfg.Get() = 'testfield' then Rec.TestField(Canceled, false);
        if Cfg.Get() = 'fielderror' then Rec.FieldError(Canceled);
        if Cfg.Get() = 'error' then Error('the save failed');
        exit(true);
    end;
}
page 50205 "Q List QC"
{
    PageType = List; SourceTable = "Q Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { ApplicationArea = All; }
        field(Note; Rec.Note) { ApplicationArea = All; } } } }
    var Cfg: Codeunit "Q Cfg";
    trigger OnModifyRecord(): Boolean
    begin
        Cfg.Add('M');
        if Cfg.Get() = 'testfield' then Rec.TestField(Canceled, false);
        exit(true);
    end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Cfg.Add('Q'); exit(true);
    end;
}
page 50206 "Q Card"
{
    PageType = Card; SourceTable = "Q Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) {
        field("Code"; Rec."Code") { ApplicationArea = All; }
        field(A; Rec.A) { ApplicationArea = All; }
        field(Note; Rec.Note) { ApplicationArea = All; } } }
    var Cfg: Codeunit "Q Cfg";
    trigger OnModifyRecord(): Boolean
    begin
        Cfg.Add('M');
        if Cfg.Get() = 'testfield' then Rec.TestField(Canceled, false);
        exit(true);
    end;
}
codeunit 50208 "Q Help"
{
    var Cfg: Codeunit "Q Cfg";
    procedure Cell(Mode: Text; Shape: Text; Expected: Text)
    var Row: Record "Q Row"; Old: Record "Q Row"; L: Page "Q List"; C: Page "Q Card";
    begin
        Old.DeleteAll();
        Old.Init(); Old."Code" := 'R1'; Old.A := 'stored'; Old.Note := 'note'; Old.Canceled := true; Old.Insert();
        Commit();
        Cfg.Set(Mode);
        Row.SetRange("Code", 'R1');
        asserterror begin
            case Shape of
                'list-var': begin L.SetTableView(Row); L.RunModal(); end;
                'list-static': if Page.RunModal(Page::"Q List", Row) = Action::LookupOK then;
                'listqc-static': if Page.RunModal(Page::"Q List QC", Row) = Action::LookupOK then;
                'card-static': if Page.RunModal(Page::"Q Card", Row) = Action::LookupOK then;
                'list-static-stmt': Page.RunModal(Page::"Q List", Row);
                'list-static-norec': if Page.RunModal(Page::"Q List") = Action::LookupOK then;
                'card-static-norec': if Page.RunModal(Page::"Q Card") = Action::LookupOK then;
                'list-static-norec-stmt': Page.RunModal(Page::"Q List");
                'card-static-stmt': Page.RunModal(Page::"Q Card", Row);
                'card-var': begin C.SetTableView(Row); C.RunModal(); end;
                'card-var-lookup': begin C.SetTableView(Row); C.LookupMode(true); C.RunModal(); end;
            end;
            Error('NO_ERROR');
        end;
        if GetLastErrorCode() <> Expected then
            Error('expected %1, got %2: %3', Expected, GetLastErrorCode(), GetLastErrorText());
    end;
}
"#;

fn check(shapes: &[(&str, &str, &str, &str, &str)]) {
    let mut src = OBJECTS.to_string();
    for (i, (name, handler, mode, shape, _)) in shapes.iter().enumerate() {
        let page = match *handler {
            "ListEdit" => "Q List",
            "ListQCEdit" => "Q List QC",
            _ => "Q Card",
        };
        let edit = if page == "Q Card" {
            "P.Note.SetValue('NEW');"
        } else {
            "P.First(); P.Note.SetValue('NEW');"
        };
        src.push_str(&format!(
            "codeunit {} \"{name}\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n    [Test] [HandlerFunctions('{handler}')] procedure {name}()\n    var H: Codeunit \"Q Help\";\n    begin\n        H.Cell('{mode}', '{shape}', 'TestField');\n    end;\n    [ModalPageHandler] procedure {handler}(var P: TestPage \"{page}\") begin {edit} end;\n}}\n",
            50300 + i
        ));
    }
    let mut wrong = Vec::new();
    for (name, _, _, _, want) in shapes {
        let got = outcomes_in(&src, name);
        let ok = matches!(
            (*want, got.first().map(|(_, o)| o)),
            ("refused", Some(Outcome::NotRun(_))) | ("pass", Some(Outcome::Pass))
        );
        if !ok {
            wrong.push(format!("{name}: want {want}, got {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_testfield_error_of_the_save_at_modal_close_runs_only_in_the_pinned_shape() {
    check(&[
        // Pinned: a static RunModal of a Card, TestField in the page's OnModifyRecord, no OnQueryClosePage (:336).
        (
            "CardStaticPageTestField",
            "CardEdit",
            "testfield",
            "card-static",
            "pass",
        ),
        (
            "ListStaticPageTestField",
            "ListEdit",
            "testfield",
            "list-static",
            "pass",
        ),
        // Pinned on a List also when the static RunModal is a statement (:314); not measured on a Card.
        (
            "ListStaticStatementTestField",
            "ListEdit",
            "testfield",
            "list-static-stmt",
            "pass",
        ),
        (
            "CardStaticStatementTestField",
            "CardEdit",
            "testfield",
            "card-static-stmt",
            "refused",
        ),
        // Not pinned: a Page variable's RunModal, with or without LookupMode.
        ("ListVariableTestField", "ListEdit", "testfield", "list-var", "refused"),
        ("CardVariableTestField", "CardEdit", "testfield", "card-var", "refused"),
        (
            "CardVariableLookupTestField",
            "CardEdit",
            "testfield",
            "card-var-lookup",
            "refused",
        ),
        // Not pinned: the TestField in the table's OnModify.
        (
            "ListStaticTableTestField",
            "ListEdit",
            "table-testfield",
            "list-static",
            "refused",
        ),
        (
            "CardStaticTableTestField",
            "CardEdit",
            "table-testfield",
            "card-static",
            "refused",
        ),
        // Not pinned: a static RunModal without a record (every pinned cell passes one), for a TestField and for an Error.
        (
            "ListStaticNoRecTestField",
            "ListEdit",
            "testfield",
            "list-static-norec",
            "refused",
        ),
        (
            "CardStaticNoRecTestField",
            "CardEdit",
            "testfield",
            "card-static-norec",
            "refused",
        ),
        (
            "ListStaticNoRecStatementTestField",
            "ListEdit",
            "testfield",
            "list-static-norec-stmt",
            "refused",
        ),
        (
            "ListStaticNoRecError",
            "ListEdit",
            "error",
            "list-static-norec",
            "refused",
        ),
        // Not pinned: a page with OnQueryClosePage.
        (
            "ListQueryCloseTestField",
            "ListQCEdit",
            "testfield",
            "listqc-static",
            "refused",
        ),
    ]);
}
