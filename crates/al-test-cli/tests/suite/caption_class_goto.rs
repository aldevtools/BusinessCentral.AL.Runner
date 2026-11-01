//! The CaptionClass expression runs of a Card and a Document page on GoToRecord, GoToKey, Last, Previous and First (al-runner
//! issue 5858). Pinned by record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:1463-1960 (the SI_ tests): GoToRecord and
//! GoToKey run each expression control once more with the row the captions came from and Caption() keeps its value; Last and
//! First run them with the new row; Previous and Next run nothing; after a SetValue the save runs them once more.

use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        None,
    )
}

const SRC: &str = r#"
table 50100 "Cap Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
        field(3; "Expr Id"; Integer) { CaptionClass = ProbeCaption(); }
    }
    keys { key(PK; Id) { } }
    procedure ProbeCaption(): Text
    var Log: Codeunit "Cap Log";
    begin
        if Id <> 0 then Log.Add(Format(Id));
        exit('3,Row ' + Format(Id));
    end;
}
codeunit 50101 "Cap Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin Buf += T + ';'; end;
    procedure Take() Result: Text begin Result := Buf; Buf := ''; end;
}
page 50102 "Cap Card"
{
    PageType = Card; SourceTable = "Cap Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } field(ExprId; Rec."Expr Id") { } } }
}
page 50103 "Cap Doc"
{
    PageType = Document; SourceTable = "Cap Row";
    layout { area(Content) { group(G) { field(Id; Rec.Id) { } field(ExprId; Rec."Expr Id") { } } } }
}
codeunit 50104 "Cap Goto Tests"
{
    Subtype = Test;
    var Log: Codeunit "Cap Log";

    local procedure Seed()
    var R: Record "Cap Row"; i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to 3 do begin R.Init(); R.Id := i; R.Insert(); end;
        Log.Take();
    end;

    local procedure Check(Want: Text; Got: Text)
    begin
        if Want <> Got then Error('want <%1> got <%2>', Want, Got);
    end;

    [Test]
    procedure CardGoToRecord()
    var P: TestPage "Cap Card"; R: Record "Cap Row";
    begin
        Seed();
        P.OpenEdit();
        Check('1;', Log.Take());
        R.Get(3);
        P.GoToRecord(R);
        Check('1;', Log.Take());
        Check('3', P.Id.Value());
        Check('Row 1', P.ExprId.Caption());
        R.Get(2);
        P.GoToRecord(R);
        Check('1;', Log.Take());
        P.Next();
        Check('', Log.Take());
        Check('Row 1', P.ExprId.Caption());
    end;

    [Test]
    procedure CardGoToKeyOpenView()
    var P: TestPage "Cap Card";
    begin
        Seed();
        P.OpenView();
        Log.Take();
        P.GoToKey(3);
        Check('1;', Log.Take());
        Check('Row 1', P.ExprId.Caption());
    end;

    [Test]
    procedure CardLastPreviousFirst()
    var P: TestPage "Cap Card";
    begin
        Seed();
        P.OpenEdit();
        Log.Take();
        P.Last();
        Check('3;', Log.Take());
        Check('Row 3', P.ExprId.Caption());
        P.Previous();
        Check('', Log.Take());
        Check('Row 3', P.ExprId.Caption());
        P.First();
        Check('1;', Log.Take());
        Check('Row 1', P.ExprId.Caption());
    end;

    [Test]
    procedure CardSetValueThenGoToRecord()
    var P: TestPage "Cap Card"; R: Record "Cap Row";
    begin
        Seed();
        P.OpenEdit();
        Log.Take();
        P.Amount.SetValue(5);
        Check('', Log.Take());
        R.Get(3);
        P.GoToRecord(R);
        Check('1;1;', Log.Take());
        Check('Row 1', P.ExprId.Caption());
        R.Get(1);
        Check('5', Format(R.Amount));
    end;

    [Test]
    procedure DocGoToKeyOpenView()
    var P: TestPage "Cap Doc";
    begin
        Seed();
        P.OpenView();
        Check('1;', Log.Take());
        P.GoToKey(3);
        Check('1;', Log.Take());
        Check('3', P.Id.Value());
        Check('Row 1', P.ExprId.Caption());
    end;
}
"#;

#[test]
fn goto_last_previous_first_run_the_pinned_caption_expressions() {
    let out = run(SRC, "Cap Goto Tests");
    let bad: Vec<_> = out.iter().filter(|(_, o)| *o != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
}
