//! Values and builtins the corpus pinned and the runtime got wrong (branch lane-values-builtins). Each test cites its case.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Ok(Value::Integer(i)) => i.to_string(),
        Ok(Value::Boolean(b)) => b.to_string(),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    out(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn last_index_of_on_an_empty_text_takes_start_zero_and_one() {
    // text/text-methods/Search1.Codeunit.al:351,353,355 (LastIndexOf_StartIndex_Empty).
    let r = text(
        "E: Text;",
        "exit(Format(E.LastIndexOf('', 1)) + ',' + Format(E.LastIndexOf('a', 1)) + ',' + Format(E.LastIndexOf('a', 0)));",
    );
    assert_eq!(r, "1,0,0");
}

const NUM_ROW: &str = r#"
table 50110 "Num Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(5; "Code Value"; Code[20]) { }
        field(6; "Short Code"; Code[2]) { }
        field(7; "Text Value"; Text[20]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50111 C
{
    [TryFunction]
    local procedure TrySetValue(var RR: RecordRef; FieldNo: Integer; V: Variant)
    begin
        RR.Field(FieldNo).Value := V;
    end;

    procedure F(): Text
    var
        Row: Record "Num Row";
        RR: RecordRef;
        R: Text;
    begin
        Row."No." := 1;
        Row.Insert();
        RR.Open(Database::"Num Row");
        RR.FindFirst();
        R := Format(TrySetValue(RR, 5, 10)) + ':' + Format(RR.Field(5).Value());
        R += ',' + Format(TrySetValue(RR, 5, 2.5)) + ':' + Format(RR.Field(5).Value());
        R += ',' + Format(TrySetValue(RR, 7, -3)) + ':' + Format(RR.Field(7).Value());
        R += ',' + Format(TrySetValue(RR, 6, 12345)) + ':' + Format(RR.Field(6).Value());
        exit(R);
    end;
}
"#;

#[test]
fn fieldref_value_takes_a_number_into_a_code_or_text_field_as_its_format() {
    // format-evaluate/NumberToTextIsFormat.al:239-246 (FieldRefSetValue_NumberOnCodeAndText_Result).
    assert_eq!(out(al_testkit::run_in(NUM_ROW, "C", "F")), "Yes:10,Yes:2.5,Yes:-3,No:");
}

#[test]
fn session_settings_of_a_fresh_instance_formats_every_property() {
    // session-system/session/SessionSettingsFreshReads.al:57 (P161_FreshInstance_FormatText).
    let r = text("S: SessionSettings;", "exit(Format(S));");
    assert_eq!(
        r,
        "Company=, ProfileId=, LanguageId=0, LocaleId=0, TimeZone=, ProfileAppId={00000000-0000-0000-0000-000000000000}, ProfileSystemScope=False"
    );
}

const READ_AS: &str = r#"
codeunit 50120 C
{
    [TryFunction]
    local procedure TryReadAs(var Content: HttpContent; var Result: Text)
    begin
        Content.ReadAs(Result);
    end;

    procedure F(): Text
    var
        Req: HttpRequestMessage;
        Content: HttpContent;
        Result: Text;
    begin
        Content := Req.Content;
        if TryReadAs(Content, Result) then
            exit('ok:[' + Result + ']');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

#[test]
fn read_as_of_an_uninitialized_content_as_a_statement_raises() {
    // json/W4XmlJsonHttp.al:343 (RequestWithoutContent_ReadAs).
    assert_eq!(
        out(al_testkit::run_in(READ_AS, "C", "F")),
        "error:Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.|NotInitialized"
    );
}

#[test]
fn an_interface_assigned_from_an_unassigned_one_raises() {
    // codeunits/interfaces/OjT71047.al:250-251 (Uninitialized_AssignedFromUninitialized, af224).
    let src = r#"
interface IShape { procedure GetArea(): Integer; }
codeunit 50130 Sq implements IShape { procedure GetArea(): Integer begin exit(4); end; }
codeunit 50131 C
{
    procedure F(): Text
    var
        A: Interface IShape;
        B: Interface IShape;
    begin
        B := A;
        exit('no error');
    end;
}
"#;
    assert_eq!(
        out(al_testkit::run_in(src, "C", "F")),
        "error:Interface not initialized|InvalidOperation"
    );
}

#[test]
fn a_variant_beyond_integer_into_an_integer_raises_a_type_conversion_overflow() {
    // types/duration/D_Duration.al:226 (a Duration); types/variant/VaTConvert.al:183-184
    // (a BigInteger).
    let msg = "error:The expression Variant cannot be type-converted to a Integer value.|TypeConversion:Overflow";
    let r = text(
        "V: Variant; B: BigInteger; D: Duration; I: Integer;",
        "Evaluate(B, '5000000000'); D := B; V := D; I := V; exit(Format(I));",
    );
    assert_eq!(r, msg);
    let r = text(
        "V: Variant; B: BigInteger; I: Integer;",
        "Evaluate(B, '3000000000'); V := B; I := V; exit(Format(I));",
    );
    assert_eq!(r, msg);
    // In range the value converts (VaTConvert.al:181).
    let r = text(
        "V: Variant; B: BigInteger; I: Integer;",
        "B := 5; V := B; I := V; exit(Format(I));",
    );
    assert_eq!(r, "5");
}

const CURR: &str = r#"
table 50140 "Curr Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal)
        {
            trigger OnValidate()
            begin
                SeenInValidate := CurrFieldNo;
                SetN(CurrFieldNo);
                SeenAfterInValidate := CurrFieldNo;
            end;
        }
        field(3; Note; Text[50]) { }
    }
    keys { key(PK; Id) { } }
    var
        SeenInValidate: Integer;
        SeenAfterInValidate: Integer;

    local procedure SetN(var N: Integer)
    begin
        N := 99;
    end;

    local procedure SetNAndPassOn(var N: Integer)
    begin
        SetN(N);
    end;

    procedure PassAndAssign(): Text
    var
        Before: Integer;
        After: Integer;
    begin
        Before := CurrFieldNo;
        SetN(CurrFieldNo);
        After := CurrFieldNo;
        exit(Format(Before) + '>' + Format(After));
    end;

    procedure PassTwoLevels(): Text
    begin
        SetNAndPassOn(CurrFieldNo);
        exit(Format(CurrFieldNo));
    end;

    procedure ValidateAmountSeen(): Text
    begin
        CurrFieldNo := 0;
        Validate(Amount, 5);
        exit(Format(SeenInValidate) + '/' + Format(SeenAfterInValidate) + '/' + Format(CurrFieldNo));
    end;

    procedure CopyThenAssign(): Text
    var
        CopyVar: Integer;
    begin
        CopyVar := CurrFieldNo;
        SetN(CopyVar);
        exit(Format(CopyVar) + '/' + Format(CurrFieldNo));
    end;
}
codeunit 50141 C
{
    procedure Pass(): Text
    var
        Row: Record "Curr Row";
    begin
        exit(Row.PassAndAssign() + ',' + Row.PassAndAssign());
    end;

    procedure TwoLevels(): Text
    var
        Row: Record "Curr Row";
    begin
        exit(Row.PassTwoLevels());
    end;

    procedure InValidate(): Text
    var
        Row: Record "Curr Row";
    begin
        exit(Row.ValidateAmountSeen());
    end;

    procedure Copy(): Text
    var
        Row: Record "Curr Row";
    begin
        exit(Row.CopyThenAssign());
    end;
}
"#;

#[test]
fn curr_field_no_passed_to_a_var_parameter_takes_what_the_callee_assigns() {
    // record/curr-field-no/CurrFieldNoVarParam.al:103-106 (CalleeAssigns), :117 (InValidate), :128 (CopyIsNotTheProperty).
    let mut b = al_testkit::Built::from_source(CURR);
    assert_eq!(out(b.run_in("C", "Pass")), "0>99,99>99");
    assert_eq!(out(b.run_in("C", "TwoLevels")), "99");
    assert_eq!(out(b.run_in("C", "InValidate")), "0/99/99");
    assert_eq!(out(b.run_in("C", "Copy")), "99/0");
}

#[test]
fn field_index_lists_flowfields_after_the_other_fields() {
    // metadata/platform-tables/CustomReportLayoutColumns.al:33-44 (Columns_Schema: field 3, a FlowField,
    // comes after field 15).
    let src = r#"
table 50150 "Ff Row"
{
    fields
    {
        field(1; A; Integer) { }
        field(2; B; Integer) { }
        field(3; C; Integer) { FieldClass = FlowField; CalcFormula = count("Ff Row"); }
        field(4; D; Integer) { }
    }
    keys { key(PK; A) { } }
}
codeunit 50151 C
{
    procedure F(): Text
    var
        RR: RecordRef;
        i: Integer;
        R: Text;
    begin
        RR.Open(Database::"Ff Row");
        for i := 1 to RR.FieldCount() do
            R += Format(RR.FieldIndex(i).Number()) + ' ';
        exit(R);
    end;
}
"#;
    assert_eq!(out(al_testkit::run_in(src, "C", "F")), "1 2 4 3 ");
}

const STREAM: &str = r#"
table 50160 "St Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 50161 C
{
    local procedure Put(var Rec: Record "St Row"; Bytes: List of [Integer]; Enc: TextEncoding)
    var
        Out: OutStream;
        B: Byte;
        I: Integer;
    begin
        Rec.DeleteAll();
        Rec.Init();
        Rec.Id := 1;
        Rec.Data.CreateOutStream(Out, Enc);
        foreach I in Bytes do begin
            B := I;
            Out.Write(B);
        end;
        Rec.Insert();
    end;

    procedure Terminated(): Text
    var
        Rec: Record "St Row";
        Ins: InStream;
        T: Text;
        N: Integer;
        R: Text;
        L: List of [Integer];
    begin
        L.AddRange(104, 101, 108, 108, 111, 0, 119, 111, 114, 108, 100, 0);
        Put(Rec, L, TextEncoding::UTF8);
        Rec.CalcFields(Data);
        Rec.Data.CreateInStream(Ins, TextEncoding::UTF8);
        N := Ins.Read(T);
        R := Format(N) + '|' + T;
        N := Ins.Read(T);
        R += ',' + Format(N) + '|' + T;
        N := Ins.Read(T);
        R += ',' + Format(N) + '|' + T;
        exit(R);
    end;

    procedure Unterminated(): Text
    var
        Rec: Record "St Row";
        Ins: InStream;
        T: Text;
        N: Integer;
        R: Text;
        L: List of [Integer];
    begin
        L.AddRange(104, 101, 108, 108, 111);
        Put(Rec, L, TextEncoding::UTF8);
        Rec.CalcFields(Data);
        Rec.Data.CreateInStream(Ins, TextEncoding::UTF8);
        N := Ins.Read(T);
        R := Format(N) + '|' + T;
        N := Ins.Read(T);
        R += ',' + Format(N) + '|' + T;
        exit(R);
    end;
}
"#;

#[test]
fn reading_text_on_a_utf8_stream_counts_one_more_than_the_bytes_and_one_at_the_end() {
    // streams/PnE04Streams.al:86-89 (ReadText_Utf8_WithTerminator), :111-113 (ReadText_Utf8_NoTerminator).
    let mut b = al_testkit::Built::from_source(STREAM);
    assert_eq!(out(b.run_in("C", "Terminated")), "6|hello,6|world,1|");
    assert_eq!(out(b.run_in("C", "Unterminated")), "6|hello,1|");
}

fn run_on(bc: &str, src: &str, routine: &str) -> Result<Value, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn a_header_name_of_blanks_is_an_argument_error_on_bc_28_and_a_format_error_from_29() {
    // http/HttpHeaders.al:304-308 (InvalidNames, `blankName`); the test is BC 28 and later (:282).
    let src = "codeunit 50170 C { procedure F(): Text var H: HttpHeaders; R: HttpRequestMessage; begin R.GetHeaders(H); H.Add(' ', 'v'); end; }";
    let code = |bc: &str| match run_on(bc, src, "F") {
        Err(Signal::Error(e)) => e.code,
        Err(Signal::Unsupported(_)) => "refused".into(),
        other => format!("{other:?}"),
    };
    assert_eq!(code("29.0"), "InvalidHeader:Format");
    assert_eq!(code("28.5"), "InvalidHeader:Argument");
    assert_eq!(code("27.5"), "refused");
}

const MEDIA_EXPORT: &str = r#"
table 50180 "Media Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Pic; Media) { }
        field(3; Src; Blob) { }
        field(4; Dst; Blob) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50181 C
{
    local procedure Import(var Row: Record "Media Row"; Png: Boolean)
    var
        Out: OutStream;
        Ins: InStream;
        B: Byte;
        I: Integer;
        Sig: array[8] of Integer;
    begin
        Sig[1] := 137; Sig[2] := 80; Sig[3] := 78; Sig[4] := 71; Sig[5] := 13; Sig[6] := 10; Sig[7] := 26; Sig[8] := 10;
        Row.Src.CreateOutStream(Out);
        for I := 1 to 8 do begin
            if Png then B := Sig[I] else B := 1;
            Out.Write(B);
        end;
        Row.Src.CreateInStream(Ins);
        Row.Pic.ImportStream(Ins, 'd');
    end;

    procedure ExportOnly(): Text
    var
        Row: Record "Media Row";
        Out: OutStream;
    begin
        Import(Row, true);
        Row.Dst.CreateOutStream(Out);
        Row.Pic.ExportStream(Out);
        exit('exported');
    end;

    procedure ExportLength(): Text
    var
        Row: Record "Media Row";
        Out: OutStream;
    begin
        Import(Row, true);
        Row.Dst.CreateOutStream(Out);
        Row.Pic.ExportStream(Out);
        exit(Format(Row.Dst.Length));
    end;

    procedure ExportLengthNoPng(): Text
    var
        Row: Record "Media Row";
        Out: OutStream;
    begin
        Import(Row, false);
        Row.Dst.CreateOutStream(Out);
        Row.Pic.ExportStream(Out);
        exit(Format(Row.Dst.Length));
    end;
}
"#;

#[test]
fn the_bytes_of_an_exported_media_are_refused_on_every_major() {
    // types/W7Media.al:141-145 (ExportStream_OfAnImportedPicture, corpus bump 34): Windows BC re-encodes the imported 68-byte PNG
    // to 99 bytes on 27, 28 and 29 (bc-linux #161), bytes the corpus does not pin. The export itself succeeds
    // (streams/StTMedia.al:170, MediaExportStream), so only a look at the exported bytes is refused.
    // ExportLengthNoPng imports control bytes, no picture: they export unchanged on every major
    // (streams/media/Y44MediaExportStream.al Media_Binary_ExportStream, corpus PR 962).
    let r = |bc: &str, routine: &str| out(run_on(bc, MEDIA_EXPORT, routine));
    for bc in ["27.5", "28.5", "29.0"] {
        assert_eq!(r(bc, "ExportOnly"), "exported", "{bc}");
        let got = r(bc, "ExportLength");
        assert!(got.starts_with("refused:"), "{bc} ExportLength: {got}");
        assert_eq!(r(bc, "ExportLengthNoPng"), "8", "{bc}");
    }
}

const BLOB_LENGTH: &str = r#"
table 50190 "Blob Row"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Code) { } }
}
codeunit 50191 C
{
    local procedure Unloaded(N: Integer): Text
    var
        Rec: Record "Blob Row";
        Got: Record "Blob Row";
        OutS: OutStream;
    begin
        Rec.DeleteAll();
        Rec.Code := 'K';
        Rec.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', N, 'x'));
        Rec.Insert();
        Got.Get('K');
        exit(Format(Got.Data.Length()));
    end;

    procedure One(): Text begin exit(Unloaded(1)); end;
    procedure Three(): Text begin exit(Unloaded(3)); end;
    procedure Five(): Text begin exit(Unloaded(5)); end;
    procedure Six(): Text begin exit(Unloaded(6)); end;
    procedure Hundred(): Text begin exit(Unloaded(100)); end;
}
"#;

