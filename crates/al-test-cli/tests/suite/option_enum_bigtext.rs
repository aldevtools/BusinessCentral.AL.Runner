//! Option to Enum and `BigText.Read` as the probes of corpus tests/al-language/types/PEOptionEnumBigText.al
//! pin them (written against a real BC 27.5 service tier; corpus branch `master`). The procedures
//! below are those probes with the observer replaced by `Error` on a difference; each cites its probe.

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
enum 69170 \"PE Kind\" { Extensible = true; AssignmentCompatibility = true; value(0; None) { } value(1; One) { } value(7; Seven) { } value(9; Nine) { } }
enum 69171 \"PE Plain Kind\" { Extensible = true; value(0; None) { } value(1; One) { } value(7; Seven) { } value(9; Nine) { } }
table 69170 \"PE Usage Row\" { fields { field(1; \"Code\"; Code[20]) { } field(2; Usage; Enum \"PE Kind\") { } field(3; Plain; Enum \"PE Plain Kind\") { } field(4; Info; Text[50]) { } field(5; Data; Blob) { } }
  keys { key(PK; \"Code\", Usage) { Clustered = true; } } }
";

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS} codeunit 69172 T {{ Subtype = Test; {tests} }}");
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(tests: &str) {
    for (name, outcome) in run(tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// PEOptionEnumBigText.al:68-97 (OptionToEnum_ByOrdinal), :100-122 (the ordinal the enum lacks), :125-141 (blank member),
// :171-184 (without AssignmentCompatibility).
#[test]
fn an_option_converts_to_an_enum_by_its_ordinal() {
    all_pass(
        "[Test] procedure ByOrdinal() var Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven,Eight,Nine; K: Enum \"PE Kind\"; begin
             Opt := Opt::One; K := Opt;
             if Format(K) <> 'One' then Error('one %1', Format(K)); if K.AsInteger() <> 1 then Error('one ordinal');
             Opt := Opt::Seven; K := Opt;
             if Format(K) <> 'Seven' then Error('seven'); if K.AsInteger() <> 7 then Error('seven ordinal');
             Opt := Opt::Nine; K := Opt; if Format(K) <> 'Nine' then Error('nine');
             Opt := Opt::None; K := Opt; if Format(K) <> 'None' then Error('zero'); if K.AsInteger() <> 0 then Error('zero ordinal');
         end;
         [Test] procedure OrdinalTheEnumLacks() var Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven,Eight,Nine; K: Enum \"PE Kind\"; begin
             K := K::Seven; Opt := Opt::Two; K := Opt;
             if '[' + Format(K) + ']' <> '[2]' then Error('two %1', Format(K)); if K.AsInteger() <> 2 then Error('two ordinal');
             Opt := Opt::Eight; K := Opt;
             if '[' + Format(K) + ']' <> '[8]' then Error('eight'); if K.AsInteger() <> 8 then Error('eight ordinal');
             if K = K::None then Error('eight is none');
         end;
         [Test] procedure BlankMember() var Opt: Option \" \",One; K: Enum \"PE Kind\"; begin
             K := K::Seven; Opt := Opt::\" \"; K := Opt;
             if Format(K) <> 'None' then Error('blank'); if K.AsInteger() <> 0 then Error('blank ordinal');
         end;
         [Test] procedure PlainEnum() var Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; K: Enum \"PE Plain Kind\"; begin
             Opt := Opt::Seven; K := Opt; if Format(K) <> 'Seven' then Error('plain'); if K.AsInteger() <> 7 then Error('plain ordinal');
         end;",
    );
}

// PEOptionEnumBigText.al:144-168 (Evaluate), :187-205 (field assignment, Validate), :208-222 (argument).
#[test]
fn an_option_reaches_an_enum_through_evaluate_a_field_and_an_argument() {
    all_pass(
        "[Test] procedure EvaluateForms() var K: Enum \"PE Kind\"; Opt: Option \"None\",One,Two; begin
             Opt := Opt::One;
             if not Evaluate(K, Format(Opt)) then Error('evaluate option text'); if Format(K) <> 'One' then Error('option text value');
             if not Evaluate(K, '1') then Error('evaluate ordinal'); if Format(K) <> 'One' then Error('ordinal text value');
             if not Evaluate(K, 'Seven') then Error('evaluate name'); if K.AsInteger() <> 7 then Error('name value');
         end;
         [Test] procedure FieldAndValidate() var Row: Record \"PE Usage Row\"; Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; begin
             Opt := Opt::Seven;
             Row.Usage := Opt; if Format(Row.Usage) <> 'Seven' then Error('field');
             Row.Plain := Opt; if Format(Row.Plain) <> 'Seven' then Error('plain field');
             Row.Usage := Row.Usage::None; Row.Validate(Usage, Opt); if Format(Row.Usage) <> 'Seven' then Error('validate');
         end;
         [Test] procedure Argument() var Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; begin
             Opt := Opt::Seven; if TakeEnum(Opt) <> 'Seven/7' then Error('argument %1', TakeEnum(Opt));
         end;
         local procedure TakeEnum(K: Enum \"PE Kind\"): Text begin exit(Format(K) + '/' + Format(K.AsInteger())); end;",
    );
}

// PEOptionEnumBigText.al:225-259 (Get with an Option on an Enum key), :262-282 (SetRange, SetFilter),
// :285-305 (FieldRef.Value).
#[test]
fn an_option_works_as_a_key_a_filter_and_a_field_ref_value() {
    all_pass(
        "[Test] procedure GetByOption() var Row: Record \"PE Usage Row\"; Found: Record \"PE Usage Row\"; Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; begin
             Row.DeleteAll(); Row.Code := 'X'; Row.Usage := Row.Usage::Seven; Row.Info := 'seven'; Row.Insert();
             Row.Usage := Row.Usage::One; Row.Info := 'one'; Row.Insert();
             Opt := Opt::Seven; if not Found.Get('X', Opt) then Error('seven'); if Found.Info <> 'seven' then Error('seven info');
             Opt := Opt::One; if not Found.Get('X', Opt) then Error('one'); if Found.Info <> 'one' then Error('one info');
             Opt := Opt::Three; asserterror Found.Get('X', Opt);
             if GetLastErrorText() <> 'The PE Usage Row does not exist. Identification fields and values: Code=''X'',Usage=''Three''' then Error('text %1', GetLastErrorText());
             if GetLastErrorCode() <> 'DB:RecordNotFound' then Error('code %1', GetLastErrorCode());
             Opt := Opt::Two; if Found.Get('X', Opt) then Error('two found');
         end;
         [Test] procedure RangeAndFilter() var Row: Record \"PE Usage Row\"; Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; begin
             Row.DeleteAll(); Row.Code := 'Y'; Row.Usage := Row.Usage::Seven; Row.Insert();
             Opt := Opt::Seven; Row.SetRange(Usage, Opt);
             if Row.Count() <> 1 then Error('range count'); if Row.GetFilter(Usage) <> 'Seven' then Error('range filter %1', Row.GetFilter(Usage));
             Row.SetFilter(Usage, '%1', Opt); if Row.Count() <> 1 then Error('filter count');
         end;
         [Test] procedure FieldRefValue() var Row: Record \"PE Usage Row\"; RecRef: RecordRef; FldRef: FieldRef; Opt: Option \"None\",One,Two,Three,Four,Five,Six,Seven; begin
             RecRef.GetTable(Row); FldRef := RecRef.Field(Row.FieldNo(Usage)); Opt := Opt::Seven; FldRef.Value := Opt;
             if Format(FldRef.Value) <> 'Seven' then Error('fieldref value %1', Format(FldRef.Value));
             if Format(Row.Usage) <> 'None' then Error('row untouched'); RecRef.SetTable(Row);
             if Format(Row.Usage) <> 'Seven' then Error('after settable');
         end;",
    );
}

fn bytes(bs: &[i32]) -> String {
    // A procedure body that writes the bytes (a negative value is skipped) to the Blob field of `R`.
    let mut s = String::from("R.Data.CreateOutStream(Out); ");
    for b in bs {
        s.push_str(&format!("B := {b}; Out.Write(B); "));
    }
    s
}

fn read_case(name: &str, bs: &[i32], enc: &str, length: i32, codes: &str) -> String {
    format!(
        "[Test] procedure {name}() var R: Record \"PE Usage Row\"; Out: OutStream; In1: InStream; B: Byte; Big: BigText; Sub: Text; i: Integer; C: Char; V: Integer; Res: Text; begin
             {bytes} R.Data.CreateInStream(In1{enc});
             if not Big.Read(In1) then Error('return'); if Big.Length() <> {length} then Error('length %1', Big.Length());
             if Big.Length() > 0 then Big.GetSubText(Sub, 1, Big.Length());
             for i := 1 to StrLen(Sub) do begin C := Sub[i]; V := C; Res += Format(V) + ' '; end;
             if Res <> '{codes}' then Error('codes %1', Res);
         end;",
        bytes = bytes(bs)
    )
}

// PEOptionEnumBigText.al:336-357 (UTF8), :360-381 (UTF8 with a byte order mark), :384-403 (default stream reads
// code page 437), :406-440 (UTF16 with and without a mark), :443-460 (Windows), :499-515 (CR LF kept).
#[test]
fn big_text_read_decodes_by_the_encoding_of_the_stream() {
    let t = [
        read_case(
            "Utf8",
            &[104, 195, 169, 226, 130, 172],
            ", TextEncoding::UTF8",
            3,
            "104 233 8364 ",
        ),
        read_case("Utf8Bom", &[239, 187, 191, 97, 98], ", TextEncoding::UTF8", 2, "97 98 "),
        read_case(
            "Default",
            &[104, 195, 169, 226, 130, 172],
            "",
            6,
            "104 8220 20 203 233 188 ",
        ),
        read_case("Utf16", &[97, 0, 98, 0], ", TextEncoding::UTF16", 2, "97 98 "),
        read_case(
            "Utf16Bom",
            &[255, 254, 97, 0, 98, 0],
            ", TextEncoding::UTF16",
            2,
            "97 98 ",
        ),
        read_case("Windows", &[233, 128], ", TextEncoding::Windows", 2, "233 8364 "),
        read_case("CrLf", &[97, 13, 10, 98], ", TextEncoding::UTF8", 4, "97 13 10 98 "),
    ]
    .join(" ");
    all_pass(&t);
}

// PEOptionEnumBigText.al:463-481 (replaces the text), :484-512 (empty stream; a second Read of the drained stream).
#[test]
fn big_text_read_replaces_the_text_and_a_drained_stream_gives_an_empty_one() {
    all_pass(
        "[Test] procedure Replaces() var R: Record \"PE Usage Row\"; Out: OutStream; In1: InStream; B: Byte; Big: BigText; Sub: Text; begin
             Big.AddText('old'); R.Data.CreateOutStream(Out); B := 110; Out.Write(B); B := 101; Out.Write(B); B := 119; Out.Write(B);
             R.Data.CreateInStream(In1, TextEncoding::UTF8);
             if not Big.Read(In1) then Error('return'); if Big.Length() <> 3 then Error('length %1', Big.Length());
             Big.GetSubText(Sub, 1, 3); if Sub <> 'new' then Error('text %1', Sub);
         end;
         [Test] procedure EmptyAndTwice() var R: Record \"PE Usage Row\"; Out: OutStream; In1: InStream; B: Byte; Big: BigText; begin
             R.Data.CreateInStream(In1, TextEncoding::UTF8);
             if not Big.Read(In1) then Error('empty return'); if Big.Length() <> 0 then Error('empty length');
             R.Data.CreateOutStream(Out); B := 97; Out.Write(B); B := 98; Out.Write(B); B := 99; Out.Write(B);
             R.Data.CreateInStream(In1, TextEncoding::UTF8);
             if not Big.Read(In1) then Error('first'); if not Big.Read(In1) then Error('second');
             if Big.Length() <> 0 then Error('second length %1', Big.Length()); if Big.TextPos('c') <> 0 then Error('textpos');
         end;",
    );
}
