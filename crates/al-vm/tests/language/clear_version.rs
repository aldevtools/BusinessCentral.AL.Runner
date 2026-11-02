//! `Clear` of a Version variable gives `0.0.0.0` (types/lt-y64-clear-types/Y64ClearTypes.al:112,124,137 Y64_Clear_Scalars;
//! al-runner issue 6656, full surface of Clear).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69710 C
{
    procedure F(): Text
    var V: Version;
    begin
        V := Version.Create(1, 2, 3, 4);
        Clear(V);
        exit(Format(V));
    end;
}
"#;

#[test]
fn clearing_a_version_gives_zero() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "0.0.0.0");
}
