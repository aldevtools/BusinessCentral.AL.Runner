//! ADR-040 amendment 2026-11-12 (A4): the size of a placeholder report output (`Value::PlaceholderSize`) answers only a
//! comparison with 0. A builtin or a .NET member given it as an argument or receiver would compare it with its own code (a
//! record filter, a List or Dictionary) or read its number, so every such call is refused before it runs
//! (`al_builtins::placeholder_output::size_guard`). Rust tests: the corpus has no case that hands BC's size to a builtin.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
dotnet { assembly("mscorlib") { type("System.String"; DnString) { } } }

table 50990 "RPS Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

report 50991 "RPS Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RPS Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'RpsWord.docx'; } }
}

codeunit 50992 "RPS Tests"
{
    // Rows 50, 150 and 5000, and the size of an Html placeholder.
    local procedure Size(): Integer
    var R: Record "RPS Row"; Tmp: Record "RPS Row" temporary; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 50; R.Insert();
        R.Id := 150; R.Insert();
        R.Id := 5000; R.Insert();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPS Word", '', ReportFormat::Html, Out);
        exit(Tmp.Data.Length());
    end;

    // As System Application's Temp Blob.Length() returns Blob.Length through a procedure.
    local procedure Wrapped(): Integer begin exit(Size()); end;

    procedure FilterNotEqual(): Integer var R: Record "RPS Row"; N: Integer; begin N := Size(); R.SetFilter(Id, '<>%1', N); exit(R.Count()); end;
    procedure FilterGreater(): Integer var R: Record "RPS Row"; N: Integer; begin N := Size(); R.SetFilter(Id, '>%1', N); exit(R.Count()); end;
    procedure FilterLess(): Integer var R: Record "RPS Row"; N: Integer; begin N := Size(); R.SetFilter(Id, '<%1', N); exit(R.Count()); end;
    procedure RangeZeroToSize(): Integer var R: Record "RPS Row"; N: Integer; begin N := Size(); R.SetRange(Id, 0, N); exit(R.Count()); end;
    procedure RangeSizeFindFirst(): Boolean var R: Record "RPS Row"; N: Integer; begin N := Size(); R.SetRange(Id, N); exit(R.FindFirst()); end;
    procedure RangeSizeIsEmpty(): Boolean var R: Record "RPS Row"; begin R.SetRange(Id, Wrapped()); exit(R.IsEmpty()); end;
    procedure FieldRefSetRange(): Integer
    var R: Record "RPS Row"; RR: RecordRef; FR: FieldRef; N: Integer;
    begin
        N := Size();
        RR.Open(Database::"RPS Row");
        FR := RR.Field(1);
        FR.SetRange(N);
        exit(RR.Count());
    end;
    procedure FieldRefSetFilter(): Integer
    var RR: RecordRef; FR: FieldRef; N: Integer;
    begin
        N := Size();
        RR.Open(Database::"RPS Row");
        FR := RR.Field(1);
        FR.SetFilter('<>%1', N);
        exit(RR.Count());
    end;
    procedure DeleteAllBelowSize(): Integer
    var R: Record "RPS Row"; N: Integer;
    begin
        N := Size();
        R.SetFilter(Id, '<%1', N);
        R.DeleteAll();
        R.Reset();
        exit(R.Count());
    end;
    procedure GetBySize(): Boolean var R: Record "RPS Row"; begin exit(R.Get(Size())); end;
    procedure ValidateSize() var R: Record "RPS Row"; N: Integer; begin N := Size(); R.Validate(Id, N); end;

    procedure ListHoldsSize(): Boolean var L: List of [Integer]; begin L.Add(Size()); exit(L.Contains(93)); end;
    procedure ListContainsSize(): Boolean var L: List of [Integer]; N: Integer; begin N := Size(); L.Add(93); exit(L.Contains(N)); end;
    procedure ListIndexOfSize(): Integer var L: List of [Integer]; N: Integer; begin N := Size(); L.Add(93); exit(L.IndexOf(N)); end;
    procedure ListRemoveSize(): Boolean var L: List of [Integer]; N: Integer; begin N := Size(); L.Add(93); exit(L.Remove(N)); end;
    procedure ListGetBySize(): Text var L: List of [Text]; N: Integer; begin N := Size(); L.Add('a'); exit(L.Get(N)); end;
    procedure DictKeySize(): Boolean
    var D: Dictionary of [Integer, Text]; N: Integer;
    begin
        N := Size();
        D.Add(N, 'size');
        D.Add(93, 'other');
        exit(D.ContainsKey(93));
    end;
    procedure DictContainsKeySize(): Boolean var D: Dictionary of [Integer, Text]; N: Integer; begin N := Size(); D.Add(93, 'a'); exit(D.ContainsKey(N)); end;
    procedure DictValueSize(): Boolean var D: Dictionary of [Integer, Integer]; N: Integer; begin N := Size(); D.Add(1, N); exit(D.ContainsKey(1)); end;

    procedure FileLenInFilter(): Integer
    var R: Record "RPS Row"; F: File; Len: Integer;
    begin
        Size();
        Report.SaveAsHtml(Report::"RPS Word", 'out/rps.html');
        F.Open('out/rps.html');
        Len := F.Len();
        F.Close();
        R.SetRange(Id, Len);
        exit(R.Count());
    end;
    procedure DotNetArgument(): Text var S: DotNet DnString; N: Integer; begin N := Size(); S := 'abcdef'; exit(S.Substring(N)); end;

    // Neighbors that keep their answers: an ordinary number in the same filter and List after a size was made, the size against
    // 0, and the type of a Variant that holds it (Library Assert's AreNotEqual asks it).
    procedure OrdinaryNumbersStillAnswer(): Text
    var R: Record "RPS Row"; L: List of [Integer]; N: Integer; V: Variant;
    begin
        N := Size();
        R.SetFilter(Id, '<>%1', 150);
        L.Add(93);
        V := N;
        exit(Format(R.Count()) + '|' + Format(L.Contains(93)) + '|' + Format(N > 0) + '|' + Format(V.IsInteger()));
    end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(SRC).run_in("RPS Tests", proc)
}

