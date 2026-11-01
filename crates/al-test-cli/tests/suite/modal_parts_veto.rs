//! A modal page with a part whose close is refused: the parts are asked first, with None, then the host. A handler that does
//! nothing gets None after one OnQueryClosePage call; a refusal by the part alone answers the same and the host is never
//! asked (corpus pages/lt-modal-query-close-veto/PageModalQueryCloseVeto.al:448-542; al-runner issue 5762).

use al_testkit::{outcomes_in, run_in, text};

const SRC: &str = r#"
table 69990 "MV Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 69991 "MV Log"
{
    SingleInstance = true;
    var HostVeto: Boolean; PartVeto: Boolean; E: Text; Mode: Integer;
    procedure Reset(H: Boolean; P: Boolean; M: Integer) begin HostVeto := H; PartVeto := P; Mode := M; E := ''; end;
    procedure Add(S: Text) begin if E <> '' then E += ';'; E += S; end;
    procedure Host(CloseAction: Action): Boolean begin Add('host(' + Format(CloseAction) + ')'); exit(not HostVeto); end;
    procedure Part(CloseAction: Action): Boolean begin Add('part(' + Format(CloseAction) + ')'); exit(not PartVeto); end;
    procedure Dump(): Text begin exit(E); end;
    procedure GetMode(): Integer begin exit(Mode); end;
}
page 69992 "MV FactBox"
{
    PageType = CardPart; SourceTable = "MV Row"; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { ApplicationArea = All; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var L: Codeunit "MV Log"; begin exit(L.Part(CloseAction)); end;
    trigger OnClosePage() var L: Codeunit "MV Log"; begin L.Add('part close'); end;
}
page 69993 "MV List"
{
    PageType = List; SourceTable = "MV Row"; Editable = false; UsageCategory = None;
    layout
    {
        area(Content) { repeater(R) { field(Id; Rec.Id) { ApplicationArea = All; } } }
        area(FactBoxes) { part(Details; "MV FactBox") { ApplicationArea = All; SubPageLink = Id = field(Id); } }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var L: Codeunit "MV Log"; begin N += 1; exit(L.Host(CloseAction)); end;
    trigger OnClosePage() var L: Codeunit "MV Log"; begin L.Add('host close'); end;
    var N: Integer;
    procedure Count(): Integer begin exit(N); end;
}
codeunit 69994 T
{
    Subtype = Test;
    var Got: Text;
    local procedure Run(H: Boolean; P: Boolean; M: Integer): Text
    var Row: Record "MV Row"; L: Codeunit "MV Log"; Pg: Page "MV List"; A: Action;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
        L.Reset(H, P, M);
        ClearLastError();
        A := Pg.RunModal();
        exit(Format(A) + '|' + Format(Pg.Count()) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + L.Dump());
    end;
    [Test] [HandlerFunctions('H')]
    procedure HostVetoNothing()
    begin
        if Run(true, false, 0) <> 'None|1|The page cannot be closed at this time.|FormCloseNotAllowed|part(None);host(OK)' then
            Error('%1', Run(true, false, 0));
    end;
    [Test] [HandlerFunctions('H')]
    procedure HostVetoOk()
    begin
        if Run(true, false, 1) <> 'None|2|The page cannot be closed at this time.|FormCloseNotAllowed|part(None);host(OK);part(None);host(OK)' then
            Error('%1', Run(true, false, 1));
    end;
    [Test] [HandlerFunctions('H')]
    procedure PartVetoOk()
    begin
        if Run(false, true, 1) <> 'None|0|The page cannot be closed at this time.|FormCloseNotAllowed|part(None);part(None)' then
            Error('%1', Run(false, true, 1));
    end;
    [Test] [HandlerFunctions('H')]
    procedure Allowed()
    begin
        if Run(false, false, 0) <> 'OK|1|||part(None);host(OK);part close;host close' then
            Error('%1', Run(false, false, 0));
    end;
    [Test] [HandlerFunctions('H')]
    procedure HostVetoHandlerError()
    var Row: Record "MV Row"; L: Codeunit "MV Log"; Pg: Page "MV List";
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert();
        L.Reset(true, false, 2);
        asserterror Pg.RunModal();
        if GetLastErrorCode() + '|' + L.Dump() <> 'TestActionNotFound|part(None);host(OK)' then
            Error('%1', GetLastErrorCode() + '|' + L.Dump());
    end;
    [ModalPageHandler]
    procedure H(var P: TestPage "MV List")
    var L: Codeunit "MV Log";
    begin
        case L.GetMode() of
            1: P.OK().Invoke();
            2: P.Cancel().Invoke();
        end;
    end;
}
codeunit 69996 T2
{
    Subtype = Test;
    [Test] [HandlerFunctions('H')]
    procedure AllowedHandlerError()
    var Row: Record "MV Row"; L: Codeunit "MV Log"; Pg: Page "MV List";
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert();
        L.Reset(false, false, 2);
        asserterror Pg.RunModal();
    end;
    [ModalPageHandler]
    procedure H(var P: TestPage "MV List")
    begin
        P.Cancel().Invoke();
    end;
}
codeunit 69995 C
{
    procedure PartVetoNotModal(): Text
    var Row: Record "MV Row"; L: Codeunit "MV Log"; P: TestPage "MV List";
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert();
        L.Reset(false, true, 0);
        P.OpenView();
        P.Close();
        exit(L.Dump());
    end;
}
"#;

#[test]
fn a_modal_page_with_a_part_answers_a_veto_as_one_without_parts() {
    for (name, outcome) in outcomes_in(SRC, "T") {
        assert_eq!(format!("{outcome:?}"), "Pass", "{name}");
    }
}

#[test]
fn an_error_in_the_handler_of_a_page_whose_close_is_allowed_is_refused() {
    let o = outcomes_in(SRC, "T2");
    assert!(format!("{:?}", o[0].1).starts_with("NotRun"), "{o:?}");
}

#[test]
fn a_parts_veto_of_a_page_not_run_modally_stays_refused() {
    let r = run_in(SRC, "C", "PartVetoNotModal");
    assert!(
        matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
        "{}",
        text(r)
    );
}
