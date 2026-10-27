//! `CurrPage.Update(false)` in a field's OnValidate that changes the filters of a Worksheet list (al-runner issues 5914 and
//! 5888; pinned by pages/lt-rz-outside-page-moves/RzUpdateFromValidate.al:340-560, corpus writer SF; a List in another key is refused). After the refresh the client holds the rows the refresh read, so the next move reads as
//! on a page without a refresh: in the primary key the row twice, in another key three times. In another key the refresh
//! reads the row it starts from once more first, also when the new filter excludes it.

use al_testkit::{Built, text};

fn page(id: u32, open: &str) -> String {
    format!(
        r#"
page {id} "Probe List {id}"
{{
    PageType = Worksheet;
    SourceTable = "Probe Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    layout
    {{
        area(Content)
        {{
            group(FilterBar)
            {{
                field(GroupFilter; GroupFilter)
                {{
                    trigger OnValidate()
                    begin
                        if GroupFilter = '' then
                            Rec.SetRange(Grp)
                        else
                            Rec.SetRange(Grp, GroupFilter);
                        CurrPage.Update(false);
                    end;
                }}
                field(Touch; Touch)
                {{
                    trigger OnValidate()
                    begin
                        CurrPage.Update(false);
                    end;
                }}
            }}
            repeater(Rows)
            {{
                Editable = false;
                field(Id; Rec.Id) {{ }}
                field(Grp; Rec.Grp) {{ }}
            }}
        }}
    }}
    actions
    {{
        area(Processing)
        {{
            action(Refresh)
            {{
                trigger OnAction()
                begin
                    CurrPage.Update(false);
                end;
            }}
        }}
    }}
    {open}
    trigger OnAfterGetRecord()
    begin
        Log.Add('R' + Format(Rec.Id));
    end;
    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C' + Format(Rec.Id));
    end;
    var
        Log: Codeunit "Probe Log";
        GroupFilter: Code[10];
        Touch: Code[10];
}}
"#
    )
}

fn src() -> String {
    let mut s = String::from(
        r#"
table 50100 "Probe Row"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } field(3; Name; Code[20]) { } }
    keys { key(PK; Id) { Clustered = true; } key(ByGroupName; Grp, Name) { } }
}
codeunit 50101 "Probe Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take() Result: Text begin Result := Entries; Entries := ''; end;
}
"#,
    );
    s += &page(50102, "");
    s += &page(50103, "trigger OnOpenPage() begin Rec.SetCurrentKey(Grp, Name); end;");
    s += r#"
codeunit 50120 T
{
    var Log: Codeunit "Probe Log";

    local procedure SeedRows()
    var Row: Record "Probe Row";
    begin
        Row.DeleteAll();
        AddRow(1, 'A', 'N4'); AddRow(2, 'B', 'N3'); AddRow(3, 'A', 'N2'); AddRow(4, 'B', 'N1');
        Log.Take();
    end;

    local procedure AddRow(Id: Integer; Grp: Code[10]; Name: Code[20])
    var Row: Record "Probe Row";
    begin
        Row.Init(); Row.Id := Id; Row.Grp := Grp; Row.Name := Name; Row.Insert();
    end;

    procedure PkNextWithoutRefresh(): Text
    var P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkNextAfterActionUpdate(): Text
    var P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.Refresh.Invoke();
        Observed += ' action=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkNextAfterValidateUpdateNoFilter(): Text
    var P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        P.Touch.SetValue('T');
        Log.Take();
        P.Next();
        Observed := 'next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkGoToRecordWithoutRefresh(): Text
    var Row: Record "Probe Row"; P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit(); Log.Take();
        Row.Get(3); P.GoToRecord(Row);
        Observed := 'gotorecord=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkFilterKeepsRow(): Text
    var P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('A');
        Observed += ' validate=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkFilterDropsRow(): Text
    var Row: Record "Probe Row"; P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('B');
        Observed += ' validate=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('');
        Observed += ' cleared=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3); P.GoToRecord(Row);
        Observed += ' gotorecord=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkGoToRecordAfterValidateFilter(): Text
    var Row: Record "Probe Row"; P: TestPage "Probe List 50102"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        P.GroupFilter.SetValue('A');
        Log.Take();
        Row.Get(3); P.GoToRecord(Row);
        Observed := 'gotorecord=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure PkManyRowsNextAfterFilter()
    var Row: Record "Probe Row"; P: TestPage "Probe List 50102"; i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to 30 do
            AddRow(i, 'A', '');
        P.OpenEdit();
        Row.Get(25); P.GoToRecord(Row);
        P.GroupFilter.SetValue('A');
        P.Next();
    end;

    procedure KeyFilterKeepsRow(): Text
    var P: TestPage "Probe List 50103"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('A');
        Observed += ' validate=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure KeyFilterDropsRow(): Text
    var Row: Record "Probe Row"; P: TestPage "Probe List 50103"; Observed: Text;
    begin
        SeedRows(); P.OpenEdit();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('B');
        Observed += ' validate=' + P.Id.Value() + ':' + Log.Take();
        P.Next();
        Observed += ' next=' + P.Id.Value() + ':' + Log.Take();
        P.GroupFilter.SetValue('');
        Observed += ' cleared=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1); P.GoToRecord(Row);
        Observed += ' gotorecord=' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;
}
"#;
    s
}

