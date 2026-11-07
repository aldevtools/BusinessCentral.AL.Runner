//! What the parts of a host read when the host moves to another row (lane L3).
//! Mirrors pages/parts/PartReadsOpen.al (Next_SixParts_ReadsPerPartAfterHostMove,
//! LastPrevious_SixParts_ReadsPerPartAfterHostMove) and pages/parts/PartViewFilter.al:381.

use al_testkit::text;

const SRC: &str = r#"
table 51400 "MV Head" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
table 51401 "MV Line"
{
    fields { field(1; "Head Code"; Code[20]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Head Code", "Line No.") { } }
}
codeunit 51402 "MV Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Reset() begin T := ''; end;
    procedure Dump(): Text begin exit(T); end;
}
page 51403 "MV Plain"
{
    PageType = ListPart;
    SourceTable = "MV Line";
    layout { area(Content) { repeater(R) { field(L; Rec."Line No.") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "MV Log"; begin Log.Add('P:' + Rec."Head Code" + '/' + Format(Rec."Line No.")); end;
}
page 51404 "MV Linked"
{
    PageType = ListPart;
    SourceTable = "MV Line";
    layout { area(Content) { repeater(R) { field(L; Rec."Line No.") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "MV Log"; begin Log.Add('L:' + Rec."Head Code" + '/' + Format(Rec."Line No.")); end;
}
page 51405 "MV Filtered"
{
    PageType = ListPart;
    SourceTable = "MV Line";
    layout { area(Content) { repeater(R) { field(L; Rec."Line No.") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "MV Log"; begin Log.Add('F:' + Rec."Head Code" + '/' + Format(Rec."Line No.")); end;
}
page 51406 "MV Hidden"
{
    PageType = ListPart;
    SourceTable = "MV Line";
    layout { area(Content) { repeater(R) { field(L; Rec."Line No.") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "MV Log"; begin Log.Add('H:' + Rec."Head Code" + '/' + Format(Rec."Line No.")); end;
}
page 51407 "MV Host"
{
    PageType = Card;
    SourceTable = "MV Head";
    layout
    {
        area(Content)
        {
            field(CodeCtl; Rec."Code") { }
            part(PlainP; "MV Plain") { }
            part(LinkedP; "MV Linked") { SubPageLink = "Head Code" = field("Code"); }
            part(FilteredP; "MV Filtered") { SubPageView = where("Line No." = filter('>10000')); SubPageLink = "Head Code" = field("Code"); }
            part(HiddenP; "MV Hidden") { Visible = ShowHidden; SubPageLink = "Head Code" = field("Code"); }
        }
    }
    var ShowHidden: Boolean;
}
codeunit 51408 "MV Test"
{
    local procedure Seed()
    var H: Record "MV Head"; L: Record "MV Line";
    begin
        H."Code" := 'H1'; H.Insert(); H."Code" := 'H2'; H.Insert(); H."Code" := 'H3'; H.Insert();
        L."Head Code" := 'H1'; L."Line No." := 10000; L.Insert(); L."Line No." := 20000; L.Insert(); L."Line No." := 30000; L.Insert();
        L."Head Code" := 'H2'; L."Line No." := 10000; L.Insert(); L."Line No." := 20000; L.Insert();
        L."Head Code" := 'H3'; L."Line No." := 10000; L.Insert();
    end;
    procedure Moves(): Text
    var Log: Codeunit "MV Log"; Doc: TestPage "MV Host"; T: Text;
    begin
        Seed();
        Doc.OpenView();
        Log.Reset(); Doc.Next(); T := 'next=' + Log.Dump();
        Log.Reset(); Doc.Last(); T += '|last=' + Log.Dump();
        Log.Reset(); Doc.Previous(); T += '|previous=' + Log.Dump();
        Log.Reset(); Doc.First(); T += '|first=' + Log.Dump();
        Doc.Close();
        exit(T);
    end;
}
"#;

/// PartReadsOpen.al:269-283 (Next: no Plain reads, linked parts read forward and put the first row back), :336-379
/// (Last forward; Previous and First read the linked parts last to first, a one-row part once).
#[test]
fn a_host_move_reads_only_its_linked_parts() {
    let r = al_testkit::run_in(SRC, "MV Test", "Moves");
    assert_eq!(
        text(r),
        "next=L:H2/10000;L:H2/20000;L:H2/10000;F:H2/20000;H:H2/10000;H:H2/20000;H:H2/10000;\
         |last=L:H3/10000;H:H3/10000;\
         |previous=L:H2/20000;L:H2/10000;F:H2/20000;H:H2/20000;H:H2/10000;\
         |first=L:H1/30000;L:H1/20000;L:H1/10000;F:H1/30000;F:H1/20000;H:H1/30000;H:H1/20000;H:H1/10000;"
    );
}

const UPDATE_SRC: &str = r#"
table 51410 "UP Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51411 "UP Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Reset() begin T := ''; end;
    procedure Dump(): Text begin exit(T); end;
}
page 51412 "UP Part"
{
    PageType = ListPart;
    SourceTable = "UP Row";
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
    procedure RunUpdate() var Log: Codeunit "UP Log"; begin Log.Add('proc'); CurrPage.Update(false); end;
    trigger OnOpenPage() var Log: Codeunit "UP Log"; begin Log.Add('part-open'); end;
    trigger OnAfterGetRecord() var Log: Codeunit "UP Log"; begin Log.Add('agr' + Format(Rec.Id)); end;
}
page 51413 "UP Host"
{
    PageType = Card;
    SourceTable = "UP Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } part(PartCtl; "UP Part") { Visible = ShowPart; } } }
    var ShowPart: Boolean;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "UP Log"; begin Log.Add('host'); CurrPage.PartCtl.Page.RunUpdate(); end;
}
codeunit 51414 "UP Test"
{
    procedure Run(): Text
    var R: Record "UP Row"; Log: Codeunit "UP Log"; Host: TestPage "UP Host"; T: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        Log.Reset();
        Host.OpenEdit();
        T := 'open=' + Log.Dump();
        Log.Reset();
        Host.Next();
        T += '|next=' + Log.Dump();
        Host.Close();
        exit(T);
    end;
}
"#;

/// A part hidden by a Visible expression whose procedure, called from the host's OnAfterGetCurrRecord, runs
/// `CurrPage.Update`: the part fills its rows again after the host's code (pages/parts/
/// HiddenPartCurrPageUpdateFalse.al:196-198).
#[test]
fn a_part_update_asked_by_host_code_refills_the_part() {
    let r = al_testkit::run_in(UPDATE_SRC, "UP Test", "Run");
    assert_eq!(
        text(r),
        "open=part-open;host;proc;agr1;agr2;agr1;agr1;agr2;agr1;|next=host;proc;agr1;agr2;agr1;"
    );
}

const CLOSE_SRC: &str = r#"
table 51420 "CL Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51421 "CL Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
page 51422 "CL Part"
{
    PageType = ListPart;
    SourceTable = "CL Row";
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var Log: Codeunit "CL Log"; begin Log.Add('p.OnQueryClosePage(' + Format(CloseAction) + ')'); exit(true); end;
    trigger OnClosePage() var Log: Codeunit "CL Log"; begin Log.Add('p.OnClosePage'); end;
}
page 51423 "CL Host Expr"
{
    PageType = Card;
    SourceTable = "CL Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } part(PartCtl; "CL Part") { Visible = ShowPart; } } }
    var ShowPart: Boolean;
}
page 51424 "CL Host Static"
{
    PageType = Card;
    SourceTable = "CL Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } part(PartCtl; "CL Part") { Visible = false; } } }
}
codeunit 51425 "CL Test"
{
    procedure Run(): Text
    var R: Record "CL Row"; Log: Codeunit "CL Log"; E: TestPage "CL Host Expr"; S: TestPage "CL Host Static"; T: Text;
    begin
        R.Id := 1; R.Insert();
        E.OpenEdit(); Log.Take(); E.Close(); T := 'expr=' + Log.Take();
        S.OpenEdit(); Log.Take(); S.Close(); T += '|static=' + Log.Take();
        exit(T);
    end;
}
"#;

/// A part hidden by a Visible expression is open, so it runs its close triggers with its host; a literal
/// `Visible = false` part never opened and runs none (pages/W1Parts.al:408-411).
#[test]
fn a_part_hidden_by_an_expression_closes_with_its_host() {
    let r = al_testkit::run_in(CLOSE_SRC, "CL Test", "Run");
    assert_eq!(text(r), "expr=p.OnQueryClosePage(None);p.OnClosePage;|static=");
}

const UNSAVED_SRC: &str = r#"
table 51430 "UH Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 51431 "UH Line"
{
    fields
    {
        field(1; "Document No."; Code[20]) { TableRelation = "UH Header"; trigger OnValidate() var Log: Codeunit "UH Log"; begin Log.Add('V:' + "Document No."); end; }
        field(2; "Line No."; Integer) { }
        field(3; Descr; Text[30]) { }
    }
    keys { key(PK; "Document No.", "Line No.") { } }
}
codeunit 51432 "UH Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
page 51433 "UH Lines"
{
    PageType = ListPart;
    SourceTable = "UH Line";
    AutoSplitKey = true;
    layout { area(Content) { repeater(R) { field(Descr; Rec.Descr) { } } } }
}
page 51434 "UH Document"
{
    PageType = Document;
    SourceTable = "UH Header";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "UH Lines") { SubPageLink = "Document No." = field("No."); } } }
}
codeunit 51435 "UH Test"
{
    procedure Run(): Text
    var Log: Codeunit "UH Log"; Host: TestPage "UH Document"; T: Text;
    begin
        Host.OpenNew();
        T := 'open=' + Log.Take();
        Host."No.".SetValue('D2');
        T += '|key=' + Log.Take();
        exit(T);
    end;
    procedure Write(): Text
    var L: Record "UH Line"; H: Record "UH Header"; Log: Codeunit "UH Log"; Host: TestPage "UH Document";
    begin
        Host.OpenNew();
        Host."No.".SetValue('D3');
        Log.Take();
        Host.Lines.Descr.SetValue('x');
        if not H.Get('D3') then exit('host not saved');
        Host.Close();
        L.FindFirst();
        exit(Log.Take() + '|' + L."Document No.");
    end;
}
"#;

/// While the host row is not stored, the part's blank line takes a blank link value and validates it, also after the host's
/// key was typed (pages/parts/PartLineBeforeHostKey.al:398, :400: `L.OnValidate(Document No.):`).
#[test]
fn a_part_of_an_unsaved_host_validates_a_blank_link() {
    let r = text(al_testkit::run_in(UNSAVED_SRC, "UH Test", "Run"));
    let (open, key) = r.split_once("|key=").unwrap();
    assert!(open.contains("V:;"), "{r}");
    assert!(key.contains("V:;") && !key.contains("V:D2"), "{r}");
}

/// A write into the part of a host row that is not stored saves the host first and links the line to it: the line's link
/// is validated with the typed key (PartLineBeforeHostKey.al:406 `HP.OnInsertRecord:D2 | H.OnInsert:D2 |
/// L.OnValidate(Document No.):D2 ...` for New(); testpage/TestPageOpenNewPartLink.al:100 for a write).
#[test]
fn a_part_write_saves_the_unsaved_host_first() {
    let r = text(al_testkit::run_in(UNSAVED_SRC, "UH Test", "Write"));
    assert!(r.starts_with("V:D3;") && r.ends_with("|D3"), "{r}");
}

const TWO_PARTS_SRC: &str = r#"
table 51440 "TP Host" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 51445 "TP Group" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 51441 "TP Line" { fields { field(1; "Host Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Descr; Text[30]) { } } keys { key(PK; "Host Code", "Line No.") { } } }
table 51446 "TP Line G" { fields { field(1; "Group Code"; Code[10]) { TableRelation = "TP Group"; } field(2; "Line No."; Integer) { } field(3; Descr; Text[30]) { } } keys { key(PK; "Group Code", "Line No.") { } } }
page 51442 "TP Lines" { PageType = ListPart; SourceTable = "TP Line"; AutoSplitKey = true; layout { area(Content) { repeater(R) { field(Descr; Rec.Descr) { } } } } }
page 51447 "TP Lines G" { PageType = ListPart; SourceTable = "TP Line G"; AutoSplitKey = true; layout { area(Content) { repeater(R) { field(Descr; Rec.Descr) { } } } } }
page 51443 "TP Card"
{
    PageType = Card;
    SourceTable = "TP Host";
    layout { area(Content) { field(Code; Rec.Code) { } part(A; "TP Lines") { SubPageLink = "Host Code" = field(Code); } part(B; "TP Lines G") { SubPageLink = "Group Code" = field(Code); } } }
}
codeunit 51444 "TP Test"
{
    procedure KeyThenClose(): Integer
    var H: Record "TP Host"; G: Record "TP Group"; C: TestPage "TP Card"; N: Integer;
    begin
        G.Code := 'G1'; G.Insert();
        C.OpenNew(); C.Code.SetValue('G1'); N := H.Count(); C.Close(); exit(N * 10 + H.Count());
    end;
    procedure KeyThenRead(): Text
    var G: Record "TP Group"; C: TestPage "TP Card";
    begin
        G.Code := 'G1'; G.Insert();
        C.OpenNew(); C.Code.SetValue('G1'); exit(C.Code.Value());
    end;
}
"#;

/// A new Card row whose key is typed while two parts link on it, one through a relation to another table, is stored at once (pages/parts/
/// PagePartLinkUnsavedHost.al:178, :218); what the page shows afterwards is refused.
#[test]
fn a_new_card_with_two_parts_linked_on_its_key_stores_the_row_when_the_key_is_typed() {
    assert_eq!(
        al_testkit::int(al_testkit::run_in(TWO_PARTS_SRC, "TP Test", "KeyThenClose")),
        11
    );
    let r = al_testkit::run_in(TWO_PARTS_SRC, "TP Test", "KeyThenRead");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

const SIBLING_SRC: &str = r#"
table 51450 "SB Host" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 51451 "SB Line" { fields { field(1; "Host Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Descr; Text[30]) { } } keys { key(PK; "Host Code", "Line No.") { } } }
page 51452 "SB Lines" { PageType = ListPart; SourceTable = "SB Line"; AutoSplitKey = true; layout { area(Content) { repeater(R) { field(Descr; Rec.Descr) { } } } } }
page 51453 "SB Card"
{
    PageType = Card;
    SourceTable = "SB Host";
    layout { area(Content) { part(A; "SB Lines") { SubPageLink = "Host Code" = field(Code); } part(B; "SB Lines") { SubPageLink = "Host Code" = field(Code); } } }
}
codeunit 51454 "SB Test"
{
    procedure Run(): Integer
    var H: Record "SB Host"; L: Record "SB Line"; C: TestPage "SB Card";
    begin
        H.Code := 'G1'; H.Insert();
        C.OpenEdit(); C.GoToKey('G1');
        C.A.Descr.SetValue('a'); C.B.Descr.SetValue('b');
        exit(L.Count());
    end;
}
"#;

/// A write into one part saves the row typed into its sibling part (pages/parts/
/// PagePartLinkUnsavedHost.al:242: `a_lines_g = 1`, `a_lines_h = 0`).
#[test]
fn a_write_into_a_part_saves_the_sibling_parts_row() {
    assert_eq!(al_testkit::int(al_testkit::run_in(SIBLING_SRC, "SB Test", "Run")), 1);
}
