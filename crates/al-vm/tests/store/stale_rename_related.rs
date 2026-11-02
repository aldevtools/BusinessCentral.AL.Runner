//! A stale buffer from the variable's own Insert and Modify, when the new transaction renames another row of a table that
//! other tables relate to (nothing references the renamed row): written whole at every Commit count
//! (record/lt-sb-stale-own-modify/SbStaleOwnModify.al TE_1C_RenameOtherRow_StaleModify, TE_2C_, TE_3C_, Y2_4C_ to Y2_12C_;
//! ALT Universal is the target of fields of other tables there). The rename rewrites no row of the relating table, so it is
//! one write of the renamed row, as for a table nobody relates to.

use al_testkit::text;

const SRC: &str = r#"
table 50170 "Rel Row" { fields { field(1; "No."; Code[20]) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } } }
table 50171 "Rel Middle" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 50172 "Rel Pointer" { fields { field(1; "No."; Code[20]) { }
    field(2; Target; Code[20]) { TableRelation = "Rel Row"."No."; } } keys { key(PK; "No.") { } } }
codeunit 50170 C
{
    procedure RenameMany(Commits: Integer; WithPointer: Boolean): Text
    var Row: Record "Rel Row"; Other: Record "Rel Row"; Another: Record "Rel Row"; Stored: Record "Rel Row";
        Middle: Record "Rel Middle"; Pointer: Record "Rel Pointer"; i: Integer;
    begin
        Commit();
        Row."No." := 'N2'; Row.Note := 'start'; Row.Insert(true); Row.Qty := 1; Row.Modify(true);
        Another."No." := 'N2-OTHER'; Another.Insert(true);
        if WithPointer then begin Pointer."No." := 'P1'; Pointer.Target := 'N2-OTHER'; Pointer.Insert(false); end;
        Other.Get('N2'); Other.Note := 'other'; Other.Modify(false);
        Commit();
        for i := 2 to Commits do begin
            Middle."No." := 'MIDDLE-' + Format(i); Middle.Insert(true);
            Commit();
        end;
        Another.Get('N2-OTHER'); Another.Rename('N2-RENAMED');
        Row.Qty := 5; Row.Modify(true);
        Stored.Get('N2');
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;
}
"#;

#[test]
fn a_rename_of_another_row_of_a_related_table_lets_the_stale_modify_write() {
    for n in [1, 2, 3, 4, 5, 10, 12] {
        let src = SRC.replacen(
            "codeunit 50170 C\n{",
            &format!("codeunit 50170 C\n{{\n    procedure Probe(): Text begin exit(RenameMany({n}, false)); end;"),
            1,
        );
        assert_eq!(text(al_testkit::run_in(&src, "C", "Probe")), "start|5", "{n}");
    }
}

#[test]
fn a_rename_that_rewrites_a_relating_row_is_still_refused() {
    let src = SRC.replacen(
        "codeunit 50170 C\n{",
        "codeunit 50170 C\n{\n    procedure Probe(): Text begin exit(RenameMany(2, true)); end;",
        1,
    );
    let r = al_testkit::run_in(&src, "C", "Probe");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