fn run(b: &mut Built, p: &str) -> String {
    text(b.run_in("T", p))
}

// The controls of issue 5914: no refresh, an action's refresh, and a refresh after an OnValidate that keeps the filters.
#[test]
fn moves_without_a_filter_change_read_as_before() {
    let mut b = Built::from_source(&src());
    assert_eq!(
        run(&mut b, "PkNextWithoutRefresh"),
        "open=1:R1;R2;R3;R4;R1;R1;C1; next=2:R2;R2;C2;"
    );
    assert_eq!(
        run(&mut b, "PkNextAfterActionUpdate"),
        "open=1:R1;R2;R3;R4;R1;R1;C1; action=1:R1;R1;R1;R2;R3;R4;R1;R1;C1; next=2:R2;R2;C2;"
    );
    assert_eq!(run(&mut b, "PkNextAfterValidateUpdateNoFilter"), "next=2:R2;R2;C2;");
    assert_eq!(
        run(&mut b, "PkGoToRecordWithoutRefresh"),
        "gotorecord=3:R2;R2;C2;R3;R3;C3;"
    );
}

// Issue 5914 (primary key): the move after the refresh reads the row twice, as without a refresh.
#[test]
fn a_move_after_a_validate_filter_refresh_in_the_primary_key_reads_as_without_one() {
    let mut b = Built::from_source(&src());
    assert_eq!(
        run(&mut b, "PkFilterKeepsRow"),
        "open=1:R1;R2;R3;R4;R1;R1;C1; validate=1:R1;R3;R1;R1;C1; next=3:R3;R3;C3;"
    );
    assert_eq!(
        run(&mut b, "PkFilterDropsRow"),
        "open=1:R1;R2;R3;R4;R1;R1;C1; validate=2:R2;R4;R2;R2;C2; next=4:R4;R4;C4; cleared=4:R4;R3;R2;R1;R4;R4;C4; \
         gotorecord=3:R1;R1;C1;R2;R2;C2;R3;R3;C3;"
    );
    assert_eq!(run(&mut b, "PkGoToRecordAfterValidateFilter"), "gotorecord=3:R3;R3;C3;");
}

// Issue 5888 (another key): the refresh runs instead of being refused.
#[test]
fn a_validate_filter_refresh_in_another_key_reads_the_start_row_once_more() {
    let mut b = Built::from_source(&src());
    assert_eq!(
        run(&mut b, "KeyFilterKeepsRow"),
        "open=3:R3;R1;R4;R2;R3;R3;R3;C3; validate=3:R3;R3;R1;R3;R3;R3;C3; next=1:R1;R1;R1;C1;"
    );
    assert_eq!(
        run(&mut b, "KeyFilterDropsRow"),
        "open=3:R3;R1;R4;R2;R3;R3;R3;C3; validate=4:R3;R4;R2;R4;R4;R4;C4; next=2:R2;R2;R2;C2; \
         cleared=2:R2;R2;R4;R1;R3;R2;R2;R2;C2; gotorecord=1:R3;R3;R3;C3;R1;R1;R1;C1;"
    );
}

// Not pinned: a refresh that read more rows than a window holds (the way back from row 25 of 30): the next move is refused.
#[test]
fn a_move_after_a_validate_filter_refresh_past_a_window_is_refused() {
    let mut b = Built::from_source(&src());
    match b.run_in("T", "PkManyRowsNextAfterFilter") {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert!(why.contains("more rows than the client holds"), "{why}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// Not pinned: the same refresh on a List in another key (corpus writer SF pinned a Worksheet, RzUpdateFromValidate.al:476-544):
// the refresh itself is refused.
#[test]
fn a_validate_filter_refresh_on_a_list_in_another_key_is_refused() {
    let mut b = Built::from_source(&src().replace("PageType = Worksheet;", "PageType = List;"));
    match b.run_in("T", "KeyFilterKeepsRow") {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert!(why.contains("pinned on a Worksheet only"), "{why}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}