#[test]
fn the_stored_length_of_a_blob_is_given_only_for_the_pinned_sizes() {
    // record/PnABlobLength.al:49-60 (Length_ByStoredSize, BC 29 only): 1 and 2 bytes are n + 6, 100 bytes and more
    // are compressed. streams/TestStreamsRuntimeContracts.al:264: 5 bytes are 11 on every major.
    let r = |bc: &str, routine: &str| out(run_on(bc, BLOB_LENGTH, routine));
    assert_eq!(r("29.0", "One"), "7");
    assert_eq!(r("29.0", "Five"), "11");
    assert_eq!(r("27.5", "Five"), "11");
    // record/flowfields/BlobCalcFieldsLength.al:222 ('new' 9, every major).
    assert_eq!(r("27.5", "Three"), "9");
    // 100 `x` are 10 on BC 29 (PnABlobLength.al:55); on BC 27 and 28 that content is not measured.
    assert_eq!(r("29.0", "Hundred"), "10");
    // Six bytes are pinned only as the text 'stored' (:350): six repeated bytes compress.
    for (bc, routine) in [("27.5", "One"), ("28.5", "One"), ("29.0", "Six"), ("27.5", "Hundred")] {
        assert!(
            r(bc, routine).starts_with("refused:"),
            "{bc} {routine}: {}",
            r(bc, routine)
        );
    }
}

