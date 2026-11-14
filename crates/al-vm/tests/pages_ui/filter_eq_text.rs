//! `TestPage.Filter.SetFilter` with a text that starts with `=`: Business Central sets no filter on a List or Worksheet the test
//! opened with OpenView, for a Boolean word or an Enum or Option member (corpus testpage/filters/TempListOffPageBooleanFilter.al,
//! WorksheetEnumEqFilter.al of corpus PR 1392, al-runner issue 6924). These tests hold another `=` text on a Boolean, Enum or Option
//! field of a List or Worksheet to a refusal, and a text on another page type or field type to main's outcome (the filter applies).

const SRC: &str = r#"
table 69990 "EQ Row"
{
    fields { field(1; Id; Integer) { } field(2; Flag; Boolean) { } field(3; Name; Code[10]) { } field(4; Kind; Option) { OptionMembers = Quote,"Order",Invoice; } }
    keys { key(PK; Id) { } }
}
page 69991 "EQ List"
{
    PageType = List; SourceTable = "EQ Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } field(Name; Rec.Name) { } field(Kind; Rec.Kind) { } } } }
}
page 69994 "EQ Temp List"
{
    PageType = List; SourceTable = "EQ Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Kind; Rec.Kind) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.Kind := 0; Rec.Insert(); Rec.Id := 2; Rec.Kind := 1; Rec.Insert(); Rec.Id := 3; Rec.Kind := 0; Rec.Insert(); Rec.Id := 4; Rec.Kind := 1; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69992 "EQ Card"
{
    PageType = Card; SourceTable = "EQ Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } } }
}
codeunit 69993 C
{
    local procedure Seed()
    var Row: Record "EQ Row";
    begin
        Row.DeleteAll();
        Add(1, false, 'A', 0); Add(2, true, 'B', 1); Add(3, false, 'C', 0); Add(4, true, 'D', 1);
    end;
    local procedure Add(Id: Integer; Flag: Boolean; Name: Code[10]; Kind: Integer)
    var Row: Record "EQ Row";
    begin
        Row.Init(); Row.Id := Id; Row.Flag := Flag; Row.Name := Name; Row.Kind := Kind; Row.Insert();
    end;
    local procedure Walk(var P: TestPage "EQ List"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    procedure ViewBoolean(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1');
        exit('[' + P.Filter.GetFilter(Flag) + ']' + Walk(P));
    end;
    procedure ViewOption(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, '=Quote');
        exit('[' + P.Filter.GetFilter(Kind) + ']' + Walk(P));
    end;
    procedure ViewOptionSecond(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, '=Order');
        exit(Walk(P));
    end;
    procedure TempViewOptionSecond(): Text
    var P: TestPage "EQ Temp List"; Ids: Text;
    begin
        P.OpenView(); P.Fill.Invoke(); P.Filter.SetFilter(Kind, '=Order');
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(Kind) + ']' + Ids);
    end;
    procedure EditBoolean(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenEdit(); P.Filter.SetFilter(Flag, '=1');
        exit(Walk(P));
    end;
    procedure ViewCode(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Name, '=B');
        exit(Walk(P));
    end;
    procedure ViewInteger(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Id, '=2');
        exit(Walk(P));
    end;
    procedure ViewBooleanOtherWord(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1|0');
        exit(Walk(P));
    end;
    procedure ViewBooleanAfterOwnFilter(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '0'); P.Filter.SetFilter(Flag, '=1');
        exit(Walk(P));
    end;
    procedure CardBoolean(): Text
    var P: TestPage "EQ Card";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1');
        exit(P.Id.Value());
    end;
}
"#;

fn run(proc: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", proc))
}

fn refusal(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// The measured cell on a database list: no filter, every row (TempListOffPageBooleanFilter.al:516 and :447-449).
#[test]
fn an_eq_text_on_a_viewed_list_sets_no_filter() {
    assert_eq!(run("ViewBoolean"), "[]1,2,3,4,");
    // Another Option member after the `=` on the temporary List sets no filter either (corpus PR 1392,
    // WorksheetEnumEqFilter.al:604-609).
    assert_eq!(run("TempViewOptionSecond"), "[]1,2,3,4,");
}

// Outside the rule's neighborhood: a Code or Integer field, a Card. Each keeps main's filter.
#[test]
fn shapes_outside_the_cell_keep_the_filter() {
    assert_eq!(run("ViewCode"), "2,");
    assert_eq!(run("ViewInteger"), "2,");
    assert_eq!(run("CardBoolean"), "2");
}

// In the neighborhood but not measured: OpenEdit, another text after the `=`, a field the test already filtered. Main applied the
// filter, which BC did on no measured shape (WorksheetEnumEqFilter.al:14-17): each refuses.
#[test]
fn unmeasured_shapes_on_a_list_refuse() {
    for (proc, why) in [
        ("EditBoolean", "did not open with its own OpenView"),
        ("ViewBooleanOtherWord", "not a measured Boolean word"),
        ("ViewBooleanAfterOwnFilter", "after a filter on the field"),
    ] {
        let got = refusal(al_testkit::run_in(SRC, "C", proc));
        assert!(got.contains(why), "{proc}: {got}");
    }
}

// On a List over a database table only the Boolean words are measured (TempListOffPageBooleanFilter.al:508-521, :534-547). An
// Option member there (the first one was measured only on the temporary List, :440; the others at WorksheetEnumEqFilter.al:598,
// also temporary) refuses. Main ran `=Quote` as dropped and `=Order` as a filter.
#[test]
fn an_option_member_on_a_database_list_refuses() {
    for proc in ["ViewOption", "ViewOptionSecond"] {
        let why = refusal(al_testkit::run_in(SRC, "C", proc));
        assert!(why.contains("List page over a database table"), "{proc}: {why}");
    }
}

/// A Worksheet and a List over a table with an Enum, for the cells of WorksheetEnumEqFilter.al (corpus PR 1392) and their neighbors.
const WS_SRC: &str = r#"
enum 69980 "EQW Type" { Extensible = false; value(0; Quote) { } value(1; "Order") { } value(2; Invoice) { } }
table 69981 "EQW Row"
{
    fields { field(1; Id; Integer) { } field(2; DocType; Enum "EQW Type") { } field(3; Kind; Option) { OptionMembers = Quote,"Order",Invoice; } field(4; IsHist; Boolean) { } field(5; Name; Code[10]) { } }
    keys { key(PK; Id) { } }
}
page 69982 "EQW Worksheet"
{
    PageType = Worksheet; SourceTable = "EQW Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } field(Kind; Rec.Kind) { } } } }
}
page 69983 "EQW Temp Worksheet"
{
    PageType = Worksheet; SourceTable = "EQW Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.DocType := Rec.DocType::Quote; Rec.Insert(); Rec.Id := 2; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); Rec.Id := 3; Rec.DocType := Rec.DocType::Invoice; Rec.Insert(); Rec.Id := 4; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69986 "EQW Temp List"
{
    PageType = List; SourceTable = "EQW Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.DocType := Rec.DocType::Quote; Rec.Insert(); Rec.Id := 2; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); Rec.Id := 3; Rec.DocType := Rec.DocType::Invoice; Rec.Insert(); Rec.Id := 4; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69984 "EQW List"
{
    PageType = List; SourceTable = "EQW Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } } } }
}
codeunit 69985 C
{
    local procedure Seed()
    var Row: Record "EQW Row";
    begin
        Row.DeleteAll();
        Add(1, 0, false); Add(2, 1, false); Add(3, 2, true); Add(4, 1, true);
    end;
    local procedure Add(Id: Integer; T: Integer; H: Boolean)
    var Row: Record "EQW Row";
    begin
        Row.Init(); Row.Id := Id; Row.DocType := "EQW Type".FromInteger(T); Row.Kind := T; Row.IsHist := H; Row.Name := 'N' + Format(Id); Row.Insert();
    end;
    local procedure Walk(var P: TestPage "EQW Worksheet"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    local procedure WalkTemp(var P: TestPage "EQW Temp Worksheet"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    procedure WsEnum(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, FilterText);
        exit('[' + P.Filter.GetFilter(DocType) + ']' + Walk(P));
    end;
    procedure WsOption(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, FilterText);
        exit('[' + P.Filter.GetFilter(Kind) + ']' + Walk(P));
    end;
    procedure WsBoolean(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(IsHist, FilterText);
        exit('[' + P.Filter.GetFilter(IsHist) + ']' + Walk(P));
    end;
    procedure WsCode(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Name, FilterText);
        exit(Walk(P));
    end;
    procedure WsEditEnum(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenEdit(); P.Filter.SetFilter(DocType, FilterText);
        exit(Walk(P));
    end;
    procedure WsEnumAfterOwnFilter(): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, 'Order'); P.Filter.SetFilter(DocType, '=Quote');
        exit(Walk(P));
    end;
    procedure TempWsEnumAfterOwnFilter(): Text
    var P: TestPage "EQW Temp Worksheet";
    begin
        P.OpenView(); P.Fill.Invoke(); P.Filter.SetFilter(DocType, 'Order'); P.Filter.SetFilter(DocType, '=Quote');
        exit('[' + P.Filter.GetFilter(DocType) + ']' + WalkTemp(P));
    end;
    procedure TempListEnum(FilterText: Text): Text
    var P: TestPage "EQW Temp List"; Ids: Text;
    begin
        P.OpenView(); P.Fill.Invoke(); P.Filter.SetFilter(DocType, FilterText);
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(DocType) + ']' + Ids);
    end;
    procedure ListEnum(FilterText: Text): Text
    var P: TestPage "EQW List"; Ids: Text;
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, FilterText);
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(DocType) + ']' + Ids);
    end;
}
"#;

fn ws(proc: &str, arg: &str) -> String {
    let arg = al_builtins::value::Value::Text(arg.to_string().into());
    al_testkit::text(al_testkit::run_with(WS_SRC, "C", proc, vec![arg]).map(|(v, _)| v))
}

fn ws0(proc: &str) -> String {
    al_testkit::text(al_testkit::run_in(WS_SRC, "C", proc))
}

// WorksheetEnumEqFilter.al (corpus PR 1392): a text that starts with `=` sets no filter on the database Worksheet for the Enum
// members Quote and Order (:634-635, :638), the Option member Order (:639) and the Boolean's `=1` (:640), and on the temporary List
// for an Enum name (:587-592).
#[test]
fn an_eq_text_on_a_viewed_worksheet_or_list_sets_no_filter() {
    for t in ["=Quote", "=Order"] {
        assert_eq!(ws("WsEnum", t), "[]1,2,3,4,", "{t}");
    }
    for t in ["=Quote", "=Order", "=Invoice"] {
        assert_eq!(ws("TempListEnum", t), "[]1,2,3,4,", "{t}");
    }
    assert_eq!(ws("WsOption", "=Order"), "[]1,2,3,4,");
    assert_eq!(ws("WsBoolean", "=1"), "[]1,2,3,4,");
}

// An ordinal after the `=` is measured only on the temporary sources (:409-410, :452, :607): on the database Worksheet it refuses.
// An Enum name on a List over a database table is not measured (the List cells are temporary, :575): it refuses. Main ran each of
// them as a filter.
#[test]
fn unmeasured_database_shapes_refuse() {
    let refused = |proc: &str, arg: &str| {
        let arg = al_builtins::value::Value::Text(arg.to_string().into());
        refusal(al_testkit::run_with(WS_SRC, "C", proc, vec![arg]).map(|(v, _)| v))
    };
    for t in ["=0", "=1"] {
        assert!(
            refused("WsEnum", t).contains("Worksheet page over a database table"),
            "{t}"
        );
    }
    assert!(refused("WsOption", "=1").contains("Worksheet page over a database table"));
    // The third member (`Invoice`, position 2) was measured on the temporary pages only (:412-419, :453-458), not on the database
    // Worksheet (:620-642). Main filtered on it (review of al-runner PR 8009, round 2: G01, G02).
    for proc in ["WsEnum", "WsOption"] {
        assert!(
            refused(proc, "=Invoice").contains("Worksheet page over a database table"),
            "{proc}"
        );
    }
    for t in ["=Quote", "=Order"] {
        assert!(
            refused("ListEnum", t).contains("List page over a database table"),
            "{t}"
        );
    }
}

// :545-569: on the temporary Worksheet an `=` text after a filter of the test keeps that filter.
#[test]
fn an_eq_text_keeps_the_earlier_filter_on_the_temporary_worksheet() {
    assert_eq!(ws0("TempWsEnumAfterOwnFilter"), "[Order]2,4,");
}

// Outside the neighborhood: a Code field keeps main's filter; `<>Quote` and a bare name keep filter.rs's parsing (:406, :408).
#[test]
fn worksheet_shapes_outside_the_cells_keep_the_filter() {
    assert_eq!(ws("WsCode", "=N2"), "2,");
    assert_eq!(ws("WsEnum", "<>Quote"), "[<>0]2,3,4,");
    assert_eq!(ws("WsEnum", "Order"), "[Order]2,4,");
}

// In the neighborhood but not measured: OpenEdit, an ordinal other than 0 and 1, a name with another case, an earlier filter on
// the database Worksheet. Main applied the filter: each refuses.
#[test]
fn unmeasured_worksheet_shapes_refuse() {
    let refused = |proc: &str, arg: &str| {
        let arg = al_builtins::value::Value::Text(arg.to_string().into());
        refusal(al_testkit::run_with(WS_SRC, "C", proc, vec![arg]).map(|(v, _)| v))
    };
    assert!(refused("WsEditEnum", "=Order").contains("did not open with its own OpenView"));
    for t in ["=2", "=order"] {
        assert!(
            refused("WsEnum", t).contains("not a measured Boolean word, member name or ordinal"),
            "{t}"
        );
    }
    assert!(refusal(al_testkit::run_in(WS_SRC, "C", "WsEnumAfterOwnFilter")).contains("after a filter on the field"));
}

// Neighbors of the cells (review of al-runner PR 8009): an Enum or Option with captions, a member name with a blank, an earlier
// filter the test did not type as a bare member name, a filter that page code wrote, an Option `<>` typed as a number, and the
// database Worksheet texts the corpus did not measure there.
const NB_SRC: &str = r#"
enum 69970 "NB Cap" { Extensible = false; value(0; Quote) { Caption = 'Offer'; } value(1; "Order") { Caption = 'Bestellung'; } value(2; "Credit Memo") { Caption = 'Credit Memo'; } }
enum 69971 "NB Plain" { Extensible = false; value(0; Quote) { } value(1; "Order") { } value(2; "Credit Memo") { } }
table 69972 "NB Row"
{
    fields { field(1; Id; Integer) { } field(2; T; Enum "NB Cap") { } field(3; K; Option) { OptionMembers = Quote,"Order",Invoice; OptionCaption = 'Offer,Bestellung,Rechnung'; } field(4; B; Boolean) { } field(5; P; Enum "NB Plain") { } field(6; O; Option) { OptionMembers = Quote,"Order",Invoice; } }
    keys { key(PK; Id) { } }
    procedure Fill()
    var I: Integer;
    begin
        Rec.DeleteAll();
        for I := 1 to 3 do begin
            Rec.Init(); Rec.Id := I; Rec.T := "NB Cap".FromInteger(I - 1); Rec.P := "NB Plain".FromInteger(I - 1); Rec.K := I - 1; Rec.O := I - 1; Rec.B := I = 3; Rec.Insert();
        end;
    end;
}
page 69973 "NB Temp WS"
{
    PageType = Worksheet; SourceTable = "NB Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(T; Rec.T) { } field(K; Rec.K) { } field(P; Rec.P) { } field(O; Rec.O) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Fill(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69974 "NB Temp List"
{
    PageType = List; SourceTable = "NB Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(T; Rec.T) { } field(K; Rec.K) { } field(P; Rec.P) { } field(O; Rec.O) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Fill(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69975 "NB Phys WS"
{
    PageType = Worksheet; SourceTable = "NB Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(T; Rec.T) { } field(K; Rec.K) { } field(P; Rec.P) { } field(O; Rec.O) { } } } }
}
page 69976 "NB Temp List OpenNe"
{
    PageType = List; SourceTable = "NB Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(O; Rec.O) { } } } }
    trigger OnOpenPage() begin Rec.Fill(); Rec.SetFilter(O, '<>%1', Rec.O::Quote); end;
}
page 69977 "NB Temp WS OpenFlt"
{
    PageType = Worksheet; SourceTable = "NB Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(P; Rec.P) { } } } }
    trigger OnOpenPage() begin Rec.Fill(); Rec.SetRange(P, Rec.P::"Order"); end;
}
codeunit 69978 C
{
    local procedure SetTw(var Pg: TestPage "NB Temp WS"; F: Text; FilterText: Text)
    begin
        case F of 'T': Pg.Filter.SetFilter(T, FilterText); 'K': Pg.Filter.SetFilter(K, FilterText); 'B': Pg.Filter.SetFilter(B, FilterText); 'P': Pg.Filter.SetFilter(P, FilterText); 'O': Pg.Filter.SetFilter(O, FilterText); end;
    end;
    local procedure GetTw(var Pg: TestPage "NB Temp WS"; F: Text): Text
    begin
        case F of 'T': exit(Pg.Filter.GetFilter(T)); 'K': exit(Pg.Filter.GetFilter(K)); 'B': exit(Pg.Filter.GetFilter(B)); 'P': exit(Pg.Filter.GetFilter(P)); 'O': exit(Pg.Filter.GetFilter(O)); end;
    end;
    procedure TempWs(F: Text; First: Text; FilterText: Text): Text
    var Pg: TestPage "NB Temp WS"; Ids: Text;
    begin
        Pg.OpenView(); Pg.Fill.Invoke();
        if First <> '' then SetTw(Pg, F, First);
        SetTw(Pg, F, FilterText);
        if Pg.First() then repeat Ids += Pg.Id.Value() + ','; until not Pg.Next();
        exit('filter=' + GetTw(Pg, F) + ' rows=' + Ids);
    end;
    procedure TempList(F: Text; FilterText: Text): Text
    var Pg: TestPage "NB Temp List"; Ids: Text;
    begin
        Pg.OpenView(); Pg.Fill.Invoke();
        case F of 'T': Pg.Filter.SetFilter(T, FilterText); 'P': Pg.Filter.SetFilter(P, FilterText); 'O': Pg.Filter.SetFilter(O, FilterText); 'K': Pg.Filter.SetFilter(K, FilterText); end;
        if Pg.First() then repeat Ids += Pg.Id.Value() + ','; until not Pg.Next();
        case F of 'T': exit('filter=' + Pg.Filter.GetFilter(T) + ' rows=' + Ids); 'P': exit('filter=' + Pg.Filter.GetFilter(P) + ' rows=' + Ids); end;
        exit('filter=' + Pg.Filter.GetFilter(O) + ' rows=' + Ids);
    end;
    procedure PhysWs(F: Text; FilterText: Text): Text
    var Row: Record "NB Row"; Pg: TestPage "NB Phys WS"; Ids: Text;
    begin
        Row.Fill(); Pg.OpenView();
        case F of 'O': Pg.Filter.SetFilter(O, FilterText); 'B': Pg.Filter.SetFilter(B, FilterText); 'P': Pg.Filter.SetFilter(P, FilterText); end;
        if Pg.First() then repeat Ids += Pg.Id.Value() + ','; until not Pg.Next();
        exit('rows=' + Ids);
    end;
    procedure OpenNe(): Text
    var Pg: TestPage "NB Temp List OpenNe";
    begin
        Pg.OpenView();
        exit('filter=' + Pg.Filter.GetFilter(O));
    end;
    procedure OpenFlt(): Text
    var Pg: TestPage "NB Temp WS OpenFlt"; R: Text;
    begin
        Pg.OpenView();
        R := 'before=' + Pg.Filter.GetFilter(P);
        Pg.Filter.SetFilter(P, '=Quote');
        R += ' after=' + Pg.Filter.GetFilter(P);
        if Pg.First() then R += ' first=' + Pg.Id.Value();
        exit(R);
    end;
}
"#;

fn nb(proc: &str, args: &[&str]) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let args = args
        .iter()
        .map(|a| al_builtins::value::Value::Text(a.to_string().into()))
        .collect();
    al_testkit::run_with(NB_SRC, "C", proc, args).map(|(v, _)| v)
}

// Where no cell measured the shape: WorksheetEnumEqFilter.al's enum and option have no captions (:25-46) and one-word members; its
// only earlier filter is the test's bare `Order` (:554); the Worksheet Boolean words are `=1` and `=0` (:472-473); an Enum ordinal
// only on the Worksheet (:418-419). Main applied each `=` text as a filter: each refuses. A `<>` text keeps filter.rs's parsing
// (:415, :456), and a filter page code wrote reads as on main.
#[test]
fn shapes_next_to_the_eq_cells_refuse_or_keep_main_filter() {
    let t = |p: &str, a: &[&str]| al_testkit::text(nb(p, a));
    let refused = |p: &str, a: &[&str]| refusal(nb(p, a));
    assert!(refused("TempWs", &["T", "", "=Order"]).contains("shows captions"));
    assert!(refused("TempWs", &["K", "", "=Order"]).contains("shows captions"));
    assert!(refused("TempWs", &["P", "", "=Credit Memo"]).contains("more than one word"));
    assert!(refused("TempWs", &["P", "<>Quote", "=Order"]).contains("after a filter on the field"));
    assert!(refused("TempWs", &["B", "", "=Yes"]).contains("Boolean word not measured"));
    assert!(refused("TempList", &["P", "=1"]).contains("Enum ordinal"));
    assert!(refused("OpenFlt", &[]).contains("after a filter on the field"));
    assert_eq!(t("TempWs", &["O", "", "<>0"]), "filter=<>Quote rows=2,3,");
    assert_eq!(t("TempWs", &["K", "", "<>Quote"]), "filter=<>Quote rows=2,3,");
    assert_eq!(t("OpenNe", &[]), "filter=<>Quote");
}

// The measured cells next to them still take the path: :554-566, :456, :612, :631, :640.
#[test]
fn the_measured_eq_cells_next_to_them_still_drop() {
    let t = |p: &str, a: &[&str]| al_testkit::text(nb(p, a));
    assert_eq!(t("TempWs", &["P", "Order", "=Quote"]), "filter=Order rows=2,");
    assert_eq!(t("TempWs", &["O", "", "<>Quote"]), "filter=<>0 rows=2,3,");
    assert_eq!(t("TempList", &["O", "=1"]), "filter= rows=1,2,3,");
    assert_eq!(t("PhysWs", &["O", "=Order"]), "rows=1,2,3,");
    assert_eq!(t("PhysWs", &["B", "=1"]), "rows=1,2,3,");
}

// Main drops the Option's first member on a List (TempListOffPageBooleanFilter.al:440, no captions); with OptionCaption it is not
// measured, and the path refuses.
#[test]
fn an_option_first_member_with_captions_on_a_list_refuses() {
    assert!(refusal(nb("TempList", &["K", "=Quote"])).contains("shows captions"));
}

// On the database Worksheet the corpus measured the Option's `=Order` and the Boolean's `=1` only (:631, :632): the first member
// and another Boolean word refuse there, as the other database shapes do.
#[test]
fn unmeasured_database_worksheet_texts_refuse() {
    for (field, text) in [("O", "=Quote"), ("B", "=Yes"), ("B", "=0")] {
        assert!(
            refusal(nb("PhysWs", &[field, text])).contains("Worksheet page over a database table"),
            "{field} {text}"
        );
    }
}

const GAP_SRC: &str = r#"
enum 69990 "EQG Gap" { Extensible = false; value(10; Alpha) { } value(20; Beta) { } value(30; Gamma) { } }
table 69991 "EQG Row"
{
    fields { field(1; Id; Integer) { } field(2; G; Enum "EQG Gap") { } }
    keys { key(PK; Id) { } }
}
page 69992 "EQG Worksheet"
{
    PageType = Worksheet; SourceTable = "EQG Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(G; Rec.G) { } } } }
}
page 69993 "EQG Temp Worksheet"
{
    PageType = Worksheet; SourceTable = "EQG Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(G; Rec.G) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.G := Rec.G::Alpha; Rec.Insert(); Rec.Id := 2; Rec.G := Rec.G::Beta; Rec.Insert(); Rec.Id := 3; Rec.G := Rec.G::Gamma; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
codeunit 69994 C
{
    procedure Ws(FilterText: Text): Text
    var Row: Record "EQG Row"; P: TestPage "EQG Worksheet"; Ids: Text; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 3 do begin Row.Init(); Row.Id := I; Row.G := "EQG Gap".FromInteger(I * 10); Row.Insert(); end;
        P.OpenView(); P.Filter.SetFilter(G, FilterText);
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(G) + ']' + Ids);
    end;
    procedure TempWs(FilterText: Text): Text
    var P: TestPage "EQG Temp Worksheet"; Ids: Text;
    begin
        P.OpenView(); P.Fill.Invoke(); P.Filter.SetFilter(G, FilterText);
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(G) + ']' + Ids);
    end;
}
"#;

fn gap(proc: &str, arg: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let arg = al_builtins::value::Value::Text(arg.to_string().into());
    al_testkit::run_with(GAP_SRC, "C", proc, vec![arg]).map(|(v, _)| v)
}

// WorksheetEnumEqFilter.al's enum has the ordinals 0, 1, 2 at the positions 0, 1, 2 (:25-32), and BC shows an Enum filter by its
// ordinal (:534, :640). An enum whose ordinals are not its positions was not measured (review of al-runner PR 8009, round 2: G03,
// G04): the text refuses, on the database and on the temporary Worksheet (main filtered on the temporary one).
#[test]
fn an_enum_whose_ordinals_are_not_its_positions_is_not_a_measured_cell() {
    for t in ["=Alpha", "=Beta"] {
        assert!(
            refusal(gap("Ws", t)).contains("Worksheet page over a database table"),
            "{t}"
        );
    }
    for t in ["=Alpha", "=Beta"] {
        assert!(refusal(gap("TempWs", t)).contains("the measured 3 members"), "{t}");
    }
}

const SIZE_SRC: &str = r#"
enum 69980 "EQS Four" { Extensible = false; value(0; First) { } value(1; Second) { } value(2; Third) { } value(3; Fourth) { } }
enum 69981 "EQS Ext" { Extensible = true; value(0; One) { } value(1; Two) { } }
enumextension 69982 "EQS Ext X" extends "EQS Ext" { value(2; Three) { } }
table 69983 "EQS Row"
{
    fields { field(1; Id; Integer) { } field(2; F; Enum "EQS Four") { } field(3; X; Enum "EQS Ext") { } field(4; O; Option) { OptionMembers = A,B,C,D; } }
    keys { key(PK; Id) { } }
}
page 69984 "EQS Worksheet"
{
    PageType = Worksheet; SourceTable = "EQS Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(F; Rec.F) { } field(X; Rec.X) { } field(O; Rec.O) { } } } }
}
page 69985 "EQS Temp Worksheet"
{
    PageType = Worksheet; SourceTable = "EQS Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(F; Rec.F) { } field(X; Rec.X) { } field(O; Rec.O) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.Insert(); Rec.Id := 2; Rec.F := Rec.F::Fourth; Rec.X := Rec.X::Three; Rec.O := Rec.O::D; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
codeunit 69986 C
{
    procedure Ws(Field: Integer; FilterText: Text): Text
    var Row: Record "EQS Row"; P: TestPage "EQS Worksheet"; Ids: Text;
    begin
        Row.DeleteAll();
        Row.Init(); Row.Id := 1; Row.Insert();
        Row.Id := 2; Row.F := Row.F::Fourth; Row.X := Row.X::Three; Row.O := Row.O::D; Row.Insert();
        P.OpenView();
        case Field of 2: P.Filter.SetFilter(F, FilterText); 3: P.Filter.SetFilter(X, FilterText); 4: P.Filter.SetFilter(O, FilterText); end;
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    procedure TempWs(Field: Integer; FilterText: Text): Text
    var P: TestPage "EQS Temp Worksheet"; Ids: Text;
    begin
        P.OpenView(); P.Fill.Invoke();
        case Field of 2: P.Filter.SetFilter(F, FilterText); 3: P.Filter.SetFilter(X, FilterText); 4: P.Filter.SetFilter(O, FilterText); end;
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
}
"#;

fn size(proc: &str, field: i32, arg: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let args = vec![
        al_builtins::value::Value::Integer(field),
        al_builtins::value::Value::Text(arg.to_string().into()),
    ];
    al_testkit::run_with(SIZE_SRC, "C", proc, args).map(|(v, _)| v)
}

// WorksheetEnumEqFilter.al's enum and option have exactly three members at the positions 0-2, the enum with no enumextension member
// (:25-46). An enum or option of another size, or an enum with a member from an enumextension, was not measured: the text refuses,
// on the database and on the temporary Worksheet (main filtered on the temporary one).
#[test]
fn an_enum_or_option_of_another_size_is_not_a_measured_cell() {
    for (field, t) in [
        (2, "=Fourth"),
        (2, "=First"),
        (3, "=Three"),
        (3, "=One"),
        (4, "=D"),
        (4, "=B"),
    ] {
        assert!(
            refusal(size("Ws", field, t)).contains("Worksheet page over a database table"),
            "{field} {t}"
        );
    }
    for (field, t) in [(2, "=Fourth"), (3, "=Three"), (4, "=D")] {
        assert!(
            refusal(size("TempWs", field, t)).contains("the measured 3 members"),
            "{field} {t}"
        );
    }
}

/// The search Worksheet of al-runner issue 6924 (HIS), as the reporter described it: a temporary source that an action refills
/// (Reset, a table procedure, CurrPage.Update(false)), header controls on page variables that the test fills, a Boolean without a
/// control and an Enum of six members.
const HIS_SRC: &str = r#"
enum 69940 "HS Type" { Extensible = true; value(0; Quote) { } value(1; "Order") { } value(2; Invoice) { } value(3; "Credit Memo") { } value(4; "Blanket Order") { } value(5; "Return Order") { } }
table 69941 "HS Line"
{
    fields { field(1; "Document Type"; Enum "HS Type") { } field(2; "No."; Code[20]) { } field(3; "Is History"; Boolean) { } field(4; Name; Code[20]) { } }
    keys { key(PK; "Document Type", "No.", "Is History") { } }
    procedure Search(Customer: Code[20])
    begin
        Rec.DeleteAll();
        Rec.Init(); Rec."Document Type" := Rec."Document Type"::Quote; Rec."No." := 'Q1'; Rec.Name := Customer; Rec.Insert();
        Rec.Init(); Rec."Document Type" := Rec."Document Type"::"Order"; Rec."No." := 'O1'; Rec.Name := Customer; Rec.Insert();
    end;
}
page 69942 "HS Search"
{
    PageType = Worksheet; SourceTable = "HS Line"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(Header) { field(DocFilter; DocFilter) { } field(CustFilter; CustFilter) { } }
            repeater(Rows) { field("No."; Rec."No.") { Editable = false; StyleExpr = Rec."Is History"; } field("Document Type"; Rec."Document Type") { Editable = false; } }
        }
    }
    actions { area(Processing) { action(SearchAction) { trigger OnAction() begin Rec.Reset(); Rec.Search(CustFilter); CurrPage.Update(false); end; } } }
    var DocFilter: Boolean; CustFilter: Code[20];
}
page 69943 "HS Search RO"
{
    PageType = Worksheet; SourceTable = "HS Line"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { StyleExpr = Rec."Is History"; } field("Document Type"; Rec."Document Type") { } } } }
    actions { area(Processing) { action(SearchAction) { trigger OnAction() begin Rec.Reset(); Rec.Search('C1'); CurrPage.Update(false); end; } } }
}
codeunit 69944 C
{
    procedure EditSearch(): Text
    var P: TestPage "HS Search";
    begin
        P.OpenEdit();
        P.DocFilter.SetValue(true);
        P.CustFilter.SetValue('C1');
        P.SearchAction.Invoke();
        P.First();
        P.Filter.SetFilter("Is History", '=1');
        P.Filter.SetFilter("Document Type", '=Quote');
        exit(Format(P.First()));
    end;
    procedure ViewSixMembers(FilterText: Text): Text
    var P: TestPage "HS Search RO"; Ids: Text;
    begin
        P.OpenView();
        P.SearchAction.Invoke();
        P.First();
        P.Filter.SetFilter("Is History", '=1');
        P.Filter.SetFilter("Document Type", FilterText);
        if P.First() then repeat Ids += P."No.".Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter("Document Type") + ']' + Ids);
    end;
    procedure ViewSixMembersBare(FilterText: Text): Text
    var P: TestPage "HS Search RO"; Ids: Text;
    begin
        P.OpenView();
        P.SearchAction.Invoke();
        P.Filter.SetFilter("Document Type", FilterText);
        if P.First() then repeat Ids += P."No.".Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter("Document Type") + ']' + Ids);
    end;
}
"#;

fn his(proc: &str, args: &[&str]) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let args = args
        .iter()
        .map(|a| al_builtins::value::Value::Text(a.to_string().into()))
        .collect();
    al_testkit::run_with(HIS_SRC, "C", proc, args).map(|(v, _)| v)
}

