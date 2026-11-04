//! Issue 7031: the first `Visible()` or `Editable()` of a field-linked part on a host opened with OpenNew tries to insert the host;
//! when the page's OnInsertRecord or the table's OnInsert raises, BC swallows the error, stores nothing and does not try again
//! (corpus pages/parts/PartLineBeforeHostKey.al R7031_* tests, codeunit 67582, pages 95604-95606).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

/// The repro of issue 7031: the host table's OnInsert requires a customer.
const ISSUE_SRC: &str = r#"
table 50103 "Repro Inbound"
{
    fields
    {
        field(1; "Entry no."; Code[10]) { }
        field(2; Customer; Code[10]) { }
    }
    keys { key(PK; "Entry no.") { Clustered = true; } }

    trigger OnInsert()
    begin
        Rec.TestField(Customer);
    end;
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
            field(Customer; Rec.Customer) { ApplicationArea = All; }
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
        T += ' hosts=' + Format(Inbound.Count());
        Card.Close();
        T += ' after-close=' + Format(Inbound.Count());
        exit(T);
    end;
}
"#;

#[test]
fn issue_repro_failing_insert_at_visible_is_swallowed() {
    assert_eq!(
        text(run_in(ISSUE_SRC, "Repro Part Visible", "NewInbound")),
        "visible=No hosts=0 after-close=0"
    );
}

/// The objects of the R7031 tests (tables 95600-95601, pages 95603-95606) with a log of their own, and a codeunit that runs the
/// probe's steps. Mode 1: the table's OnInsert raises; 2: the page's OnInsertRecord raises; 0: both only log. Modes 3 and 4 are
/// not in the probe: OnInsert writes a row of another table before it raises (3), the test reads GetLastErrorText (4).
const PROBE_SRC: &str = r#"
table 95600 "LT R7031 Host"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Customer; Code[10]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }

    trigger OnInsert()
    var
        Log: Codeunit "LT R7031 State";
        Other: Record "LT R7031 Other";
    begin
        Log.Add('H.OnInsert:' + "No.");
        if Log.Mode() = 3 then begin
            Other.Id := 1;
            Other.Insert();
        end;
        if Log.Mode() in [1, 3, 4] then
            TestField(Customer);
    end;
}

table 95601 "LT R7031 Line"
{
    fields
    {
        field(1; "Document No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[30]) { }
    }
    keys { key(PK; "Document No.", "Line No.") { Clustered = true; } }
}

table 95607 "LT R7031 Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 95602 "LT R7031 State"
{
    SingleInstance = true;

    var
        CurrentMode: Integer;
        Entries: Text;

    procedure SetMode(NewMode: Integer)
    begin
        CurrentMode := NewMode;
    end;

    procedure Mode(): Integer
    begin
        exit(CurrentMode);
    end;

    procedure Add(Entry: Text)
    begin
        if Entries <> '' then
            Entries += ' | ';
        Entries += Entry;
    end;

    procedure Take(): Text
    var
        T: Text;
    begin
        T := '[' + Entries + ']';
        Entries := '';
        exit(T);
    end;
}

page 95603 "LT R7031 Lines"
{
    PageType = ListPart;
    SourceTable = "LT R7031 Line";

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Description; Rec.Description) { }
            }
        }
    }
}

page 95604 "LT R7031 CardConstTrue"
{
    PageType = Card;
    SourceTable = "LT R7031 Host";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Customer; Rec.Customer) { }
            part(Lines; "LT R7031 Lines")
            {
                SubPageLink = "Document No." = field("No.");
                Visible = true;
            }
        }
    }

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT R7031 State";
    begin
        Log.Add('P.OnInsertRecord:' + Rec."No.");
        if Log.Mode() = 2 then
            Rec.TestField(Customer);
        exit(true);
    end;
}

page 95605 "LT R7031 CardVarFalse"
{
    PageType = Card;
    SourceTable = "LT R7031 Host";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Customer; Rec.Customer) { }
            part(Lines; "LT R7031 Lines")
            {
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
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

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT R7031 State";
    begin
        Log.Add('P.OnInsertRecord:' + Rec."No.");
        if Log.Mode() = 2 then
            Rec.TestField(Customer);
        exit(true);
    end;

    var
        PartVisible: Boolean;
}

page 95606 "LT R7031 CardVarTrue"
{
    PageType = Card;
    SourceTable = "LT R7031 Host";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Customer; Rec.Customer) { }
            part(Lines; "LT R7031 Lines")
            {
                SubPageLink = "Document No." = field("No.");
                Visible = PartVisible;
            }
        }
    }

    trigger OnOpenPage()
    begin
        PartVisible := true;
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT R7031 State";
    begin
        Log.Add('P.OnInsertRecord:' + Rec."No.");
        if Log.Mode() = 2 then
            Rec.TestField(Customer);
        exit(true);
    end;

    var
        PartVisible: Boolean;
}

