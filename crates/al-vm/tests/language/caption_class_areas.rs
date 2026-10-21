//! CaptionClass areas other than 3 on table fields and page controls (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al,
//! writer QD, corpus bump 34). The subscriber stands in for Base Application's VAT resolver (area 2).

use al_testkit::text;

const OBJECTS: &str = r#"
codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }
codeunit 50401 Vat
{
    SingleInstance = true;
    var Log: Text;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        Log += 'resolve(' + CaptionArea + ',' + CaptionExpr + ')';
        if (CaptionArea = '2') and CaptionExpr.StartsWith('0,') then begin
            Caption := CopyStr(CaptionExpr, 3) + ' Excl. VAT';
            Resolved := true;
        end;
    end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
}
codeunit 50402 Calls
{
    SingleInstance = true;
    var Log: Text; Count: Integer;
    procedure Hit(FieldNumber: Integer; RecId: Integer) begin Count += 1; Log += Format(FieldNumber) + '@' + Format(RecId) + ';'; end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
    procedure Total(): Integer begin exit(Count); end;
}
table 50400 CapRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(10; Lit; Decimal) { Caption = 'Cap Lit'; CaptionClass = '2,0,Inv. Discount Amount'; }
        field(11; Unresolved; Decimal) { Caption = 'Cap Unresolved'; CaptionClass = '9,x'; }
        field(20; ExprVat; Decimal) { Caption = 'Cap Expr Vat'; CaptionClass = Cc(20); }
        field(21; ExprEmpty; Integer) { Caption = 'Cap Expr Empty'; CaptionClass = Cc(21); }
        field(22; Expr3; Integer) { Caption = 'Cap Expr 3'; CaptionClass = Cc(22); }
        field(23; ExprArea; Integer) { Caption = 'Cap Expr Area'; CaptionClass = Cc(23); }
        field(24; ExprId; Integer) { Caption = 'Cap Expr Id'; CaptionClass = Cc(24); }
    }
    keys { key(PK; Id) { } }
    procedure Cc(FieldNumber: Integer): Text[80]
    var C: Codeunit Calls;
    begin
        C.Hit(FieldNumber, Id);
        case FieldNumber of
            20: exit('2,0,Amount');
            21: exit('');
            22: exit('3,');
            23: exit('AB,x');
            24: exit('3,Id ' + Format(Id) + ' call ' + Format(C.Total()));
        end;
    end;
}
page 50403 CapCard
{
    PageType = Card;
    SourceTable = CapRow;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
            field(Lit; Rec.Lit) { }
            field(ExprVat; Rec.ExprVat) { }
            field(ExprEmpty; Rec.ExprEmpty) { }
            field(ExprId; Rec.ExprId) { }
        }
    }
}
page 50404 CapList
{
    PageType = List;
    SourceTable = CapRow;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(ExprVat; Rec.ExprVat) { } field(ExprId; Rec.ExprId) { } } } }
}
page 50405 CapPart
{
    PageType = ListPart;
    SourceTable = CapRow;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(ExprId; Rec.ExprId) { } } } }
}
table 50406 CapHostRow { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 50407 CapHost
{
    PageType = Card;
    SourceTable = CapHostRow;
    layout { area(Content) { field(Id; Rec.Id) { } part(Lines; CapPart) { } } }
}
"#;

/// The calls a page fill logged, with the empty-record calls (`@0;`) left out: whether there were any, then the rest.
const SUMMARIZE: &str = r#"
    local procedure Summarize(Calls: Text): Text
    var Empty: Boolean;
    begin
        while Calls.Contains('@0;') do begin
            Empty := true;
            Calls := Calls.Remove(Calls.IndexOf('@0;') - 2, 5);
        end;
        exit(Format(Empty) + ':' + Calls);
    end;"#;

fn run(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 50409 T\n{{\n{body}\n}}\n");
    text(al_testkit::run_in(&src, "T", "F"))
}

/// QdCaptionClassAreas.al:233-258 Literals_FieldCaption (`lit20`): a literal area 2 goes to OnResolveCaptionClass; P84Captions.al
/// (Table_Log_A after Table_NoSubscriber): a text the session translated raises no event again; a text nobody resolves stays.
#[test]
fn a_literal_numeric_area_is_translated_once_per_session() {
    let got = run(r#"
    procedure F(): Text
    var R: Record CapRow; V: Codeunit Vat; S: Text;
    begin
        V.Take();
        S := R.FieldCaption(Lit) + '/' + V.Take() + '|';
        S += R.FieldCaption(Lit) + '/' + V.Take() + '|';
        S += R.FieldCaption(Unresolved) + '/' + V.Take();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "Inv. Discount Amount Excl. VAT/resolve(2,0,Inv. Discount Amount)|Inv. Discount Amount Excl. VAT/|9,x/resolve(9,x)"
    );
}

/// QdCaptionClassAreas.al:262-296 Expressions_FieldCaption and :323-347 Expression_PerRead_CallCount: the expression runs once
/// per read with the reading record; its result goes through the resolvers; '' and '3,' read as the field's Caption.
#[test]
fn an_expression_runs_per_read_and_its_result_is_resolved() {
    let got = run(r#"
    procedure F(): Text
    var R: Record CapRow; C: Codeunit Calls; S: Text;
    begin
        R.Id := 7;
        S := R.FieldCaption(ExprVat) + '|' + R.FieldCaption(ExprEmpty) + '|' + R.FieldCaption(Expr3) + '|';
        S += R.FieldCaption(ExprId) + '|' + R.FieldCaption(ExprId) + '|' + C.Take();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "Amount Excl. VAT|Cap Expr Empty|Cap Expr 3|Id 7 call 4|Id 7 call 5|20@7;21@7;22@7;24@7;24@7;"
    );
}

/// An expression result whose area is not a number is not pinned: refused.
#[test]
fn an_expression_result_with_an_area_that_is_not_a_number_is_refused() {
    let src = format!(
        "{OBJECTS}\ncodeunit 50409 T {{ procedure F(): Text var R: Record CapRow; begin exit(R.FieldCaption(ExprArea)); end; }}"
    );
    let r = al_testkit::run_in(&src, "T", "F");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("not a number")),
        "{r:?}"
    );
}

