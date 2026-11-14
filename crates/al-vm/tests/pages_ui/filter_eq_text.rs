//! `TestPage.Filter.SetFilter` with a text that starts with `=`: Business Central sets no filter on a List or Worksheet the test
//! opened with OpenView, for a Boolean word or an Enum or Option member (corpus testpage/filters/TempListOffPageBooleanFilter.al,
//! WorksheetEnumEqFilter.al of corpus PR 1392, al-runner issue 6924). These tests hold the shapes outside those cells to main's
//! outcome (the filter is applied), and such a shape on a database source the corpus did not measure to a refusal.

const SRC: &str = r#"
table 69990 "EQ Row"
{
    fields { field(1; Id; Integer) { } field(2; Flag; Boolean) { } field(3; Name; Code[10]) { } field(4; Kind; Option) { OptionMembers = Quote,"Order"; } }
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

// Not measured: OpenEdit, a Code or Integer field, another text after the `=`, a field the test
// already filtered, a Card.
// Each keeps main's filter.
#[test]
fn shapes_outside_the_cell_keep_the_filter() {
    assert_eq!(run("EditBoolean"), "2,4,");
    assert_eq!(run("ViewCode"), "2,");
    assert_eq!(run("ViewInteger"), "2,");
    assert_eq!(run("ViewBooleanOtherWord"), "1,2,3,4,");
    assert_eq!(run("ViewBooleanAfterOwnFilter"), "2,4,");
    assert_eq!(run("CardBoolean"), "2");
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

// WorksheetEnumEqFilter.al (corpus PR 1392): a text that starts with `=` sets no filter on the database Worksheet for an Enum
// member's name (:626-630), an Option member's name after the first (:631) and the Boolean's `=1` (:632), and on the temporary List
// for an Enum name (:575-596).
#[test]
fn an_eq_text_on_a_viewed_worksheet_or_list_sets_no_filter() {
    for t in ["=Quote", "=Order", "=Invoice"] {
        assert_eq!(ws("WsEnum", t), "[]1,2,3,4,", "{t}");
        assert_eq!(ws("TempListEnum", t), "[]1,2,3,4,", "{t}");
    }
    for t in ["=Order", "=Invoice"] {
        assert_eq!(ws("WsOption", t), "[]1,2,3,4,", "{t}");
    }
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

// Not measured: OpenEdit, a Code field, an ordinal other than 0 and 1, a name with another case, an earlier filter on the
// database Worksheet. Each keeps main's filter; `<>Quote` and a bare name keep filter.rs's parsing (:406, :408).
#[test]
fn worksheet_shapes_outside_the_cells_keep_the_filter() {
    assert_eq!(ws("WsEditEnum", "=Order"), "2,4,");
    assert_eq!(ws("WsCode", "=N2"), "2,");
    assert_eq!(ws("WsEnum", "=2"), "[Invoice]3,");
    assert_eq!(ws("WsEnum", "=order"), "[Order]2,4,");
    assert_eq!(ws0("WsEnumAfterOwnFilter"), "1,");
    assert_eq!(ws("WsEnum", "<>Quote"), "[<>0]2,3,4,");
    assert_eq!(ws("WsEnum", "Order"), "[Order]2,4,");
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

// Main's filter where no cell measured the shape: WorksheetEnumEqFilter.al's enum and option have no captions (:25-46) and one-word
// members; its only earlier filter is the test's bare `Order` (:554); its `<>` text is `<>Quote` (:415, :456); the Worksheet
// Boolean words are `=1` and `=0` (:472-473); an Enum ordinal only on the Worksheet (:418-419).
#[test]
fn shapes_next_to_the_eq_cells_keep_main_filter() {
    let t = |p: &str, a: &[&str]| al_testkit::text(nb(p, a));
    assert_eq!(t("TempWs", &["T", "", "=Order"]), "filter=Bestellung rows=2,");
    assert_eq!(t("TempWs", &["K", "", "=Order"]), "filter=Order rows=2,");
    assert_eq!(t("TempWs", &["P", "", "=Credit Memo"]), "filter=Credit Memo rows=3,");
    assert_eq!(t("TempWs", &["P", "<>Quote", "=Order"]), "filter=Order rows=2,");
    assert_eq!(t("TempWs", &["O", "", "<>0"]), "filter=<>Quote rows=2,3,");
    assert_eq!(t("TempWs", &["K", "", "<>Quote"]), "filter=<>Quote rows=2,3,");
    assert_eq!(t("TempWs", &["B", "", "=Yes"]), "filter=Yes rows=3,");
    assert_eq!(t("TempList", &["P", "=1"]), "filter=Order rows=2,");
    assert_eq!(t("OpenNe", &[]), "filter=<>Quote");
    assert_eq!(t("OpenFlt", &[]), "before=1 after=Quote first=1");
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
// measured, and the path refuses instead of filtering.
#[test]
fn an_option_first_member_with_captions_on_a_list_refuses() {
    assert!(refusal(nb("TempList", &["K", "=Quote"])).contains("first member of an Option with captions"));
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
