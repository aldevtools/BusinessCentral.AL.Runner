//! A list whose OnOpenPage leaves Rec on a row with more rows around it than a window: the open reads at most 20 rows from
//! the row on and steps back over at most 20 rows above it; First then reads on up from the top row it holds
//! (pages/lt-y37-long-list-open/Y37LongListTests.al:91-111).
use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 61990 "LL Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 61991 "LL Log"
{
    SingleInstance = true;
    var
        Buffer: Text;
    procedure Add(T: Text) begin Buffer += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Buffer; Buffer := ''; exit(R); end;
}
page 61992 "LL List"
{
    PageType = List; SourceTable = "LL Row"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    trigger OnOpenPage() begin Rec.Get(25); Log.Add('O:' + Format(Rec.Id)); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Format(Rec.Id)); end;
    var
        Log: Codeunit "LL Log";
}
codeunit 61993 C
{
    Subtype = Test;

    var
        Log: Codeunit "LL Log";

    local procedure Check(Name: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1: got [%2], want [%3]', Name, Got, Want);
    end;

    [Test]
    procedure OpenOnRow25Of30()
    var
        R: Record "LL Row";
        P: TestPage "LL List";
        i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to 30 do begin
            R.Id := i;
            R.Insert();
        end;
        Log.Take();
        P.OpenView();
        Check('open', Log.Take(), 'O:25;R:25;R:26;R:27;R:28;R:29;R:30;R:25;R:24;R:23;R:22;R:21;R:20;R:19;R:18;R:17;R:16;R:15;R:14;R:13;R:12;R:11;R:10;R:9;R:8;R:7;R:6;R:5;R:25;R:25;C:25;');
        P.Next();
        Check('next', Log.Take(), 'R:26;R:26;C:26;');
        P.First();
        Check('first', Format(P.IdCtl.Value()) + ' ' + Log.Take(), '1 R:5;R:4;R:3;R:2;R:1;R:1;C:1;');
        P.Last();
        Check('last', Log.Take(), 'R:30;R:30;C:30;');
    end;
}
"#;

#[test]
fn a_long_list_opens_on_the_moved_rec() {
    let got = outcomes_in(SRC, "C");
    assert!(matches!(got.as_slice(), [(_, Outcome::Pass)]), "{got:?}");
}