/// Every such open marks page-open-caption-class-empty-record-count: the number of empty-record runs is no oracle (owner 2026-10-21).
/// QdCaptionClassAreas.al:422-481 Page_ControlCaptions: OpenView of a card runs every expression field 20 times with an empty
/// record, then once per control with the current row; Caption() runs nothing; an empty result reads as '' on a control.
#[test]
fn a_card_open_runs_the_expressions_and_caption_reads_what_it_filled() {
    al_builtins::assumptions::clear_hits();
    let got = run(r#"
    procedure F(): Text
    var R: Record CapRow; C: Codeunit Calls; P: TestPage CapCard; Calls: Text; Empty: Integer; S: Text;
    begin
        R.Id := 3; R.Insert();
        C.Take();
        P.OpenView();
        Calls := C.Take();
        while Calls.Contains('@0;') do begin
            Empty += 1;
            Calls := Calls.Remove(Calls.IndexOf('@0;') - 2, 5);
        end;
        S := Format(Empty) + '|' + Calls + '|';
        S += P.Lit.Caption() + '|' + P.ExprVat.Caption() + '|' + P.ExprEmpty.Caption() + '|' + P.ExprId.Caption() + '|';
        S += P.ExprId.Caption() + '|' + C.Take();
        P.Close();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "100|20@3;21@3;24@3;|Inv. Discount Amount Excl. VAT|Amount Excl. VAT||Id 3 call 103|Id 3 call 103|"
    );
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount]
    );
}