#[test]
fn a_textconst_going_into_a_variant_is_refused_outside_the_pinned_cells() {
    // types/variant/VaTTextConst.al:144-151 (TextConst_In_Variant) and :287-288: a global TextConst assigned to a local Variant
    // or handed to a Variant parameter of a local procedure holds a TextConstant, which Format reads as its text
    // (al-sema lower/label_variant.rs). As the exit value of a Variant function it is not pinned and stays refused. A Label
    // formats as its text in all three.
    let src = |decl: &str, body: &str| {
        format!(
            "codeunit 50200 C {{ var Simple: {decl}; local procedure Take(V: Variant): Text begin exit(Format(V)); end; \
             procedure Give(): Variant begin exit(Simple); end; \
             procedure F(): Text var V: Variant; begin {body} end; }}"
        )
    };
    for (body, pinned) in [
        ("V := Simple; exit(Format(V));", true),
        ("exit(Take(Simple));", true),
        ("V := Give(); exit(Format(V));", false),
    ] {
        let r = out(al_testkit::run_in(&src("TextConst ENU = 'Hello'", body), "C", "F"));
        if pinned {
            assert_eq!(r, "Hello", "{body}");
        } else {
            assert!(r.starts_with("refused:") && r.contains("TextConst"), "{body}: {r}");
        }
        assert_eq!(
            out(al_testkit::run_in(&src("Label 'Hello'", body), "C", "F")),
            "Hello",
            "{body}"
        );
    }
    // A TextConst read as a text stays supported.
    let r = out(al_testkit::run_in(
        &src("TextConst ENU = 'Hello'", "exit(Simple);"),
        "C",
        "F",
    ));
    assert_eq!(r, "Hello");
}

