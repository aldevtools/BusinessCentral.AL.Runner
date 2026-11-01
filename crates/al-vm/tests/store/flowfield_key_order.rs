//! The order of the rows after SetCurrentKey on a FlowField whose value depends on the row: by the FlowField's value, ties
//! by the primary key, `SetAscending(FlowField, false)` reversing the FlowField only (record/lt-set-current-key/
//! Y43SetAscendingFlowField.al, corpus PR 966; SetCurrentKeyFlowFieldOrder.al:85-180 for a count FlowField).

use al_testkit::text;

const SRC: &str = r#"
table 69960 "FK Line"
{
    fields { field(1; Id; Integer) { } field(2; DocType; Option) { OptionMembers = Quote,Order; } field(3; DocNo; Code[20]) { } field(4; Amount; Decimal) { } }
    keys { key(PK; Id) { } }
}
table 69961 "FK Header"
{
    fields
    {
        field(1; DocType; Option) { OptionMembers = Quote,Order; }
        field(2; "No."; Code[20]) { }
        field(3; PostDate; Date) { }
        field(4; Amount; Decimal) { FieldClass = FlowField; CalcFormula = sum("FK Line".Amount where(DocType = field(DocType), DocNo = field("No."))); }
        field(5; Lines; Integer) { FieldClass = FlowField; CalcFormula = count("FK Line" where(DocType = field(DocType), DocNo = field("No."))); }
        field(6; Any; Boolean) { FieldClass = FlowField; CalcFormula = exist("FK Line" where(DocType = field(DocType), DocNo = field("No."))); }
    }
    keys { key(PK; DocType, "No.") { } }
}
codeunit 69962 "FK Probe"
{
    local procedure H(DocType: Option Quote,"Order"; No: Code[20]; D: Date)
    var R: Record "FK Header";
    begin
        R.DocType := DocType; R."No." := No; R.PostDate := D; R.Insert();
    end;

    local procedure L(Id: Integer; DocType: Option Quote,"Order"; No: Code[20]; A: Decimal)
    var R: Record "FK Line";
    begin
        R.Id := Id; R.DocType := DocType; R.DocNo := No; R.Amount := A; R.Insert();
    end;

    local procedure Seed()
    var T: Option Quote,"Order";
    begin
        H(T::Order, 'H4', 20240115D); H(T::Order, 'H2', 20240115D); H(T::Quote, 'Q1', 20240115D); H(T::Order, 'H5', 20240115D);
        H(T::Order, 'H1', 20240115D); H(T::Order, 'H6', 20240115D - 400); H(T::Order, 'H3', 20240115D);
        L(7, T::Order, 'H4', 30); L(3, T::Order, 'H1', 20); L(5, T::Order, 'H2', 50); L(1, T::Order, 'H1', 10);
        L(2, T::Order, 'H5', -5); L(4, T::Order, 'H6', 7); L(6, T::Quote, 'Q1', 100);
    end;

    local procedure Filter(var R: Record "FK Header")
    begin
        R.SetRange(DocType, R.DocType::Order);
        R.SetFilter(PostDate, '>=%1&<=%2', 20240115D - 30, 20240115D);
    end;

    local procedure Walk(var R: Record "FK Header"): Text
    var T: Text;
    begin
        if R.FindSet() then
            repeat
                T += R."No." + ':' + Format(R.Amount) + ' ';
            until R.Next() = 0;
        exit(T);
    end;

    procedure Descending(): Text
    var R: Record "FK Header"; T: Text;
    begin
        Seed();
        Filter(R);
        R.SetCurrentKey(Amount);
        R.SetAscending(Amount, false);
        T := Format(R.Count()) + '|';
        R.FindFirst();
        T += R."No." + ':' + Format(R.Amount) + '|';
        T += Walk(R) + '|' + Format(R.GetAscending(Amount)) + Format(R.Ascending()) + R.CurrentKey() + '|';
        R.FindLast();
        T += R."No.";
        while R.Next(-1) <> 0 do
            T += ' ' + R."No.";
        R.SetAscending(Amount, true);
        T += '|' + Walk(R);
        R.SetAscending(Amount, false);
        R.Ascending(true);
        T += '|' + Format(R.GetAscending(Amount)) + ' ' + Walk(R);
        exit(T);
    end;

    procedure ThenPrimaryKey(): Text
    var R: Record "FK Header";
    begin
        Seed();
        Filter(R);
        R.SetCurrentKey(Amount, "No.");
        R.SetAscending(Amount, false);
        exit(Walk(R) + Format(R.GetAscending("No.")));
    end;

    procedure CountKey(): Text
    var R: Record "FK Header"; T: Text;
    begin
        Seed();
        Filter(R);
        R.SetCurrentKey(Lines);
        R.SetRange("No.", 'H1', 'H2');
        if R.FindSet() then
            repeat
                T += R."No." + ':' + Format(R.Lines) + ' ';
            until R.Next() = 0;
        exit(T);
    end;

    // Find('-') and Find('+') are FindFirst and FindLast; Find('=') is refused.
    procedure FindForms(): Text
    var R: Record "FK Header"; T: Text;
    begin
        Seed();
        Filter(R);
        R.SetCurrentKey(Amount);
        R.Ascending(false);
        if R.Find('-') then
            repeat
                T += R."No." + ' ';
            until R.Next() = 0;
        R.Find('+');
        exit(T + R."No.");
    end;

    procedure FindEqual(): Boolean
    var R: Record "FK Header";
    begin
        Seed();
        R.SetCurrentKey(Amount);
        exit(R.Find('='));
    end;

    // Only a sum and a count are pinned.
    procedure ExistKey(): Boolean
    var R: Record "FK Header";
    begin
        Seed();
        R.SetCurrentKey(Any);
        exit(R.FindFirst());
    end;
}
"#;

#[test]
fn set_ascending_false_on_a_sum_flowfield_key() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "FK Probe", "Descending")),
        "5|H2:50|H2:50 H1:30 H4:30 H3:0 H5:-5 |NoNoAmount|H5 H3 H4 H1 H2|H5:-5 H3:0 H1:30 H4:30 H2:50 |Yes H5:-5 H3:0 H1:30 H4:30 H2:50 "
    );
}

#[test]
fn a_flowfield_key_then_the_primary_key() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "FK Probe", "ThenPrimaryKey")),
        "H2:50 H1:30 H4:30 H3:0 H5:-5 Yes"
    );
}

#[test]
fn a_count_flowfield_key() {
    assert_eq!(text(al_testkit::run_in(SRC, "FK Probe", "CountKey")), "H2:1 H1:2 ");
}

#[test]
fn another_flowfield_kind_stays_refused() {
    let r = al_testkit::run_in(SRC, "FK Probe", "ExistKey");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn find_minus_and_plus_on_a_flowfield_key() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "FK Probe", "FindForms")),
        "H2 H4 H1 H3 H5 H5"
    );
    let r = al_testkit::run_in(SRC, "FK Probe", "FindEqual");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
