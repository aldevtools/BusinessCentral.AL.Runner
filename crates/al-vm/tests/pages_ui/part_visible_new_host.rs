//! Issue 6920: `Visible()` of a part whose host is a new, unsaved record while rows of the part's blank link key exist. The
//! repro of the issue: a Card host whose part is hidden by a page variable that only OnAfterGetCurrRecord of a saved record
//! sets.

use al_builtins::value::Value;
use al_testkit::{Built, text};

const ISSUE_SRC: &str = r#"
table 50103 "Repro Inbound"
{
    fields { field(1; "Entry no."; Code[10]) { } }
    keys { key(PK; "Entry no.") { Clustered = true; } }
}

table 50104 "Repro Trp"
{
    fields
    {
        field(1; "Entry no."; Code[10]) { }
        field(2; "Entry no. inbound"; Code[10]) { }
    }
    keys { key(PK; "Entry no.") { Clustered = true; } }
}

page 50101 "Repro Trp Sub"
{
    PageType = ListPart;
    SourceTable = "Repro Trp";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Entry no."; Rec."Entry no.") { ApplicationArea = All; }
            }
        }
    }
}

page 50102 "Repro Inbound Card"
{
    PageType = Card;
    SourceTable = "Repro Inbound";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field("Entry no."; Rec."Entry no.") { ApplicationArea = All; }
            part(subform; "Repro Trp Sub")
            {
                ApplicationArea = All;
                SubPageLink = "Entry no. inbound" = field("Entry no.");
                Visible = SubFormVisible;
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        SubFormVisible := false;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        if Rec."Entry no." <> '' then
            SubFormVisible := true;
    end;

    var
        SubFormVisible: Boolean;
}

codeunit 50102 "Repro Part Visible"
{
    procedure NewInbound(): Text
    var
        Inbound: Record "Repro Inbound";
        Trp: Record "Repro Trp";
        Card: TestPage "Repro Inbound Card";
        T: Text;
    begin
        Trp."Entry no." := 'T1';
        Trp.Insert();
        Card.OpenNew();
        T := 'visible=' + Format(Card.subform.Visible());
        T += ' headers=' + Format(Inbound.Count());
        Card.Close();
        T += ' after-close=' + Format(Inbound.Count());
        exit(T);
    end;
}
"#;

/// The repro of issue 6920.
#[test]
fn visible_of_a_hidden_part_on_a_new_card_with_blank_key_rows() {
    assert_eq!(
        text(al_testkit::run_in(ISSUE_SRC, "Repro Part Visible", "NewInbound")),
        "visible=No headers=1 after-close=1"
    );
}

/// The objects of corpus pages/parts/PartLineBeforeHostKey.al (tables 67573 and 67576, log 67579, part 67578, hosts 95160-95166 of
/// writer r6920) and a codeunit that runs the probe's steps on each host.
const PROBE_SRC: &str = r#"
table 67573 "LT DL Header"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('H.OnValidate(No.):' + "No.");
            end;
        }
        field(2; Name; Text[30])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('H.OnValidate(Name):' + Name + ' key=' + "No.");
            end;
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('H.OnInsert:' + "No.");
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('H.OnModify:' + "No.");
    end;
}

table 67576 "LT DL Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Document No."; Code[20])
        {
            DataClassification = SystemMetadata;
            TableRelation = "LT DL Header";

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Document No.):' + "Document No.");
            end;
        }
        field(2; "Line No."; Integer)
        {
            DataClassification = SystemMetadata;
        }
        field(3; Description; Text[30])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Description):' + Description + ' key=' + "Document No." + '/' + Format("Line No."));
            end;
        }
        field(4; Quantity; Decimal)
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Quantity):' + Format(Quantity) + ' key=' + "Document No." + '/' + Format("Line No."));
            end;
        }
    }

    keys
    {
        key(PK; "Document No.", "Line No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('L.OnInsert:' + "Document No." + '/' + Format("Line No."));
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('L.OnModify:' + "Document No." + '/' + Format("Line No."));
    end;
}

codeunit 67579 "LT DL Log"
{
    SingleInstance = true;

    var
        Lines: List of [Text];

    procedure Reset()
    begin
        Clear(Lines);
    end;

    procedure Add(Line: Text)
    begin
        Lines.Add(Line);
    end;

    procedure Dump(): Text
    var
        Line: Text;
        Result: Text;
    begin
        foreach Line in Lines do begin
            if Result <> '' then
                Result += ' | ';
            Result += Line;
        end;
        exit('[' + Result + ']');
    end;
}

page 67578 "LT DL Lines"
{
    PageType = ListPart;
    SourceTable = "LT DL Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('LP.OnNewRecord:' + Rec."Document No.");
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('LP.OnInsertRecord:' + Rec."Document No." + '/' + Format(Rec."Line No."));
        exit(true);
    end;
}

page 95160 "LT R6920 DocConst"
{
    PageType = Document;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = false;
                Editable = false;
            }
        }
    }
}

