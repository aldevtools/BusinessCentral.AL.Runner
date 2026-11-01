//! AL source to the real .NET sidecar, through the whole runtime path (lexer, sema, lowering,
//! VM, marshalling, process, reflection). Own fixtures: an OnPrem-style `dotnet { }` block with
//! the DotNet probe app's calls (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, ADR-024). Needs .NET 8; without it a
//! test skips visibly, and `ALRT_REQUIRE_DOTNET=1` turns a skip into a failure.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_host::dotnet::DotNetHost;
use al_sema::workspace::AppSource;
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use al_vm::dotnet::DotNetConfig;
use al_vm::{Engine, Session};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SRC: &str = r#"
dotnet
{
    assembly("mscorlib")
    {
        type("System.Convert"; DnConvert) { }
        type("System.Globalization.NumberStyles"; DnNumberStyles) { }
        type("System.Text.StringBuilder"; DnStringBuilder) { }
        type("System.Type"; DnType) { }
        type("System.DateTimeOffset"; DnDateTimeOffset) { }
        type("System.DateTime"; DnDateTime) { }
        type("System.IO.Path"; DnPath) { }
        type("System.TimeSpan"; DnTimeSpan) { }
        type("System.IO.MemoryStream"; DnMemoryStream) { }
        type("System.String[]"; DnStringArray) { }
        type("System.Collections.ArrayList"; DnArrayList) { }
        type("System.Collections.Queue"; DnQueue) { }
    }
    assembly("System.Text.RegularExpressions")
    {
        type("System.Text.RegularExpressions.Regex"; DnRegex) { }
    }
    assembly("System")
    {
        type("System.Uri"; DnUri) { }
    }
}

table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}