codeunit 95608 "LT R7031 Run"
{
    local procedure Seed(Mode: Integer)
    var
        Line: Record "LT R7031 Line";
        Log: Codeunit "LT R7031 State";
    begin
        Log.Take();
        Log.SetMode(Mode);
        Line."Document No." := '';
        Line."Line No." := 10000;
        Line.Description := 'blank key';
        Line.Insert();
    end;

    local procedure Stored(Name: Text): Text
    var
        Host: Record "LT R7031 Host";
        Line: Record "LT R7031 Line";
    begin
        exit(' ' + Name + '=' + Format(Host.Count()) + ' hosts, ' + Format(Line.Count()) + ' lines');
    end;

    local procedure Logged(Name: Text): Text
    var
        Log: Codeunit "LT R7031 State";
    begin
        exit(' ' + Name + '=' + Log.Take());
    end;

    procedure ConstTrue(Mode: Integer; VisibleFirst: Boolean): Text
    var
        Card: TestPage "LT R7031 CardConstTrue";
        T: Text;
    begin
        Seed(Mode);
        Card.OpenNew();
        T := Logged('open');
        if VisibleFirst then begin
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log') + Stored('after_visible');
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log');
        end else begin
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log') + Stored('after_editable');
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log');
        end;
        T += Stored('before_close');
        Card.Close();
        exit(T + Logged('close_log') + Stored('after_close'));
    end;

    procedure VarFalse(Mode: Integer; VisibleFirst: Boolean): Text
    var
        Card: TestPage "LT R7031 CardVarFalse";
        T: Text;
    begin
        Seed(Mode);
        Card.OpenNew();
        T := Logged('open');
        if VisibleFirst then begin
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log') + Stored('after_visible');
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log');
        end else begin
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log') + Stored('after_editable');
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log');
        end;
        T += Stored('before_close');
        Card.Close();
        exit(T + Logged('close_log') + Stored('after_close'));
    end;

    procedure VarTrue(Mode: Integer; VisibleFirst: Boolean): Text
    var
        Card: TestPage "LT R7031 CardVarTrue";
        T: Text;
    begin
        Seed(Mode);
        Card.OpenNew();
        T := Logged('open');
        if VisibleFirst then begin
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log') + Stored('after_visible');
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log');
        end else begin
            T += ' editable=' + Format(Card.Lines.Editable()) + Logged('editable_log') + Stored('after_editable');
            T += ' visible=' + Format(Card.Lines.Visible()) + Logged('visible_log');
        end;
        T += Stored('before_close');
        Card.Close();
        exit(T + Logged('close_log') + Stored('after_close'));
    end;

    procedure ConstTrueVisibleThenSetValue(): Text
    var
        Card: TestPage "LT R7031 CardConstTrue";
    begin
        Seed(1);
        Card.OpenNew();
        Card.Lines.Visible();
        Card.Customer.SetValue('C1');
        Card.Close();
        exit('closed');
    end;

    procedure ConstTrueVisibleThenLastError(): Text
    var
        Card: TestPage "LT R7031 CardConstTrue";
        T: Text;
    begin
        Seed(4);
        Card.OpenNew();
        T := Format(Card.Lines.Visible());
        exit(T + ' ' + GetLastErrorText());
    end;
}
"#;

fn run_probe(routine: &str, mode: i32, visible_first: bool) -> String {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(
        PROBE_SRC,
        "LT R7031 Run",
        routine,
        vec![Value::Integer(mode), Value::Boolean(visible_first)],
    );
    text(r.map(|(v, _)| v))
}

/// The expected trace of one R7031 test: `visible` is what the part's Visible() answers, `log` what the first call logs, `hosts`
/// the hosts stored after it (PartLineBeforeHostKey.al R7031_{ConstTrue,VarFalse,VarTrue}_{TableError,PageError,LogOnly}_*).
fn expected(visible: &str, log: &str, hosts: u32, visible_first: bool) -> String {
    let stored = format!("{hosts} hosts, 1 lines");
    let (first, second) = if visible_first {
        (
            format!(" visible={visible} visible_log={log} after_visible={stored}"),
            " editable=Yes editable_log=[]".to_string(),
        )
    } else {
        (
            format!(" editable=Yes editable_log={log} after_editable={stored}"),
            format!(" visible={visible} visible_log=[]"),
        )
    };
    format!(" open=[]{first}{second} before_close={stored} close_log=[] after_close={stored}")
}

#[test]
fn every_visible_shape_and_mode_matches_the_probe() {
    const BOTH: &str = "[P.OnInsertRecord: | H.OnInsert:]";
    const PAGE: &str = "[P.OnInsertRecord:]";
    for (routine, visible) in [("ConstTrue", "Yes"), ("VarFalse", "No"), ("VarTrue", "Yes")] {
        for (mode, log, hosts) in [(1, BOTH, 0), (2, PAGE, 0), (0, BOTH, 1)] {
            for visible_first in [true, false] {
                assert_eq!(
                    run_probe(routine, mode, visible_first),
                    expected(visible, log, hosts, visible_first),
                    "{routine} mode {mode} visible_first {visible_first}"
                );
            }
        }
    }
}

/// A trigger that wrote a row before it raised: whether BC rolls the row back is not pinned.
#[test]
fn a_failed_insert_that_wrote_before_raising_is_refused() {
    let r = al_testkit::run_with(
        PROBE_SRC,
        "LT R7031 Run",
        "ConstTrue",
        vec![
            al_builtins::value::Value::Integer(3),
            al_builtins::value::Value::Boolean(true),
        ],
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

/// What GetLastErrorText reads after the swallowed error is not pinned.
#[test]
fn last_error_text_after_the_swallowed_error_is_refused() {
    let r = run_in(PROBE_SRC, "LT R7031 Run", "ConstTrueVisibleThenLastError");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

/// A field set after the swallowed insert: whether Close then inserts the host is not pinned.
#[test]
fn a_set_value_after_the_swallowed_insert_is_refused() {
    let r = run_in(PROBE_SRC, "LT R7031 Run", "ConstTrueVisibleThenSetValue");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
