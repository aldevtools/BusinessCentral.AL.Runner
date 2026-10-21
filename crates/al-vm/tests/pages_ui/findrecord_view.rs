//! OnFindRecord (and OnNextRecord) on a page whose SourceTableView has a WHERE part. Pinned by
//! tests/al-language/pagefindrecord/QcFindViewTests.al (corpus bump 33), so these paths mark no assumption.
//! Shapes from Base Application: Interaction Log Entries (list), Service Invoice (document card with a fallback),
//! Sales Order Subform (list part).

use al_testkit::text;

const OBJECTS: &str = r#"
enum 69980 "FV Kind" { Extensible = false; value(0; A) { } value(1; B) { } }
table 69980 "FV Row" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "FV Kind") { } } keys { key(PK; Id) { } } }
table 69981 "FV Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 69982 "FV Line" { fields { field(1; "Doc Kind"; Enum "FV Kind") { } field(2; "Doc No."; Code[20]) { } field(3; "Line No."; Integer) { } }
  keys { key(PK; "Doc Kind", "Doc No.", "Line No.") { } } }
page 69980 "FV List" { PageType = List; SourceTable = "FV Row"; SourceTableView = where(Kind = filter(A)); Editable = false;
  layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; Found: Boolean;
  begin Log.NoteFilters(Rec); Found := Rec.Find(Which); Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end; }
page 69981 "FV Next List" { PageType = List; SourceTable = "FV Row"; SourceTableView = where(Kind = filter(A)); Editable = false;
  layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; Found: Boolean;
  begin Log.NoteFilters(Rec); Found := Rec.Find(Which); Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end;
  trigger OnNextRecord(Steps: Integer): Integer
  var Log: Codeunit "FV Log"; Moved: Integer;
  begin Moved := Rec.Next(Steps); Log.Add('next(' + Format(Steps) + ')=' + Format(Moved) + ':' + Format(Rec.Id)); exit(Moved); end; }
page 69982 "FV Desc List" { PageType = List; SourceTable = "FV Row"; SourceTableView = sorting(Id) order(descending) where(Kind = filter(A)); Editable = false;
  layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; Found: Boolean;
  begin Log.NoteFilters(Rec); Found := Rec.Find(Which); Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end; }
page 69983 "FV Free List" { PageType = List; SourceTable = "FV Row"; SourceTableView = where(Kind = filter(A)); Editable = false;
  layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; All: Record "FV Row"; Found: Boolean;
  begin All.SetPosition(Rec.GetPosition()); Found := All.Find(Which); if Found then Rec := All;
    Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end;
  trigger OnNextRecord(Steps: Integer): Integer
  var Log: Codeunit "FV Log"; All: Record "FV Row"; Moved: Integer;
  begin All.SetPosition(Rec.GetPosition()); All.Find('='); Moved := All.Next(Steps); if Moved <> 0 then Rec := All;
    Log.Add('next(' + Format(Steps) + ')=' + Format(Moved) + ':' + Format(Rec.Id)); exit(Moved); end; }
