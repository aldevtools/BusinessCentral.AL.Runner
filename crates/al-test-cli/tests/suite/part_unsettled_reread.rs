//! A ListPart without OnAfterGetCurrRecord on the last row of two, after a page-variable control's OnValidate ran
//! `CurrPage.Update()` on a Document host (crates/al-vm/src/pages/part_unsettled.rs). Only a new row the part started with
//! `New()`, left untouched until the host closes, is pinned (pages/parts/PartNewRowPageVariable.al:1218). A stored row, and a
//! new row the part or the host moved off before the close, keep the reads of the shape before (ThPartUpdateCursor.al).

use al_testrun::Outcome;

const SRC: &str = r#"
table 69960 "UR Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 69961 "UR Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } field(4; Descr; Text[30]) { }
      field(5; Val; Text[30]) { trigger OnValidate() var L: Codeunit "UR Log"; begin L.Add('T-V ' + Format("Line No.") + ' ' + Val); end; } }
    keys { key(PK; "Doc No.", "Line No.") { } }
    trigger OnInsert() var L: Codeunit "UR Log"; begin L.Add('T-I ' + Format("Line No.")); end;
    trigger OnModify() var L: Codeunit "UR Log"; begin L.Add('T-M ' + Format("Line No.")); end;
}
codeunit 69962 "UR Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin if Entries <> '' then Entries += ' | '; Entries += Line; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
}
page 69963 "UR Lines"
{
    PageType = ListPart; SourceTable = "UR Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) {
        field("Line No."; Rec."Line No.") { }
        field(Descr; Rec.Descr) { }
        field(Upd; UpdTxt) { trigger OnValidate() var L: Codeunit "UR Log"; begin L.Add('V-UPD ' + Format(Rec."Line No.")); Rec.Validate(Val, UpdTxt); CurrPage.Update(); end; }
    } } }
    var UpdTxt: Text;
    trigger OnAfterGetRecord() var L: Codeunit "UR Log"; begin L.Add('AGR ' + Format(Rec."Line No.")); UpdTxt := Rec.Val; end;
    trigger OnNewRecord(B: Boolean) var L: Codeunit "UR Log"; begin L.Add('NEW ' + Format(Rec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "UR Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "UR Log"; begin L.Add('MOD ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
}
page 69964 "UR Doc"
{
    PageType = Document; SourceTable = "UR Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "UR Lines") { SubPageLink = "Doc No." = field("No."); } } }
}
codeunit 69965 C
{
    Subtype = Test;
    var Log: Codeunit "UR Log";

    local procedure Seed(N: Integer)
    var Head: Record "UR Head"; Line: Record "UR Line"; i: Integer;
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head."No." := 'D1'; Head.Insert(); Head."No." := 'D2'; Head.Insert();
        for i := 1 to N do begin Line.Init(); Line."Doc No." := 'D1'; Line."Line No." := i * 10000; Line.Descr := 'r' + Format(i); Line.Val := 'old'; Line.Insert(); end;
        Log.Take();
    end;

    // A stored row: the reads of ThPartUpdateCursor.al, nothing at the close.
    [Test]
    procedure StoredLastOfTwo_Close()
    var P: TestPage "UR Doc"; T: Text;
    begin
        Seed(2); P.OpenEdit(); P.GoToKey('D1'); P.Lines.Last(); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Close();
        Error('upd=%1 closed=%2', T, Log.Take());
    end;

    [Test]
    procedure StoredLastOfTwo_FirstThenClose()
    var P: TestPage "UR Doc"; T: Text; U: Text;
    begin
        Seed(2); P.OpenEdit(); P.GoToKey('D1'); P.Lines.Last(); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Lines.First(); U := Log.Take(); P.Close();
        Error('upd=%1 first=%2 closed=%3', T, U, Log.Take());
    end;

    [Test]
    procedure StoredLastOfTwo_HostNextThenClose()
    var P: TestPage "UR Doc"; T: Text; U: Text;
    begin
        Seed(2); P.OpenEdit(); P.GoToKey('D1'); P.Lines.Last(); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Next(); U := Log.Take(); P.Close();
        Error('upd=%1 next=%2 closed=%3', T, U, Log.Take());
    end;

    // The pinned new row, then a move of the part or of the host before the close: the close reads nothing.
    [Test]
    procedure NewRow_FirstThenClose()
    var P: TestPage "UR Doc"; U: Text;
    begin
        Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('B'); P.Lines.Upd.SetValue('1'); Log.Take();
        P.Lines.First(); U := Log.Take(); P.Close();
        Error('first=%1 closed=%2', U, Log.Take());
    end;

    [Test]
    procedure NewRow_HostNextThenClose()
    var P: TestPage "UR Doc"; U: Text;
    begin
        Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('B'); P.Lines.Upd.SetValue('1'); Log.Take();
        P.Next(); U := Log.Take(); P.Close();
        Error('next=%1 closed=%2', U, Log.Take());
    end;

    // The pinned shape itself (PartNewRowPageVariable.al:1218): the row is read once more at the close.
    [Test]
    procedure NewRow_Close()
    var P: TestPage "UR Doc"; T: Text;
    begin
        Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('B'); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Close();
        Error('upd=%1 closed=%2', T, Log.Take());
    end;
}
"#;

fn failure(outcomes: &[(String, Outcome)], name: &str) -> String {
    match &outcomes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
    {
        Outcome::Fail(why) => why.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn only_an_untouched_new_row_reads_again_at_the_close() {
    let o = al_testkit::outcomes_in(SRC, "C");
    let reads = "AGR 20000 | AGR 10000 | AGR 20000 | AGR 20000";
    let upd = format!("upd=[V-UPD 20000 | T-V 20000 1 | MOD 20000 1 | T-M 20000 | {reads}]");
    let f = failure(&o, "StoredLastOfTwo_Close");
    assert!(f.starts_with(&format!("{upd} closed=[]")), "{f}");
    let f = failure(&o, "StoredLastOfTwo_FirstThenClose");
    assert!(f.starts_with(&format!("{upd} first=[] closed=[]")), "{f}");
    let f = failure(&o, "StoredLastOfTwo_HostNextThenClose");
    assert!(
        f.starts_with(&format!("{upd} next=[NEW 0 | NEW 0 | NEW 0] closed=[]")),
        "{f}"
    );
    let f = failure(&o, "NewRow_FirstThenClose");
    assert!(f.ends_with("closed=[]") || f.contains("closed=[]\n"), "{f}");
    let f = failure(&o, "NewRow_HostNextThenClose");
    assert!(f.contains("closed=[]"), "{f}");
    let f = failure(&o, "NewRow_Close");
    assert!(f.contains("| AGR 20000 | AGR 10000] closed=[AGR 20000]"), "{f}");
}
