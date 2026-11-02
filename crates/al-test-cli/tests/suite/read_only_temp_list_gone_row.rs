//! A List with Editable = false over a temporary source that loses its only row to an action (al-runner issue 6664). Real BC
//! values from corpus writer Y66 (pages/lt-y66-temp-list-gone-row/Y66TempListGoneRow.al:235, :400, not on corpus master yet).

use al_testrun::Outcome;

const SRC: &str = r#"
table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
codeunit 50101 Log
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
}
page 50102 "My List"
{
    PageType = List;
    Editable = false;
    SourceTable = Row;
    SourceTableTemporary = true;
    SourceTableView = sorting(Id) order(descending);
    layout { area(Content) { repeater(Lines) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(MyFilter) { trigger OnAction() begin Fill(0); end; }
            action(MyFilter1) { trigger OnAction() begin Fill(1); end; }
        }
    }
    trigger OnOpenPage() begin Fill(1); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec.Id)); end;
    var L: Codeunit Log;
    local procedure Fill(Rows: Integer)
    var I: Integer;
    begin
        Rec.DeleteAll();
        for I := 1 to Rows do begin Rec.Init(); Rec.Id := I * 10; Rec.Name := 'N' + Format(I * 10); Rec.Insert(); end;
    end;
}
codeunit 50103 T
{
    Subtype = Test;
    var L: Codeunit Log;
    local procedure Check(Name: Text; Got: Text; Want: Text) begin if Got <> Want then Error('%1: got [%2], want [%3]', Name, Got, Want); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure RunModalActionEmptiesTheList()
    begin
        L.Take();
        Page.RunModal(Page::"My List");
    end;

    [ModalPageHandler]
    procedure Handler(var MyList: TestPage "My List")
    begin
        Check('open', L.Take(), 'R:10;R:10;C:10;');
        MyList.MyFilter.Invoke();
        Check('action', L.Take(), 'R:10;N:0;C:0;');
        Check('first', Format(MyList.First()), 'No');
        Check('value', MyList.Name.Value(), '');
        Check('last', Format(MyList.Last()), 'No');
    end;

    [Test]
    procedure OpenEditActionEmptiesTheList()
    var MyList: TestPage "My List";
    begin
        L.Take();
        MyList.OpenEdit();
        L.Take();
        MyList.MyFilter.Invoke();
        Check('action', L.Take(), 'R:10;');
        Check('first', Format(MyList.First()), 'No');
        Check('value', MyList.Name.Value(), '');
    end;

    [Test]
    procedure OpenViewActionEmptiesTheList()
    var MyList: TestPage "My List";
    begin
        L.Take();
        MyList.OpenView();
        L.Take();
        MyList.MyFilter.Invoke();
        Check('action', L.Take(), 'R:10;');
        Check('first', Format(MyList.First()), 'No');
        Check('value', MyList.Name.Value(), '');
    end;

    [Test]
    [HandlerFunctions('RefillHandler')]
    procedure RefillAfterTheBlankRecIsRefused()
    begin
        Page.RunModal(Page::"My List");
    end;

    [ModalPageHandler]
    procedure RefillHandler(var MyList: TestPage "My List")
    begin
        MyList.MyFilter.Invoke();
        MyList.MyFilter1.Invoke();
    end;
}
"#;

#[test]
fn the_reporters_shape_and_open_edit_run_and_a_refill_is_refused() {
    let out = al_testkit::outcomes_in(SRC, "T");
    let of = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert!(matches!(of("RunModalActionEmptiesTheList"), Outcome::Pass), "{out:?}");
    assert!(matches!(of("OpenEditActionEmptiesTheList"), Outcome::Pass), "{out:?}");
    assert!(matches!(of("OpenViewActionEmptiesTheList"), Outcome::Pass), "{out:?}");
    assert!(
        matches!(of("RefillAfterTheBlankRecIsRefused"), Outcome::NotRun(ref why) if why.contains("second action")),
        "{out:?}"
    );
}