page 69984 "FV Card" { PageType = Card; SourceTable = "FV Row"; SourceTableView = where(Kind = const(A)); Editable = false;
  layout { area(Content) { field(Id; Rec.Id) { } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; Found: Boolean;
  begin Log.NoteFilters(Rec); Found := Rec.Find(Which); Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end; }
page 69985 "FV Fallback Card" { PageType = Document; SourceTable = "FV Row"; SourceTableView = where(Kind = filter(A)); Editable = false;
  layout { area(Content) { group(General) { field(Id; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log";
  begin
    Log.NoteFilters(Rec);
    if Rec.Find(Which) then begin Log.Add('find(' + Which + ')=Yes:' + Format(Rec.Id)); exit(true); end;
    Rec.SetRange(Id);
    if Rec.Find(Which) then begin Log.Add('find(' + Which + ')=No,again=Yes:' + Format(Rec.Id)); exit(true); end;
    Log.Add('find(' + Which + ')=No,again=No');
    exit(false);
  end; }
page 69986 "FV Lines Part" { PageType = ListPart; SourceTable = "FV Line"; SourceTableView = where("Doc Kind" = filter(A)); AutoSplitKey = true; DelayedInsert = true;
  layout { area(Content) { repeater(Lines) { field("Line No."; Rec."Line No.") { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "FV Log"; Found: Boolean;
  begin Log.NoteLineFilters(Rec); Found := Rec.Find(Which); Log.Add('find(' + Which + ')=' + Format(Found) + ':' + Format(Rec."Line No.")); exit(Found); end; }
page 69987 "FV Doc" { PageType = Document; SourceTable = "FV Head";
  layout { area(Content) { group(General) { field("No."; Rec."No.") { } }
    part(LinesPart; "FV Lines Part") { SubPageLink = "Doc No." = field("No."); } } } }
codeunit 69980 "FV Log"
{
    SingleInstance = true;
    var Buf: Text; Filters: Text;
    procedure Add(Entry: Text) begin Buf += Entry + ';'; end;
    procedure Take() Result: Text begin Result := Buf; Buf := ''; end;
    procedure NoteFilters(var Row: Record "FV Row")
    var Current: Integer; G0: Text; G2: Text;
    begin
        if Filters <> '' then exit;
        Current := Row.FilterGroup(); Row.FilterGroup(0); G0 := Row.GetFilters(); Row.FilterGroup(2); G2 := Row.GetFilters(); Row.FilterGroup(Current);
        Filters := 'group=' + Format(Current) + ' g0=[' + G0 + '] g2=[' + G2 + '] key=' + Row.CurrentKey() + ' asc=' + Format(Row.Ascending());
    end;
    procedure NoteLineFilters(var Line: Record "FV Line")
    var Current: Integer; G0: Text; G2: Text; G4: Text;
    begin
        if Filters <> '' then exit;
        Current := Line.FilterGroup(); Line.FilterGroup(0); G0 := Line.GetFilters(); Line.FilterGroup(2); G2 := Line.GetFilters();
        Line.FilterGroup(4); G4 := Line.GetFilters(); Line.FilterGroup(Current);
        Filters := 'group=' + Format(Current) + ' g0=[' + G0 + '] g2=[' + G2 + '] g4=[' + G4 + ']';
    end;
    procedure TakeFilters() Result: Text begin Result := Filters; Filters := ''; end;
}
codeunit 69981 Seed
{
    procedure Run()
    var Row: Record "FV Row"; Head: Record "FV Head"; Log: Codeunit "FV Log"; I: Integer;
    begin
        for I := 1 to 6 do begin
            Row.Init(); Row.Id := I;
            if I mod 2 = 0 then Row.Kind := Row.Kind::B else Row.Kind := Row.Kind::A;
            Row.Insert();
        end;
        Head."No." := 'H1'; Head.Insert(); Head."No." := 'H2'; Head.Insert();
        Line(Enum::"FV Kind"::A, 'H1', 10000); Line(Enum::"FV Kind"::A, 'H1', 20000); Line(Enum::"FV Kind"::B, 'H1', 15000);
        Line(Enum::"FV Kind"::A, 'H2', 30000); Line(Enum::"FV Kind"::B, 'H2', 40000);
        Log.Take(); Log.TakeFilters();
    end;
    local procedure Line(Kind: Enum "FV Kind"; DocNo: Code[20]; LineNo: Integer)
    var L: Record "FV Line";
    begin L."Doc Kind" := Kind; L."Doc No." := DocNo; L."Line No." := LineNo; L.Insert(); end;
}
"#;

fn run(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 69989 T\n{{\n{body}\n}}\n");
    al_builtins::assumptions::clear_hits();
    let got = text(al_testkit::run_in(&src, "T", "F"));
    // Pinned by corpus bump 33: no path here runs under an assumption.
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
    got
}

/// QcFindViewTests.al:78 ViewList_OpenView_FindStringAndRow, :97 ViewList_FirstNextLastPrevious, :135 ViewList_GoToKey.
#[test]
fn a_list_with_a_filtered_view_is_asked_the_open_string_once_and_walks_the_view() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; L: TestPage "FV List"; R: Text;
    begin
        Seed.Run();
        L.OpenView();
        R := L.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        R += '|' + Format(L.First()) + ':' + L.Id.Value();
        while L.Next() do R += ',' + L.Id.Value();
        R += '|' + Format(L.Last()) + ':' + L.Id.Value() + '|' + Format(L.Previous()) + ':' + L.Id.Value();
        R += '|' + Format(L.GoToKey(2)) + ':' + L.Id.Value() + '|' + Format(L.GoToKey(5)) + ':' + L.Id.Value();
        R += '|' + Log.Take();
        L.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "1|find(=><)=Yes:1;|group=0 g0=[] g2=[Kind: A] key=Id asc=Yes|Yes:1,3,5|Yes:5|Yes:3|No:3|Yes:5|"
    );
}

/// QcFindViewTests.al:157 ViewList_PageRunOnFilteredOutRow, :179 ViewList_PageRunOnRowInView.
#[test]
fn a_list_run_on_a_record_is_asked_from_the_handed_row() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; Row: Record "FV Row"; L: TestPage "FV List"; R: Text;
    begin
        Seed.Run();
        Row.Get(4);
        L.Trap();
        Page.Run(Page::"FV List", Row);
        R := L.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        L.Close();
        Row.Get(3);
        L.Trap();
        Page.Run(Page::"FV List", Row);
        R += '|' + L.Id.Value() + '|' + Log.Take();
        L.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "5|find(=>)=Yes:5;|group=0 g0=[] g2=[Kind: A] key=Id asc=Yes|3|find(=>)=Yes:3;"
    );
}

/// QcFindViewTests.al:201 NextList_OpenAndWalk, :236 NextList_NextLog.
#[test]
fn a_list_with_on_next_record_reads_ahead_at_open() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; L: TestPage "FV Next List"; R: Text;
    begin
        Seed.Run();
        L.OpenView();
        R := L.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        R += '|' + Format(L.First()) + ':' + L.Id.Value();
        while L.Next() do R += ',' + L.Id.Value();
        R += '|' + Format(L.Last()) + ':' + L.Id.Value() + '|' + Format(L.GoToKey(4)) + ':' + L.Id.Value();
        L.First();
        Log.Take();
        R += '|' + Format(L.Next()) + ':' + L.Id.Value() + '|' + Log.Take();
        L.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "1|find(=><)=Yes:1;next(1)=1:3;next(1)=1:5;next(1)=0:5;next(-1)=0:1;|group=0 g0=[] g2=[Kind: A] key=Id asc=Yes|Yes:1,3,5|Yes:5|No:5|Yes:3|"
    );
}

/// QcFindViewTests.al:257 DescList_OpenAndWalk: the open is pinned; whether the page is asked for the moves after it is not.
#[test]
fn a_descending_filtered_view_opens_on_the_lowest_row_and_refuses_the_moves() {
    let src = format!(
        "{OBJECTS}\ncodeunit 69989 T\n{{\n{}\n}}\n",
        r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; L: TestPage "FV Desc List";
    begin
        Seed.Run();
        L.OpenView();
        exit(L.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters());
    end;
    procedure G(): Boolean
    var Seed: Codeunit Seed; L: TestPage "FV Desc List";
    begin
        Seed.Run();
        L.OpenView();
        exit(L.First());
    end;"#
    );
    assert_eq!(
        text(al_testkit::run_in(&src, "T", "F")),
        "1|find(=><)=Yes:1;|group=0 g0=[] g2=[Kind: A] key=Id asc=No"
    );
    match al_testkit::run_in(&src, "T", "G") {
        Err(al_builtins::native::Signal::Unsupported(m)) => {
            assert!(m.contains("descending filtered SourceTableView"), "{m}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// QcFindViewTests.al:294 FreeList_OpenAndWalk: the view does not filter the rows the triggers serve.
#[test]
fn the_view_does_not_filter_rows_the_triggers_serve() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; L: TestPage "FV Free List"; R: Text;
    begin
        Seed.Run();
        L.OpenView();
        R := L.Id.Value() + '|' + Log.Take();
        R += '|' + Format(L.First()) + ':' + L.Id.Value();
        while L.Next() do R += ',' + L.Id.Value();
        R += '|' + Format(L.Last()) + ':' + L.Id.Value() + '|' + Format(L.GoToKey(2)) + ':' + L.Id.Value();
        L.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "1|find(=><)=Yes:1;next(1)=1:2;next(1)=1:3;next(1)=1:4;next(1)=1:5;next(1)=1:6;next(1)=0:6;next(-1)=0:1;|Yes:1,2,3,4,5,6|Yes:6|Yes:2"
    );
}

/// QcFindViewTests.al:361 ViewCard_OpenView_FindStringAndRow, :380 ViewCard_Navigate.
#[test]
fn a_card_with_a_filtered_view_opens_and_navigates() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; C: TestPage "FV Card"; R: Text;
    begin
        Seed.Run();
        C.OpenView();
        R := C.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        R += '|' + Format(C.Next()) + ':' + C.Id.Value() + '|' + Log.Take();
        R += '|' + Format(C.Last()) + ':' + C.Id.Value() + '|' + Log.Take();
        R += '|' + Format(C.Previous()) + ':' + C.Id.Value() + '|' + Format(C.First()) + ':' + C.Id.Value();
        R += '|' + Format(C.GoToKey(2)) + ':' + C.Id.Value() + '|' + Format(C.GoToKey(5)) + ':' + C.Id.Value() + '|' + Log.Take();
        C.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "1|find(=><)=Yes:1;|group=0 g0=[] g2=[Kind: A] key=Id asc=Yes|Yes:3||Yes:5|find(+)=Yes:5;|Yes:3|Yes:1|No:1|Yes:5|find(-)=Yes:1;find(-)=Yes:1;find(-)=Yes:1;find(-)=Yes:1;"
    );
}

/// QcFindViewTests.al:412 ViewCard_PageRunOnFilteredOutRow: '=' fails, then '=><'.
#[test]
fn a_card_run_on_a_row_outside_the_view_is_asked_equal_then_the_open_string() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; Row: Record "FV Row"; C: TestPage "FV Card"; R: Text;
    begin
        Seed.Run();
        Row.Get(4);
        C.Trap();
        Page.Run(Page::"FV Card", Row);
        R := C.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        C.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "5|find(=)=No:4;find(=><)=Yes:5;|group=0 g0=[] g2=[Kind: A] key=Id asc=Yes"
    );
}

/// QcFindViewTests.al:436 FallbackCard_OpenView, :457 FallbackCard_KeyFilterOnFilteredOutRow, :480 FallbackCard_KeyFilterOnRowInView.
#[test]
fn a_service_invoice_style_fallback_card() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; Row: Record "FV Row"; C: TestPage "FV Fallback Card"; R: Text;
    begin
        Seed.Run();
        C.OpenView();
        R := C.Id.Value() + '|' + Log.Take();
        R += '|' + Format(C.Next()) + ':' + C.Id.Value() + '|' + Format(C.GoToKey(4)) + ':' + C.Id.Value();
        C.Close();
        Log.Take();
        Log.TakeFilters();
        Row.SetRange(Id, 4);
        Row.FindFirst();
        C.Trap();
        Page.Run(Page::"FV Fallback Card", Row);
        R += '|' + C.Id.Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        C.Close();
        Row.SetRange(Id, 3);
        Row.FindFirst();
        C.Trap();
        Page.Run(Page::"FV Fallback Card", Row);
        R += '|' + C.Id.Value() + '|' + Log.Take();
        R += '|' + Format(C.Next()) + ':' + C.Id.Value();
        C.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "1|find(=><)=Yes:1;|Yes:3|No:3|5|find(=)=No,again=No;find(=><)=No,again=Yes:5;|group=0 g0=[Id: 4] g2=[Kind: A] key=Id asc=Yes|3|find(=)=Yes:3;|No:3"
    );
}