codeunit 50100 C
{
    local procedure Fill(var Blob: Record "ALT Blob"; Text: Text)
    var
        O: OutStream;
    begin
        Blob.Data.CreateOutStream(O);
        O.WriteText(Text);
    end;

    local procedure ReadBlob(var Blob: Record "ALT Blob"): Text
    var
        I: InStream;
        Result: Text;
    begin
        Blob.Data.CreateInStream(I);
        I.ReadText(Result);
        exit(Result);
    end;

    // CopyStream_WithAMemoryStream (probe codeunit 61400)
    procedure CopyIntoMemory(): Text
    var
        Memory: DotNet DnMemoryStream;
        Source: Record "ALT Blob" temporary;
        I: InStream;
    begin
        Fill(Source, 'hello world');
        Source.Data.CreateInStream(I);
        Memory := Memory.MemoryStream();
        CopyStream(Memory, I);
        exit(Format(Memory.Length) + ':' + Format(Memory.Position) + ':' + Format(I.EOS()));
    end;

    procedure CopyIntoMemoryAfterOneByteRead(): Text
    var
        Memory: DotNet DnMemoryStream;
        Source: Record "ALT Blob" temporary;
        I: InStream;
        First: Byte;
    begin
        Fill(Source, 'hello world');
        Source.Data.CreateInStream(I);
        I.Read(First);
        Memory := Memory.MemoryStream();
        CopyStream(Memory, I);
        exit(Format(Memory.Length));
    end;

    local procedure LoadedMemory(var Memory: DotNet DnMemoryStream)
    var
        Source: Record "ALT Blob" temporary;
        I: InStream;
    begin
        Fill(Source, 'hello world');
        Source.Data.CreateInStream(I);
        Memory := Memory.MemoryStream();
        CopyStream(Memory, I);
    end;

    procedure CopyOutWholeFromPositionSix(): Text
    var
        Memory: DotNet DnMemoryStream;
        Target: Record "ALT Blob" temporary;
        O: OutStream;
    begin
        LoadedMemory(Memory);
        Memory.Position := 6;
        Target.Data.CreateOutStream(O);
        CopyStream(O, Memory);
        exit(ReadBlob(Target) + '|' + Format(Memory.Position));
    end;

    procedure CopyOutWithACount(): Text
    var
        Memory: DotNet DnMemoryStream;
        Target: Record "ALT Blob" temporary;
        O: OutStream;
    begin
        LoadedMemory(Memory);
        Memory.Position := 0;
        Target.Data.CreateOutStream(O);
        CopyStream(O, Memory, 5);
        exit(ReadBlob(Target) + '|' + Format(Memory.Position));
    end;

    procedure CopyIntoMemoryWithACountIsRefused(): Text
    var
        Memory: DotNet DnMemoryStream;
        Source: Record "ALT Blob" temporary;
        I: InStream;
    begin
        Fill(Source, 'hello world');
        Source.Data.CreateInStream(I);
        Memory := Memory.MemoryStream();
        CopyStream(Memory, I, 3);
        exit('copied');
    end;

    // tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al (codeunit 61400), IntegerProperties_...
    procedure LengthMinusOne(): Integer
    var
        Builder: DotNet DnStringBuilder;
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('abcd');
        exit(Builder.Length - 1);
    end;

    procedure ForBound(): Integer
    var
        Builder: DotNet DnStringBuilder;
        Index: Integer;
        Total: Integer;
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('abcd');
        for Index := 1 to Builder.Length - 1 do
            Total += 1;
        exit(Total);
    end;

    procedure HalfLength(): Integer
    var
        Builder: DotNet DnStringBuilder;
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('abcd');
        exit(Builder.Length div 2);
    end;

    procedure SchemeIsUpper(): Boolean
    var
        Address: DotNet DnUri;
    begin
        Address := Address.Uri('https://example.com/x');
        exit(Address.Scheme = 'HTTPS');
    end;

    procedure SchemeIsLower(): Boolean
    var
        Address: DotNet DnUri;
    begin
        Address := Address.Uri('https://example.com/x');
        exit(Address.Scheme = 'https');
    end;

    // NullStringResult_IsEmptyInExpressions
    procedure NullStringEqualsEmpty(): Boolean
    var
        Path: DotNet DnPath;
    begin
        exit(Path.GetDirectoryName('/') = '');
    end;

    procedure NullStringPlusX(): Text
    var
        Path: DotNet DnPath;
        Result: Text;
    begin
        Result := Path.GetDirectoryName('/') + 'x';
        exit(Result);
    end;

    procedure NullStringFormat(): Text
    var
        Path: DotNet DnPath;
    begin
        exit(Format(Path.GetDirectoryName('/')));
    end;

    procedure NullStringAssignedToText(): Integer
    var
        Path: DotNet DnPath;
        Result: Text;
    begin
        Result := Path.GetDirectoryName('/');
        exit(StrLen(Result));
    end;

    procedure StrLenOfNullString(): Integer
    var
        Path: DotNet DnPath;
    begin
        exit(StrLen(Path.GetDirectoryName('/')));
    end;

    // FormatOfDotNetObjects_IsToString
    procedure FormatOfType(): Text
    var
        Builder: DotNet DnStringBuilder;
        Kind: DotNet DnType;
    begin
        Builder := Builder.StringBuilder();
        Kind := Builder.GetType();
        exit(Format(Kind));
    end;

    procedure FormatOfEnum(): Text
    var
        Styles: DotNet DnNumberStyles;
    begin
        Styles := Styles.Number;
        exit(Format(Styles, 0, 2));
    end;

    procedure FormatOfBuilder(): Text
    var
        Builder: DotNet DnStringBuilder;
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('hello');
        exit(Format(Builder));
    end;

    procedure FormatOfNullObject(): Text
    var
        Nothing: DotNet DnStringBuilder;
    begin
        exit(Format(Nothing));
    end;

    // EnumOperators
    procedure EnumSum(): Text
    var
        Number: DotNet DnNumberStyles;
        Other: DotNet DnNumberStyles;
    begin
        Number := Number.Number;
        Other := Other.AllowCurrencySymbol;
        exit(Format(Number + Other));
    end;

    procedure EnumEquality(): Text
    var
        Number: DotNet DnNumberStyles;
        Also: DotNet DnNumberStyles;
        Other: DotNet DnNumberStyles;
    begin
        Number := Number.Number;
        Also := Also.Number;
        Other := Other.AllowCurrencySymbol;
        exit(Format(Number = Also) + Format(Number = Other) + Format(Number <> Other));
    end;

    procedure ObjectAsOperand(): Integer
    var
        Builder: DotNet DnStringBuilder;
    begin
        Builder := Builder.StringBuilder();
        exit(Builder + 1);
    end;

    // The host clock: the same one CurrentDateTime reads.
    procedure OffsetNowYear(): Integer
    var
        Stamp: DotNet DnDateTimeOffset;
    begin
        Stamp := Stamp.Now;
        exit(Stamp.Year);
    end;

    procedure OffsetNowUtcHour(): Integer
    var
        Stamp: DotNet DnDateTimeOffset;
    begin
        Stamp := Stamp.UtcNow;
        exit(Stamp.Hour);
    end;

    procedure OffsetNowMillisecond(): Integer
    var
        Stamp: DotNet DnDateTimeOffset;
    begin
        Stamp := Stamp.Now;
        exit(Stamp.Millisecond);
    end;

    procedure OffsetIsZero(): Boolean
    var
        Stamp: DotNet DnDateTimeOffset;
        Zone: Duration;
    begin
        Stamp := Stamp.Now;
        Zone := Stamp.Offset;
        exit(Zone = 0);
    end;

    procedure LocalMinusOffsetEqualsClock(): Boolean
    var
        Stamp: DotNet DnDateTimeOffset;
        Shifted: DateTime;
    begin
        Stamp := Stamp.Now;
        Shifted := Stamp.LocalDateTime - Stamp.Offset;
        exit(CurrentDateTime() - Shifted = 1);
    end;

    procedure DateTimeNowIsTheClock(): Boolean
    var
        When: DotNet DnDateTime;
        Value: DateTime;
    begin
        Value := When.Now;
        exit(CurrentDateTime() - Value = 1);
    end;

    procedure DateTimeTodayIsToday(): Boolean
    var
        When: DotNet DnDateTime;
        Value: DateTime;
    begin
        Value := When.Today;
        exit(DT2Date(Value) = Today());
    end;

    // Foreach_OverClrCollections (probe codeunit 61400)
    procedure ForeachStrings(): Text
    var
        Regex: DotNet DnRegex;
        Parts: DotNet DnStringArray;
        Piece: Text;
        Total: Text;
    begin
        Parts := Regex.Split('a,b,c', ',');
        foreach Piece in Parts do
            Total += Piece + '|';
        exit(Total);
    end;

    procedure ForeachInts(): Text
    var
        Numbers: DotNet DnArrayList;
        Number: Integer;
        Total: Text;
    begin
        Numbers := Numbers.ArrayList();
        Numbers.Add(1);
        Numbers.Add(2);
        Numbers.Add(3);
        foreach Number in Numbers do
            Total += Format(Number) + '|';
        exit(Total);
    end;

    procedure ForeachVariant(): Text
    var
        Numbers: DotNet DnArrayList;
        Anything: Variant;
        Total: Text;
    begin
        Numbers := Numbers.ArrayList();
        Numbers.Add(1);
        Numbers.Add(2);
        foreach Anything in Numbers do
            Total += Format(Anything) + '/' + Format(Anything.IsInteger()) + '|';
        exit(Total);
    end;

    procedure ForeachQueue(): Text
    var
        Waiting: DotNet DnQueue;
        Piece: Text;
        Total: Text;
    begin
        Waiting := Waiting.Queue();
        Waiting.Enqueue('x');
        Waiting.Enqueue('y');
        foreach Piece in Waiting do
            Total += Piece + '|';
        exit(Total + Format(Waiting.Count));
    end;

    procedure ForeachBreak(): Integer
    var
        Numbers: DotNet DnArrayList;
        Number: Integer;
        Seen: Integer;
    begin
        Numbers := Numbers.ArrayList();
        Numbers.Add(1);
        Numbers.Add(2);
        Numbers.Add(3);
        foreach Number in Numbers do begin
            Seen += 1;
            if Number = 2 then
                break;
        end;
        exit(Seen);
    end;

    procedure ForeachTextOverInts(): Text
    var
        Numbers: DotNet DnArrayList;
        Piece: Text;
    begin
        Numbers := Numbers.ArrayList();
        Numbers.Add(1);
        asserterror foreach Piece in Numbers do;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ForeachModified(): Text
    var
        Numbers: DotNet DnArrayList;
        Number: Integer;
        Count: Integer;
    begin
        Numbers := Numbers.ArrayList();
        Numbers.Add(1);
        Numbers.Add(2);
        asserterror foreach Number in Numbers do begin
            Count += 1;
            if Count = 1 then
                Numbers.Add(3);
        end;
        exit(Format(Count) + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ForeachNull(): Text
    var
        Missing: DotNet DnArrayList;
        Number: Integer;
    begin
        asserterror foreach Number in Missing do;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ForeachDateOverText(): Text
    var
        Texts: DotNet DnArrayList;
        Day: Date;
    begin
        Texts := Texts.ArrayList();
        Texts.Add('text');
        asserterror foreach Day in Texts do;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ToInt(Value: Text): Integer
    var
        Convert: DotNet DnConvert;
    begin
        exit(Convert.ToInt32(Value) + 0);
    end;
}
"#;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn host() -> Option<Arc<SidecarHost>> {
    let h = Arc::new(SidecarHost::new(SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        ..Default::default()
    }));
    match h.availability() {
        Ok(()) => Some(h),
        Err(why) => {
            assert!(
                std::env::var("ALRT_REQUIRE_DOTNET").is_err(),
                "ALRT_REQUIRE_DOTNET is set but .NET is unavailable: {why}"
            );
            eprintln!("SKIPPED (no usable .NET): DotNet {why}");
            None
        }
    }
}

