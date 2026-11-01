//! Page views and filters (lane T5 of the bump-29 triage): SourceTableView constants, enum links, FilterGroup(-1) in OnOpenPage,
//! a descending view with OnFindRecord, RunModal with an Enum or Option filter, and what TestPage.Filter.GetFilter shows for an
//! Enum field. AL source → compile → run as a test codeunit. Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
enum 62900 "PV Status" { Extensible = false; value(0; Open) { } value(1; Released) { } }
enum 62901 "PV Kind" { Extensible = false; value(0; First) { } value(1; Second) { } value(2; Third) { } }
table 62900 "PV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Status; Enum "PV Status") { }
        field(6; Grp; Code[10]) { }
        field(7; D; Date) { }
        field(8; T; Text[10]) { }
        field(9; Kind; Enum "PV Kind") { }
        field(10; Opt; Option) { OptionMembers = A,B,C; }
    }
    keys { key(PK; Id) { } }
}
table 62901 "PV Line"
{
    fields { field(1; Kind; Enum "PV Kind") { } field(2; No; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; Kind, No) { } }
}
table 62902 "PV Head"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Enum "PV Kind") { } }
    keys { key(PK; Id) { } }
}
codeunit 62903 "PV Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
page 62910 "PV Const0D" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(D = const(0D));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
page 62911 "PV Filter0D" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(D = filter(0D));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
page 62912 "PV ConstOthers" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(T = const(''), Status = const(Open));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
page 62913 "PV ConstBlankCode" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(Grp = const(''));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
page 62914 "PV OrGroup" { PageType = List; SourceTable = "PV Row";
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
  trigger OnOpenPage()
  var Log: Codeunit "PV Log";
  begin
    Rec.FilterGroup(-1);
    Rec.SetRange(Grp, 'A');
    Rec.SetRange(Name, 'n3');
    Log.Add('count=' + Format(Rec.Count()));
    Rec.FilterGroup(0);
  end; }
page 62915 "PV OrGroup Reads" { PageType = List; SourceTable = "PV Row";
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
  trigger OnOpenPage()
  begin
    Rec.FilterGroup(-1);
    Rec.SetRange(Grp, 'A');
    Rec.SetRange(Name, 'n3');
    Rec.FilterGroup(0);
  end;
  trigger OnAfterGetRecord()
  begin
  end; }
page 62916 "PV Head Card" { PageType = Card; SourceTable = "PV Head";
  layout { area(Content) { field(IdCtl; Rec.Id) { } part(Lines; "PV Line Part") { SubPageLink = Kind = field(Kind); } } }
  actions { area(Processing) {
    action(LinesByField) { RunObject = page "PV Line List"; RunPageLink = Kind = field(Kind); }
    action(LinesByConst) { RunObject = page "PV Line List"; RunPageLink = Kind = const(Second); } } } }
page 62917 "PV Line Part" { PageType = ListPart; SourceTable = "PV Line"; AutoSplitKey = true;
  layout { area(Content) { repeater(R) { field(KindCtl; Rec.Kind) { } field(NoCtl; Rec.No) { } field(DescrCtl; Rec.Descr) { } } } } }
page 62918 "PV Line List" { PageType = List; SourceTable = "PV Line";
  layout { area(Content) { repeater(R) { field(KindCtl; Rec.Kind) { } field(NoCtl; Rec.No) { } field(DescrCtl; Rec.Descr) { } } } } }
page 62919 "PV Desc Find" { PageType = List; SourceTable = "PV Row"; SourceTableView = sorting(Id) order(descending); Editable = false;
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "PV Log"; Found: Boolean;
  begin
    Found := Rec.Find(Which);
    Log.Add('find(' + Which + ')=' + Format(Rec.Id));
    exit(Found);
  end; }