/// QdCaptionClassAreas.al:591-630 QF_List_OpenView and :632-668 QF_List_OpenEdit (corpus bump 37): a List runs the expressions
/// with an empty record, then once per control with the FIRST row only (`rest=20@3;23@3;24@3;`, not the other rows it reads);
/// Next runs nothing and Caption() still reads the open's value (`next_exprId = Id 3` on row 5); Close runs nothing.
#[test]
fn a_list_runs_the_expressions_for_its_first_row_only_and_next_and_close_run_nothing() {
    for open in ["OpenView", "OpenEdit"] {
        al_builtins::assumptions::clear_hits();
        let got = run(&format!(
            r#"{SUMMARIZE}
    procedure F(): Text
    var R: Record CapRow; C: Codeunit Calls; P: TestPage CapList; S: Text; Open: Text;
    begin
        R.Id := 3; R.Insert(); R.Id := 5; R.Insert();
        C.Take();
        P.{open}();
        S := Summarize(C.Take()) + '|' + P.Id.Value() + '|';
        Open := P.ExprId.Caption();
        S += Format(P.Next()) + '|' + Summarize(C.Take()) + '|' + P.Id.Value() + '|' + Format(P.ExprId.Caption() = Open) + '|';
        S += P.ExprVat.Caption() + '|';
        P.Close();
        exit(S + Summarize(C.Take()));
    end;"#
        ));
        assert_eq!(got, "Yes:20@3;24@3;|3|Yes|No:|5|Yes|Amount Excl. VAT|No:", "{open}");
        assert_eq!(
            al_builtins::assumptions::take_hits(),
            vec![al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount],
            "{open}"
        );
    }
}

/// QdCaptionClassAreas.al QF_Card_Next (corpus bump 37): Next on a card opened with OpenView runs nothing and Caption() keeps
/// the open's value.
#[test]
fn next_on_a_card_runs_no_expression_and_keeps_the_caption() {
    al_builtins::assumptions::clear_hits();
    let got = run(&format!(
        r#"{SUMMARIZE}
    procedure F(): Text
    var R: Record CapRow; C: Codeunit Calls; P: TestPage CapCard; S: Text; Open: Text;
    begin
        R.Id := 3; R.Insert(); R.Id := 5; R.Insert();
        C.Take();
        P.OpenView();
        S := Summarize(C.Take()) + '|';
        Open := P.ExprId.Caption();
        S += Format(P.Next()) + '|' + Summarize(C.Take()) + '|' + P.Id.Value() + '|' + Format(P.ExprId.Caption() = Open);
        P.Close();
        exit(S + '|' + Summarize(C.Take()));
    end;"#
    ));
    assert_eq!(got, "Yes:20@3;21@3;24@3;|Yes|No:|5|Yes|No:");
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount]
    );
}

/// QdCaptionClassAreas.al QF_ListPart_OnCard (corpus bump 37): a ListPart on a card opened with OpenView runs the expressions
/// for its first row; Next on the part runs nothing and Caption() keeps the open's value.
#[test]
fn a_list_part_on_a_card_runs_the_expressions_for_its_first_row_and_next_runs_nothing() {
    al_builtins::assumptions::clear_hits();
    let got = run(&format!(
        r#"{SUMMARIZE}
    procedure F(): Text
    var R: Record CapRow; H: Record CapHostRow; C: Codeunit Calls; P: TestPage CapHost; S: Text; Open: Text;
    begin
        R.Id := 3; R.Insert(); R.Id := 5; R.Insert(); H.Id := 1; H.Insert();
        C.Take();
        P.OpenView();
        S := Summarize(C.Take()) + '|' + P.Lines.Id.Value() + '|';
        Open := P.Lines.ExprId.Caption();
        S += Format(P.Lines.Next()) + '|' + Summarize(C.Take()) + '|' + P.Lines.Id.Value() + '|' + Format(P.Lines.ExprId.Caption() = Open);
        P.Close();
        exit(S + '|' + Summarize(C.Take()));
    end;"#
    ));
    assert_eq!(got, "Yes:24@3;|3|Yes|No:|5|Yes|No:");
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount]
    );
}

