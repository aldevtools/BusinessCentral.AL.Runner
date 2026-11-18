//! ADR-040 amendment 2026-11-12b, point 4: a page compares its row with xRec and with the stored row to see what changed. A
//! comparison of a placeholder's size with anything but 0 has no answer, so the test that made one is refused, whichever step
//! made it: an action, a field, a part's row, a modal page under a handler, or the close at the end of the test. Rust tests:
//! the corpus has no case that puts BC's size on a page.

use al_testrun::Outcome;

const SRC: &str = r#"
table 50960 "PSP Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Txt; Text[30]) { }
        field(4; Data; Blob) { }
    }
    keys { key(PK; Id) { } }
    trigger OnModify()
    var L: Record "PSP Log";
    begin
        L.Id := L.Count() + 1;
        L.Insert();
    end;
}

table 50961 "PSP Log"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50962 "PSP Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

report 50963 "PSP Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "PSP Other") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'PspWord.docx'; } }
}

codeunit 50964 "PSP Size"
{
    // The size of an Html placeholder.
    procedure Size(): Integer
    var Tmp: Record "PSP Row" temporary; Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PSP Word", '', ReportFormat::Html, Out);
        exit(Tmp.Data.Length());
    end;
}

page 50965 "PSP Card"
{
    PageType = Card;
    SourceTable = "PSP Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Txt; Rec.Txt) { } } }
    actions
    {
        area(Processing)
        {
            action(SetSize) { trigger OnAction() var S: Codeunit "PSP Size"; begin Rec.Amount := S.Size(); end; }
            action(SetEight) { trigger OnAction() begin Rec.Amount := 8; end; }
        }
    }
}

page 50966 "PSP Lines"
{
    PageType = ListPart;
    SourceTable = "PSP Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Txt; Rec.Txt) { } } } }
}

page 50967 "PSP Host"
{
    PageType = Card;
    SourceTable = "PSP Other";
    layout { area(Content) { field(Id; Rec.Id) { } part(Lines; "PSP Lines") { } } }
}

codeunit 50968 "PSP Tests"
{
    Subtype = Test;

    local procedure Seed(WithSize: Boolean)
    var R: Record "PSP Row"; L: Record "PSP Log"; O: Record "PSP Other"; S: Codeunit "PSP Size";
    begin
        L.DeleteAll(); R.DeleteAll(); O.DeleteAll();
        O.Id := 1; O.Insert();
        R.Id := 1;
        if WithSize then R.Amount := S.Size() else R.Amount := 7;
        R.Insert();
        R.Id := 2; R.Amount := 7; R.Insert();
    end;

    local procedure Check(Condition: Boolean)
    begin
        if not Condition then Error('check failed');
    end;

    [Test]
    procedure ActionTwiceThenFieldRead()
    var P: TestPage "PSP Card"; L: Record "PSP Log";
    begin
        Seed(false);
        P.OpenEdit();
        P.SetSize.Invoke();
        P.SetSize.Invoke();
        P.Txt.Value();
        Check(L.Count() >= 0);
    end;

    [Test]
    procedure ActionThenSetValueAndClose()
    var P: TestPage "PSP Card";
    begin
        Seed(false);
        P.OpenEdit();
        P.SetSize.Invoke();
        P.Txt.SetValue('x');
        P.Close();
    end;

    [Test]
    procedure ActionThenTestEnds()
    var P: TestPage "PSP Card";
    begin
        Seed(false);
        P.OpenEdit();
        P.SetSize.Invoke();
        P.Txt.SetValue('x');
    end;

    [Test]
    procedure PartRowEditedAndLeft()
    var H: TestPage "PSP Host";
    begin
        Seed(true);
        H.OpenEdit();
        H.Lines.First();
        H.Lines.Txt.SetValue('x');
        H.Lines.Next();
    end;

    [Test]
    [HandlerFunctions('SetSizeTwiceAndOk')]
    procedure ModalPageUnderHandler()
    var R: Record "PSP Row";
    begin
        Seed(false);
        R.Get(1);
        Page.RunModal(Page::"PSP Card", R);
    end;

    [ModalPageHandler]
    procedure SetSizeTwiceAndOk(var P: TestPage "PSP Card")
    begin
        P.SetSize.Invoke();
        P.SetSize.Invoke();
        P.OK().Invoke();
    end;

}

// Neighbors that keep main's answers (a session of their own: a refused test taints the tests after it in its codeunit): the
// same steps with an ordinary number, and a page that only shows a row holding the size.
codeunit 50969 "PSP Neighbor Tests"
{
    Subtype = Test;

    local procedure Seed(WithSize: Boolean)
    var R: Record "PSP Row"; L: Record "PSP Log"; O: Record "PSP Other"; S: Codeunit "PSP Size";
    begin
        L.DeleteAll(); R.DeleteAll(); O.DeleteAll();
        O.Id := 1; O.Insert();
        R.Id := 1;
        if WithSize then R.Amount := S.Size() else R.Amount := 7;
        R.Insert();
        R.Id := 2; R.Amount := 7; R.Insert();
    end;

    [Test]
    procedure OrdinaryNumberTwice()
    var P: TestPage "PSP Card"; L: Record "PSP Log";
    begin
        Seed(false);
        P.OpenEdit();
        P.SetEight.Invoke();
        P.SetEight.Invoke();
        P.Txt.Value();
        if L.Count() <> 1 then Error('log rows: %1', L.Count());
    end;

    [Test]
    procedure ViewOfRowHoldingTheSize()
    var P: TestPage "PSP Card"; R: Record "PSP Row";
    begin
        Seed(true);
        R.Get(1);
        P.OpenView();
        P.GoToRecord(R);
        if P.Id.Value() <> '1' then Error('id: %1', P.Id.Value());
        P.Close();
    end;
}
"#;

const SIZE: &str = "the byte size of a placeholder";

fn outcomes() -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(SRC, "PSP Tests")
}

fn outcome_of<'a>(all: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name} in {all:?}"))
        .1
}

#[test]
fn placeholder_size_compared_by_a_page_refuses_the_test() {
    let all = outcomes();
    let wrong: Vec<String> = [
        "ActionTwiceThenFieldRead",
        "ActionThenSetValueAndClose",
        "ActionThenTestEnds",
        "PartRowEditedAndLeft",
        "ModalPageUnderHandler",
    ]
    .iter()
    .filter_map(|n| match outcome_of(&all, n) {
        Outcome::NotRun(why) if why.contains(SIZE) => None,
        other => Some(format!("{n}: {other:?}")),
    })
    .collect();
    assert!(wrong.is_empty(), "not refused:\n{}", wrong.join("\n"));
}

#[test]
fn placeholder_size_leaves_pages_that_compare_no_size() {
    let all = al_testkit::outcomes_in(SRC, "PSP Neighbor Tests");
    assert_eq!(outcome_of(&all, "OrdinaryNumberTwice"), &Outcome::Pass);
    assert_eq!(outcome_of(&all, "ViewOfRowHoldingTheSize"), &Outcome::Pass);
}