const MEDIA_OWN_BLOB: &str = r#"
table 50210 "Media Own Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Pic; Media) { }
        field(3; Blob; Blob) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 50211 C
{
    local procedure Fill(var Rec: Record "Media Own Row")
    var
        Out: OutStream;
        B: Byte;
    begin
        Rec.Blob.CreateOutStream(Out);
        B := 65;
        Out.Write(B);
    end;

    procedure OwnBlob(): Text
    var
        Rec: Record "Media Own Row";
        Ins: InStream;
    begin
        Rec.Id := 1;
        Fill(Rec);
        Rec.Insert();
        Rec.CalcFields(Blob);
        Rec.Blob.CreateInStream(Ins);
        Rec.Pic.ImportStream(Ins, 'desc', 'image/png');
        exit(Format(Rec.Pic.HasValue()));
    end;

    procedure OwnBlobAfterModify(): Text
    var
        Rec: Record "Media Own Row";
        Ins: InStream;
    begin
        Rec.Id := 1;
        Rec.Insert();
        Fill(Rec);
        Rec.Modify();
        Rec.CalcFields(Blob);
        Rec.Blob.CreateInStream(Ins);
        Rec.Pic.ImportStream(Ins, 'desc', 'image/png');
        exit(Format(Rec.Pic.HasValue()));
    end;

    procedure OtherRecordsBlob(): Text
    var
        Rec: Record "Media Own Row";
        Src: Record "Media Own Row";
        Ins: InStream;
    begin
        Fill(Src);
        Src.Blob.CreateInStream(Ins);
        Rec.Id := 1;
        Rec.Pic.ImportStream(Ins, 'desc');
        exit(Format(Rec.Pic.HasValue()));
    end;
}
"#;