// al-runner issue 6924 (HIS): the test fills header controls, so it opened the page with OpenEdit; WorksheetEnumEqFilter.al opened
// every page with OpenView (:260, :319, :378). Main applied the `'=1'` filter there and the row was gone, a wrong result. Not
// measured: it refuses before the first `=` text takes effect.
#[test]
fn an_eq_text_on_a_page_the_test_opened_for_edit_refuses() {
    let why = refusal(his("EditSearch", &[]));
    assert!(
        why.contains("'=1'") && why.contains("did not open with its own OpenView"),
        "{why}"
    );
}

// A blank before the `=` (review of al-runner PR 8023): WorksheetEnumEqFilter.al wrote every text with the `=` first (:412-419,
// :634-640). Main parsed `' =1'` and `' =Quote'` as filters. Not measured: each refuses, on the database and the temporary sources.
#[test]
fn an_eq_text_with_a_blank_before_the_eq_refuses() {
    let refused = |proc: &str, arg: &str| {
        let arg = al_builtins::value::Value::Text(arg.to_string().into());
        refusal(al_testkit::run_with(WS_SRC, "C", proc, vec![arg]).map(|(v, _)| v))
    };
    for (proc, t) in [("WsBoolean", " =1"), ("WsEnum", " =Quote"), ("TempListEnum", " =Quote")] {
        let why = refused(proc, t);
        assert!(why.contains("a blank before the '='"), "{proc} {t}: {why}");
    }
}

// The probe's enum has three members (WorksheetEnumEqFilter.al:25-32); the issue's has six. Main filtered on `=Quote` there (the
// page read the filter back as `Quote`, BC reads ''), right only by chance for the row kept. Not measured: it refuses.
#[test]
fn an_eq_text_on_an_enum_of_six_members_refuses() {
    for t in ["=Quote", "=Order"] {
        let why = refusal(his("ViewSixMembers", &[t]));
        assert!(why.contains("the measured 3 members"), "{t}: {why}");
    }
}

// A text without the `=` on the same Enum keeps filter.rs's parsing, as on main.
#[test]
fn a_bare_member_on_an_enum_of_six_members_keeps_the_filter() {
    assert_eq!(al_testkit::text(his("ViewSixMembersBare", &["Quote"])), "[Quote]Q1,");
}
