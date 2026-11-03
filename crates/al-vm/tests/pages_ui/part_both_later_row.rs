//! al-runner issue 6877 (follow-up of 6038): a ListPart with UpdatePropagation = Both that stands on a stored row other than
//! its first and runs CurrPage.Update(false) in a control's OnValidate stays on its row, on a Document, Card and ListPlus host,
//! with and without a change of the host's stored row before (probe pages/lt-th-part-update-cursor/R6877PartBothLaterRow.al on
//! branch al-runtime-rs-staging-r6877, BC 28.5 on Windows: all 18 combinations). The objects are copied from the probe; the
//! procedure returns what the probe observes.

const SRC: &str = r#"
table 89880 "LT TH Doc" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 89881 "LT TH Line"
{
    fields
    {
        field(1; "Document No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(5; "Qty. to Ship"; Decimal) { }
        field(6; Note; Text[30]) { }
        field(7; "Bin Code"; Code[20]) { }
    }
    keys { key(PK; "Document No.", "Line No.") { Clustered = true; } }
}
codeunit 89884 "LT TH Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Reset() begin Entries := ''; end;
    procedure Add(Entry: Text) begin Entries += Entry + ' '; end;
    procedure Get(): Text begin exit(Entries); end;
}
table 95000 "LT R6877 Doc" { fields { field(1; "No."; Code[20]) { } field(2; Info; Text[30]) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 95001 "LT R6877 Lines"
{
    PageType = ListPart;
    SourceTable = "LT TH Line";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Group)
            {
                field("Line No."; Rec."Line No.") { }
                field(Note; Rec.Note)
                {
                    trigger OnValidate()
                    var Log: Codeunit "LT TH Log";
                    begin
                        Log.Add('N.V'); CurrPage.Update(false); Log.Add('N.V.end');
                    end;
                }
                field("Qty. to Ship"; Rec."Qty. to Ship")
                {
                    trigger OnValidate()
                    var Head: Record "LT R6877 Doc"; Log: Codeunit "LT TH Log";
                    begin
                        Log.Add('Q.V'); Head.Get(Rec."Document No."); Head.Info := 'changed'; Head.Modify();
                        CurrPage.Update(false); Log.Add('Q.V.end');
                    end;
                }
                field("Bin Code"; Rec."Bin Code") { }
            }
        }
    }
    actions { area(Processing) { action(NextLine) { trigger OnAction() var Log: Codeunit "LT TH Log"; begin Log.Add('ACT'); Rec.Next(); end; } } }
    trigger OnAfterGetRecord() var Log: Codeunit "LT TH Log"; begin Log.Add('R' + Format(Rec."Line No." div 10000)); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "LT TH Log"; begin Log.Add('C' + Format(Rec."Line No." div 10000)); end;
}
PAGES
codeunit 95005 "LT R6877 Probe"
{
    local procedure Seed()
    var Head: Record "LT R6877 Doc"; Line: Record "LT TH Line"; Log: Codeunit "LT TH Log"; n: Integer;
    begin
        Head."No." := 'D1'; Head.Insert();
        for n := 1 to 3 do begin Line."Document No." := 'D1'; Line."Line No." := n * 10000; Line.Insert(); end;
        Log.Reset();
    end;

    local procedure Stored(): Text
    var Head: Record "LT R6877 Doc"; Line: Record "LT TH Line"; Rows: Text;
    begin
        Head.Get('D1');
        if Line.FindSet() then
            repeat
                Rows += Format(Line."Line No." div 10000) + ':' + Line.Note + '/' + Format(Line."Qty. to Ship") + '/' + Line."Bin Code" + ' ';
            until Line.Next() = 0;
        exit(Head.Info + ' ' + Rows);
    end;
BODIES
}
"#;