page 62920 "PV List" { PageType = List; SourceTable = "PV Row";
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } field(KindCtl; Rec.Kind) { } field(OptCtl; Rec.Opt) { } } } } }
page 62921 "PV View List" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(Kind = const(First)); Editable = false;
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } field(KindCtl; Rec.Kind) { } } } } }
page 62922 "PV Const0D Code" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(D = const(0D));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
  trigger OnOpenPage() begin end; }
page 62923 "PV FilterKind" { PageType = List; SourceTable = "PV Row"; SourceTableView = where(Kind = filter(Second));
  layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
"#;

const HELPERS: &str = r#"
    var Seen: Text; NewKind: Text; NewOpt: Text; FilterSeen: Text;
    local procedure Check(What: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then Error('%1: expected [%2] but was [%3]', What, Want, Got);
    end;
    local procedure AddRow(NewId: Integer; NewName: Text[30]; NewGrp: Code[10]; NewD: Date)
    var Row: Record "PV Row";
    begin
        Row.Init(); Row.Id := NewId; Row.Name := NewName; Row.Grp := NewGrp; Row.D := NewD; Row.Insert();
    end;
    local procedure Seed()
    var Row: Record "PV Row"; Log: Codeunit "PV Log";
    begin
        Row.DeleteAll();
        AddRow(1, 'n1', 'A', 0D);
        AddRow(2, 'n3', 'B', 0D);
        AddRow(3, 'n2', 'A', DMY2Date(1, 2, 2024));
        AddRow(4, 'n4', 'C', DMY2Date(1, 3, 2024));
        Log.Take();
        Seen := ''; NewKind := ''; NewOpt := ''; FilterSeen := '';
    end;
    local procedure SeedKinds()
    var Row: Record "PV Row";
    begin
        Row.DeleteAll();
        Row.Init(); Row.Id := 1; Row.Kind := Row.Kind::First; Row.Opt := Row.Opt::A; Row.Insert();
        Row.Init(); Row.Id := 2; Row.Kind := Row.Kind::Second; Row.Opt := Row.Opt::B; Row.Insert();
        Row.Init(); Row.Id := 3; Row.Kind := Row.Kind::First; Row.Opt := Row.Opt::A; Row.Insert();
        Row.Init(); Row.Id := 4; Row.Kind := Row.Kind::Third; Row.Opt := Row.Opt::C; Row.Insert();
        Row.Init(); Row.Id := 5; Row.Kind := Row.Kind::Second; Row.Opt := Row.Opt::B; Row.Insert();
        Seen := ''; NewKind := ''; NewOpt := ''; FilterSeen := '';
    end;
    local procedure SeedHead(HeadKind: Enum "PV Kind")
    var Head: Record "PV Head"; Line: Record "PV Line";
    begin
        Head.DeleteAll(); Line.DeleteAll();
        Head.Id := 1; Head.Kind := HeadKind; Head.Insert();
        Line.Kind := Line.Kind::First; Line.No := 10000; Line.Descr := 'first line'; Line.Insert();
        Line.Kind := Line.Kind::Second; Line.No := 10000; Line.Descr := 'second line'; Line.Insert();
    end;
    local procedure LineRows(): Text
    var Line: Record "PV Line"; R: Text;
    begin
        if Line.FindSet() then
            repeat R += Format(Line.Kind) + '/' + Format(Line.No) + '/' + Line.Descr + ','; until Line.Next() = 0;
        exit(R);
    end;
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62930 T {{ Subtype = Test; {HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

fn not_run_with(tests: &str, why: &str) {
    let o = outcomes(tests);
    assert_eq!(o.len(), 1, "{o:?}");
    assert!(matches!(&o[0].1, Outcome::NotRun(r) if r.contains(why)), "{o:?}");
}

const WALK: &str = r#"
        if List.First() then
            repeat Result += List.IdCtl.Value() + ','; until not List.Next();
"#;

// pages/lt-cd-testpage-leads/CdTestPageLeads.al:508 Item58_SourceTableView_Const0D and :521 Item58_SourceTableView_Filter0D.
#[test]
fn a_source_table_view_const_or_filter_0d_shows_the_blank_dates() {
    passes(&format!(
        r#"
    [Test] procedure Const0D() var List: TestPage "PV Const0D"; Result: Text;
    begin Seed(); List.OpenView(); {WALK} List.Close(); Check('ids', Result, '1,2,'); end;
    [Test] procedure Filter0D() var List: TestPage "PV Filter0D"; Result: Text;
    begin Seed(); List.OpenView(); {WALK} List.Close(); Check('ids', Result, '1,2,'); end;
"#
    ));
}

// pages/lt-cd-testpage-leads/CdTestPageLeads.al:539 Item58_SourceTableView_ConstEmptyTextAndEnum: const('') on a Text field
// does not filter; const(Open) on an Enum field does.
#[test]
fn a_source_table_view_const_blank_on_a_text_field_does_not_filter() {
    passes(&format!(
        r#"
    [Test] procedure ConstOthers() var Row: Record "PV Row"; List: TestPage "PV ConstOthers"; Result: Text;
    begin
        Seed();
        Row.Get(2); Row.T := 'x'; Row.Modify();
        Row.Get(3); Row.Status := Row.Status::Released; Row.Modify();
        List.OpenView(); {WALK} List.Close();
        Check('ids', Result, '1,2,4,');
    end;
"#
    ));
}

// Only a Text field is pinned for const('') in a SourceTableView; a Code field is refused.
#[test]
fn a_source_table_view_const_blank_on_a_code_field_is_refused() {
    not_run_with(
        &format!(
            r#"
    [Test] procedure ConstBlankCode() var List: TestPage "PV ConstBlankCode"; Result: Text;
    begin Seed(); List.OpenView(); {WALK} List.Close(); end;
"#
        ),
        "const('')",
    );
}

// pages/lt-cd-testpage-leads/CdTestPageLeads.al:776 Item63_FilterGroupMinus1_InOnOpenPage: the record counts the OR rows inside
// OnOpenPage, the page shows every row.
#[test]
fn filter_group_minus_1_from_on_open_page_does_not_limit_the_rows_shown() {
    passes(&format!(
        r#"
    [Test] procedure OrGroup() var List: TestPage "PV OrGroup"; Log: Codeunit "PV Log"; Result: Text;
    begin
        Seed(); List.OpenView(); {WALK}
        Check('log', Log.Take(), 'count=3;');
        List.Close();
        Check('ids', Result, '1,2,3,4,');
    end;
"#
    ));
}

// What Rec holds at group -1 after OnOpenPage is not pinned: a page that reads Rec again later is refused.
#[test]
fn filter_group_minus_1_left_on_a_page_with_later_triggers_is_refused() {
    not_run_with(
        &format!(
            r#"
    [Test] procedure OrGroupReads() var List: TestPage "PV OrGroup Reads"; Result: Text;
    begin Seed(); List.OpenView(); {WALK} List.Close(); end;
"#
        ),
        "FilterGroup(-1)",
    );
}

// pages/lt-cd-testpage-leads/CdTestPageLeads.al:587 Item59_EnumSubPageLink_NewLine: Next() onto the blank line, then New():
// the blank line burns one AutoSplitKey interval, the typed line gets the last line + 20000.
#[test]
fn auto_split_key_after_next_onto_the_blank_line_burns_one_interval() {
    passes(
        r#"
    [Test] procedure EnumSubPageLink() var Card: TestPage "PV Head Card";
    begin
        SeedHead("PV Kind"::Second);
        Card.OpenEdit();
        Card.Lines.First();
        Check('part-first', Card.Lines.KindCtl.Value() + '/' + Card.Lines.DescrCtl.Value(), 'Second/second line');
        Check('part-next', Format(Card.Lines.Next()), 'Yes');
        Card.Lines.New();
        Check('new-kind-shown', Card.Lines.KindCtl.Value(), 'Second');
        Card.Lines.DescrCtl.SetValue('typed line');
        Card.Lines.First();
        Card.Close();
        Check('lines', LineRows(), 'First/10000/first line,Second/10000/second line,Second/30000/typed line,');
    end;
"#,
    );
}

// pages/lt-cd-testpage-leads/CdTestPageLeads.al:626 Item61_RunPageLinkEnumKey_ByField_New (the ordinal) and :650 _ByConst_New
// (the name).
#[test]
fn a_run_page_link_field_on_an_enum_shows_the_ordinal_in_the_test_page_filter() {
    passes(
        r#"
    [Test] procedure ByField() var Card: TestPage "PV Head Card"; List: TestPage "PV Line List";
    begin
        SeedHead("PV Kind"::Third);
        Card.OpenView();
        List.Trap();
        Card.LinesByField.Invoke();
        Check('filter', List.Filter.GetFilter(Kind), '2');
        List.New();
        Check('new-kind-shown', List.KindCtl.Value(), 'Third');
        List.NoCtl.SetValue(5);
        List.DescrCtl.SetValue('typed');
        List.Close();
        Card.Close();
        Check('lines', LineRows(), 'First/10000/first line,Second/10000/second line,Third/5/typed,');
    end;
    [Test] procedure ByConst() var Card: TestPage "PV Head Card"; List: TestPage "PV Line List";
    begin
        SeedHead("PV Kind"::First);
        Card.OpenView();
        List.Trap();
        Card.LinesByConst.Invoke();
        Check('filter', List.Filter.GetFilter(Kind), 'Second');
        List.Close();
        Card.Close();
    end;
"#,
    );
}

// pages/lt-ce-page-run/CePageRun.al OpenView_DescendingWithOnFindRecord_CurrentRow: a descending SourceTableView, OnFindRecord is
// asked `=><` on open and the blank key finds the lowest id.
#[test]
fn a_descending_view_with_on_find_record_opens_with_the_client_find() {
    passes(
        r#"
    [Test] procedure DescFind() var L: TestPage "PV Desc Find"; Log: Codeunit "PV Log";
    begin
        Seed();
        L.OpenView();
        Check('current', L.IdCtl.Value(), '1');
        Check('first_find', CopyStr(Log.Take(), 1, 13), 'find(=><)=1;');
        L.Close();
    end;
"#,
    );
}

// pages/lt-ce-page-run/CePageRun.al RunModal_EnumFilter_RowsAndNewRowDefault and RunModal_OptionFilter_RowsAndNewRowDefault: a
// handed record filtered to one Enum or Option value the blank row does not hold shows no blank line; New() still starts a line.
#[test]
fn run_modal_with_an_enum_or_option_filter_shows_no_blank_line() {
    passes(
        r#"
    [Test] [HandlerFunctions('NewRowHandler')]
    procedure EnumFilter() var Row: Record "PV Row";
    begin
        SeedKinds();
        Row.SetRange(Kind, Row.Kind::Second);
        Check('action', Format(Page.RunModal(Page::"PV List", Row)), 'LookupOK');
        Check('rows', Seen, '[2][5]');
        Check('new_kind', NewKind, 'First');
        Row.Reset(); Row.Get(90);
        Check('stored', Format(Row.Kind) + '/' + Format(Row.Opt), 'First/A');
    end;
    [Test] [HandlerFunctions('NewRowHandler')]
    procedure OptionFilter() var Row: Record "PV Row";
    begin
        SeedKinds();
        Row.SetRange(Opt, Row.Opt::B);
        Check('action', Format(Page.RunModal(Page::"PV List", Row)), 'LookupOK');
        Check('rows', Seen, '[2][5]');
        Check('new_opt', NewOpt, 'A');
    end;
    [ModalPageHandler]
    procedure NewRowHandler(var L: TestPage "PV List")
    begin
        if L.First() then
            repeat Seen += '[' + L.IdCtl.Value() + ']'; until not L.Next();
        L.New();
        L.IdCtl.SetValue(90);
        NewKind := L.KindCtl.Value();
        NewOpt := L.OptCtl.Value();
        L.OK().Invoke();
    end;
"#,
    );
}

// pages/lt-ce-page-run/CePageRun.al SetTableView_ConflictingFilterOnTheSameField, SetTableView_FilterOnAnotherField and
// SetTableView_WiderFilterOnTheSameField: a SetRange value and a SourceTableView const on an Enum field read as the ordinal, a
// SetFilter text as the names.
#[test]
fn set_table_view_enum_filters_read_as_ordinals_in_the_test_page_filter() {
    passes(
        r#"
    [Test] [HandlerFunctions('ViewHandler')]
    procedure Conflicting() var Row: Record "PV Row"; Pg: Page "PV View List";
    begin
        SeedKinds();
        Row.SetRange(Kind, Row.Kind::Second);
        Pg.SetTableView(Row);
        Pg.RunModal();
        Check('rows', Seen, 'none');
        Check('filter', FilterSeen, 'Kind=1 Id=');
    end;
    [Test] [HandlerFunctions('ViewHandler')]
    procedure OtherField() var Row: Record "PV Row"; Pg: Page "PV View List";
    begin
        SeedKinds();
        Row.SetRange(Id, 1, 2);
        Pg.SetTableView(Row);
        Pg.RunModal();
        Check('rows', Seen, '[1]');
        Check('filter', FilterSeen, 'Kind=0 Id=1..2');
    end;
    [Test] [HandlerFunctions('ViewHandler')]
    procedure Wider() var Row: Record "PV Row"; Pg: Page "PV View List";
    begin
        SeedKinds();
        Row.SetFilter(Kind, '%1|%2', Row.Kind::First, Row.Kind::Second);
        Pg.SetTableView(Row);
        Pg.RunModal();
        Check('rows', Seen, '[1][3]');
        Check('filter', FilterSeen, 'Kind=First|Second Id=');
    end;
    [ModalPageHandler]
    procedure ViewHandler(var L: TestPage "PV View List")
    begin
        FilterSeen := 'Kind=' + L.Filter.GetFilter(Kind) + ' Id=' + L.Filter.GetFilter(Id);
        if not L.First() then begin Seen := 'none'; exit; end;
        repeat Seen += '[' + L.IdCtl.Value() + ']'; until not L.Next();
    end;
"#,
    );
}

// Review of PR 360: the text a `0D` view filter reads back is not pinned, so every read refuses: a TestPage Filter read, and a
// page with code that could read it.
#[test]
fn a_source_table_view_0d_filter_read_back_is_refused() {
    not_run_with(
        r#"
    [Test] procedure ReadFilter() var List: TestPage "PV Const0D";
    begin Seed(); List.OpenView(); Error('read %1', List.Filter.GetFilter(D)); end;
"#,
        "const(0D)",
    );
    not_run_with(
        r#"
    [Test] procedure PageWithCode() var List: TestPage "PV Const0D Code";
    begin Seed(); List.OpenView(); List.Close(); end;
"#,
        "const(0D)",
    );
}

// Review of PR 360: an Enum filter given as text with one value is not a typed value: it reads back as the name, as a RunPageLink
// const(Second) does (CdTestPageLeads.al:650 Item61_RunPageLinkEnumKey_ByConst_New `Second`), never as the ordinal.
#[test]
fn an_enum_filter_given_as_text_reads_as_the_name() {
    passes(
        r#"
    [Test] procedure TestFilterText() var List: TestPage "PV List";
    begin
        SeedKinds();
        List.OpenView();
        List.Filter.SetFilter(Kind, 'Second');
        Check('filter', List.Filter.GetFilter(Kind), 'Second');
        List.Close();
    end;
    [Test] procedure ViewFilterText() var List: TestPage "PV FilterKind";
    begin
        SeedKinds();
        List.OpenView();
        Check('filter', List.Filter.GetFilter(Kind), 'Second');
        List.Close();
    end;
"#,
    );
}