page 95161 "LT R6920 DocAfterGet"
{
    PageType = Document;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
                Editable = PartVisible;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        PartVisible := true;
    end;

    var
        PartVisible: Boolean;
}

page 95162 "LT R6920 DocOpenPage"
{
    PageType = Document;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
                Editable = PartVisible;
            }
        }
    }

    trigger OnOpenPage()
    begin
        PartVisible := true;
    end;

    var
        PartVisible: Boolean;
}

page 95163 "LT R6920 CardConst"
{
    PageType = Card;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = false;
                Editable = false;
            }
        }
    }
}

page 95164 "LT R6920 CardAfterGet"
{
    PageType = Card;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
                Editable = PartVisible;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        PartVisible := true;
    end;

    var
        PartVisible: Boolean;
}

page 95165 "LT R6920 CardOpenPage"
{
    PageType = Card;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
                Editable = PartVisible;
            }
        }
    }

    trigger OnOpenPage()
    begin
        PartVisible := true;
    end;

    var
        PartVisible: Boolean;
}

page 95166 "LT R6920 CardIssue"
{
    PageType = Card;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
                Editable = PartVisible;
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        PartVisible := false;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        if Rec."No." <> '' then
            PartVisible := true;
    end;

    var
        PartVisible: Boolean;
}