#[test]
fn a_media_imported_from_a_blob_of_its_own_record_is_refused() {
    // streams/PnE06Media.al:66-84 (ImportStream_ThreeArguments_ForComparison): a stream on the record's own Blob
    // field leaves the Media field without a value and writes no Tenant Media row. Why is not pinned, so that import is refused.
    let r = out(al_testkit::run_in(MEDIA_OWN_BLOB, "C", "OwnBlob"));
    assert!(r.starts_with("refused:"), "{r}");
    // The row inserted first, the Blob written and the row modified: the import works (streams/media/P102Media.al:24-41).
    assert_eq!(
        out(al_testkit::run_in(MEDIA_OWN_BLOB, "C", "OwnBlobAfterModify")),
        "Yes"
    );
    // A stream on another record's Blob imports as before (streams/StTMedia.al:166-167 imports from a Temp Blob).
    assert_eq!(out(al_testkit::run_in(MEDIA_OWN_BLOB, "C", "OtherRecordsBlob")), "Yes");
}

const READ_AS_SHAPES: &str = r#"
codeunit 50220 C
{
    procedure DirectUsed(): Text
    var
        Req: HttpRequestMessage;
        Txt: Text;
    begin
        Txt := 'preset';
        exit(Format(Req.Content.ReadAs(Txt)) + '|' + Txt);
    end;

    procedure DirectStatement(): Text
    var
        Req: HttpRequestMessage;
        Txt: Text;
    begin
        Req.Content.ReadAs(Txt);
        exit('ran');
    end;

    procedure StreamUsed(): Text
    var
        Req: HttpRequestMessage;
        Ins: InStream;
    begin
        exit(Format(Req.Content.ReadAs(Ins)));
    end;

    procedure CopyUsed(): Text
    var
        Req: HttpRequestMessage;
        Content: HttpContent;
        Txt: Text;
    begin
        Content := Req.Content;
        Txt := 'kept';
        exit(Format(Content.ReadAs(Txt)) + '|' + Txt);
    end;

    procedure StreamStatement(): Text
    var
        Req: HttpRequestMessage;
        Ins: InStream;
    begin
        Req.Content.ReadAs(Ins);
        exit('ran');
    end;

    procedure CopyStreamUsed(): Text
    var
        Req: HttpRequestMessage;
        Content: HttpContent;
        Ins: InStream;
    begin
        Content := Req.Content;
        exit(Format(Content.ReadAs(Ins)));
    end;

    procedure CopyStreamStatement(): Text
    var
        Req: HttpRequestMessage;
        Content: HttpContent;
        Ins: InStream;
    begin
        Content := Req.Content;
        Content.ReadAs(Ins);
        exit('ran');
    end;

    procedure StreamUsedIntoAUsedStream(): Text
    var
        Req: HttpRequestMessage;
        Other: HttpContent;
        Ins: InStream;
    begin
        Other.WriteFrom('old');
        Other.ReadAs(Ins);
        exit(Format(Req.Content.ReadAs(Ins)));
    end;

    procedure CopyGetHeadersUsed(): Text
    var
        Req: HttpRequestMessage;
        Content: HttpContent;
        Headers: HttpHeaders;
    begin
        Content := Req.Content;
        exit(Format(Content.GetHeaders(Headers)));
    end;
}
"#;