struct Fixture {
    program: std::sync::Arc<al_ir::Program>,
    blocked: Vec<Vec<Option<String>>>,
    oi: usize,
}

fn fixture() -> Fixture {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), SRC.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    Fixture {
        program: l.program,
        blocked,
        oi,
    }
}

impl Fixture {
    fn session(&self, host: Option<Arc<dyn DotNetHost>>) -> Session {
        let mut cfg = DotNetConfig::default();
        if let Some(h) = host {
            cfg.host = h;
        }
        Session::new(Engine::with_dotnet(self.program.clone(), self.blocked.clone(), cfg))
    }

    fn call(&self, s: &mut Session, name: &str, args: Vec<Value>) -> Result<Value, Signal> {
        let ri = self.program.objects[self.oi]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap_or_else(|| panic!("no routine {name}"));
        if let Some(Some(r)) = self.blocked[self.oi].get(ri) {
            return Err(Signal::Unsupported(format!("lowering: {r}")));
        }
        let inst = s.instance(self.oi);
        s.call(self.oi, ri, inst, args).map(|(v, _)| v)
    }
}

fn text(v: Result<Value, Signal>) -> String {
    match v {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn why(v: Result<Value, Signal>) -> String {
    match v {
        Err(Signal::Unsupported(s)) => s,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn the_fixture_lowers_without_any_refusal() {
    let f = fixture();
    for (i, r) in f.program.objects[f.oi].routines.iter().enumerate() {
        assert!(
            f.blocked[f.oi][i].is_none(),
            "{} is refused statically: {:?}",
            r.name,
            f.blocked[f.oi][i]
        );
    }
}

fn run(name: &str) -> Option<Result<Value, Signal>> {
    let h = host()?;
    let f = fixture();
    let mut s = f.session(Some(h));
    Some(f.call(&mut s, name, vec![]))
}

fn int(name: &str) -> i32 {
    match run(name) {
        None => i32::MIN,
        Some(Ok(Value::Integer(i))) => i,
        other => panic!("{name}: {other:?}"),
    }
}

fn boolean(name: &str) -> Option<bool> {
    match run(name)? {
        Ok(Value::Boolean(b)) => Some(b),
        other => panic!("{name}: {other:?}"),
    }
}

fn txt(name: &str) -> Option<String> {
    Some(text(run(name)?))
}

fn refusal(name: &str) -> Option<String> {
    Some(why(run(name)?))
}

#[test]
fn integer_properties_take_part_in_integer_expressions() {
    assert!(matches!(int("LengthMinusOne"), 3 | i32::MIN));
    assert!(matches!(int("ForBound"), 3 | i32::MIN));
    assert!(matches!(int("HalfLength"), 2 | i32::MIN));
}

#[test]
fn a_dotnet_string_compares_case_sensitively_with_a_text_literal() {
    assert!(matches!(boolean("SchemeIsUpper"), Some(false) | None));
    assert!(matches!(boolean("SchemeIsLower"), Some(true) | None));
}

#[test]
fn a_null_string_is_empty_in_expressions_and_when_assigned() {
    assert!(matches!(boolean("NullStringEqualsEmpty"), Some(true) | None));
    assert!(matches!(txt("NullStringPlusX").as_deref(), Some("x") | None));
    assert!(matches!(txt("NullStringFormat").as_deref(), Some("") | None));
    assert!(matches!(int("NullStringAssignedToText"), 0 | i32::MIN));
}

#[test]
fn strlen_of_a_null_string_is_refused_because_bc_raises_an_uncatchable_error() {
    if let Some(w) = refusal("StrLenOfNullString") {
        assert!(w.contains("asserterror cannot catch"), "{w}");
    }
    if let Some(w) = refusal("FormatOfNullObject") {
        assert!(w.contains("null .NET object"), "{w}");
    }
}

#[test]
fn format_of_a_dotnet_object_is_its_to_string() {
    assert!(matches!(
        txt("FormatOfType").as_deref(),
        Some("System.Text.StringBuilder") | None
    ));
    assert!(matches!(txt("FormatOfEnum").as_deref(), Some("Number") | None));
    assert!(matches!(txt("FormatOfBuilder").as_deref(), Some("hello") | None));
}

#[test]
fn enums_add_as_numbers_and_compare() {
    assert!(matches!(txt("EnumSum").as_deref(), Some("367") | None));
    assert!(matches!(txt("EnumEquality").as_deref(), Some("YesNoYes") | None));
}

#[test]
fn an_object_as_an_operand_is_refused_naming_its_clr_type() {
    if let Some(w) = refusal("ObjectAsOperand") {
        assert!(w.contains("System.Text.StringBuilder") && w.contains("operand"), "{w}");
    }
}

#[test]
fn datetimeoffset_now_reads_the_host_clock() {
    // The default host clock is 2024-01-15T12:00:00.000Z.
    assert!(matches!(int("OffsetNowYear"), 2024 | i32::MIN));
    assert!(matches!(int("OffsetNowUtcHour"), 12 | i32::MIN));
    assert!(matches!(int("OffsetNowMillisecond"), 0 | i32::MIN));
    assert!(matches!(boolean("OffsetIsZero"), Some(true) | None));
}

#[test]
fn datetime_now_today_and_datetimeoffset_local_time_agree_with_current_datetime() {
    assert!(matches!(boolean("DateTimeNowIsTheClock"), Some(true) | None));
    assert!(matches!(boolean("DateTimeTodayIsToday"), Some(true) | None));
    assert!(matches!(boolean("LocalMinusOffsetEqualsClock"), Some(true) | None));
}

#[test]
fn copystream_into_a_memorystream_copies_all_bytes_and_leaves_the_position_at_the_end() {
    assert!(matches!(txt("CopyIntoMemory").as_deref(), Some("11:11:Yes") | None));
    assert!(matches!(
        txt("CopyIntoMemoryAfterOneByteRead").as_deref(),
        Some("10") | None
    ));
}

#[test]
fn copystream_out_of_a_memorystream_copies_the_whole_stream_or_the_count_from_the_position() {
    assert!(matches!(
        txt("CopyOutWholeFromPositionSix").as_deref(),
        Some("hello world|0") | None
    ));
    assert!(matches!(txt("CopyOutWithACount").as_deref(), Some("hello|5") | None));
}

#[test]
fn copystream_with_a_count_into_a_dotnet_stream_is_refused() {
    if let Some(w) = refusal("CopyIntoMemoryWithACountIsRefused") {
        assert!(w.contains("CopyStream") && w.contains("not pinned"), "{w}");
    }
}

#[test]
fn foreach_over_clr_collections() {
    assert!(matches!(txt("ForeachStrings").as_deref(), Some("a|b|c|") | None));
    assert!(matches!(txt("ForeachInts").as_deref(), Some("1|2|3|") | None));
    assert!(matches!(txt("ForeachVariant").as_deref(), Some("1/Yes|2/Yes|") | None));
    // The Queue is not consumed.
    assert!(matches!(txt("ForeachQueue").as_deref(), Some("x|y|2") | None));
    assert!(matches!(int("ForeachBreak"), 2 | i32::MIN));
}

#[test]
fn foreach_failures_have_their_pinned_texts_and_codes() {
    let want = [
        // The newest major: the BC28PLUS text (al-language-onprem-dotnet/TestDotNetValueKindsContracts.al:299-306).
        (
            "ForeachTextOverInts",
            "The value \"1\" can't be evaluated into type Text.|Evaluate:Argument",
        ),
        (
            "ForeachModified",
            "1|A call to System.Collections.ArrayList+ArrayListEnumeratorSimple.MoveNext failed with this message: Collection was modified; enumeration operation may not execute.|DotNetInvoke:InvalidOperation",
        ),
        (
            "ForeachNull",
            "A DotNet variable has not been instantiated. Attempting to call System.Collections.ArrayList.GetEnumerator in CodeUnit C: ForeachNull|DotNetVariableNotCreated",
        ),
    ];
    for (name, text) in want {
        if let Some(got) = txt(name) {
            assert_eq!(got, text, "{name}");
        }
    }
    // On BC 28 and later the corpus pins only the start of the Date-over-text error, so it is refused.
    if let Some(r) = refusal("ForeachDateOverText") {
        assert!(r.contains("foreach of a Date over a .NET text"), "{r}");
    }
}