codeunit 50990 "R6920 Test"
{
    local procedure Take(): Text
    var
        Log: Codeunit "LT DL Log";
        T: Text;
    begin
        T := Log.Dump();
        Log.Reset();
        exit(T);
    end;

    local procedure Stored(): Text
    var
        Header: Record "LT DL Header";
        Line: Record "LT DL Line";
    begin
        exit(Format(Header.Count()) + 'h' + Format(Line.Count()) + 'l');
    end;

    // How often the part ran OnNewRecord (none, one, many: the corpus compresses two and more) and whether the link field was
    // validated.
    local procedure PartNew(Log: Text): Text
    var
        N: Integer;
    begin
        N := Log.Split('LP.OnNewRecord').Count() - 1;
        case N of
            0:
                exit('none/' + Format(Log.Contains('L.OnValidate(Document No.)')));
            1:
                exit('one/' + Format(Log.Contains('L.OnValidate(Document No.)')));
        end;
        exit('many/' + Format(Log.Contains('L.OnValidate(Document No.)')));
    end;

    local procedure Seed(Rows: Boolean)
    var
        Header: Record "LT DL Header";
        Line: Record "LT DL Line";
    begin
        Line.DeleteAll();
        Header.DeleteAll();
        if Rows then begin
            Line."Document No." := '';
            Line."Line No." := 10000;
            Line.Description := 'seeded';
            Line.Insert();
        end;
        Take();
    end;

    procedure DocConst(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 DocConst";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        Take();
        if VisibleFirst then
            asserterror Host.Lines.Visible()
        else
            asserterror Host.Lines.Editable();
        T := 'first=' + GetLastErrorText() + '/' + GetLastErrorCode() + '#log=' + Take();
        asserterror Host.Lines.Description.Value();
        T += '#read=' + GetLastErrorText() + '/' + GetLastErrorCode() + '#log=' + Take();
        Host.Close();
        T += '#close=' + Take();
        exit(T);
    end;

    procedure DocAfterGet(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 DocAfterGet";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        T := 'open=' + PartNew(Take()) + '#';
        if VisibleFirst then
            T += 'visible=' + Format(Host.Lines.Visible())
        else
            T += 'editable=' + Format(Host.Lines.Editable());
        T += '#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        if VisibleFirst then
            T += '#editable=' + Format(Host.Lines.Editable())
        else
            T += '#visible=' + Format(Host.Lines.Visible());
        T += '#' + Format(Take().Contains('H.OnInsert'));
        T += '#read=[' + Host.Lines.Description.Value() + ']#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        T += '#again=' + Format(Host.Lines.Visible()) + '/' + Format(Host.Lines.Editable());
        Host.Close();
        T += '#close=' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        exit(T);
    end;

    procedure DocOpenPage(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 DocOpenPage";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        T := 'open=' + PartNew(Take()) + '#';
        if VisibleFirst then
            T += 'visible=' + Format(Host.Lines.Visible())
        else
            T += 'editable=' + Format(Host.Lines.Editable());
        T += '#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        if VisibleFirst then
            T += '#editable=' + Format(Host.Lines.Editable())
        else
            T += '#visible=' + Format(Host.Lines.Visible());
        T += '#' + Format(Take().Contains('H.OnInsert'));
        T += '#read=[' + Host.Lines.Description.Value() + ']#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        T += '#again=' + Format(Host.Lines.Visible()) + '/' + Format(Host.Lines.Editable());
        Host.Close();
        T += '#close=' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        exit(T);
    end;

    procedure CardConst(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 CardConst";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        Take();
        if VisibleFirst then
            asserterror Host.Lines.Visible()
        else
            asserterror Host.Lines.Editable();
        T := 'first=' + GetLastErrorText() + '/' + GetLastErrorCode() + '#log=' + Take();
        asserterror Host.Lines.Description.Value();
        T += '#read=' + GetLastErrorText() + '/' + GetLastErrorCode() + '#log=' + Take();
        Host.Close();
        T += '#close=' + Take();
        exit(T);
    end;

    procedure CardAfterGet(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 CardAfterGet";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        T := 'open=' + PartNew(Take()) + '#';
        if VisibleFirst then
            T += 'visible=' + Format(Host.Lines.Visible())
        else
            T += 'editable=' + Format(Host.Lines.Editable());
        T += '#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        if VisibleFirst then
            T += '#editable=' + Format(Host.Lines.Editable())
        else
            T += '#visible=' + Format(Host.Lines.Visible());
        T += '#' + Format(Take().Contains('H.OnInsert'));
        T += '#read=[' + Host.Lines.Description.Value() + ']#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        T += '#again=' + Format(Host.Lines.Visible()) + '/' + Format(Host.Lines.Editable());
        Host.Close();
        T += '#close=' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        exit(T);
    end;

    procedure CardOpenPage(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 CardOpenPage";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        T := 'open=' + PartNew(Take()) + '#';
        if VisibleFirst then
            T += 'visible=' + Format(Host.Lines.Visible())
        else
            T += 'editable=' + Format(Host.Lines.Editable());
        T += '#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        if VisibleFirst then
            T += '#editable=' + Format(Host.Lines.Editable())
        else
            T += '#visible=' + Format(Host.Lines.Visible());
        T += '#' + Format(Take().Contains('H.OnInsert'));
        T += '#read=[' + Host.Lines.Description.Value() + ']#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        T += '#again=' + Format(Host.Lines.Visible()) + '/' + Format(Host.Lines.Editable());
        Host.Close();
        T += '#close=' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        exit(T);
    end;

    procedure CardIssue(Rows: Boolean; NewHost: Boolean; VisibleFirst: Boolean): Text
    var
        Host: TestPage "LT R6920 CardIssue";
        T: Text;
    begin
        Seed(Rows);
        if NewHost then
            Host.OpenNew()
        else
            Host.OpenEdit();
        T := 'open=' + PartNew(Take()) + '#';
        if VisibleFirst then
            T += 'visible=' + Format(Host.Lines.Visible())
        else
            T += 'editable=' + Format(Host.Lines.Editable());
        T += '#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        if VisibleFirst then
            T += '#editable=' + Format(Host.Lines.Editable())
        else
            T += '#visible=' + Format(Host.Lines.Visible());
        T += '#' + Format(Take().Contains('H.OnInsert'));
        T += '#read=[' + Host.Lines.Description.Value() + ']#' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        T += '#again=' + Format(Host.Lines.Visible()) + '/' + Format(Host.Lines.Editable());
        Host.Close();
        T += '#close=' + Format(Take().Contains('H.OnInsert')) + '#' + Stored();
        exit(T);
    end;
}
"#;

fn probe(host: &str, rows: bool, new_host: bool, visible_first: bool) -> String {
    let mut b = Built::from_source(PROBE_SRC);
    let (oi, ri) = b.routine_in("R6920 Test", host);
    let args = vec![
        Value::Boolean(rows),
        Value::Boolean(new_host),
        Value::Boolean(visible_first),
    ];
    text(b.call(oi, ri, args).map(|(v, _)| v))
}

/// What PartLineBeforeHostKey.al R6920_<host>_<rows>_<open>_<first> pins for a host whose part's Visible is a page variable:
/// on OpenNew the first Visible() or Editable() of the part inserts the host (H.OnInsert), with or without rows of the
/// blank key, and nothing later inserts again; on OpenEdit of an empty header table nothing is inserted and the part shows
/// no row. Visible() gives the variable as the page code left it; Editable() gives Yes although the part's Editable is the
/// same variable.
fn expected(visible: bool, rows: bool, new_host: bool, visible_first: bool) -> String {
    let yn = |b: bool| if b { "Yes" } else { "No" };
    let lines = if rows { 1 } else { 0 };
    let h = if new_host { 1 } else { 0 };
    let first = if visible_first {
        format!("visible={}", yn(visible))
    } else {
        "editable=Yes".to_string()
    };
    let second = if visible_first {
        "editable=Yes".to_string()
    } else {
        format!("visible={}", yn(visible))
    };
    let read = if rows && new_host { "seeded" } else { "" };
    // `open`: `[]` on OpenNew with rows of the blank key; `fired at least once: LP.OnNewRecord, L.OnValidate(Document No.)` on
    // OpenNew without; `fired at least once: LP.OnNewRecord` on OpenEdit.
    let open = match (new_host, rows) {
        (true, true) => "none/No",
        (true, false) => "many/Yes",
        (false, _) => "many/No",
    };
    format!(
        "open={open}#{first}#{}#{h}h{lines}l#{second}#No#read=[{read}]#No#{h}h{lines}l#again={}/Yes#close=No#{h}h{lines}l",
        yn(new_host),
        yn(visible)
    )
}

fn check_host(host: &str, visible: bool) {
    for rows in [true, false] {
        for new_host in [true, false] {
            for visible_first in [true, false] {
                assert_eq!(
                    probe(host, rows, new_host, visible_first),
                    expected(visible, rows, new_host, visible_first),
                    "{host} rows={rows} new_host={new_host} visible_first={visible_first}"
                );
            }
        }
    }
}

/// PartLineBeforeHostKey.al R6920_DocAfterGet_*: the variable OnAfterGetRecord would set stays false on a new host.
#[test]
fn document_host_visible_set_in_on_after_get_record() {
    check_host("DocAfterGet", false);
}

/// PartLineBeforeHostKey.al R6920_DocOpenPage_*.
#[test]
fn document_host_visible_set_in_on_open_page() {
    check_host("DocOpenPage", true);
}

/// PartLineBeforeHostKey.al R6920_CardAfterGet_*.
#[test]
fn card_host_visible_set_in_on_after_get_record() {
    check_host("CardAfterGet", false);
}

/// PartLineBeforeHostKey.al R6920_CardOpenPage_*.
#[test]
fn card_host_visible_set_in_on_open_page() {
    check_host("CardOpenPage", true);
}

/// PartLineBeforeHostKey.al R6920_CardIssue_*: the shape of issue 6920.
#[test]
fn card_host_of_the_issue() {
    check_host("CardIssue", false);
}

/// PartLineBeforeHostKey.al R6920_{Doc,Card}Const_*: a part whose Visible is the constant false is not on the page; Visible(),
/// Editable() and a field read fail with TestPartNotFound and run no trigger.
#[test]
fn a_part_hidden_by_a_constant_is_not_on_the_page() {
    for host in ["DocConst", "CardConst"] {
        for rows in [true, false] {
            for new_host in [true, false] {
                for visible_first in [true, false] {
                    let got = probe(host, rows, new_host, visible_first);
                    let parts: Vec<&str> = got.split('#').collect();
                    assert_eq!(parts.len(), 5, "{got}");
                    for (part, lead) in [(parts[0], "first="), (parts[2], "read=")] {
                        let msg = part.strip_prefix(lead).unwrap_or_else(|| panic!("{got}"));
                        assert!(
                            msg.starts_with("The part with ID = ")
                                && msg.ends_with(" was not found on the page./TestPartNotFound"),
                            "{host} {got}"
                        );
                    }
                    assert_eq!(
                        [parts[1], parts[3], parts[4]],
                        ["log=[]", "log=[]", "close=[]"],
                        "{host} {got}"
                    );
                }
            }
        }
    }
}
