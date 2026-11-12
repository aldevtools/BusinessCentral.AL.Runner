//! ADR-040 amendment 2026-11-12b (accepted 2026-11-12): a placeholder's size (`Value::PlaceholderSize`) stored in an Integer,
//! BigInteger or Decimal field of a stored or a temporary row. The value goes with the row and answers as a number variable
//! holding it does (only a comparison with 0); every platform read of the field refuses where it happens. One test per read
//! path the amendment lists (point 4). Rust tests: the corpus has no case that stores BC's size in a field.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
table 50980 "PSF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Big; BigInteger) { }
        field(4; Dec; Decimal) { }
        field(5; Txt; Text[30]) { }
        field(6; Opt; Option) { OptionMembers = A,B,C; }
        field(7; Data; Blob) { }
        field(8; RelId; Integer) { TableRelation = "PSF Other".Id; }
        field(9; HasOther; Boolean) { FieldClass = FlowField; CalcFormula = exist("PSF Other" where(Id = field(Amount))); }
    }
    keys { key(PK; Id) { } }
}

table 50981 "PSF Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50982 "PSF Head"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Total; Integer) { FieldClass = FlowField; CalcFormula = sum("PSF Row".Amount where(Id = field(Id))); }
        field(3; Looked; Integer) { FieldClass = FlowField; CalcFormula = lookup("PSF Row".Amount where(Id = field(Id))); }
        field(4; Highest; Integer) { FieldClass = FlowField; CalcFormula = max("PSF Row".Amount where(Id = field(Id))); }
        field(5; Lowest; Integer) { FieldClass = FlowField; CalcFormula = min("PSF Row".Amount where(Id = field(Id))); }
        field(6; Mean; Integer) { FieldClass = FlowField; CalcFormula = average("PSF Row".Amount where(Id = field(Id))); }
    }
    keys { key(PK; Id) { } }
}

table 50983 "PSF Keyed"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Integer) { } }
    keys { key(PK; Id) { } key(ByAmount; Amount) { } }
}

table 50984 "PSF Copy"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50985 "PSF Copy Text"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Text[30]) { } }
    keys { key(PK; Id) { } }
}

report 50986 "PSF Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "PSF Other") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'PsfWord.docx'; } }
}

report 50987 "PSF Data"
{
    UsageCategory = None;
    dataset { dataitem(R; "PSF Row") { column(Amount; Amount) { } } }
}

xmlport 50988 "PSF Export"
{
    Direction = Export;
    schema { textelement(Root) { tableelement(R; "PSF Row") { fieldelement(Amount; R.Amount) { } } } }
}

