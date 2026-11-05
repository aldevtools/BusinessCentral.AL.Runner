//! Init on a record that a SetLoadFields fetch filled partially, then a new key, Validate and Insert (al-runner issue 7000). Measured
//! on BC 29 by corpus branch al-runtime-rs-staging-r6961, record/load-fields/R7000InitAfterLoadFields.al: after Init the fields the
//! fetch did not load hold their initial values, never the fetched row's, read directly or in OnValidate and OnInsert, with the key
//! kept or changed, with or without SetLoadFields() before Init. AreFieldsLoaded after Init: No for such a field (also after a
//! read), Yes for the key.

use al_testkit::text;

const SRC: &str = r#"
table 50861 "IL Line"
{
    fields
    {
        field(1; "Document Type"; Option) { OptionMembers = Quote,"Order"; }
        field(3; "Document No."; Code[20]) { }
        field(4; "Line No."; Integer) { }
        field(5; Type; Option)
        {
            OptionMembers = " ",Item,Resource;
            trigger OnValidate()
            var Log: Record "IL Log";
            begin
                Log.Add('Type validate: No=' + "No." + ' Descr=' + Description + ' Qty=' + Format(Quantity) + ' Note=' + Note);
                "No." := '';
            end;
        }
        field(6; "No."; Code[20])
        {
            trigger OnValidate()
            var Log: Record "IL Log";
            begin
                Log.Add('No validate: Descr=' + Description + ' Qty=' + Format(Quantity) + ' Amount=' + Format(Amount) + ' Note=' + Note);
                Description := 'Item ' + "No.";
                Amount := Quantity * 10;
            end;
        }
        field(7; Description; Text[50]) { }
        field(8; Quantity; Decimal) { InitValue = 1; }
        field(9; Amount; Decimal) { }
        field(10; Note; Text[30]) { }
    }
    keys { key(PK; "Document Type", "Document No.", "Line No.") { Clustered = true; } }
    trigger OnInsert()
    var Log: Record "IL Log";
    begin
        Log.Add('insert: ' + Dump());
    end;
    procedure Dump(): Text
    begin
        exit(Format("Document Type") + '/' + "Document No." + '/' + Format("Line No.") + ' Type=' + Format(Type) + ' No=' + "No." +
            ' Descr=' + Description + ' Qty=' + Format(Quantity) + ' Amount=' + Format(Amount) + ' Note=' + Note);
    end;
}
table 50862 "IL Log"
{
    fields { field(1; Seq; Integer) { AutoIncrement = true; } field(2; Msg; Text[250]) { } }
    keys { key(PK; Seq) { Clustered = true; } }
    procedure Add(T: Text)
    var L: Record "IL Log";
    begin
        L.Init(); L.Seq := 0; L.Msg := CopyStr(T, 1, 250); L.Insert();
    end;
    procedure Dump(): Text
    var L: Record "IL Log"; R: Text;
    begin
        if L.FindSet() then repeat R += L.Msg + ';'; until L.Next() = 0;
        exit(R);
    end;
}
codeunit 50863 C
{
    local procedure AddLine(LineNo: Integer; Descr: Text[50]; Qty: Decimal; Note: Text[30])
    var L: Record "IL Line";
    begin
        L."Document Type" := L."Document Type"::"Order"; L."Document No." := 'D1'; L."Line No." := LineNo;
        L.Type := L.Type::Resource; L."No." := 'R1'; L.Description := Descr; L.Quantity := Qty; L.Amount := Qty * 10; L.Note := Note;
        L.Insert();
    end;

    local procedure Rows(): Text
    var L: Record "IL Line"; R: Text;
    begin
        if L.FindSet() then repeat R += L.Dump() + ';'; until L.Next() = 0;
        exit(R);
    end;

    local procedure FetchLast(var L: Record "IL Line"): Integer
    begin
        AddLine(10000, 'old10', 5, 'n10');
        AddLine(20000, 'old20', 7, 'n20');
        L.SetLoadFields("Line No.");
        L.SetRange("Document Type", L."Document Type"::"Order");
        L.SetRange("Document No.", 'D1');
        if L.FindLast() then exit(L."Line No.");
    end;

    procedure IssuePattern(): Text
    var L: Record "IL Line"; Log: Record "IL Log"; LineNo: Integer; R: Text;
    begin
        LineNo := FetchLast(L);
        L.SetLoadFields();
        L.Init();
        R := L.Dump() + '|';
        L."Line No." := LineNo + 10000;
        L.Validate(Type, L.Type::Item);
        L.Validate("No.", 'I1');
        L.Insert(true);
        exit(R + Log.Dump() + '|' + Rows());
    end;

    procedure StillPartial(): Text
    var L: Record "IL Line"; Log: Record "IL Log"; LineNo: Integer;
    begin
        LineNo := FetchLast(L);
        L.Init();
        L."Line No." := LineNo + 10000;
        L.Validate(Type, L.Type::Item);
        L.Validate("No.", 'I1');
        L.Insert(true);
        exit(Log.Dump() + '|' + Rows());
    end;

    procedure KeyKept(): Text
    var L: Record "IL Line"; R: Text;
    begin
        FetchLast(L);
        L.Init();
        R := Format(L.AreFieldsLoaded(Note)) + Format(L.AreFieldsLoaded("Line No.")) + ' ' + L.Dump() + ' ';
        exit(R + Format(L.AreFieldsLoaded(Note)));
    end;

    procedure PlainInsert(): Text
    var L: Record "IL Line"; Log: Record "IL Log";
    begin
        FetchLast(L);
        L.Init();
        L."Line No." := 30000;
        L.Insert();
        exit(Log.Dump() + '|' + Rows());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

const OLD: &str = "Order/D1/10000 Type=Resource No=R1 Descr=old10 Qty=5 Amount=50 Note=n10;\
Order/D1/20000 Type=Resource No=R1 Descr=old20 Qty=7 Amount=70 Note=n20;";

// R7000InitAfterLoadFields.al Init_AfterLoadFieldsReset_NewKey_Validate_InsertTrue and Init_StillPartial_NewKey_Validate_InsertTrue.
#[test]
fn init_after_a_partial_fetch_then_a_new_key_validate_and_insert_uses_initial_values() {
    let log = "Type validate: No= Descr= Qty=1 Note=;No validate: Descr= Qty=1 Amount=0 Note=;\
insert: Order/D1/30000 Type=Item No=I1 Descr=Item I1 Qty=1 Amount=10 Note=;";
    let new = "Order/D1/30000 Type=Item No=I1 Descr=Item I1 Qty=1 Amount=10 Note=;";
    assert_eq!(
        run("IssuePattern"),
        format!("Order/D1/20000 Type=  No= Descr= Qty=1 Amount=0 Note=|{log}|{OLD}{new}")
    );
    assert_eq!(run("StillPartial"), format!("{log}|{OLD}{new}"));
}

// Init_AfterLoadFields_KeyKept_Reads, Init_AfterLoadFields_NewKey_PlainInsert.
#[test]
fn init_after_a_partial_fetch_reads_initial_values_and_reports_them_not_loaded() {
    assert_eq!(
        run("KeyKept"),
        "NoYes Order/D1/20000 Type=  No= Descr= Qty=1 Amount=0 Note= No"
    );
    assert_eq!(
        run("PlainInsert"),
        format!("|{OLD}Order/D1/30000 Type=  No= Descr= Qty=1 Amount=0 Note=;")
    );
}