const DOC_OBJECTS: &str = r#"
codeunit 50420 Calls
{
    SingleInstance = true;
    var Log: Text;
    procedure Hit(FieldNumber: Integer; RecId: Integer) begin Log += Format(FieldNumber) + '@' + Format(RecId) + ';'; end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
    var Sfx: Text;
    procedure SetSuffix(S: Text) begin Sfx := S; end;
    procedure Suffix(): Text begin exit(Sfx); end;
}
table 50421 CapDoc
{
    fields { field(1; "No."; Code[10]) { } field(2; Seq; Integer) { } field(3; Descr; Text[30]) { } field(4; Mode; Text[10]) { } field(60; "Head Expr"; Decimal) { CaptionClass = Hc(); } }
    keys { key(PK; "No.") { } }
    procedure Hc(): Text[80] var C: Codeunit Calls; begin C.Hit(60, Seq); exit('3,Head ' + Format(Seq)); end;
}
table 50422 CapLine
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Amount; Decimal) { }
        field(70; "Expr A"; Decimal) { CaptionClass = Lc(70); } field(71; "Expr B"; Decimal) { CaptionClass = Lc(71); } }
    keys { key(PK; "Doc No.", "Line No.") { } }
    procedure Lc(FieldNumber: Integer): Text[80] var C: Codeunit Calls; begin C.Hit(FieldNumber, "Line No."); exit('3,Line ' + Format("Line No.") + C.Suffix()); end;
}
page 50423 CapLines
{
    PageType = ListPart; SourceTable = CapLine;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Amount; Rec.Amount) { } field(ExprA; Rec."Expr A") { } field(ExprB; Rec."Expr B") { } } } }
}
page 50424 CapDocPage
{
    PageType = Document; SourceTable = CapDoc;
    layout { area(Content) { group(G) { field("No."; Rec."No.") { } field(Seq; Rec.Seq) { } field(Descr; Rec.Descr) { } field(HeadExpr; Rec."Head Expr") { }
            field(Mode; Rec.Mode) { trigger OnValidate() var C: Codeunit Calls; begin C.SetSuffix(' ' + Rec.Mode); CurrPage.Update(); end; } }
        part(Lines; CapLines) { SubPageLink = "Doc No." = field("No."); } } }
    actions { area(Processing) { action(Refresh) { trigger OnAction() begin CurrPage.Update(); end; } } }
}
codeunit 50425 T
{
    var C: Codeunit Calls;
    local procedure Step(): Text
    var Calls: Text; Empty: Boolean;
    begin
        Calls := C.Take();
        while Calls.Contains('@0;') do begin
            Empty := true;
            Calls := Calls.Remove(Calls.IndexOf('@0;') - 2, 5);
        end;
        exit(Format(Empty) + ':' + Calls + '|');
    end;
    local procedure Seed()
    var D: Record CapDoc; L: Record CapLine;
    begin
        L.DeleteAll(); D.DeleteAll();
        D."No." := 'D1'; D.Seq := 1; D.Insert(); D."No." := 'D2'; D.Seq := 2; D.Insert();
        L."Doc No." := 'D1'; L."Line No." := 1; L.Insert(); L."Line No." := 2; L.Insert();
        L."Doc No." := 'D2'; L."Line No." := 3; L.Insert(); L."Line No." := 4; L.Insert();
        C.Take();
    end;
    procedure Open(): Text
    var H: TestPage CapDocPage; S: Text;
    begin
        Seed(); H.OpenEdit(); S := Step();
        S += H.HeadExpr.Caption() + ',' + H.Lines.ExprA.Caption() + '|' + Step();
        H.Close(); exit(S + Step());
    end;
    procedure PartSteps(): Text
    var H: TestPage CapDocPage; S: Text;
    begin
        Seed(); H.OpenEdit(); C.Take();
        H.Lines.Amount.SetValue(5); S := Step();
        S += Format(H.Lines.Next()) + Step();
        S += H.Lines."Line No.".Value() + ',' + H.Lines.ExprA.Caption() + '|' + Step();
        H.Close(); exit(S + Step());
    end;
    procedure HostSteps(): Text
    var H: TestPage CapDocPage; S: Text;
    begin
        Seed(); H.OpenEdit(); C.Take();
        H.Descr.SetValue('x'); S := Step();
        H.Refresh.Invoke(); S += Step();
        S += H.Lines.ExprA.Caption() + '|' + Step();
        S += Format(H.Next()) + Step();
        S += H.HeadExpr.Caption() + ',' + H.Lines."Line No.".Value() + ',' + H.Lines.ExprA.Caption() + '|' + Step();
        H.Close(); exit(S + Step());
    end;
    procedure UpdateNoLines(): Text
    var H: TestPage CapDocPage; D: Record CapDoc;
    begin
        Seed(); D."No." := 'D3'; D.Insert();
        H.OpenEdit(); H.GoToKey('D3');
        H.Mode.SetValue('x');
        exit(H.Lines.ExprA.Caption());
    end;
    procedure OpenNew(): Text
    var H: TestPage CapDocPage; S: Text;
    begin
        Seed(); H.OpenNew(); C.Take();
        S := H.HeadExpr.Caption() + ',' + H.Lines.ExprA.Caption() + '|';
        H."No.".SetValue('D9'); S += Step();
        S += H.Lines."Line No.".Value() + ',' + H.Lines.ExprA.Caption() + '|' + Step();
        H.Close(); exit(S + Step());
    end;
}
"#;