fn pages_and_bodies() -> (String, String) {
    let mut pages = String::new();
    let mut bodies = String::new();
    for (name, ty, id) in [
        ("Doc", "Document", 95002),
        ("Card", "Card", 95003),
        ("Plus", "ListPlus", 95004),
    ] {
        pages += &format!(
            r#"page {id} "LT R6877 {name}"
{{
    PageType = {ty};
    SourceTable = "LT R6877 Doc";
    layout {{ area(Content) {{ field("No."; Rec."No.") {{ }} field(Info; Rec.Info) {{ }}
        part(Lines; "LT R6877 Lines") {{ SubPageLink = "Document No." = field("No."); UpdatePropagation = Both; }} }} }}
    trigger OnAfterGetRecord() var Log: Codeunit "LT TH Log"; begin Log.Add('HR'); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "LT TH Log"; begin Log.Add('HC'); end;
}}
"#
        );
        bodies += &format!(
            r#"
    procedure Run{name}(Nav: Integer; Ctl: Integer) Seen: Text
    var P: TestPage "LT R6877 {name}"; Log: Codeunit "LT TH Log";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('D1'); P.Lines.First();
        if Nav = 1 then P.Lines.Next();
        if Nav = 2 then begin P.Lines.Next(); P.Lines.NextLine.Invoke(); end;
        if Nav = 3 then P.Lines.NextLine.Invoke();
        Seen := P.Lines."Line No.".Value() + ';';
        Log.Reset();
        if Ctl = 1 then P.Lines.Note.SetValue('A') else P.Lines."Qty. to Ship".SetValue('5');
        Seen += P.Lines."Line No.".Value() + ';' + Log.Get() + ';' + P.Info.Value() + ';';
        Log.Reset();
        P.Lines."Bin Code".SetValue('Z');
        Seen += P.Lines."Line No.".Value() + ';' + Log.Get() + ';';
        P.Close();
        Seen += Stored();
    end;
"#
        );
    }
    (pages, bodies)
}

fn run(host: &str, nav: u8, ctl: u8) -> String {
    let (pages, bodies) = pages_and_bodies();
    let src = SRC.replace("PAGES", &pages).replace("BODIES", &bodies);
    let mut b = al_testkit::Built::from_source(&src);
    let (oi, ri) = b.routine_in("LT R6877 Probe", &format!("Run{host}"));
    let args = vec![
        al_builtins::value::Value::Integer(i32::from(nav)),
        al_builtins::value::Value::Integer(i32::from(ctl)),
    ];
    al_testkit::text(b.call(oi, ri, args).map(|(v, _)| v))
}

// R6877PartBothLaterRow.al R_<host>_Nav<n>_Ctl<c>: Nav 1 = line 2 by Next, 2 = line 3 by Next and the part action's Rec.Next(),
// 3 = line 2 by the action from line 1; Ctl 1 = Note (Update(false)), 2 = Qty. to Ship (changes the host's stored row first).
fn expect(host: &str, nav: u8, ctl: u8, line: &str, read: &str) {
    let (trigger, info, stored) = if ctl == 1 {
        ("N.V N.V.end", "", " ")
    } else {
        ("Q.V Q.V.end", "changed", "changed ")
    };
    let bin = match line {
        "20000" => "1:/0/ 2:/0/Z 3:/0/ ",
        _ => "1:/0/ 2:/0/ 3:/0/Z ",
    };
    let want = format!("{line};{line};{trigger} HR HC {read} ;{info};{line};;{stored}{bin}");
    assert_eq!(run(host, nav, ctl), want, "{host} nav {nav} ctl {ctl}");
}

#[test]
fn a_part_with_both_stays_on_its_later_row_after_curr_page_update_in_on_validate() {
    for host in ["Doc", "Card", "Plus"] {
        for ctl in [1, 2] {
            expect(host, 1, ctl, "20000", "R2 R3 R2 R1 R2 R2 C2");
            expect(host, 3, ctl, "20000", "R2 R3 R2 R1 R2 R2 C2");
            expect(host, 2, ctl, "30000", "R3 R2 R1 R3 R3 C3");
        }
    }
}

#[test]
fn a_part_with_both_on_its_first_row_still_runs() {
    for host in ["Doc", "Card", "Plus"] {
        let seen = run(host, 0, 1);
        assert!(seen.starts_with("10000;10000;"), "{host}: {seen}");
    }
}
