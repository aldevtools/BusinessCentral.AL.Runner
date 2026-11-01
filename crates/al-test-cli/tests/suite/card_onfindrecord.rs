//! A card run with Page.Run(Id, Rec): which Which strings OnFindRecord is asked and what the card reads at open and around
//! an action (pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al, codeunit 81164).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62700 "CF Row"
{
    fields { field(1; Id; Integer) { } field(2; Descr; Text[30]) { } field(3; Note; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 62701 "CF Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ';'; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
}
page 62702 "CF Card TF"
{
    PageType = Card; SourceTable = "CF Row"; SourceTableTemporary = true;
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(DescrCtl; Rec.Descr) { } field(NoteCtl; Rec.Note) { } } }
    actions { area(Processing) { action(Show) { trigger OnAction() begin Log.Add('A:' + Rec.Descr + '/' + Rec.Note); end; } } }
    var Log: Codeunit "CF Log";
    trigger OnOpenPage() begin Log.Add('O:' + Format(Rec.Id) + '/' + Rec.Descr + '/' + Rec.Note); Rec.Note := 'in memory'; end;
    trigger OnFindRecord(Which: Text): Boolean begin Log.Add('F:' + Which + ':' + Format(Rec.Id) + '/' + Rec.Descr + '/' + Rec.Note); exit(true); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id) + '/' + Rec.Descr + '/' + Rec.Note); end;
}
"#;

// BeCardOnFindRecord.al:292 CardTempFind_PageRun_TempRec.
#[test]
fn a_card_with_on_find_record_is_asked_equals_and_reads_its_row_around_an_action() {
    let src = format!(
        "{OBJECTS}\ncodeunit 62790 T {{ Subtype = Test;
        var Log: Codeunit \"CF Log\"; Got: Text;
        procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
        [PageHandler] procedure H(var C: TestPage \"CF Card TF\") begin
            Got := Log.Take(); Check('O:1/passed/;F:=:1/passed/in memory;R:1/passed/in memory', Got);
            Check('in memory', C.NoteCtl.Value());
            C.Show.Invoke();
            Got := Log.Take();
            Check('R:1/passed/in memory;A:passed/in memory;R:1/passed/in memory;F:=:1/passed/in memory;R:1/passed/in memory', Got);
            C.Close(); Check('', Log.Take()); end;
        [Test] [HandlerFunctions('H')]
        procedure Run() var Row: Record \"CF Row\" temporary; begin
            Row.Id := 1; Row.Descr := 'passed'; Row.Insert(); Log.Take();
            Page.Run(Page::\"CF Card TF\", Row); end;
        }}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// BeCardOnFindRecord.al:374 CardTemp_PageRun_StoredRec: BC shows the stored row on a card over a temporary table; whether it
// is copied into the temporary table is not pinned, so the run is refused.
#[test]
fn a_stored_record_handed_to_a_temporary_card_is_refused() {
    let src = format!(
        "{OBJECTS}\ncodeunit 62791 T {{ Subtype = Test;
        [PageHandler] procedure H(var C: TestPage \"CF Card TF\") begin end;
        [Test] [HandlerFunctions('H')]
        procedure Run() var Row: Record \"CF Row\"; begin
            Row.DeleteAll(); Row.Id := 1; Row.Descr := 'stored'; Row.Insert();
            Page.Run(Page::\"CF Card TF\", Row); end;
        }}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 1);
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("temporary")),
        "{o:?}"
    );
}