page 50989 "PSF Card"
{
    PageType = Card;
    SourceTable = "PSF Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}

page 50991 "PSF Id Card"
{
    PageType = Card;
    SourceTable = "PSF Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
}

codeunit 50990 "PSF Tests"
{
    // The size of an Html placeholder.
    local procedure Size(): Integer
    var Tmp: Record "PSF Row" temporary; O: Record "PSF Other"; Out: OutStream;
    begin
        if O.IsEmpty() then begin O.Id := 99; O.Insert(); end;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PSF Word", '', ReportFormat::Html, Out);
        exit(Tmp.Data.Length());
    end;

    // Row 1 with the size in Amount, stored.
    local procedure Stored(var R: Record "PSF Row")
    begin
        R.DeleteAll();
        R.Init();
        R.Id := 1;
        R.Amount := Size();
        R.Insert();
    end;

    // The shape of the outside reporter's tests: the size stored and never read again.
    procedure StoreOnly(): Integer var R: Record "PSF Row"; begin Stored(R); exit(R.Id); end;
    procedure StoreTemporaryOnly(): Integer
    var R: Record "PSF Row" temporary;
    begin
        R.Id := 1; R.Amount := Size(); R.Insert();
        exit(R.Count());
    end;
    procedure ModifyOnly(): Integer
    var R: Record "PSF Row";
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
        R.Amount := Size(); R.Modify();
        exit(R.Count());
    end;

    // Read back as a number variable holding it answers.
    procedure ReadBack(): Text
    var R: Record "PSF Row"; R2: Record "PSF Row"; N: Integer;
    begin
        Stored(R);
        R2.Get(1);
        N := R2.Amount;
        exit(Format(R2.Amount > 0) + '|' + Format(N > 0) + '|' + Format(R2.Amount <> 0));
    end;
    procedure BigAndDecimal(): Text
    var R: Record "PSF Row";
    begin
        R.DeleteAll(); R.Id := 1; R.Big := Size(); R.Dec := Size(); R.Insert();
        R.Get(1);
        exit(Format(R.Big > 0) + '|' + Format(R.Dec > 0));
    end;
    procedure TemporaryReadBack(): Text
    var R: Record "PSF Row" temporary;
    begin
        R.Id := 1; R.Amount := Size(); R.Insert();
        R.FindFirst();
        exit(Format(R.Amount > 0));
    end;
    procedure RecordAssignAndCopy(): Text
    var R: Record "PSF Row"; R2: Record "PSF Row"; Tmp: Record "PSF Row" temporary;
    begin
        Stored(R);
        R2 := R;
        Tmp := R;
        Tmp.Insert();
        Tmp.Get(1);
        exit(Format(R2.Amount > 0) + '|' + Format(Tmp.Amount > 0));
    end;
    procedure TransferFieldsIntoInteger(): Text
    var R: Record "PSF Row"; C: Record "PSF Copy";
    begin
        Stored(R);
        C.TransferFields(R);
        exit(Format(C.Amount > 0));
    end;

    // A field of another type would read it.
    procedure TransferFieldsIntoText(): Text
    var R: Record "PSF Row"; C: Record "PSF Copy Text";
    begin
        Stored(R);
        C.TransferFields(R);
        exit(C.Amount);
    end;
    procedure IntoTextFieldThroughVariant(): Text
    var R: Record "PSF Row"; V: Variant;
    begin
        V := Size();
        R.Txt := V;
        exit(R.Txt);
    end;
    procedure IntoOptionField(): Integer
    var R: Record "PSF Row";
    begin
        R.Opt := Size();
        exit(R.Opt);
    end;

    // Point 4: every platform read of the field refuses.
    procedure RangeOnField(): Integer var R: Record "PSF Row"; begin Stored(R); R.SetRange(Amount, 1, 1000000); exit(R.Count()); end;
    procedure FilterNotZeroOnField(): Boolean var R: Record "PSF Row"; begin Stored(R); R.SetFilter(Amount, '<>0'); exit(R.FindFirst()); end;
    procedure RangeOnTemporaryField(): Boolean
    var R: Record "PSF Row" temporary;
    begin
        R.Id := 1; R.Amount := Size(); R.Insert();
        R.SetRange(Amount, 0);
        exit(R.IsEmpty());
    end;
    procedure KeyHoldingField(): Integer
    var K: Record "PSF Keyed";
    begin
        K.Id := 1; K.Amount := Size(); K.Insert();
        exit(K.Count());
    end;
    procedure CalcSumsOfField(): Integer var R: Record "PSF Row"; begin Stored(R); R.CalcSums(Amount); exit(R.Count()); end;
    procedure FlowSum(): Boolean var R: Record "PSF Row"; H: Record "PSF Head"; begin Stored(R); H.Id := 1; H.CalcFields(Total); exit(H.Total > 0); end;
    procedure FlowLookup(): Boolean var R: Record "PSF Row"; H: Record "PSF Head"; begin Stored(R); H.Id := 1; H.CalcFields(Looked); exit(H.Looked > 0); end;
    procedure FlowMax(): Boolean var R: Record "PSF Row"; H: Record "PSF Head"; begin Stored(R); H.Id := 1; H.CalcFields(Highest); exit(H.Highest > 0); end;
    procedure FlowMin(): Boolean var R: Record "PSF Row"; H: Record "PSF Head"; begin Stored(R); H.Id := 1; H.CalcFields(Lowest); exit(H.Lowest > 0); end;
    procedure FlowAverage(): Boolean var R: Record "PSF Row"; H: Record "PSF Head"; begin Stored(R); H.Id := 1; H.CalcFields(Mean); exit(H.Mean > 0); end;
    procedure FlowExistWhereField(): Boolean var R: Record "PSF Row"; begin Stored(R); R.CalcFields(HasOther); exit(R.HasOther); end;
    procedure FormatField(): Text var R: Record "PSF Row"; begin Stored(R); exit(Format(R.Amount)); end;
    procedure FormatFieldRef(): Text var R: Record "PSF Row"; RR: RecordRef; begin Stored(R); RR.GetTable(R); exit(Format(RR.Field(2).Value)); end;
    procedure FieldRefIntoText(): Text var R: Record "PSF Row"; RR: RecordRef; T: Text; begin Stored(R); RR.GetTable(R); T := RR.Field(2).Value; exit(T); end;
    procedure FieldRefIntoInteger(): Boolean var R: Record "PSF Row"; RR: RecordRef; N: Integer; begin Stored(R); RR.GetTable(R); N := RR.Field(2).Value; exit(N > 0); end;
    procedure TestFieldNoValue() var R: Record "PSF Row"; begin Stored(R); R.TestField(Amount); end;
    procedure TestFieldWithValue() var R: Record "PSF Row"; begin Stored(R); R.TestField(Amount, 5); end;
    procedure ValidateNoValue() var R: Record "PSF Row"; begin Stored(R); R.Validate(Amount); end;
    procedure ValidateRelation() var R: Record "PSF Row"; begin R.RelId := Size(); R.Validate(RelId); end;
    procedure FieldRefValidateNoValue() var R: Record "PSF Row"; RR: RecordRef; FR: FieldRef; begin Stored(R); RR.GetTable(R); FR := RR.Field(8); FR.Value := R.Amount; FR.Validate(); end;
    procedure FieldErrorOfField() var R: Record "PSF Row"; begin Stored(R); R.FieldError(Amount); end;
    procedure ReportColumn(): Text
    var R: Record "PSF Row"; Out: OutStream; Tmp: Record "PSF Row" temporary;
    begin
        Stored(R);
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PSF Data", '', ReportFormat::Xml, Out);
        exit('saved');
    end;
    procedure XmlPortExport(): Text
    var R: Record "PSF Row"; Out: OutStream; Tmp: Record "PSF Row" temporary;
    begin
        Stored(R);
        Tmp.Data.CreateOutStream(Out);
        Xmlport.Export(Xmlport::"PSF Export", Out);
        exit('exported');
    end;
    procedure JsonOfField(): Text var R: Record "PSF Row"; J: JsonObject; begin Stored(R); J.Add('a', R.Amount); exit('added'); end;
    procedure PageField(): Text
    var R: Record "PSF Row"; P: TestPage "PSF Card";
    begin
        Stored(R);
        P.OpenView();
        P.GoToRecord(R);
        exit(P.Amount.Value());
    end;

    // Record comparison by value: a page compares its row with xRec and the stored row, IsDirty with the row read.
    procedure PageWithoutTheField(): Text
    var R: Record "PSF Row"; P: TestPage "PSF Id Card";
    begin
        Stored(R);
        P.OpenEdit();
        P.GoToRecord(R);
        exit(P.Id.Value());
    end;
    procedure IsDirtyOfRecord(): Boolean var R: Record "PSF Row"; begin Stored(R); R.Get(1); exit(R.IsDirty()); end;
    procedure PageOnOrdinaryRow(): Text
    var R: Record "PSF Row"; P: TestPage "PSF Card";
    begin
        Size();
        R.DeleteAll(); R.Id := 1; R.Amount := 7; R.Insert();
        P.OpenView();
        P.GoToRecord(R);
        exit(P.Amount.Value());
    end;

    // Neighbors that keep main's answers: an ordinary number in the same field, filtered, summed and formatted.
    procedure OrdinaryNumberInField(): Text
    var R: Record "PSF Row"; H: Record "PSF Head";
    begin
        Size();
        R.DeleteAll(); R.Id := 1; R.Amount := 7; R.Insert();
        R.SetRange(Amount, 7);
        R.CalcSums(Amount);
        H.Id := 1; H.CalcFields(Total);
        exit(Format(R.Count()) + '|' + Format(R.Amount) + '|' + Format(H.Total));
    end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(SRC).run_in("PSF Tests", proc)
}

const SIZE: &str = "the byte size of a placeholder";

/// Each read path of `procs` refuses with the size refusal, and not at the store (the store keeps the value; amendment point 1).
fn assert_reads_refused(procs: &[&str]) {
    let wrong: Vec<String> = procs
        .iter()
        .filter_map(|p| match run(p) {
            Err(Signal::Unsupported(why)) if why.contains(SIZE) && !why.contains("storing in a record field") => None,
            other => Some(format!("{p}: {other:?}")),
        })
        .collect();
    assert!(wrong.is_empty(), "not refused as a read of the size:\n{}", wrong.join("\n"));
}

#[test]
fn placeholder_size_stored_in_a_field_and_not_read_runs() {
    assert_eq!(al_testkit::int(run("StoreOnly")), 1);
    assert_eq!(al_testkit::int(run("StoreTemporaryOnly")), 1);
    assert_eq!(al_testkit::int(run("ModifyOnly")), 1);
}

#[test]
fn placeholder_size_read_back_from_a_field_answers_only_against_0() {
    assert_eq!(al_testkit::text(run("ReadBack")), "Yes|Yes|Yes");
    assert_eq!(al_testkit::text(run("BigAndDecimal")), "Yes|Yes");
    assert_eq!(al_testkit::text(run("TemporaryReadBack")), "Yes");
}

#[test]
fn placeholder_size_goes_with_the_row_into_number_fields() {
    assert_eq!(al_testkit::text(run("RecordAssignAndCopy")), "Yes|Yes");
    assert_eq!(al_testkit::text(run("TransferFieldsIntoInteger")), "Yes");
    assert_eq!(al_testkit::boolean(run("FieldRefIntoInteger")), true);
}

#[test]
fn placeholder_size_into_a_field_of_another_type_is_refused() {
    assert_reads_refused(&["IntoOptionField"]);
    // A Variant that holds an Integer into a Text field raises the conversion error of the pinned matrix cell (Variant holding
    // Integer to Text, `conv/rows.rs`), whatever the number: main's answer, kept.
    match run("IntoTextFieldThroughVariant") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "TypeConversion"),
        other => panic!("expected the conversion error, got {other:?}"),
    }
    // TransferFields between fields of different types refuses before it reads any value (main's refusal, kept).
    assert!(matches!(run("TransferFieldsIntoText"), Err(Signal::Unsupported(_))));
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_a_filter_or_a_key() {
    assert_reads_refused(&["RangeOnField", "FilterNotZeroOnField", "RangeOnTemporaryField", "KeyHoldingField", "FlowExistWhereField"]);
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_calcsums_and_flowfields() {
    assert_reads_refused(&["CalcSumsOfField", "FlowSum", "FlowLookup", "FlowMax", "FlowMin", "FlowAverage"]);
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_format_and_fieldref() {
    assert_reads_refused(&["FormatField", "FormatFieldRef", "FieldRefIntoText"]);
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_testfield_validate_and_fielderror() {
    assert_reads_refused(&["TestFieldNoValue", "TestFieldWithValue", "ValidateNoValue", "ValidateRelation", "FieldRefValidateNoValue", "FieldErrorOfField"]);
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_exports_and_pages() {
    assert_reads_refused(&["ReportColumn", "XmlPortExport", "JsonOfField", "PageField"]);
}

#[test]
fn placeholder_size_in_a_field_is_refused_by_a_record_comparison() {
    assert_reads_refused(&["PageWithoutTheField"]);
    // IsDirty refuses a field whose kind it does not compare (`isdirty.rs`, main's refusal, kept).
    assert!(matches!(run("IsDirtyOfRecord"), Err(Signal::Unsupported(_))));
}

#[test]
fn placeholder_size_leaves_an_ordinary_number_in_the_same_field() {
    assert_eq!(al_testkit::text(run("OrdinaryNumberInField")), "1|7|7");
    assert_eq!(al_testkit::text(run("PageOnOrdinaryRow")), "7");
}