const SIZE: &str = "the byte size of a placeholder";

#[test]
fn report_placeholder_size_is_refused_as_a_filter_value() {
    for p in [
        "FilterNotEqual",
        "FilterGreater",
        "FilterLess",
        "RangeZeroToSize",
        "RangeSizeFindFirst",
        "RangeSizeIsEmpty",
        "FieldRefSetRange",
        "FieldRefSetFilter",
        "DeleteAllBelowSize",
        "GetBySize",
        "ValidateSize",
        "FileLenInFilter",
    ] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(SIZE), "{p}: {why}"),
            other => panic!("{p}: expected the size to be refused, got {other:?}"),
        }
    }
}

#[test]
fn report_placeholder_size_is_refused_as_a_list_or_dictionary_element_key_or_index() {
    for p in [
        "ListHoldsSize",
        "ListContainsSize",
        "ListIndexOfSize",
        "ListRemoveSize",
        "ListGetBySize",
        "DictKeySize",
        "DictContainsKeySize",
        "DictValueSize",
    ] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(SIZE), "{p}: {why}"),
            other => panic!("{p}: expected the size to be refused, got {other:?}"),
        }
    }
}

#[test]
fn report_placeholder_size_is_refused_as_a_dotnet_argument() {
    match run("DotNetArgument") {
        Err(Signal::Unsupported(why)) => assert!(why.contains(SIZE), "{why}"),
        other => panic!("expected the size to be refused, got {other:?}"),
    }
}

#[test]
fn report_placeholder_size_leaves_ordinary_numbers_and_the_comparison_with_0() {
    assert_eq!(al_testkit::text(run("OrdinaryNumbersStillAnswer")), "2|Yes|Yes|Yes");
}
