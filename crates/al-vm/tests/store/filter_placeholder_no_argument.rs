//! SetFilter with a placeholder and no argument for it (al-runner issue 6997): on a Code and a Text field the placeholder is the text
//! `%1`, so `<>%1` keeps the blank row; on a Decimal field BC raises "The variable "%1" was not understood."
//! (record/filters/PlaceholderWithoutArgument.al, staging r6980).

use al_testkit::{error_message, text};

const SRC: &str = r#"
table 69900 Row
{
    fields { field(1; Id; Integer) { } field(2; "No."; Code[20]) { } field(3; Name; Text[30]) { } field(4; Qty; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69900 C
{
    local procedure Seed()
    var R: Record Row;
    begin
        R.Id := 1; R."No." := ''; R.Name := ''; R.Insert();
        R.Id := 2; R."No." := 'A'; R.Name := 'A'; R.Insert();
        R.Id := 3; R."No." := '%1'; R.Name := '%1'; R.Insert();
    end;

    local procedure Ids(var R: Record Row): Text
    var Result: Text;
    begin
        if not R.FindSet() then exit('none');
        repeat Result += Format(R.Id); until R.Next() = 0;
        exit(Result);
    end;

    procedure CodeAndText(): Text
    var R: Record Row; Out: Text;
    begin
        Seed();
        R.SetFilter("No.", '<>%1');
        Out := R.GetFilters() + ' ' + Ids(R);
        R.SetFilter("No.", '%1..%2');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        R.SetFilter("No.", '<>%1&<>A');
        Out += ' | ' + Ids(R);
        R.SetFilter("No.", '%2');
        Out += ' | ' + Ids(R);
        R.Reset();
        R.SetFilter(Name, '>%1');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        exit(Out);
    end;

    procedure DecimalField()
    var R: Record Row;
    begin
        R.SetFilter(Qty, '<>%1');
    end;
}
"#;

#[test]
fn a_placeholder_without_an_argument_is_the_text_on_code_and_text() {
    assert_eq!(
        text(al_testkit::run(SRC, "CodeAndText")),
        "No.: <>%1 12 | No.: %1..%2 3 | 1 | none | Name: >%1 2"
    );
}

#[test]
fn a_placeholder_without_an_argument_raises_on_a_decimal() {
    assert_eq!(
        error_message(al_testkit::run(SRC, "DecimalField")),
        "The filter \"<>%1\" is not valid for the Qty field on the Row table. The variable \"%1\" was not understood."
    );
}
