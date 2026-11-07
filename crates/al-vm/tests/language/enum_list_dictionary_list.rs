//! The lists of Enum.Ordinals and Enum.Names, and Contains on a list of dictionaries
//! (collections/CsCollections.al).

use al_testkit::{run_in, text};

const SRC: &str = r#"
enum 50330 Color { value(0; Red) { } value(1; Green) { } value(5; Blue) { } }

codeunit 50331 T
{
    procedure Ordinals(): Text
    var
        L: List of [Integer];
        R: Text;
    begin
        L := Enum::Color.Ordinals();
        asserterror L.Add(9);
        R := GetLastErrorText() + '|' + GetLastErrorCode();
        asserterror L.Reverse();
        R += '|' + GetLastErrorCode();
        asserterror L.Set(1, 9);
        R += '|' + GetLastErrorCode();
        exit(R + '|' + Format(L.Count()) + '|' + Format(L.Get(3)));
    end;

    procedure Names(): Text
    var
        L: List of [Text];
    begin
        L := Enum::Color.Names();
        asserterror L.Insert(1, 'x');
        exit(GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryContains(var L: List of [Dictionary of [Integer, Code[20]]]; D: Dictionary of [Integer, Code[20]]; var R: Boolean)
    begin
        R := L.Contains(D);
    end;

    procedure Dictionaries(): Text
    var
        L: List of [Dictionary of [Integer, Code[20]]];
        D: Dictionary of [Integer, Code[20]];
        D2: Dictionary of [Integer, Code[20]];
        R: Boolean;
        Out: Text;
    begin
        D.Add(1, 'A');
        D2.Add(2, 'B');
        L.Add(D);
        L.Add(D2);
        if TryContains(L, D, R) then
            Out := Format(R);
        if not TryContains(L, D2, R) then
            Out += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Out + '|' + Format(L.IndexOf(D)));
    end;
}
"#;

#[test]
fn the_lists_of_enum_ordinals_and_names_cannot_be_changed() {
    assert_eq!(
        text(run_in(SRC, "T", "Ordinals")),
        "The requested operation is not supported.|NotSupportedOperation|NotSupportedOperation|NotSupportedOperation|3|5"
    );
    assert_eq!(text(run_in(SRC, "T", "Names")), "NotSupportedOperation");
}

#[test]
fn a_list_of_dictionaries_finds_the_same_dictionary_and_cannot_compare_another() {
    assert_eq!(
        text(run_in(SRC, "T", "Dictionaries")),
        "Yes|Unable to compare operands of type NavDictionary`2 with NavDictionary`2.|InvalidComparison|1"
    );
}
