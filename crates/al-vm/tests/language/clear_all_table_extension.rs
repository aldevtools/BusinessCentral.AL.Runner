//! ClearAll in a table extension resets the extension's globals and the base table's, and keeps the record's fields and
//! filters; ClearAll in the base table resets the extension's globals too (al-runner issue 6967;
//! pages/extensions/CyClearAllExtensions.al:137-162, record/ProbeClearAllSetKeyHyperlink.al:358-385).

use al_testkit::text;

const SRC: &str = r#"
table 69990 "CA Buffer"
{
    fields { field(1; "Line No."; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Line No.") { } }
    var
        BaseFlag: Text;
    procedure SetBase(V: Text) begin BaseFlag := V; end;
    procedure GetBase(): Text begin exit(BaseFlag); end;
    procedure ClearInBase() begin ClearAll(); end;
}
tableextension 69991 "CA Buffer Ext" extends "CA Buffer"
{
    fields { field(10; ExtNo; Integer) { } }
    var
        RootPath: Text;
    procedure SetRoot(V: Text) begin RootPath := V; end;
    procedure GetRoot(): Text begin exit(RootPath); end;
    procedure Load(): Text
    var Before: Text;
    begin
        Before := RootPath;
        ClearAll();
        exit(Before + '|' + RootPath + '|' + Name + '|' + Format(ExtNo) + '|' + GetBase());
    end;
}
codeunit 69992 C
{
    procedure InExt(): Text
    var B: Record "CA Buffer" temporary;
    begin
        B.Name := 'kept';
        B.ExtNo := 4;
        B.SetRange("Line No.", 3);
        B.SetBase('base');
        B.SetRoot('root');
        exit(B.Load() + '|' + B.GetFilters() + '|' + B.GetRoot() + '|' + B.GetBase());
    end;

    procedure InBase(): Text
    var B: Record "CA Buffer";
    begin
        B.Name := 'kept';
        B.SetBase('base');
        B.SetRoot('root');
        B.ClearInBase();
        exit('[' + B.GetBase() + '|' + B.GetRoot() + '|' + B.Name + ']');
    end;
}
"#;

#[test]
fn clear_all_in_a_table_extension_resets_its_globals_and_the_base_tables_and_keeps_the_record() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "InExt")),
        "root||kept|4||Line No.: 3||"
    );
}

#[test]
fn clear_all_in_the_base_table_resets_the_extensions_globals() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "InBase")), "[||kept]");
}