/// QcFindViewTests.al:505 DocPart_OpenEdit_LinesOfH1, :540 DocPart_OpenEdit_LinesOfH2, :570 DocPart_PageRunOnRecord.
#[test]
fn a_list_part_with_a_filtered_view_and_a_sub_page_link() {
    let got = run(r#"
    procedure F(): Text
    var Seed: Codeunit Seed; Log: Codeunit "FV Log"; Head: Record "FV Head"; D: TestPage "FV Doc"; R: Text;
    begin
        Seed.Run();
        D.OpenEdit();
        R := Format(D.GoToKey('H1')) + '|' + D.LinesPart."Line No.".Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        R += '|' + Format(D.LinesPart.First()) + ':' + D.LinesPart."Line No.".Value();
        while D.LinesPart.Next() do R += ',' + D.LinesPart."Line No.".Value();
        R += '|' + Format(D.LinesPart.Last()) + ':' + D.LinesPart."Line No.".Value();
        D.GoToKey('H2');
        Log.Take(); Log.TakeFilters();
        R += '|' + Format(D.LinesPart.First()) + ':' + D.LinesPart."Line No.".Value();
        while D.LinesPart.Next() do R += ',' + D.LinesPart."Line No.".Value();
        R += '|' + Log.Take();
        D.Close();
        Log.Take(); Log.TakeFilters();
        Head.Get('H1');
        D.Trap();
        Page.Run(Page::"FV Doc", Head);
        R += '|' + D."No.".Value() + '|' + D.LinesPart."Line No.".Value() + '|' + Log.Take() + '|' + Log.TakeFilters();
        D.Close();
        exit(R);
    end;"#);
    assert_eq!(
        got,
        "Yes|10000|find(=><)=Yes:10000;|group=0 g0=[] g2=[Doc Kind: A] g4=[Doc No.: H1]|Yes:10000,20000,0|Yes:20000|Yes:30000,0||H1|10000|find(=><)=Yes:10000;|group=0 g0=[] g2=[Doc Kind: A] g4=[Doc No.: H1]"
    );
}
