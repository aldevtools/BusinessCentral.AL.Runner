//! The closes of a TableRelation lookup's list page that pages/close-errors/CemLookupTests.al pins (al-runner issue 7479) run
//! only in the pinned shape: an origin page opened with OpenEdit, a looked-up control and table field without triggers, a
//! lookup page that is a List with Editable = false, OnQueryClosePage and OnClosePage only, no action, no part, no subscriber,
//! a handler that calls only GoToKey, OK, Cancel or Close(), and a close nobody vetoes. Every other shape keeps main's
//! behavior (only LookupOK writes the pick) or is refused, and the last error after a swallowed close error is not readable
//! (crates/al-vm/src/pages/relation_lookup_close.rs; PR 7816 review round 2, scenarios of the reviewer's probe).

use al_testrun::Outcome;

const SRC: &str = r#"
table 50400 "Q Code" { LookupPageId = "Q Codes"; fields { field(1; Code; Code[10]) { } field(2; Txt; Text[30]) { } } keys { key(PK; Code) { } } }
table 50401 "Q Code FB" { LookupPageId = "Q Codes FB"; fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50402 "Q Code Card" { LookupPageId = "Q Codes Card"; fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50403 "Q Code Sub" { LookupPageId = "Q Codes Sub"; fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50407 "Q Code Act" { LookupPageId = "Q Codes Act"; fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50404 "Q Row"
{
    fields { field(1; No; Code[10]) { } field(2; Ref; Code[10]) { TableRelation = "Q Code"; } field(3; RefFB; Code[10]) { TableRelation = "Q Code FB"; }
      field(4; RefCard; Code[10]) { TableRelation = "Q Code Card"; } field(5; RefSub; Code[10]) { TableRelation = "Q Code Sub"; }
      field(6; RefVal; Code[10]) { TableRelation = "Q Code"; } field(7; RefAct; Code[10]) { TableRelation = "Q Code Act"; } }
    keys { key(PK; No) { } }
}
table 50405 "Q Line" { fields { field(1; No; Code[10]) { } field(2; LineNo; Integer) { } field(3; Ref; Code[10]) { TableRelation = "Q Code"; } } keys { key(PK; No, LineNo) { } } }
codeunit 50406 "Q Cfg"
{
    SingleInstance = true;
    var M: Integer; F: Integer; L: Text;
    procedure Set(NewM: Integer; NewF: Integer) begin M := NewM; F := NewF; L := ''; end;
    procedure Mode(): Integer begin exit(M); end;
    procedure Fail(): Integer begin exit(F); end;
    procedure Log(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
    [EventSubscriber(ObjectType::Page, Page::"Q Codes Sub", 'OnQueryClosePageEvent', '', false, false)]
    local procedure QSub(var Rec: Record "Q Code Sub"; var AllowClose: Boolean) begin L += 'evQ;'; end;
}
page 50410 "Q Codes"
{
    PageType = List; SourceTable = "Q Code"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Txt; Rec.Txt) { } } } }
    var Cfg: Codeunit "Q Cfg";
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Cfg.Log('Q ' + Format(CloseAction));
        case Cfg.Fail() of
            1: Message('q msg');
            2: exit(false);
        end;
        exit(true);
    end;
    trigger OnClosePage() begin Cfg.Log('C'); if Cfg.Fail() = 3 then Error('c err'); end;
}
page 50411 "Q Code Info" { PageType = CardPart; SourceTable = "Q Code FB"; layout { area(Content) { field(Code; Rec.Code) { ApplicationArea = All; } } } }
page 50412 "Q Codes FB"
{
    PageType = List; SourceTable = "Q Code FB"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } area(FactBoxes) { part(Info; "Q Code Info") { ApplicationArea = All; SubPageLink = Code = field(Code); } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end;
    trigger OnClosePage() begin end;
}
page 50413 "Q Codes Card" { PageType = Card; SourceTable = "Q Code Card"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { field(Code; Rec.Code) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end;
    trigger OnClosePage() begin end; }
page 50414 "Q Codes Sub" { PageType = List; SourceTable = "Q Code Sub"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end;
    trigger OnClosePage() begin end; }
page 50415 "Q Codes Act" { PageType = List; SourceTable = "Q Code Act"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
    actions { area(Processing) { action(CloseMe) { ApplicationArea = All; trigger OnAction() begin CurrPage.Close(); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end;
    trigger OnClosePage() begin end; }
page 50420 "Q Card"
{
    PageType = Card; SourceTable = "Q Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(No; Rec.No) { } field(Ref; Rec.Ref) { } field(RefFB; Rec.RefFB) { } field(RefCard; Rec.RefCard) { } field(RefSub; Rec.RefSub) { }
      field(RefAct; Rec.RefAct) { }
      field(RefVal; Rec.RefVal) { trigger OnValidate() var Cfg: Codeunit "Q Cfg"; begin Cfg.Log('V ' + Rec.RefVal); end; trigger OnAfterLookup(Selected: RecordRef) var Cfg: Codeunit "Q Cfg"; begin Cfg.Log('AL'); end; } } }
}
page 50421 "Q Lines" { PageType = ListPart; SourceTable = "Q Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec.LineNo) { } field(Ref; Rec.Ref) { } } } } }
page 50423 "Q Doc"
{
    PageType = Document; SourceTable = "Q Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(No; Rec.No) { } field(Ref; Rec.Ref) { } part(Lines; "Q Lines") { SubPageLink = No = field(No); } } }
}
codeunit 50430 "Q Tests"
{
    Subtype = Test;
    var Cfg: Codeunit "Q Cfg";
    local procedure Setup(M: Integer; F: Integer)
    var C: Record "Q Code"; C1: Record "Q Code FB"; C2: Record "Q Code Card"; C3: Record "Q Code Sub"; C4: Record "Q Code Act"; R: Record "Q Row"; L: Record "Q Line";
    begin
        C.DeleteAll(); C1.DeleteAll(); C2.DeleteAll(); C3.DeleteAll(); C4.DeleteAll(); R.DeleteAll(); L.DeleteAll();
        C.Code := 'C1'; C.Insert(); C.Code := 'C2'; C.Insert();
        C1.Code := 'C1'; C1.Insert(); C2.Code := 'C1'; C2.Insert(); C3.Code := 'C1'; C3.Insert(); C4.Code := 'C1'; C4.Insert();
        R.No := 'R1'; R.Insert(); L.No := 'R1'; L.LineNo := 1; L.Insert();
        Commit();
        Cfg.Set(M, F);
    end;
    local procedure Prior() begin asserterror Error('PRIOR'); end;
    {TEST}


    [ModalPageHandler] procedure CodesModal(var P: TestPage "Q Codes")
    begin
        case Cfg.Mode() of
            1: P.OK().Invoke();
            2: P.Cancel().Invoke();
            3: P.Close();
            6: begin P.Next(); P.Close(); end;
        end;
    end;
    [ModalPageHandler] procedure FBModal(var P: TestPage "Q Codes FB") begin if Cfg.Mode() = 3 then P.Close(); end;
    [ModalPageHandler] procedure CardModal(var P: TestPage "Q Codes Card") begin if Cfg.Mode() = 3 then P.Close(); end;
    [ModalPageHandler] procedure SubModal(var P: TestPage "Q Codes Sub") begin if Cfg.Mode() = 3 then P.Close(); end;
    [ModalPageHandler] procedure ActModal(var P: TestPage "Q Codes Act") begin if Cfg.Mode() = 5 then P.CloseMe.Invoke(); end;
}
"#;

/// One scenario per run: a refused scenario would mark the later tests of its codeunit as not run.
const TESTS: &[&str] = &[
    r#"[Test] [HandlerFunctions('CodesModal')] procedure A01_Card_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenEdit(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure A02_CardView_Returns() var P: TestPage "Q Card"; begin Setup(0, 0); P.OpenView(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure A03_CardView_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenView(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure A04_CardNew_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenNew(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure B01_Card_Returns_QueryMsg_LastError() var P: TestPage "Q Card"; T: Text; begin Setup(0, 1); P.OpenEdit(); Prior(); P.Ref.Lookup(); T := GetLastErrorText(); Error('OBS v=%1 last=%2', P.Ref.Value(), T); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure B02_Card_Returns_ClosePageErr_LastError() var P: TestPage "Q Card"; T: Text; begin Setup(0, 3); P.OpenEdit(); Prior(); P.Ref.Lookup(); T := GetLastErrorText(); Error('OBS v=%1 last=%2', P.Ref.Value(), T); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure B03_DocPart_Returns_QueryMsg_LastError() var P: TestPage "Q Doc"; T: Text; begin Setup(0, 1); P.OpenEdit(); P.Lines.First(); Prior(); P.Lines.Ref.Lookup(); T := GetLastErrorText(); Error('OBS v=%1 last=%2', P.Lines.Ref.Value(), T); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure B04_Card_Returns_QueryMsg_Code() var P: TestPage "Q Card"; T: Text; begin Setup(0, 1); P.OpenEdit(); ClearLastError(); P.Ref.Lookup(); T := GetLastErrorCode(); Error('OBS v=%1 code=%2', P.Ref.Value(), T); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C01_Card_Cancel_Vetoed() var P: TestPage "Q Card"; begin Setup(2, 2); P.OpenEdit(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C02_Card_Close_Vetoed() var P: TestPage "Q Card"; begin Setup(3, 2); P.OpenEdit(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C03_Card_Returns_Vetoed() var P: TestPage "Q Card"; begin Setup(0, 2); P.OpenEdit(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('ActModal')] procedure C04_Card_CustomActionCurrPageClose() var P: TestPage "Q Card"; begin Setup(5, 0); P.OpenEdit(); P.RefAct.Lookup(); Error('OBS v=%1 log=%2', P.RefAct.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C05_Card_MoveThenClose() var P: TestPage "Q Card"; begin Setup(6, 0); P.OpenEdit(); P.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C07_Card_ValidateField_Returns() var P: TestPage "Q Card"; begin Setup(0, 0); P.OpenEdit(); P.RefVal.Lookup(); Error('OBS v=%1 log=%2', P.RefVal.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure C08_Card_ValidateField_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenEdit(); P.RefVal.Lookup(); Error('OBS v=%1 log=%2', P.RefVal.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CodesModal')] procedure D08_DocPart_Returns_Vetoed() var P: TestPage "Q Doc"; begin Setup(0, 2); P.OpenEdit(); P.Lines.First(); P.Lines.Ref.Lookup(); Error('OBS v=%1 log=%2', P.Lines.Ref.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('FBModal')] procedure E01_Card_LookupPageWithFactBox_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenEdit(); P.RefFB.Lookup(); Error('OBS v=%1 log=%2', P.RefFB.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('FBModal')] procedure E02_Card_LookupPageWithFactBox_Returns() var P: TestPage "Q Card"; begin Setup(0, 0); P.OpenEdit(); P.RefFB.Lookup(); Error('OBS v=%1 log=%2', P.RefFB.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CardModal')] procedure E03_Card_LookupPageIsCard_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenEdit(); P.RefCard.Lookup(); Error('OBS v=%1 log=%2', P.RefCard.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('CardModal')] procedure E04_Card_LookupPageIsCard_Returns() var P: TestPage "Q Card"; begin Setup(0, 0); P.OpenEdit(); P.RefCard.Lookup(); Error('OBS v=%1 log=%2', P.RefCard.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('SubModal')] procedure E05_Card_LookupPageWithSubscriber_Close() var P: TestPage "Q Card"; begin Setup(3, 0); P.OpenEdit(); P.RefSub.Lookup(); Error('OBS v=%1 log=%2', P.RefSub.Value(), Cfg.Take()); end;"#,
    r#"[Test] [HandlerFunctions('SubModal')] procedure E06_Card_LookupPageWithSubscriber_Returns() var P: TestPage "Q Card"; begin Setup(0, 0); P.OpenEdit(); P.RefSub.Lookup(); Error('OBS v=%1 log=%2', P.RefSub.Value(), Cfg.Take()); end;"#,
];

fn outcomes() -> Vec<(String, Outcome)> {
    TESTS
        .iter()
        .flat_map(|t| al_testkit::outcomes_in(&SRC.replace("{TEST}", t), "Q Tests"))
        .collect()
}

fn text(o: &Outcome) -> String {
    format!("{o:?}")
}

#[test]
fn the_pinned_shape_writes_the_row_the_list_stands_on() {
    let o = outcomes();
    let (_, a01) = o.iter().find(|(n, _)| n == "A01_Card_Close").expect("A01");
    assert!(text(a01).contains("OBS v=C1 log=Q LookupCancel;C;"), "{a01:?}");
}

#[test]
fn neighbors_of_the_pinned_shape_do_not_write_the_pick() {
    let o = outcomes();
    let mut wrong = Vec::new();
    for (name, outcome) in &o {
        if name == "A01_Card_Close" || name.starts_with("B0") {
            continue;
        }
        if text(outcome).contains("v=C") {
            wrong.push(format!("{name}: {outcome:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_last_error_after_a_swallowed_close_error_is_not_read() {
    let o = outcomes();
    let mut wrong = Vec::new();
    for (name, outcome) in o.iter().filter(|(n, _)| n.starts_with("B0")) {
        if !matches!(outcome, Outcome::NotRun(_)) {
            wrong.push(format!("{name}: {outcome:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