fn run_doc(routine: &str) -> (String, Vec<&'static str>) {
    al_builtins::assumptions::clear_hits();
    let got = text(al_testkit::run_in(DOC_OBJECTS, "T", routine));
    let hits = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    (got, hits)
}

/// QdCaptionClassAreas.al:958-1110 (corpus bump 45, writer QO): a Document page with a ListPart opened with OpenEdit and
/// OpenNew, a SetValue in the part and the host, the part's Next after a SetValue (it runs the line's expressions with the
/// row it leaves), a CurrPage.Update (the host's expression three times, the line's once) and the host's Next (the part's
/// new first row only). Only the number of empty-record runs stays under its assumption.
#[test]
fn caption_class_expressions_on_a_document_page_and_its_part_are_pinned() {
    let cases = [
        ("Open", "Yes:60@1;70@1;71@1;|Head 1,Line 1|No:|No:|"),
        ("PartSteps", "No:|YesNo:70@1;71@1;|2,Line 1|No:|No:|"),
        ("OpenNew", "Head 0,Line 0|No:|0,Line 0|No:|No:|"),
    ];
    let mut wrong = Vec::new();
    for (routine, want) in cases {
        let (got, hits) = run_doc(routine);
        if got != want || hits.contains(&"page-open-caption-class-expressions") {
            wrong.push(format!("{routine}: got {got} want {want} {hits:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    // CurrPage.Update after a SetValue on the host: BC runs the host's expression three times, then the part's once
    // (`update = rest=60@1;60@1;60@1;70@1;71@1;`); the runtime does not match that and keeps the mark. The host's Next after it
    // refills the part with its first row only.
    let (got, hits) = run_doc("HostSteps");
    assert!(got.starts_with("No:|"), "{got}");
    assert!(
        got.ends_with("|Line 1|No:|YesNo:70@3;71@3;|Head 1,3,Line 3|No:|No:|"),
        "{got}"
    );
    assert!(hits.contains(&"page-open-caption-class-expressions"), "{hits:?}");
}

/// QdCaptionClassAreas.al:1110-1137 QO_Card_OpenEdit (corpus bump 45): a Card opened with OpenEdit runs the expressions as
/// with OpenView; a SetValue runs nothing; Next() after it runs them once with the row it saves and leaves, and Caption()
/// keeps that value.
#[test]
fn a_card_opened_for_edit_runs_the_expressions_with_the_saved_row_at_next() {
    al_builtins::assumptions::clear_hits();
    let got = run(&format!(
        r#"{SUMMARIZE}
    procedure F(): Text
    var R: Record CapRow; C: Codeunit Calls; P: TestPage CapCard; S: Text;
    begin
        R.Id := 3; R.Insert(); R.Id := 5; R.Insert();
        C.Take();
        P.OpenEdit();
        S := Summarize(C.Take()) + '|';
        P.Lit.SetValue(5);
        S += Summarize(C.Take()) + '|';
        S += Format(P.Next()) + '|' + Summarize(C.Take()) + '|' + P.Id.Value() + '|' + CopyStr(P.ExprId.Caption(), 1, 4);
        P.Close();
        exit(S + '|' + Summarize(C.Take()));
    end;"#
    ));
    assert_eq!(got, "Yes:20@3;21@3;24@3;|No:|Yes|No:20@3;21@3;24@3;|5|Id 3|No:");
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount]
    );
}

/// A CurrPage.Update in the OnValidate of a host field refreshes the captions of a part that has no row (Microsoft's ERM
/// Purchase Order PurchaseOrderChangePricesInclVATRefreshesPage: Prices Including VAT on a document without lines, then the
/// subform's "Direct Unit Cost" caption). Not pinned by the corpus: it runs under the assumption.
#[test]
fn an_update_of_the_host_refreshes_the_captions_of_a_part_without_rows() {
    let (got, hits) = run_doc("UpdateNoLines");
    assert_eq!(got, "Line 0 x");
    assert!(hits.contains(&"page-open-caption-class-expressions"), "{hits:?}");
}