#[test]
fn read_as_of_an_uninitialized_content_runs_the_pinned_shapes() {
    // http/HttpRequest.al:220 (FreshRequestContent): `Req.Content.ReadAs(Txt)` with its result used answers No and leaves
    // the text. http/PnE12HttpContent.al:192: the same call as a statement raises. json/W4XmlJsonHttp.al:342-343
    // (RequestWithoutContent_ReadAs): a copy in a variable, called as a statement, raises
    // (read_as_of_an_uninitialized_content_as_a_statement_raises).
    let r = |routine: &str| out(al_testkit::run_in(READ_AS_SHAPES, "C", routine));
    let not_initialized =
        "error:Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.|NotInitialized";
    assert_eq!(r("DirectUsed"), "No|preset");
    assert_eq!(r("DirectStatement"), not_initialized);
    // http/PnE12HttpContent.al:201-273 (PW_RequestWithoutContent_CopyAndStream, al-runner issue 7466): a copy with the
    // result used answers No and leaves the text (`copyTextUsed`); ReadAs into an InStream with the result used answers No
    // on the request's content (`streamUsed`) and on a copy (`copyStreamUsed`), and raises as a statement on the request's
    // content (`streamStatement`). This test expected a refusal for CopyUsed and StreamUsed before that probe existed.
    assert_eq!(r("CopyUsed"), "No|kept");
    assert_eq!(r("StreamUsed"), "No");
    assert_eq!(r("CopyStreamUsed"), "No");
    assert_eq!(r("StreamStatement"), not_initialized);
    // Not pinned: ReadAs(InStream) as a statement on a copy, GetHeaders on a copy with the result used, and ReadAs(InStream)
    // into a stream variable that already holds a stream (the old stream would stay readable).
    assert!(
        r("StreamUsedIntoAUsedStream").starts_with("refused:"),
        "{}",
        r("StreamUsedIntoAUsedStream")
    );
    assert!(
        r("CopyStreamStatement").starts_with("refused:"),
        "{}",
        r("CopyStreamStatement")
    );
    assert!(
        r("CopyGetHeadersUsed").starts_with("refused:"),
        "{}",
        r("CopyGetHeadersUsed")
    );
}
