//! A CaptionClass expression that can reach a Record write, on a page whose empty-record runs have no pinned number (al-runner
//! issue 7391): the open runs while no write runs, and refuses when a write does run, as a dialog does
//! (`pages::CaptionDialogs::Refused`).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 50110 "RW Rec"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; NotTaken; Text[30]) { CaptionClass = NotTakenCap(); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure NotTakenCap(): Text
    var
        Other: Record "RW Log";
    begin
        if Rec."Code" = 'WRITE' then begin
            Other.Init();
            Other.Id := 1;
            Other.Insert();
        end;
        exit('3,Not taken');
    end;
}

table 50111 "RW Taken"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Taken; Text[30]) { CaptionClass = TakenCap(); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure TakenCap(): Text
    var
        Other: Record "RW Log";
    begin
        Other.Init();
        Other.Id := 2;
        if Other.Insert() then;
        exit('3,Taken');
    end;
}

table 50112 "RW Log"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50110 "RW Card"
{
    PageType = Card;
    SourceTable = "RW Rec";
    layout { area(Content) { field("Code"; Rec."Code") { } field(NotTaken; Rec.NotTaken) { } } }
}

page 50111 "RW Taken Card"
{
    PageType = Card;
    SourceTable = "RW Taken";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Taken; Rec.Taken) { } } }
}

codeunit 50113 C
{
    procedure NotTakenOpen(): Text
    var
        Card: TestPage "RW Card";
    begin
        Card.OpenNew();
        exit(Card.NotTaken.Caption);
    end;

    procedure TakenOpen(): Text
    var
        Card: TestPage "RW Taken Card";
    begin
        Card.OpenNew();
        exit(Card.Taken.Caption);
    end;

    procedure TakenFieldCaption(): Text
    var
        R: Record "RW Taken";
        L: Record "RW Log";
    begin
        exit(R.FieldCaption(Taken) + '|' + Format(L.Count()));
    end;
}
"#;

fn out(routine: &str) -> String {
    match al_testkit::Built::from_source(SRC).run_in("C", routine) {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

/// The write is reachable but the empty record never takes its branch: the open runs.
#[test]
fn a_write_the_runs_do_not_take_lets_the_open_run() {
    assert_eq!(out("NotTakenOpen"), "Not taken");
}

/// A write that runs during the open's empty-record runs refuses the open.
#[test]
fn a_write_that_runs_refuses_the_open() {
    let got = out("TakenOpen");
    assert!(
        got.starts_with("refused:") && got.contains("Insert raised by a CaptionClass expression"),
        "{got}"
    );
}

/// Main's result is kept: a FieldCaption read outside a page open runs the write, as on main.
#[test]
fn a_field_caption_read_outside_a_page_open_keeps_mains_result() {
    assert_eq!(out("TakenFieldCaption"), "Taken|1");
}
